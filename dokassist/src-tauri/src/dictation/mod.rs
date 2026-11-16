//! Local dictation: live microphone dictation and transcription of recorded
//! audio files, using the open-weight speech models from the Handy app
//! (Parakeet v3, Whisper large-v3-turbo) through transcribe.cpp.
//!
//! Speech recognition runs in the separate `dokassist-asr` worker process —
//! see `asr-worker/Cargo.toml` for why it cannot share our address space with
//! llama.cpp. Audio stays in memory and is piped to the worker; nothing is
//! sent over the network and no plaintext audio touches the disk.

pub mod audio;
pub mod capture;
pub mod models;
pub mod worker;

use crate::error::AppError;
use std::path::Path;
use worker::{Transcript, WorkerJob};

/// A live dictation in progress: the microphone is open and the worker is
/// already loading its model in the background.
pub struct ActiveDictation {
    pub id: u64,
    pub recorder: capture::Recorder,
    pub job: WorkerJob,
}

/// Resolve the worker and the active model, failing with a user-actionable
/// message if either is missing.
///
/// The language is always auto-detected: Swiss practices dictate in more than
/// one language, and Whisper treats a hint as a forced language (a French
/// dictation in a German-language practice would come out garbled).
pub fn prepare_job(data_dir: &Path) -> Result<WorkerJob, AppError> {
    let model = models::active_model(data_dir).ok_or_else(|| {
        AppError::Dictation("No dictation model installed. Download one in Settings.".into())
    })?;
    let worker = worker::locate_worker().ok_or_else(|| {
        AppError::Dictation("The transcription component is missing from this installation.".into())
    })?;
    WorkerJob::spawn(&worker, &models::model_path(data_dir, model), None)
}

/// Transcribe an in-memory audio file (blocking — call from `spawn_blocking`).
pub fn transcribe_file_bytes(
    data_dir: &Path,
    bytes: Vec<u8>,
    filename: &str,
    on_progress: impl FnMut(usize, usize),
) -> Result<Transcript, AppError> {
    // Start the worker first so model load overlaps with decoding.
    let job = prepare_job(data_dir)?;
    let pcm = audio::decode_to_pcm16k(bytes, filename)?;
    job.transcribe(&pcm, on_progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end check against a real model and worker. Run with:
    /// `DOKASSIST_ASR_TEST_MODEL=/path/model.gguf DOKASSIST_ASR_TEST_AUDIO=/path/audio.wav \
    ///  cargo test dictation::tests::real_transcription -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a downloaded speech model and the built dokassist-asr worker"]
    fn real_transcription() {
        let model =
            std::env::var("DOKASSIST_ASR_TEST_MODEL").expect("set DOKASSIST_ASR_TEST_MODEL");
        let audio =
            std::env::var("DOKASSIST_ASR_TEST_AUDIO").expect("set DOKASSIST_ASR_TEST_AUDIO");
        let worker = worker::locate_worker().expect("build ../asr-worker first");
        let bytes = std::fs::read(&audio).unwrap();
        let job = WorkerJob::spawn(&worker, Path::new(&model), None).unwrap();
        let pcm = audio::decode_to_pcm16k(bytes, &audio).unwrap();
        let t = job
            .transcribe(&pcm, |d, n| eprintln!("chunk {d}/{n}"))
            .unwrap();
        eprintln!("{t:?}");
        assert!(!t.text.trim().is_empty());
    }
}
