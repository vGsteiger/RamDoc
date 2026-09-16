//! Promotion records bridge the offline clinical quantization study into the
//! app without turning a research result into an implicit trust decision.
//!
//! The offline gate binds a promoted GGUF to the study manifest, recipe,
//! held-out results, llama.cpp commit and per-category non-inferiority result.
//! Import re-hashes the full artifact before it enters the model registry and
//! keeps the validated record beside the GGUF for the settings UI.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use ring::digest::{Context as DigestContext, SHA256};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const PROMOTION_KIND: &str = "ramdoc-clinical-quantization-promotion";
const PROMOTION_SCHEMA_VERSION: u32 = 1;
const MAX_PROMOTION_BYTES: u64 = 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 60 * 1024 * 1024 * 1024;
const COPY_BUFFER_BYTES: usize = 1024 * 1024;
const SIDECAR_SUFFIX: &str = ".ramdoc-promotion.json";

const REQUIRED_CATEGORIES: &[&str] = &[
    "medication",
    "dose",
    "date",
    "negation",
    "uncertainty",
    "chronology",
    "unsupported_claim",
    "german_swiss",
    "report_generation",
    "tool_call",
    "general_instruction",
    "long_context",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantizationPromotion {
    pub schema_version: u32,
    pub kind: String,
    pub study_id: String,
    pub display_name: String,
    pub created_at: String,
    pub artifact: PromotedArtifact,
    pub evidence: PromotionEvidence,
    pub decision: PromotionDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromotedArtifact {
    pub filename: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub base_model_sha256: String,
    pub quantization: String,
    pub recipe_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionEvidence {
    pub study_manifest_sha256: String,
    pub held_out_results_sha256: String,
    pub llama_cpp_commit: String,
    pub categories: Vec<String>,
    pub baseline_artifacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionDecision {
    pub recommended: bool,
    pub pareto_frontier_expanded: bool,
    pub dominates: Vec<String>,
    pub category_regression_upper_confidence: BTreeMap<String, f64>,
    pub category_regression_limits: BTreeMap<String, f64>,
}

/// Compact, non-sensitive metadata returned with a registered model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuantizationPromotionSummary {
    pub study_id: String,
    pub created_at: String,
    pub quantization: String,
    pub recipe_sha256: String,
    pub study_manifest_sha256: String,
    pub held_out_results_sha256: String,
    pub llama_cpp_commit: String,
    pub categories: Vec<String>,
    pub baseline_artifacts: Vec<String>,
    pub dominates: Vec<String>,
    pub worst_category_regression: f64,
}

/// Fast, user-facing preview of a promotion record. Does not hash the GGUF.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromotionPreview {
    pub display_name: String,
    pub study_id: String,
    pub filename: String,
    pub size_bytes: u64,
    pub quantization: String,
    pub artifact_found: bool,
    pub artifact_size_matches: bool,
    pub artifact_bytes: Option<u64>,
    pub artifact_path: Option<String>,
    pub dominates: Vec<String>,
    pub baseline_artifacts: Vec<String>,
    pub worst_category_regression: f64,
}

#[derive(Debug)]
pub struct InstalledPromotion {
    pub record: QuantizationPromotion,
    pub model_path: PathBuf,
}

impl QuantizationPromotion {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.schema_version != PROMOTION_SCHEMA_VERSION {
            return validation_error(format!(
                "unsupported quantization promotion schema {}; expected {}",
                self.schema_version, PROMOTION_SCHEMA_VERSION
            ));
        }
        if self.kind != PROMOTION_KIND {
            return validation_error(format!(
                "invalid quantization promotion kind {:?}",
                self.kind
            ));
        }
        validate_text(&self.study_id, "study_id", 128)?;
        validate_text(&self.display_name, "display_name", 256)?;
        validate_text(&self.created_at, "created_at", 64)?;
        if !self.created_at.contains('T') || !self.created_at.ends_with('Z') {
            return validation_error("created_at must be an RFC 3339 UTC timestamp");
        }

        validate_filename(&self.artifact.filename)?;
        validate_sha256(&self.artifact.sha256, "artifact.sha256")?;
        validate_sha256(
            &self.artifact.base_model_sha256,
            "artifact.base_model_sha256",
        )?;
        validate_sha256(&self.artifact.recipe_sha256, "artifact.recipe_sha256")?;
        validate_text(&self.artifact.quantization, "artifact.quantization", 128)?;
        if self.artifact.size_bytes == 0 || self.artifact.size_bytes > MAX_ARTIFACT_BYTES {
            return validation_error(format!(
                "artifact.size_bytes must be in 1..={MAX_ARTIFACT_BYTES}"
            ));
        }

        validate_sha256(
            &self.evidence.study_manifest_sha256,
            "evidence.study_manifest_sha256",
        )?;
        validate_sha256(
            &self.evidence.held_out_results_sha256,
            "evidence.held_out_results_sha256",
        )?;
        if self.evidence.llama_cpp_commit.len() != 40
            || !self
                .evidence
                .llama_cpp_commit
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return validation_error(
                "evidence.llama_cpp_commit must be a lowercase 40-character commit SHA",
            );
        }

        let expected: BTreeSet<&str> = REQUIRED_CATEGORIES.iter().copied().collect();
        let actual: BTreeSet<&str> = self
            .evidence
            .categories
            .iter()
            .map(String::as_str)
            .collect();
        if actual != expected || self.evidence.categories.len() != expected.len() {
            return validation_error(
                "evidence.categories must contain every RamDoc clinical and general category exactly once",
            );
        }
        if self.evidence.baseline_artifacts.is_empty() {
            return validation_error("evidence.baseline_artifacts must not be empty");
        }
        let mut baseline_ids = BTreeSet::new();
        for baseline in &self.evidence.baseline_artifacts {
            validate_text(baseline, "evidence.baseline_artifacts[]", 128)?;
            if !baseline_ids.insert(baseline) {
                return validation_error("evidence.baseline_artifacts contains duplicates");
            }
        }

        if !self.decision.recommended || !self.decision.pareto_frontier_expanded {
            return validation_error(
                "the app only imports artifacts that the offline gate recommended as a Pareto-frontier expansion",
            );
        }
        if self.decision.dominates.is_empty() {
            return validation_error("decision.dominates must not be empty");
        }
        if self
            .decision
            .dominates
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != self.decision.dominates.len()
        {
            return validation_error("decision.dominates contains duplicates");
        }
        if !self
            .decision
            .dominates
            .iter()
            .all(|id| self.evidence.baseline_artifacts.contains(id))
        {
            return validation_error(
                "decision.dominates contains an artifact absent from evidence.baseline_artifacts",
            );
        }

        let regression_categories: BTreeSet<&str> = self
            .decision
            .category_regression_upper_confidence
            .keys()
            .map(String::as_str)
            .collect();
        let limit_categories: BTreeSet<&str> = self
            .decision
            .category_regression_limits
            .keys()
            .map(String::as_str)
            .collect();
        if regression_categories != expected || limit_categories != expected {
            return validation_error(
                "decision regression evidence and limits must cover every required category",
            );
        }
        for category in REQUIRED_CATEGORIES {
            let regression = self.decision.category_regression_upper_confidence[*category];
            let limit = self.decision.category_regression_limits[*category];
            if !regression.is_finite()
                || !limit.is_finite()
                || !(0.0..=1.0).contains(&regression)
                || !(0.0..=1.0).contains(&limit)
            {
                return validation_error(format!(
                    "category {category} has a non-finite or out-of-range regression value"
                ));
            }
            if regression > limit + f64::EPSILON {
                return validation_error(format!(
                    "category {category} regressed by {regression:.6}, above its {limit:.6} limit"
                ));
            }
        }
        Ok(())
    }

    pub fn summary(&self) -> QuantizationPromotionSummary {
        QuantizationPromotionSummary {
            study_id: self.study_id.clone(),
            created_at: self.created_at.clone(),
            quantization: self.artifact.quantization.clone(),
            recipe_sha256: self.artifact.recipe_sha256.clone(),
            study_manifest_sha256: self.evidence.study_manifest_sha256.clone(),
            held_out_results_sha256: self.evidence.held_out_results_sha256.clone(),
            llama_cpp_commit: self.evidence.llama_cpp_commit.clone(),
            categories: self.evidence.categories.clone(),
            baseline_artifacts: self.evidence.baseline_artifacts.clone(),
            dominates: self.decision.dominates.clone(),
            worst_category_regression: self
                .decision
                .category_regression_upper_confidence
                .values()
                .copied()
                .fold(0.0, f64::max),
        }
    }
}

fn validation_error<T>(message: impl Into<String>) -> Result<T, AppError> {
    Err(AppError::Validation(message.into()))
}

fn validate_text(value: &str, field: &str, max_len: usize) -> Result<(), AppError> {
    if value.trim().is_empty()
        || value.trim() != value
        || value.len() > max_len
        || value.chars().any(char::is_control)
    {
        return validation_error(format!(
            "{field} must be trimmed, non-empty, at most {max_len} characters, and contain no control characters"
        ));
    }
    Ok(())
}

fn validate_filename(filename: &str) -> Result<(), AppError> {
    validate_text(filename, "artifact.filename", 255)?;
    if !filename.ends_with(".gguf")
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains("..")
        || Path::new(filename)
            .file_name()
            .and_then(|name| name.to_str())
            != Some(filename)
    {
        return validation_error(
            "artifact.filename must be a plain .gguf filename without path traversal",
        );
    }
    Ok(())
}

fn validate_sha256(digest: &str, field: &str) -> Result<(), AppError> {
    if digest.len() != 64
        || digest.bytes().all(|byte| byte == b'0')
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return validation_error(format!(
            "{field} must be a non-placeholder lowercase 64-character SHA-256 digest"
        ));
    }
    Ok(())
}

pub fn promotion_path_for_model(model_path: &Path) -> PathBuf {
    let filename = model_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    model_path.with_file_name(format!("{filename}{SIDECAR_SUFFIX}"))
}

pub fn read_promotion(path: &Path) -> Result<QuantizationPromotion, AppError> {
    let file = File::open(path).map_err(|error| {
        AppError::Validation(format!(
            "cannot open quantization promotion '{}': {error}",
            path.display()
        ))
    })?;
    let metadata = file.metadata().map_err(|error| {
        AppError::Validation(format!(
            "cannot inspect quantization promotion '{}': {error}",
            path.display()
        ))
    })?;
    if !metadata.is_file() {
        return validation_error("quantization promotion must be a regular file");
    }

    let mut bytes = Vec::new();
    file.take(MAX_PROMOTION_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            AppError::Validation(format!(
                "cannot read quantization promotion '{}': {error}",
                path.display()
            ))
        })?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_PROMOTION_BYTES {
        return validation_error(format!(
            "quantization promotion must be a non-empty file no larger than {MAX_PROMOTION_BYTES} bytes"
        ));
    }

    let promotion: QuantizationPromotion = serde_json::from_slice(&bytes).map_err(|error| {
        AppError::Validation(format!("invalid quantization promotion JSON: {error}"))
    })?;
    promotion.validate()?;
    Ok(promotion)
}

pub fn promotion_summary_for_model(
    model_path: &Path,
) -> Result<Option<QuantizationPromotionSummary>, AppError> {
    let path = promotion_path_for_model(model_path);
    if !path.exists() {
        return Ok(None);
    }
    let promotion = read_promotion(&path)?;
    if promotion.artifact.filename != model_path.file_name().unwrap_or_default().to_string_lossy() {
        return validation_error("promotion sidecar filename does not match its GGUF");
    }
    Ok(Some(promotion.summary()))
}

/// Revalidate a promoted artifact immediately before model loading.
///
/// Listing models intentionally reads only the small sidecar; hashing a 5 GiB
/// file on every settings refresh would be hostile. Loading is the security
/// boundary, so it pays the full sequential hash cost again and rejects any
/// post-import mutation.
pub fn verify_promoted_model(
    model_path: &Path,
) -> Result<Option<QuantizationPromotionSummary>, AppError> {
    let sidecar = promotion_path_for_model(model_path);
    if !sidecar.exists() {
        return Ok(None);
    }
    let promotion = read_promotion(&sidecar)?;
    if promotion.artifact.filename != model_path.file_name().unwrap_or_default().to_string_lossy() {
        return validation_error("promotion sidecar filename does not match its GGUF");
    }
    let (digest, size, magic) = hash_file(model_path)?;
    if magic != *b"GGUF" {
        return validation_error("promoted model no longer has GGUF magic");
    }
    if digest != promotion.artifact.sha256 || size != promotion.artifact.size_bytes {
        return validation_error(
            "promoted model changed after import; refusing to load content that was not evaluated",
        );
    }
    Ok(Some(promotion.summary()))
}

fn resolve_artifact_source(
    promotion_path: &Path,
    record: &QuantizationPromotion,
    artifact_path: Option<&Path>,
) -> Result<PathBuf, AppError> {
    if let Some(path) = artifact_path {
        if path.as_os_str().is_empty() {
            return validation_error("model file path cannot be empty");
        }
        return Ok(path.to_path_buf());
    }
    let source_dir = promotion_path.parent().ok_or_else(|| {
        AppError::Validation("quantization promotion has no parent directory".to_string())
    })?;
    Ok(source_dir.join(&record.artifact.filename))
}

/// Parse a promotion record and report whether its GGUF is already locatable.
///
/// This is the Settings preview path: it must stay cheap, so it never hashes
/// the multi-gigabyte artifact. Import still re-hashes before registration.
pub fn inspect_promotion(
    promotion_path: &Path,
    artifact_path: Option<&Path>,
) -> Result<PromotionPreview, AppError> {
    let record = read_promotion(promotion_path)?;
    let source = resolve_artifact_source(promotion_path, &record, artifact_path)?;
    let (artifact_found, artifact_size_matches, artifact_bytes, located) =
        match fs::metadata(&source) {
            Ok(meta) if meta.is_file() => (
                true,
                meta.len() == record.artifact.size_bytes,
                Some(meta.len()),
                Some(source.to_string_lossy().into_owned()),
            ),
            _ => (false, false, None, None),
        };
    let summary = record.summary();
    Ok(PromotionPreview {
        display_name: record.display_name,
        study_id: record.study_id,
        filename: record.artifact.filename,
        size_bytes: record.artifact.size_bytes,
        quantization: record.artifact.quantization,
        artifact_found,
        artifact_size_matches,
        artifact_bytes,
        artifact_path: located,
        dominates: summary.dominates,
        baseline_artifacts: summary.baseline_artifacts,
        worst_category_regression: summary.worst_category_regression,
    })
}

fn hash_file(path: &Path) -> Result<(String, u64, [u8; 4]), AppError> {
    hash_file_with_progress(path, None, |_| {})
}

fn emit_progress(last_emitted_pct: &mut i32, progress: f64, on_progress: &mut impl FnMut(f64)) {
    let pct = (progress.clamp(0.0, 1.0) * 100.0).floor() as i32;
    if pct != *last_emitted_pct {
        on_progress(progress.clamp(0.0, 1.0));
        *last_emitted_pct = pct;
    }
}

fn hash_file_with_progress(
    path: &Path,
    expected_size: Option<u64>,
    mut on_progress: impl FnMut(f64),
) -> Result<(String, u64, [u8; 4]), AppError> {
    let mut file = File::open(path).map_err(|error| {
        AppError::Validation(format!(
            "cannot open promoted GGUF '{}': {error}",
            path.display()
        ))
    })?;
    let mut digest = DigestContext::new(&SHA256);
    let mut total = 0_u64;
    let mut magic = [0_u8; 4];
    let mut magic_len = 0_usize;
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    let mut last_emitted_pct = -1_i32;
    emit_progress(&mut last_emitted_pct, 0.0, &mut on_progress);
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            AppError::Validation(format!(
                "cannot read promoted GGUF '{}': {error}",
                path.display()
            ))
        })?;
        if read == 0 {
            break;
        }
        if magic_len < magic.len() {
            let copy = (magic.len() - magic_len).min(read);
            magic[magic_len..magic_len + copy].copy_from_slice(&buffer[..copy]);
            magic_len += copy;
        }
        total = total.saturating_add(read as u64);
        if total > MAX_ARTIFACT_BYTES {
            return validation_error("promoted GGUF exceeds the 60 GiB import limit");
        }
        digest.update(&buffer[..read]);
        if let Some(expected) = expected_size.filter(|size| *size > 0) {
            emit_progress(
                &mut last_emitted_pct,
                total as f64 / expected as f64,
                &mut on_progress,
            );
        }
    }
    emit_progress(&mut last_emitted_pct, 1.0, &mut on_progress);
    Ok((hex::encode(digest.finish().as_ref()), total, magic))
}

fn write_sidecar(
    destination_model: &Path,
    promotion: &QuantizationPromotion,
) -> Result<(), AppError> {
    let destination = promotion_path_for_model(destination_model);
    let temporary = destination.with_extension("json.importing");
    let mut bytes = serde_json::to_vec_pretty(promotion).map_err(|error| {
        AppError::Validation(format!("cannot serialise quantization promotion: {error}"))
    })?;
    bytes.push(b'\n');
    if bytes.len() as u64 > MAX_PROMOTION_BYTES {
        return validation_error("normalised quantization promotion is unexpectedly large");
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| {
            AppError::Validation(format!(
                "cannot create promotion sidecar '{}': {error}",
                temporary.display()
            ))
        })?;
    let result = (|| -> Result<(), AppError> {
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, &destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Publish a fully written temporary GGUF without ever replacing a destination.
///
/// `std::fs::rename` replaces an existing path on Unix, so an existence check
/// followed by rename has a race. A hard link gives us an atomic no-clobber
/// publish because both files live in the app's model directory. If another
/// import won the race, accept it only when its complete content still matches
/// the promotion record.
fn publish_temporary_model(
    temporary: &Path,
    destination: &Path,
    record: &QuantizationPromotion,
) -> Result<bool, AppError> {
    match fs::hard_link(temporary, destination) {
        Ok(()) => {
            if let Err(error) = fs::remove_file(temporary) {
                let _ = fs::remove_file(destination);
                return Err(error.into());
            }
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            fs::remove_file(temporary)?;
            let (digest, size, magic) = hash_file(destination)?;
            if magic != *b"GGUF"
                || digest != record.artifact.sha256
                || size != record.artifact.size_bytes
            {
                return validation_error(format!(
                    "refusing to replace concurrently imported model '{}' with different content",
                    destination.display()
                ));
            }
            Ok(false)
        }
        Err(error) => {
            let _ = fs::remove_file(temporary);
            Err(AppError::Filesystem(error))
        }
    }
}

/// Verify and copy a promoted GGUF into RamDoc's model directory.
///
/// The destination is created with `create_new`; an existing different file is
/// never overwritten. A pre-existing file is accepted only after its complete
/// digest and length match the promotion record.
pub fn install_promotion(
    promotion_path: &Path,
    destination_dir: &Path,
    artifact_path: Option<&Path>,
    mut on_progress: impl FnMut(f64),
) -> Result<InstalledPromotion, AppError> {
    let record = read_promotion(promotion_path)?;
    let source = resolve_artifact_source(promotion_path, &record, artifact_path)?;
    let source_metadata = fs::metadata(&source).map_err(|_| {
        AppError::Validation(format!(
            "Could not find '{}' next to the selected promotion record. Place the GGUF in the same folder, or choose the model file in Settings.",
            record.artifact.filename
        ))
    })?;
    if !source_metadata.is_file() {
        return validation_error("promoted GGUF source is not a regular file");
    }
    if source_metadata.len() != record.artifact.size_bytes {
        return validation_error("promoted GGUF size differs from the promotion record");
    }

    fs::create_dir_all(destination_dir)?;
    let destination = destination_dir.join(&record.artifact.filename);
    if destination.exists() {
        let (digest, size, magic) = hash_file_with_progress(
            &destination,
            Some(record.artifact.size_bytes),
            &mut on_progress,
        )?;
        if magic != *b"GGUF"
            || digest != record.artifact.sha256
            || size != record.artifact.size_bytes
        {
            return validation_error(format!(
                "refusing to overwrite existing model '{}' with different content",
                destination.display()
            ));
        }
        write_sidecar(&destination, &record)?;
        return Ok(InstalledPromotion {
            record,
            model_path: destination,
        });
    }

    let temporary = destination.with_extension("gguf.importing");
    let mut source_file = File::open(&source)?;
    let mut destination_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| {
            AppError::Validation(format!(
                "cannot create model import '{}': {error}",
                temporary.display()
            ))
        })?;
    let expected_size = record.artifact.size_bytes;
    let copy_result = (|| -> Result<(String, u64, [u8; 4]), AppError> {
        let mut digest = DigestContext::new(&SHA256);
        let mut total = 0_u64;
        let mut magic = [0_u8; 4];
        let mut magic_len = 0_usize;
        let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
        let mut last_emitted_pct = -1_i32;
        emit_progress(&mut last_emitted_pct, 0.0, &mut on_progress);
        loop {
            let read = source_file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            if magic_len < magic.len() {
                let copy = (magic.len() - magic_len).min(read);
                magic[magic_len..magic_len + copy].copy_from_slice(&buffer[..copy]);
                magic_len += copy;
            }
            total = total.saturating_add(read as u64);
            if total > MAX_ARTIFACT_BYTES {
                return validation_error("promoted GGUF exceeds the 60 GiB import limit");
            }
            digest.update(&buffer[..read]);
            destination_file.write_all(&buffer[..read])?;
            if expected_size > 0 {
                emit_progress(
                    &mut last_emitted_pct,
                    total as f64 / expected_size as f64,
                    &mut on_progress,
                );
            }
        }
        destination_file.sync_all()?;
        emit_progress(&mut last_emitted_pct, 1.0, &mut on_progress);
        Ok((hex::encode(digest.finish().as_ref()), total, magic))
    })();

    let (digest, size, magic) = match copy_result {
        Ok(result) => result,
        Err(error) => {
            drop(destination_file);
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
    };
    drop(destination_file);
    if magic != *b"GGUF" {
        let _ = fs::remove_file(&temporary);
        return validation_error("promoted artifact does not have GGUF magic");
    }
    if digest != record.artifact.sha256 || size != record.artifact.size_bytes {
        let _ = fs::remove_file(&temporary);
        return validation_error("promoted GGUF content differs from the promotion record");
    }
    let published_new = publish_temporary_model(&temporary, &destination, &record)?;
    if let Err(error) = write_sidecar(&destination, &record) {
        if published_new {
            let _ = fs::remove_file(&destination);
        }
        return Err(error);
    }

    Ok(InstalledPromotion {
        record,
        model_path: destination,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn promotion_for(bytes: &[u8]) -> QuantizationPromotion {
        let digest = ring::digest::digest(&SHA256, bytes);
        let categories = REQUIRED_CATEGORIES
            .iter()
            .map(|category| (*category).to_string())
            .collect::<Vec<_>>();
        let regressions = categories
            .iter()
            .map(|category| (category.clone(), 0.0))
            .collect();
        let limits = categories
            .iter()
            .map(|category| (category.clone(), 0.01))
            .collect();
        QuantizationPromotion {
            schema_version: PROMOTION_SCHEMA_VERSION,
            kind: PROMOTION_KIND.to_string(),
            study_id: "unit-study-v1".to_string(),
            display_name: "Clinical mixed-bit unit model".to_string(),
            created_at: "2026-08-17T12:00:00Z".to_string(),
            artifact: PromotedArtifact {
                filename: "clinical-mix.gguf".to_string(),
                sha256: hex::encode(digest.as_ref()),
                size_bytes: bytes.len() as u64,
                base_model_sha256: "a".repeat(64),
                quantization: "RamDoc-Mix-v1".to_string(),
                recipe_sha256: "b".repeat(64),
            },
            evidence: PromotionEvidence {
                study_manifest_sha256: "c".repeat(64),
                held_out_results_sha256: "d".repeat(64),
                llama_cpp_commit: "e".repeat(40),
                categories,
                baseline_artifacts: vec!["q4-standard".to_string()],
            },
            decision: PromotionDecision {
                recommended: true,
                pareto_frontier_expanded: true,
                dominates: vec!["q4-standard".to_string()],
                category_regression_upper_confidence: regressions,
                category_regression_limits: limits,
            },
        }
    }

    fn write_promotion(path: &Path, promotion: &QuantizationPromotion) {
        fs::write(path, serde_json::to_vec_pretty(promotion).unwrap()).unwrap();
    }

    fn install(promotion_path: &Path, destination: &Path) -> Result<InstalledPromotion, AppError> {
        install_promotion(promotion_path, destination, None, |_| {})
    }

    #[test]
    fn validates_complete_pareto_promotion() {
        promotion_for(b"GGUFunit").validate().unwrap();
    }

    #[test]
    fn rejects_a_category_regression_above_its_limit() {
        let mut promotion = promotion_for(b"GGUFunit");
        promotion
            .decision
            .category_regression_upper_confidence
            .insert("dose".to_string(), 0.02);
        let error = promotion.validate().unwrap_err().to_string();
        assert!(error.contains("dose"), "unexpected error: {error}");
    }

    #[test]
    fn rejects_path_traversal_and_non_recommended_records() {
        let mut promotion = promotion_for(b"GGUFunit");
        promotion.artifact.filename = "../model.gguf".to_string();
        assert!(promotion.validate().is_err());

        let mut promotion = promotion_for(b"GGUFunit");
        promotion.decision.recommended = false;
        assert!(promotion.validate().is_err());

        let mut promotion = promotion_for(b"GGUFunit");
        promotion.artifact.recipe_sha256 = "0".repeat(64);
        assert!(promotion.validate().is_err());

        let mut promotion = promotion_for(b"GGUFunit");
        promotion
            .evidence
            .baseline_artifacts
            .push("q4-standard".to_string());
        assert!(promotion.validate().is_err());
    }

    #[test]
    fn install_hashes_copies_and_persists_the_promotion_sidecar() {
        let source = tempdir().unwrap();
        let destination = tempdir().unwrap();
        let bytes = b"GGUFunit-test-model";
        fs::write(source.path().join("clinical-mix.gguf"), bytes).unwrap();
        let promotion_path = source.path().join("promotion.json");
        write_promotion(&promotion_path, &promotion_for(bytes));

        let installed = install(&promotion_path, destination.path()).unwrap();
        assert_eq!(fs::read(&installed.model_path).unwrap(), bytes);
        let summary = promotion_summary_for_model(&installed.model_path)
            .unwrap()
            .unwrap();
        assert_eq!(summary.study_id, "unit-study-v1");
        assert_eq!(summary.dominates, vec!["q4-standard"]);
        verify_promoted_model(&installed.model_path).unwrap();

        fs::write(&installed.model_path, b"GGUFchanged-after-import").unwrap();
        let error = verify_promoted_model(&installed.model_path)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("changed after import"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn tampered_source_is_rejected_without_leaving_a_model() {
        let source = tempdir().unwrap();
        let destination = tempdir().unwrap();
        let approved = b"GGUFapproved";
        fs::write(source.path().join("clinical-mix.gguf"), b"GGUFtampered").unwrap();
        let promotion_path = source.path().join("promotion.json");
        write_promotion(&promotion_path, &promotion_for(approved));

        assert!(install(&promotion_path, destination.path()).is_err());
        assert!(!destination.path().join("clinical-mix.gguf").exists());
    }

    #[test]
    fn inspect_reports_a_missing_sibling_without_hashing() {
        let source = tempdir().unwrap();
        let bytes = b"GGUFunit-preview";
        let promotion_path = source.path().join("promotion.json");
        write_promotion(&promotion_path, &promotion_for(bytes));

        let preview = inspect_promotion(&promotion_path, None).unwrap();
        assert!(!preview.artifact_found);
        assert_eq!(preview.filename, "clinical-mix.gguf");
        assert_eq!(preview.display_name, "Clinical mixed-bit unit model");
    }

    #[test]
    fn install_accepts_an_explicit_artifact_path_outside_the_record_folder() {
        let record_dir = tempdir().unwrap();
        let artifact_dir = tempdir().unwrap();
        let destination = tempdir().unwrap();
        let bytes = b"GGUFunit-separate-gguf";
        let artifact = artifact_dir.path().join("elsewhere.gguf");
        fs::write(&artifact, bytes).unwrap();
        let promotion_path = record_dir.path().join("promotion.json");
        write_promotion(&promotion_path, &promotion_for(bytes));

        let preview = inspect_promotion(&promotion_path, Some(&artifact)).unwrap();
        assert!(preview.artifact_found);
        assert!(preview.artifact_size_matches);

        let mut progress = Vec::new();
        let installed = install_promotion(
            &promotion_path,
            destination.path(),
            Some(&artifact),
            |value| progress.push(value),
        )
        .unwrap();
        assert_eq!(fs::read(&installed.model_path).unwrap(), bytes);
        assert!(
            progress.first().copied() == Some(0.0) && progress.last().copied() == Some(1.0),
            "progress should start at 0 and finish at 1, got {progress:?}"
        );
    }

    #[test]
    fn read_promotion_rejects_an_oversized_record_without_unbounded_read() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("huge.json");
        let oversized = vec![b'{'; (MAX_PROMOTION_BYTES as usize) + 2];
        fs::write(&path, oversized).unwrap();
        let error = read_promotion(&path).unwrap_err().to_string();
        assert!(
            error.contains("no larger than"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn atomic_publish_never_replaces_a_concurrent_destination() {
        let directory = tempdir().unwrap();
        let temporary = directory.path().join("clinical-mix.gguf.importing");
        let destination = directory.path().join("clinical-mix.gguf");
        fs::write(&temporary, b"GGUFapproved").unwrap();
        fs::write(&destination, b"GGUFother-import").unwrap();

        let error =
            publish_temporary_model(&temporary, &destination, &promotion_for(b"GGUFapproved"))
                .unwrap_err()
                .to_string();

        assert!(
            error.contains("concurrently imported"),
            "unexpected error: {error}"
        );
        assert_eq!(fs::read(destination).unwrap(), b"GGUFother-import");
        assert!(!temporary.exists());
    }
}
