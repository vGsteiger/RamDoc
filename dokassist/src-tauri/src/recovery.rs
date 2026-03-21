use crate::constants::{KEYCHAIN_SERVICE, RECOVERY_ATTEMPTS_ACCOUNT};
use crate::error::AppError;
use crate::keychain;
use bip39::{Language, Mnemonic};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

/// App-specific KDF salt — not secret, prevents cross-app key reuse.
const KDF_SALT: &[u8; 16] = b"dokassist-v1-key";

type RecoveryKeys = (Vec<String>, [u8; 32], [u8; 32]);
/// Vault file format version byte.
const VAULT_VERSION: u8 = 1;

/// Attempt thresholds beyond which lockout begins (after the Nth failure).
const LOCKOUT_AFTER: u32 = 3;
/// Maximum lockout duration in seconds (1 hour).
const MAX_LOCKOUT_SECS: u64 = 3600;

#[derive(Serialize, Deserialize, Default)]
struct RecoveryAttemptState {
    count: u32,
    locked_until_secs: u64,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn lockout_duration(count: u32) -> u64 {
    if count <= LOCKOUT_AFTER {
        return 0;
    }
    // Exponential: 2^(count - LOCKOUT_AFTER) seconds, capped at MAX_LOCKOUT_SECS
    let exp = count - LOCKOUT_AFTER;
    let secs = 2u64.saturating_pow(exp);
    secs.min(MAX_LOCKOUT_SECS)
}

/// Read attempt state from macOS Keychain (no biometric required).
/// Falls back to default (zero attempts) if no entry exists yet.
fn read_attempt_state() -> RecoveryAttemptState {
    keychain::retrieve_metadata(KEYCHAIN_SERVICE, RECOVERY_ATTEMPTS_ACCOUNT)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Persist attempt state to macOS Keychain (no biometric required).
fn write_attempt_state(state: &RecoveryAttemptState) {
    if let Ok(json) = serde_json::to_vec(state) {
        let _ = keychain::store_metadata(KEYCHAIN_SERVICE, RECOVERY_ATTEMPTS_ACCOUNT, &json);
    }
}

/// Check whether recovery is currently rate-limited.
/// Returns `Err(AppError::RateLimited(secs))` if locked out, `Ok(())` otherwise.
pub fn check_recovery_rate_limit(_data_dir: &Path) -> Result<(), AppError> {
    let state = read_attempt_state();
    let now = now_secs();
    if state.locked_until_secs > now {
        return Err(AppError::RateLimited(state.locked_until_secs - now));
    }
    Ok(())
}

/// Record a failed recovery attempt and update the lockout state.
pub fn record_failed_attempt(_data_dir: &Path) {
    let mut state = read_attempt_state();
    state.count = state.count.saturating_add(1);
    let lockout = lockout_duration(state.count);
    state.locked_until_secs = if lockout > 0 { now_secs() + lockout } else { 0 };
    write_attempt_state(&state);
    log::warn!(
        "Recovery attempt {} failed. Lockout: {}s",
        state.count,
        lockout
    );
}

/// Clear the recovery attempt counter after a successful recovery.
pub fn clear_recovery_attempts(_data_dir: &Path) {
    let _ = keychain::delete_metadata(KEYCHAIN_SERVICE, RECOVERY_ATTEMPTS_ACCOUNT);
}

/// Derive db_key and fs_key deterministically from 32-byte mnemonic entropy.
///
/// Uses Argon2id (64 MiB, 3 iterations, parallelism 4) so that brute-forcing
/// the 24-word phrase is memory-hard even if the vault marker is obtained.
fn derive_keys_from_entropy(entropy: &[u8; 32]) -> Result<([u8; 32], [u8; 32]), AppError> {
    use argon2::{Algorithm, Argon2, Params, Version};
    let params = Params::new(65536, 3, 4, Some(64))
        .map_err(|e| AppError::Crypto(format!("Argon2 params: {}", e)))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut derived = [0u8; 64];
    argon2
        .hash_password_into(entropy, KDF_SALT, &mut derived)
        .map_err(|e| AppError::Crypto(format!("Key derivation failed: {}", e)))?;
    let mut db_key = [0u8; 32];
    let mut fs_key = [0u8; 32];
    db_key.copy_from_slice(&derived[..32]);
    fs_key.copy_from_slice(&derived[32..]);
    derived.zeroize();
    Ok((db_key, fs_key))
}

/// Generate a BIP-39 24-word mnemonic and derive db_key / fs_key from it.
///
/// Returns `(mnemonic_words, db_key, fs_key)` and writes a 1-byte vault marker
/// to `vault_path`. The marker's existence signals "app initialized" to `state.rs`;
/// its version byte enables future vault-format migrations.
pub fn create_recovery(vault_path: &Path) -> Result<RecoveryKeys, AppError> {
    // Generate 256 bits of entropy for a 24-word mnemonic
    let mut entropy = [0u8; 32];
    rand::rng().fill(&mut entropy);

    // Create mnemonic from entropy
    let mnemonic = Mnemonic::from_entropy(&entropy)
        .map_err(|e| AppError::Crypto(format!("Failed to generate mnemonic: {}", e)))?;

    // Derive db_key and fs_key from entropy
    let (db_key, fs_key) = derive_keys_from_entropy(&entropy)?;

    // Zeroize entropy after derivation
    entropy.zeroize();

    // Write 1-byte vault marker (signals "app initialized" to state.rs)
    fs::write(vault_path, [VAULT_VERSION]).map_err(|e| {
        AppError::Filesystem(std::io::Error::new(
            e.kind(),
            format!("Failed to write recovery vault: {}", e),
        ))
    })?;

    let words: Vec<String> = mnemonic.words().map(|w: &str| w.to_string()).collect();
    Ok((words, db_key, fs_key))
}

/// Recover master keys from a 24-word mnemonic phrase.
///
/// `vault_path` must exist and contain the correct version byte — its presence
/// signals that the app has been initialized. Keys are re-derived deterministically
/// via Argon2id; no encrypted payload is needed, so recovery works even if the
/// data directory was wiped (as long as the user has the 24 words).
pub fn recover_from_mnemonic(
    words: &[String],
    vault_path: &Path,
) -> Result<([u8; 32], [u8; 32]), AppError> {
    // Reconstruct mnemonic from words
    let mut mnemonic_string = words.join(" ");
    let mnemonic = Mnemonic::parse_in(Language::English, &mnemonic_string)
        .map_err(|_| AppError::InvalidRecoveryPhrase)?;

    // Zeroize mnemonic string after parsing
    mnemonic_string.zeroize();

    // Get entropy from mnemonic
    let mut entropy_vec = mnemonic.to_entropy();

    // Verify we have 32 bytes of entropy (24-word mnemonic = 256 bits)
    if entropy_vec.len() != 32 {
        entropy_vec.zeroize();
        return Err(AppError::Crypto("Invalid mnemonic entropy".to_string()));
    }

    let mut entropy = [0u8; 32];
    entropy.copy_from_slice(&entropy_vec);
    entropy_vec.zeroize();

    // Read vault marker and verify version byte
    let vault_bytes = fs::read(vault_path).map_err(|e| {
        AppError::Filesystem(std::io::Error::new(
            e.kind(),
            format!("Failed to read recovery vault: {}", e),
        ))
    })?;
    if vault_bytes.first() != Some(&VAULT_VERSION) {
        entropy.zeroize();
        return Err(AppError::InvalidRecoveryPhrase);
    }

    // Derive keys from entropy and return
    let result = derive_keys_from_entropy(&entropy);
    entropy.zeroize();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_and_recover() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().join("recovery.vault");

        // Create recovery — keys are derived from mnemonic entropy
        let (words, db_key, fs_key) = create_recovery(&vault_path).unwrap();

        // Verify we got 24 words
        assert_eq!(words.len(), 24);

        // Verify vault marker was written
        assert!(vault_path.exists());

        // Recover keys using only the mnemonic and vault marker
        let (recovered_db_key, recovered_fs_key) =
            recover_from_mnemonic(&words, &vault_path).unwrap();

        // Derivation is deterministic: same words → same keys
        assert_eq!(db_key, recovered_db_key);
        assert_eq!(fs_key, recovered_fs_key);
    }

    #[test]
    fn test_recover_wrong_mnemonic_gives_different_keys() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().join("recovery.vault");

        // Create recovery with one mnemonic
        let (_, db_key, _) = create_recovery(&vault_path).unwrap();

        // Build a different valid mnemonic
        let mut wrong_entropy = [0u8; 32];
        rand::rng().fill(&mut wrong_entropy);
        let wrong_mnemonic = bip39::Mnemonic::from_entropy(&wrong_entropy).unwrap();
        let wrong_words: Vec<String> = wrong_mnemonic
            .words()
            .map(|w: &str| w.to_string())
            .collect();

        // Recovery succeeds (valid mnemonic + existing vault), but keys differ
        let (wrong_db_key, _) = recover_from_mnemonic(&wrong_words, &vault_path).unwrap();
        assert_ne!(db_key, wrong_db_key);
    }

    #[test]
    fn test_recover_invalid_mnemonic() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().join("recovery.vault");

        // Create a valid vault marker
        std::fs::write(&vault_path, [VAULT_VERSION]).unwrap();

        // Try to recover with invalid mnemonic words
        let invalid_words = vec!["invalid".to_string(); 24];

        let result = recover_from_mnemonic(&invalid_words, &vault_path);

        // Should fail with InvalidRecoveryPhrase (BIP-39 parse error)
        assert!(result.is_err());
    }

    #[test]
    fn test_recover_missing_vault() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().join("nonexistent.vault");

        // Generate valid mnemonic
        let mut entropy = [0u8; 32];
        rand::rng().fill(&mut entropy);
        let mnemonic = bip39::Mnemonic::from_entropy(&entropy).unwrap();
        let words: Vec<String> = mnemonic.words().map(|w: &str| w.to_string()).collect();

        let result = recover_from_mnemonic(&words, &vault_path);

        // Should fail because vault file doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_recover_wrong_vault_version() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().join("recovery.vault");

        // Write a vault with an unknown version byte
        std::fs::write(&vault_path, [0xFFu8]).unwrap();

        // Generate valid mnemonic
        let mut entropy = [0u8; 32];
        rand::rng().fill(&mut entropy);
        let mnemonic = bip39::Mnemonic::from_entropy(&entropy).unwrap();
        let words: Vec<String> = mnemonic.words().map(|w: &str| w.to_string()).collect();

        let result = recover_from_mnemonic(&words, &vault_path);

        // Should fail with InvalidRecoveryPhrase (version mismatch)
        assert!(matches!(result, Err(AppError::InvalidRecoveryPhrase)));
    }
}
