//! Microphone capture for live dictation.
//!
//! The cpal stream is not `Send` on macOS, so it lives on a dedicated thread
//! for the lifetime of the recording. Samples are downmixed to mono in the
//! audio callback and kept only in memory.

use crate::error::AppError;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

/// Hard cap on a single live dictation (30 minutes).
pub const MAX_RECORDING_SECONDS: u32 = 30 * 60;

/// How often the level callback fires while recording.
const LEVEL_INTERVAL: Duration = Duration::from_millis(100);

struct Shared {
    samples: Mutex<Vec<f32>>,
    /// Peak absolute amplitude since the last level report, as f32 bits.
    peak: AtomicU32,
    max_samples: usize,
    truncated: AtomicBool,
}

/// An in-progress microphone recording.
pub struct Recorder {
    shared: Arc<Shared>,
    sample_rate: u32,
    stop_tx: mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

/// Captured audio returned by [`Recorder::stop`].
pub struct Recording {
    /// Mono samples at `sample_rate`.
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    /// True when the recording hit [`MAX_RECORDING_SECONDS`] and was cut.
    pub truncated: bool,
}

fn mic_error(msg: impl std::fmt::Display) -> AppError {
    AppError::Dictation(format!("Microphone unavailable: {msg}"))
}

fn push_frames<T>(input: &[T], channels: usize, shared: &Shared)
where
    T: Sample,
    f32: FromSample<T>,
{
    let mut peak = 0.0f32;
    let Ok(mut buf) = shared.samples.lock() else {
        return;
    };
    let room = shared.max_samples.saturating_sub(buf.len());
    if room == 0 {
        shared.truncated.store(true, Ordering::Relaxed);
        return;
    }
    let scale = 1.0 / channels as f32;
    for frame in input.chunks_exact(channels).take(room) {
        let s = frame.iter().map(|s| f32::from_sample(*s)).sum::<f32>() * scale;
        peak = peak.max(s.abs());
        buf.push(s);
    }
    drop(buf);
    let prev = f32::from_bits(shared.peak.load(Ordering::Relaxed));
    if peak > prev {
        shared.peak.store(peak.to_bits(), Ordering::Relaxed);
    }
}

impl Recorder {
    /// Open the default input device and start recording. `on_level` receives
    /// a 0–1 peak level about ten times per second for UI metering.
    pub fn start(on_level: impl Fn(f32) + Send + 'static) -> Result<Self, AppError> {
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(u32, Arc<Shared>), AppError>>();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        let thread = std::thread::Builder::new()
            .name("dictation-capture".into())
            .spawn(move || {
                let setup = || -> Result<(cpal::Stream, u32, Arc<Shared>), AppError> {
                    let host = cpal::default_host();
                    let device = host
                        .default_input_device()
                        .ok_or_else(|| mic_error("no input device found"))?;
                    let config = device.default_input_config().map_err(mic_error)?;
                    let rate = config.sample_rate();
                    let channels = config.channels().max(1) as usize;
                    let shared = Arc::new(Shared {
                        samples: Mutex::new(Vec::with_capacity(rate as usize * 60)),
                        peak: AtomicU32::new(0),
                        max_samples: rate as usize * MAX_RECORDING_SECONDS as usize,
                        truncated: AtomicBool::new(false),
                    });
                    let err_fn = |e: cpal::Error| log::warn!("Dictation input stream error: {e}");

                    macro_rules! build {
                        ($t:ty) => {{
                            let shared = shared.clone();
                            device.build_input_stream(
                                config.clone().into(),
                                move |data: &[$t], _: &_| push_frames(data, channels, &shared),
                                err_fn,
                                None,
                            )
                        }};
                    }
                    let stream = match config.sample_format() {
                        SampleFormat::F32 => build!(f32),
                        SampleFormat::I16 => build!(i16),
                        SampleFormat::I32 => build!(i32),
                        SampleFormat::U16 => build!(u16),
                        SampleFormat::I8 => build!(i8),
                        other => {
                            return Err(mic_error(format!("unsupported sample format {other}")))
                        }
                    }
                    .map_err(mic_error)?;
                    stream.play().map_err(mic_error)?;
                    Ok((stream, rate, shared))
                };

                let (stream, shared) = match setup() {
                    Ok((stream, rate, shared)) => {
                        let _ = ready_tx.send(Ok((rate, shared.clone())));
                        (stream, shared)
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };

                // Keep the stream alive until asked to stop (or the owner is dropped).
                while let Err(mpsc::RecvTimeoutError::Timeout) =
                    stop_rx.recv_timeout(LEVEL_INTERVAL)
                {
                    let peak = f32::from_bits(shared.peak.swap(0, Ordering::Relaxed));
                    on_level(peak.min(1.0));
                }
                drop(stream);
            })
            .map_err(|e| mic_error(format!("failed to start capture thread: {e}")))?;

        let (sample_rate, shared) = ready_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| mic_error("timed out opening the microphone"))??;

        Ok(Self {
            shared,
            sample_rate,
            stop_tx,
            thread: Some(thread),
        })
    }

    /// Stop recording and return the captured mono audio.
    pub fn stop(mut self) -> Recording {
        self.halt();
        let samples = self
            .shared
            .samples
            .lock()
            .map(|mut s| std::mem::take(&mut *s))
            .unwrap_or_default();
        Recording {
            samples,
            sample_rate: self.sample_rate,
            truncated: self.shared.truncated.load(Ordering::Relaxed),
        }
    }

    fn halt(&mut self) {
        let _ = self.stop_tx.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Recorder {
    /// Cancelling a dictation (or locking the app) drops the recorder: the
    /// stream is closed and the captured audio is zeroed before release.
    fn drop(&mut self) {
        self.halt();
        if let Ok(mut s) = self.shared.samples.lock() {
            s.iter_mut().for_each(|x| *x = 0.0);
            s.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared(max: usize) -> Shared {
        Shared {
            samples: Mutex::new(Vec::new()),
            peak: AtomicU32::new(0),
            max_samples: max,
            truncated: AtomicBool::new(false),
        }
    }

    #[test]
    fn push_frames_downmixes_and_tracks_peak() {
        let s = shared(100);
        push_frames(&[0.5f32, -0.5, 1.0, 0.0], 2, &s);
        assert_eq!(*s.samples.lock().unwrap(), vec![0.0, 0.5]);
        assert_eq!(f32::from_bits(s.peak.load(Ordering::Relaxed)), 0.5);
    }

    #[test]
    fn push_frames_converts_integer_samples() {
        let s = shared(100);
        push_frames(&[i16::MAX, 0], 1, &s);
        let buf = s.samples.lock().unwrap();
        assert!((buf[0] - 1.0).abs() < 1e-3 && buf[1] == 0.0);
    }

    #[test]
    fn push_frames_stops_at_cap() {
        let s = shared(3);
        push_frames(&[0.1f32; 5], 1, &s);
        assert_eq!(s.samples.lock().unwrap().len(), 3);
        push_frames(&[0.1f32; 5], 1, &s);
        assert!(s.truncated.load(Ordering::Relaxed));
    }
}
