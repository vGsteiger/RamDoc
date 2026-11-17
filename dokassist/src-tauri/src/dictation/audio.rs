//! Audio decoding and conversion to the 16 kHz mono f32 PCM the speech
//! models expect. Everything happens in memory: decrypted dictation audio is
//! never written to disk in plaintext.

use crate::error::AppError;
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};
use std::io::Cursor;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// Sample rate expected by every supported speech model.
pub const TARGET_RATE: u32 = 16_000;

/// Longest recorded dictation we accept for transcription (3 hours). Matches
/// the hard limit enforced by the `dokassist-asr` worker.
pub const MAX_AUDIO_SECONDS: u64 = 3 * 60 * 60;

/// MIME types (and the extension hint passed to the demuxer) that can be
/// transcribed. `audio/*` types not listed here are still attempted by
/// probing the content.
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "wav", "mp3", "m4a", "mp4", "aac", "flac", "ogg", "oga", "aif", "aiff", "caf",
];

/// True if `mime_type` is audio we should try to transcribe.
pub fn is_audio_mime(mime_type: &str) -> bool {
    mime_type.starts_with("audio/")
}

fn extension_hint(filename: &str) -> Option<String> {
    let ext = filename.rsplit_once('.')?.1.to_ascii_lowercase();
    SUPPORTED_EXTENSIONS.contains(&ext.as_str()).then_some(ext)
}

fn decode_error(msg: impl std::fmt::Display) -> AppError {
    AppError::Dictation(format!("Could not decode audio: {msg}"))
}

/// Decode an in-memory audio file (wav, mp3, m4a/aac, flac, ogg/vorbis, aiff,
/// caf) into 16 kHz mono f32 PCM.
pub fn decode_to_pcm16k(bytes: Vec<u8>, filename: &str) -> Result<Vec<f32>, AppError> {
    let mss = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = extension_hint(filename) {
        hint.with_extension(&ext);
    }

    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| decode_error(format!("unsupported or corrupt audio file ({e})")))?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| decode_error("file contains no audio track"))?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or_else(|| decode_error("audio track has no codec parameters"))?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|e| decode_error(format!("unsupported codec ({e})")))?;

    let mut mono: Vec<f32> = Vec::new();
    let mut rate: Option<u32> = None;
    let mut interleaved: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            // Truncated files: keep what decoded cleanly.
            Err(SymphoniaError::IoError(_)) => break,
            Err(e) => return Err(decode_error(e)),
        };
        if packet.track_id != track_id {
            continue;
        }
        let buf = match decoder.decode(&packet) {
            Ok(buf) => buf,
            // Skip a corrupt packet rather than failing the whole dictation.
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(decode_error(e)),
        };

        let spec = buf.spec();
        let this_rate = spec.rate();
        match rate {
            None => rate = Some(this_rate),
            Some(r) if r != this_rate => {
                return Err(decode_error("sample rate changes mid-stream"));
            }
            _ => {}
        }
        let channels = spec.channels().count().max(1);

        interleaved.resize(buf.samples_interleaved(), 0.0);
        buf.copy_to_slice_interleaved(&mut interleaved);
        mono.extend(downmix(&interleaved, channels));

        if mono.len() as u64 > MAX_AUDIO_SECONDS * this_rate as u64 {
            return Err(AppError::Dictation(
                "Recording is longer than the 3 hour transcription limit".into(),
            ));
        }
    }

    let rate = rate.ok_or_else(|| decode_error("file contains no decodable audio"))?;
    resample_to_16k(&mono, rate)
}

/// Average interleaved frames down to a single channel.
pub fn downmix(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    let scale = 1.0 / channels as f32;
    interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() * scale)
        .collect()
}

/// Resample mono PCM from `rate` to 16 kHz with a band-limited FFT resampler.
pub fn resample_to_16k(mono: &[f32], rate: u32) -> Result<Vec<f32>, AppError> {
    if rate == TARGET_RATE || mono.is_empty() {
        return Ok(mono.to_vec());
    }
    if rate == 0 {
        return Err(decode_error("invalid sample rate 0"));
    }

    let resample_err =
        |e: &dyn std::fmt::Display| AppError::Dictation(format!("Resampling failed: {e}"));
    let mut resampler = Fft::<f32>::new(
        rate as usize,
        TARGET_RATE as usize,
        1024,
        1,
        FixedSync::Input,
    )
    .map_err(|e| resample_err(&e))?;

    let input = InterleavedSlice::new(mono, 1, mono.len()).map_err(|e| resample_err(&e))?;
    let capacity = resampler.process_all_needed_output_len(mono.len());
    let mut out = vec![0.0f32; capacity];
    let mut output =
        InterleavedSlice::new_mut(&mut out, 1, capacity).map_err(|e| resample_err(&e))?;
    let (_, produced) = resampler
        .process_all_into_buffer(&input, &mut output, mono.len(), None)
        .map_err(|e| resample_err(&e))?;
    out.truncate(produced);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, seconds: f32, freq: f32) -> Vec<f32> {
        let n = (rate as f32 * seconds) as usize;
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin() * 0.5)
            .collect()
    }

    /// Minimal 16-bit PCM WAV encoder for tests.
    fn wav_i16(samples: &[f32], rate: u32, channels: u16) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
        out.extend_from_slice(&(channels * 2).to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for s in samples {
            out.extend_from_slice(&((s * i16::MAX as f32) as i16).to_le_bytes());
        }
        out
    }

    #[test]
    fn downmix_averages_channels() {
        assert_eq!(downmix(&[1.0, 0.0, 0.5, 0.5], 2), vec![0.5, 0.5]);
        assert_eq!(downmix(&[0.25, 0.75], 1), vec![0.25, 0.75]);
    }

    #[test]
    fn resample_48k_to_16k_keeps_duration() {
        let input = sine(48_000, 2.0, 440.0);
        let out = resample_to_16k(&input, 48_000).unwrap();
        let expected = 32_000i64;
        assert!(
            (out.len() as i64 - expected).abs() < 200,
            "got {} samples",
            out.len()
        );
    }

    #[test]
    fn resample_is_identity_at_16k() {
        let input = sine(16_000, 0.5, 300.0);
        assert_eq!(resample_to_16k(&input, 16_000).unwrap(), input);
    }

    #[test]
    fn decodes_stereo_44k_wav_to_mono_16k() {
        let mono = sine(44_100, 1.0, 440.0);
        let stereo: Vec<f32> = mono.iter().flat_map(|s| [*s, *s]).collect();
        let wav = wav_i16(&stereo, 44_100, 2);
        let pcm = decode_to_pcm16k(wav, "Diktat.WAV").unwrap();
        assert!((pcm.len() as i64 - 16_000).abs() < 200, "len {}", pcm.len());
        let peak = pcm.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.3 && peak < 0.7, "peak {peak}");
    }

    #[test]
    fn rejects_non_audio_bytes() {
        let err = decode_to_pcm16k(b"%PDF-1.7 not audio".to_vec(), "x.pdf").unwrap_err();
        assert!(matches!(err, AppError::Dictation(_)));
    }

    #[test]
    fn audio_mime_detection() {
        assert!(is_audio_mime("audio/mpeg"));
        assert!(is_audio_mime("audio/x-m4a"));
        assert!(!is_audio_mime("application/pdf"));
    }
}
