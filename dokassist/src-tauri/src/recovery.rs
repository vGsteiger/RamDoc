use crate::crypto;
use crate::error::AppError;
use bip39::{Language, Mnemonic};
use rand::RngCore;
use std::fs;
use std::path::Path;
use zeroize::Zeroize;

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
    let words: Vec<String> = mnemonic
        .word_iter()
        .map(|w: &str| w.to_string())
        .collect();

    Ok(words)
}

/// Recover master keys from a mnemonic + recovery.vault file.
/// Returns (db_key, fs_key).
pub fn recover_from_mnemonic(
    words: &[String],
    vault_path: &Path,
) -> Result<([u8; 32], [u8; 32]), AppError> {
    // Reconstruct mnemonic from words
    let mnemonic_string = words.join(" ");
    let mnemonic = Mnemonic::parse_in(Language::English, &mnemonic_string)
        .map_err(|_| AppError::InvalidRecoveryPhrase)?;

    // Get entropy from mnemonic
    let entropy = mnemonic.to_entropy();

    // Verify we have 32 bytes of entropy
    if entropy.len() != 32 {
        return Err(AppError::Crypto("Invalid mnemonic entropy".to_string()));
    }

    // Derive recovery key from entropy
    let mut recovery_key = [0u8; 32];
    recovery_key.copy_from_slice(&entropy);

    // Read encrypted vault
    let encrypted_vault = fs::read(vault_path).map_err(|e| {
        AppError::Filesystem(std::io::Error::new(
            e.kind(),
            format!("Failed to read recovery vault: {}", e),
        ))
    })?;

    // Decrypt vault
    let vault_plaintext = crypto::decrypt(&recovery_key, &encrypted_vault)?;

    // Zeroize recovery key
    recovery_key.zeroize();

    // Verify vault has correct length (64 bytes = 2 × 32-byte keys)
    if vault_plaintext.len() != 64 {
        return Err(AppError::Crypto(
            "Invalid recovery vault format".to_string(),
        ));
    }

    // Extract keys
    let mut db_key = [0u8; 32];
    let mut fs_key = [0u8; 32];
    db_key.copy_from_slice(&vault_plaintext[..32]);
    fs_key.copy_from_slice(&vault_plaintext[32..]);

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
            .word_iter()
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
        let words: Vec<String> = mnemonic.word_iter().map(|w: &str| w.to_string()).collect();

        let result = recover_from_mnemonic(&words, &vault_path);

        // Should fail because vault file doesn't exist
        assert!(result.is_err());
    }
}
