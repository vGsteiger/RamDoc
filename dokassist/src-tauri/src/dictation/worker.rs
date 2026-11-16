//! Client for the out-of-process `dokassist-asr` transcription worker.
//!
//! The worker is spawned per job. Live dictation spawns it the moment the
//! recording starts, so the model loads while the clinician speaks and the
//! transcript is ready almost as soon as they stop.

use crate::error::AppError;
use serde::Deserialize;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

pub const WORKER_NAME: &str = "dokassist-asr";

/// Environment override for the worker binary (development and tests).
pub const WORKER_ENV: &str = "DOKASSIST_ASR_WORKER";

/// Generous budget for model load before the worker reports ready.
const READY_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum WorkerEvent {
    Ready {
        #[allow(dead_code)]
        arch: String,
    },
    Progress {
        done: usize,
        total: usize,
    },
    Result {
        text: String,
        language: Option<String>,
        audio_ms: u64,
    },
    Error {
        message: String,
    },
}

/// Final transcription result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Transcript {
    pub text: String,
    pub language: Option<String>,
    pub audio_ms: u64,
}

/// Locate the worker executable.
///
/// Order: `$DOKASSIST_ASR_WORKER`, next to the app executable (where the
/// bundler places sidecars, `RamDoc.app/Contents/MacOS/`), then — in debug
/// builds only — the worker crate's own `target/` directory.
pub fn locate_worker() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os(WORKER_ENV).map(PathBuf::from) {
        return p.is_file().then_some(p);
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        let candidate = dir.join(WORKER_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    #[cfg(debug_assertions)]
    {
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../asr-worker/target");
        for profile in ["release", "debug"] {
            let candidate = crate_dir.join(profile).join(WORKER_NAME);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn worker_error(msg: impl std::fmt::Display) -> AppError {
    AppError::Dictation(msg.to_string())
}

/// A spawned worker that is loading (or has loaded) its model and is waiting
/// for audio on stdin. Dropping it kills the process.
pub struct WorkerJob {
    child: Child,
    stdin: Option<ChildStdin>,
    events: mpsc::Receiver<WorkerEvent>,
    ready: bool,
}

impl WorkerJob {
    pub fn spawn(worker: &Path, model: &Path, language: Option<&str>) -> Result<Self, AppError> {
        let mut cmd = Command::new(worker);
        cmd.arg("--model").arg(model);
        if let Some(lang) = language {
            cmd.arg("--language").arg(lang);
        }
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Native logging is disabled in the worker; drop anything else so
            // nothing transcript-related reaches the system log.
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| worker_error(format!("Failed to start transcription worker: {e}")))?;

        let stdout = child.stdout.take().expect("stdout is piped");
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("dictation-worker-reader".into())
            .spawn(move || {
                for line in BufReader::new(stdout).lines() {
                    let Ok(line) = line else { break };
                    match serde_json::from_str::<WorkerEvent>(&line) {
                        Ok(event) => {
                            if tx.send(event).is_err() {
                                break;
                            }
                        }
                        Err(_) => log::warn!("Ignoring malformed transcription worker output"),
                    }
                }
            })
            .map_err(|e| worker_error(format!("Failed to start worker reader: {e}")))?;

        Ok(Self {
            stdin: child.stdin.take(),
            child,
            events: rx,
            ready: false,
        })
    }

    fn wait_ready(&mut self) -> Result<(), AppError> {
        if self.ready {
            return Ok(());
        }
        match self.events.recv_timeout(READY_TIMEOUT) {
            Ok(WorkerEvent::Ready { .. }) => {
                self.ready = true;
                Ok(())
            }
            Ok(WorkerEvent::Error { message }) => Err(worker_error(message)),
            Ok(_) => Err(worker_error("Transcription worker protocol error")),
            Err(_) => Err(worker_error(
                "Transcription worker exited or timed out while loading the model",
            )),
        }
    }

    /// Send 16 kHz mono PCM, wait for the transcript. `on_progress(done, total)`
    /// is called once per transcribed chunk.
    pub fn transcribe(
        mut self,
        pcm: &[f32],
        mut on_progress: impl FnMut(usize, usize),
    ) -> Result<Transcript, AppError> {
        self.wait_ready()?;

        {
            let mut stdin = self
                .stdin
                .take()
                .ok_or_else(|| worker_error("Transcription worker stdin closed"))?;
            let mut bytes = Vec::with_capacity(64 * 1024);
            for chunk in pcm.chunks(16 * 1024) {
                bytes.clear();
                bytes.extend(chunk.iter().flat_map(|s| s.to_le_bytes()));
                stdin
                    .write_all(&bytes)
                    .map_err(|e| worker_error(format!("Failed to send audio to worker: {e}")))?;
            }
            // Dropping stdin signals end of audio.
        }

        // Transcription is faster than real time on Apple silicon; allow a
        // wide margin for slower machines and very long recordings.
        let audio_secs = pcm.len() as u64 / super::audio::TARGET_RATE as u64;
        let deadline = Duration::from_secs(120 + audio_secs * 2);
        let started = std::time::Instant::now();
        loop {
            let remaining = deadline
                .checked_sub(started.elapsed())
                .ok_or_else(|| worker_error("Transcription timed out"))?;
            match self.events.recv_timeout(remaining) {
                Ok(WorkerEvent::Progress { done, total }) => on_progress(done, total),
                Ok(WorkerEvent::Result {
                    text,
                    language,
                    audio_ms,
                }) => {
                    return Ok(Transcript {
                        text,
                        language,
                        audio_ms,
                    })
                }
                Ok(WorkerEvent::Error { message }) => return Err(worker_error(message)),
                Ok(WorkerEvent::Ready { .. }) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(worker_error("Transcription timed out"))
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(worker_error("Transcription worker exited unexpectedly"))
                }
            }
        }
    }
}

impl Drop for WorkerJob {
    fn drop(&mut self) {
        drop(self.stdin.take());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_worker_events() {
        let ready: WorkerEvent = serde_json::from_str(
            r#"{"event":"ready","arch":"parakeet","backend":"MTL0","languages":["de"]}"#,
        )
        .unwrap();
        assert!(matches!(ready, WorkerEvent::Ready { .. }));
        let result: WorkerEvent = serde_json::from_str(
            r#"{"event":"result","text":"Hallo","language":null,"audio_ms":1200}"#,
        )
        .unwrap();
        assert!(
            matches!(result, WorkerEvent::Result { ref text, audio_ms: 1200, .. } if text == "Hallo")
        );
    }

    #[test]
    fn env_override_must_point_at_a_file() {
        // A non-existent override must not fall through to other locations.
        std::env::set_var(WORKER_ENV, "/definitely/not/here/dokassist-asr");
        assert!(locate_worker().is_none());
        std::env::remove_var(WORKER_ENV);
    }
}
