//! Whitelisted speech-to-text models — the same open-weight GGUF builds the
//! Handy dictation app uses, published by the transcribe.cpp maintainers.
//!
//! Every entry is pinned to a SHA-256 checked against the Hugging Face LFS
//! pointer before and after download (same policy as the LLM catalog).

use crate::error::AppError;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub struct AsrModelEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub filename: &'static str,
    pub download_url: &'static str,
    pub lfs_pointer_url: &'static str,
    pub pinned_sha256: &'static str,
    pub size_bytes: u64,
    pub license: &'static str,
    pub languages: &'static str,
    pub description: &'static str,
    pub recommended: bool,
}

pub const ASR_MODELS: &[AsrModelEntry] = &[
    // CC-BY-4.0; NVIDIA Parakeet TDT 0.6B v3, 25 European languages incl. de/fr/it.
    AsrModelEntry {
        id: "parakeet-tdt-0.6b-v3",
        name: "Parakeet TDT 0.6B v3",
        filename: "parakeet-tdt-0.6b-v3-Q5_K_M.gguf",
        download_url: "https://huggingface.co/handy-computer/parakeet-tdt-0.6b-v3-gguf/resolve/main/parakeet-tdt-0.6b-v3-Q5_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/handy-computer/parakeet-tdt-0.6b-v3-gguf/raw/main/parakeet-tdt-0.6b-v3-Q5_K_M.gguf",
        pinned_sha256: "cc722e76adc1a629fc0b2535de879d99b8160d07ad4c0215e2ca7d7ea0ae4b8f",
        size_bytes: 548_946_272,
        license: "CC-BY-4.0",
        languages: "25 European languages (de, fr, it, en, …), auto-detected",
        description: "Fast and accurate with punctuation. The default dictation model in Handy.",
        recommended: true,
    },
    // MIT; OpenAI Whisper large-v3-turbo, 99 languages.
    AsrModelEntry {
        id: "whisper-large-v3-turbo",
        name: "Whisper Large v3 Turbo",
        filename: "whisper-large-v3-turbo-Q5_K_M.gguf",
        download_url: "https://huggingface.co/handy-computer/whisper-large-v3-turbo-gguf/resolve/main/whisper-large-v3-turbo-Q5_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/handy-computer/whisper-large-v3-turbo-gguf/raw/main/whisper-large-v3-turbo-Q5_K_M.gguf",
        pinned_sha256: "06fc9923c09fb3ae878ea12c34d1be3ed18fad039ab14b4b045c7b6c40af56b2",
        size_bytes: 619_628_096,
        license: "MIT",
        languages: "99 languages; uses the practice language as a hint",
        description: "Broadest language coverage and robust with accents; slower than Parakeet.",
        recommended: false,
    },
];

pub fn find(id: &str) -> Option<&'static AsrModelEntry> {
    ASR_MODELS.iter().find(|m| m.id == id)
}

pub fn models_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("models").join("asr")
}

pub fn model_path(data_dir: &Path, entry: &AsrModelEntry) -> PathBuf {
    models_dir(data_dir).join(entry.filename)
}

/// In-progress downloads land here and are renamed only after verification,
/// so a partial file is never mistaken for an installed model.
pub fn partial_path(data_dir: &Path, entry: &AsrModelEntry) -> PathBuf {
    models_dir(data_dir).join(format!("{}.part", entry.filename))
}

fn active_file(data_dir: &Path) -> PathBuf {
    models_dir(data_dir).join("active-model")
}

pub fn is_installed(data_dir: &Path, entry: &AsrModelEntry) -> bool {
    std::fs::metadata(model_path(data_dir, entry))
        .map(|m| m.len() == entry.size_bytes)
        .unwrap_or(false)
}

/// The model used for dictation: the user's explicit choice if it is still
/// installed, otherwise the first installed model in catalog order.
pub fn active_model(data_dir: &Path) -> Option<&'static AsrModelEntry> {
    let chosen = std::fs::read_to_string(active_file(data_dir))
        .ok()
        .and_then(|id| find(id.trim()))
        .filter(|m| is_installed(data_dir, m));
    chosen.or_else(|| ASR_MODELS.iter().find(|m| is_installed(data_dir, m)))
}

pub fn set_active_model(data_dir: &Path, id: &str) -> Result<(), AppError> {
    let entry =
        find(id).ok_or_else(|| AppError::Validation(format!("Unknown dictation model '{id}'")))?;
    if !is_installed(data_dir, entry) {
        return Err(AppError::Validation(format!(
            "Dictation model '{id}' is not installed"
        )));
    }
    std::fs::create_dir_all(models_dir(data_dir))?;
    std::fs::write(active_file(data_dir), entry.id)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct AsrModelInfo {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub license: String,
    pub languages: String,
    pub description: String,
    pub recommended: bool,
    pub installed: bool,
    pub active: bool,
}

pub fn list(data_dir: &Path) -> Vec<AsrModelInfo> {
    let active = active_model(data_dir).map(|m| m.id);
    ASR_MODELS
        .iter()
        .map(|m| AsrModelInfo {
            id: m.id.into(),
            name: m.name.into(),
            size_bytes: m.size_bytes,
            license: m.license.into(),
            languages: m.languages.into(),
            description: m.description.into(),
            recommended: m.recommended,
            installed: is_installed(data_dir, m),
            active: active == Some(m.id),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_entries_are_well_formed() {
        for m in ASR_MODELS {
            assert_eq!(m.pinned_sha256.len(), 64, "{}", m.id);
            assert!(m.pinned_sha256.bytes().all(|b| b.is_ascii_hexdigit()));
            assert!(m.download_url.ends_with(m.filename));
            assert!(m.lfs_pointer_url.ends_with(m.filename));
            assert!(m
                .download_url
                .starts_with("https://huggingface.co/handy-computer/"));
            assert!(m.lfs_pointer_url.contains("/raw/main/"));
        }
        assert_eq!(ASR_MODELS.iter().filter(|m| m.recommended).count(), 1);
    }

    #[test]
    fn active_model_prefers_choice_then_catalog_order() {
        let dir = tempfile::tempdir().unwrap();
        assert!(active_model(dir.path()).is_none());

        // Install the second catalog entry only (sparse file of the right size).
        let whisper = &ASR_MODELS[1];
        std::fs::create_dir_all(models_dir(dir.path())).unwrap();
        let f = std::fs::File::create(model_path(dir.path(), whisper)).unwrap();
        f.set_len(whisper.size_bytes).unwrap();
        assert_eq!(active_model(dir.path()).unwrap().id, whisper.id);

        // A choice that is not installed is ignored.
        std::fs::write(active_file(dir.path()), ASR_MODELS[0].id).unwrap();
        assert_eq!(active_model(dir.path()).unwrap().id, whisper.id);
        assert!(set_active_model(dir.path(), ASR_MODELS[0].id).is_err());

        set_active_model(dir.path(), whisper.id).unwrap();
        let listed = list(dir.path());
        assert!(listed
            .iter()
            .any(|m| m.id == whisper.id && m.installed && m.active));
        assert!(listed
            .iter()
            .any(|m| m.id == ASR_MODELS[0].id && !m.installed && !m.active));
    }

    #[test]
    fn truncated_file_is_not_installed() {
        let dir = tempfile::tempdir().unwrap();
        let m = &ASR_MODELS[0];
        std::fs::create_dir_all(models_dir(dir.path())).unwrap();
        std::fs::write(model_path(dir.path(), m), b"partial").unwrap();
        assert!(!is_installed(dir.path(), m));
    }
}
