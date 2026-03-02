use crate::crypto;
use crate::error::AppError;
use bip39::{Language, Mnemonic};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

const ATTEMPTS_FILENAME: &str = "recovery.attempts";
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

fn read_attempt_state(data_dir: &Path) -> RecoveryAttemptState {
    let path = data_dir.join(ATTEMPTS_FILENAME);
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_attempt_state(data_dir: &Path, state: &RecoveryAttemptState) {
    let path = data_dir.join(ATTEMPTS_FILENAME);
    if let Ok(json) = serde_json::to_string(state) {
        let _ = fs::write(&path, json);
    }
}

/// Check whether recovery is currently rate-limited.
/// Returns `Err(AppError::RateLimited(secs))` if locked out, `Ok(())` otherwise.
pub fn check_recovery_rate_limit(data_dir: &Path) -> Result<(), AppError> {
    let state = read_attempt_state(data_dir);
    let now = now_secs();
    if state.locked_until_secs > now {
        return Err(AppError::RateLimited(state.locked_until_secs - now));
    }
    Ok(())
}

/// Record a failed recovery attempt and update the lockout state.
pub fn record_failed_attempt(data_dir: &Path) {
    let mut state = read_attempt_state(data_dir);
    state.count = state.count.saturating_add(1);
    let lockout = lockout_duration(state.count);
    state.locked_until_secs = if lockout > 0 {
        now_secs() + lockout
    } else {
        0
    };
    write_attempt_state(data_dir, &state);
    log::warn!(
        "Recovery attempt {} failed. Lockout: {}s",
        state.count,
        lockout
    );
}

/// Clear the recovery attempt counter after a successful recovery.
pub fn clear_recovery_attempts(data_dir: &Path) {
    let path = data_dir.join(ATTEMPTS_FILENAME);
    let _ = fs::remove_file(&path);
}

/// Generate a BIP-39 24-word mnemonic from the master keys.
/// Returns the mnemonic words AND writes recovery.vault to the given path.
pub fn create_recovery(
    db_key: &[u8; 32],
    fs_key: &[u8; 32],
    vault_path: &Path,
) -> Result<Vec<String>, AppError> {
    // Generate 256 bits of entropy for a 24-word mnemonic
    let mut entropy = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut entropy);

    // Create mnemonic from entropy
    let mnemonic = Mnemonic::from_entropy(&entropy)
        .map_err(|e| AppError::Crypto(format!("Failed to generate mnemonic: {}", e)))?;

    // Use entropy as encryption key for recovery vault
    let mut recovery_key = [0u8; 32];
    recovery_key.copy_from_slice(&entropy);
    entropy.zeroize();

    // Create recovery vault data: [db_key || fs_key] = 64 bytes
    let mut vault_plaintext = Vec::with_capacity(64);
    vault_plaintext.extend_from_slice(db_key);
    vault_plaintext.extend_from_slice(fs_key);

    // Encrypt the vault
    let encrypted_vault = crypto::encrypt(&recovery_key, &vault_plaintext)?;

    // Zeroize sensitive data
    vault_plaintext.zeroize();
    recovery_key.zeroize();

    // Write encrypted vault to file
    fs::write(vault_path, &encrypted_vault).map_err(|e| {
        AppError::Filesystem(std::io::Error::new(
            e.kind(),
            format!("Failed to write recovery vault: {}", e),
        ))
    })?;

    // Return mnemonic words
    let words: Vec<String> = mnemonic.words().map(|w: &str| w.to_string()).collect();

    Ok(words)
}

/// Recover master keys from a mnemonic + recovery.vault file.
/// Returns (db_key, fs_key).
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
    let mut entropy = mnemonic.to_entropy();

    // Verify we have 32 bytes of entropy
    if entropy.len() != 32 {
        entropy.zeroize();
        return Err(AppError::Crypto("Invalid mnemonic entropy".to_string()));
    }

    // Derive recovery key from entropy
    let mut recovery_key = [0u8; 32];
    recovery_key.copy_from_slice(&entropy);

    // Zeroize entropy after copying
    entropy.zeroize();

    // Read encrypted vault
    let encrypted_vault = fs::read(vault_path).map_err(|e| {
        AppError::Filesystem(std::io::Error::new(
            e.kind(),
            format!("Failed to read recovery vault: {}", e),
        ))
    })?;

    // Decrypt vault
    let mut vault_plaintext = crypto::decrypt(&recovery_key, &encrypted_vault)?;

    // Zeroize recovery key
    recovery_key.zeroize();

    // Verify vault has correct length (64 bytes = 2 × 32-byte keys)
    if vault_plaintext.len() != 64 {
        vault_plaintext.zeroize();
        return Err(AppError::Crypto(
            "Invalid recovery vault format".to_string(),
        ));
    }

    // Extract keys
    let mut db_key = [0u8; 32];
    let mut fs_key = [0u8; 32];
    db_key.copy_from_slice(&vault_plaintext[..32]);
    fs_key.copy_from_slice(&vault_plaintext[32..]);

    // Zeroize vault plaintext after extraction
    vault_plaintext.zeroize();

    Ok((db_key, fs_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_create_and_recover() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().join("recovery.vault");

        // Generate test keys
        let db_key = crate::crypto::generate_key();
        let fs_key = crate::crypto::generate_key();

        // Create recovery
        let words = create_recovery(&db_key, &fs_key, &vault_path).unwrap();

        // Verify we got 24 words
        assert_eq!(words.len(), 24);

        // Verify vault file was created
        assert!(vault_path.exists());

        // Recover keys
        let (recovered_db_key, recovered_fs_key) =
            recover_from_mnemonic(&words, &vault_path).unwrap();

        // Verify keys match
        assert_eq!(db_key, recovered_db_key);
        assert_eq!(fs_key, recovered_fs_key);
    }

    #[test]
    fn test_recover_wrong_mnemonic() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().join("recovery.vault");

        // Generate test keys and create recovery
        let db_key = crate::crypto::generate_key();
        let fs_key = crate::crypto::generate_key();
        let _words = create_recovery(&db_key, &fs_key, &vault_path).unwrap();

        // Try to recover with different mnemonic
        let mut wrong_entropy = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut wrong_entropy);
        let wrong_mnemonic = Mnemonic::from_entropy(&wrong_entropy).unwrap();
        let wrong_words: Vec<String> = wrong_mnemonic
            .words()
            .map(|w: &str| w.to_string())
            .collect();

        let result = recover_from_mnemonic(&wrong_words, &vault_path);

        // Should fail because decryption will fail with wrong key
        assert!(result.is_err());
    }

    #[test]
    fn test_recover_invalid_mnemonic() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().join("recovery.vault");

        // Create a dummy vault file
        fs::write(&vault_path, b"dummy data").unwrap();

        // Try to recover with invalid mnemonic
        let invalid_words = vec!["invalid".to_string(); 24];

        let result = recover_from_mnemonic(&invalid_words, &vault_path);

        // Should fail with InvalidRecoveryPhrase
        assert!(result.is_err());
    }

    #[test]
    fn test_recover_missing_vault() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().join("nonexistent.vault");

        // Generate valid mnemonic
        let mut entropy = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut entropy);
        let mnemonic = Mnemonic::from_entropy(&entropy).unwrap();
        let words: Vec<String> = mnemonic.words().map(|w: &str| w.to_string()).collect();

        let result = recover_from_mnemonic(&words, &vault_path);

        // Should fail because vault file doesn't exist
        assert!(result.is_err());
    }
}
