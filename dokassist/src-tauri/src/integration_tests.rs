#[cfg(test)]
mod integration_tests {
    use crate::{crypto, recovery};
    use tempfile::TempDir;

    #[test]
    fn test_full_crypto_flow() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().join("recovery.vault");

        // Step 1: Generate master keys
        let db_key = crypto::generate_key();
        let fs_key = crypto::generate_key();

        // Step 2: Create recovery vault
        let mnemonic_words = recovery::create_recovery(&db_key, &fs_key, &vault_path).unwrap();
        assert_eq!(mnemonic_words.len(), 24);
        assert!(vault_path.exists());

        // Step 3: Test encryption/decryption with db_key
        let plaintext = b"Sensitive patient data";
        let ciphertext = crypto::encrypt(&db_key, plaintext).unwrap();
        let decrypted = crypto::decrypt(&db_key, &ciphertext).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());

        // Step 4: Simulate losing keys and recovering from mnemonic
        let (recovered_db_key, recovered_fs_key) =
            recovery::recover_from_mnemonic(&mnemonic_words, &vault_path).unwrap();

        assert_eq!(db_key, recovered_db_key);
        assert_eq!(fs_key, recovered_fs_key);

        // Step 5: Verify recovered key can decrypt data
        let decrypted_with_recovered = crypto::decrypt(&recovered_db_key, &ciphertext).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted_with_recovered.as_slice());
    }

    #[test]
    fn test_encryption_with_different_keys() {
        let key1 = crypto::generate_key();
        let key2 = crypto::generate_key();
        let data = b"Test data";

        let encrypted_with_key1 = crypto::encrypt(&key1, data).unwrap();

        // Should not be able to decrypt with different key
        let result = crypto::decrypt(&key2, &encrypted_with_key1);
        assert!(result.is_err());

        // Should work with correct key
        let decrypted = crypto::decrypt(&key1, &encrypted_with_key1).unwrap();
        assert_eq!(data.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_large_data_encryption() {
        let key = crypto::generate_key();
        let large_data = vec![42u8; 10 * 1024 * 1024]; // 10 MB

        let encrypted = crypto::encrypt(&key, &large_data).unwrap();
        let decrypted = crypto::decrypt(&key, &encrypted).unwrap();

        assert_eq!(large_data, decrypted);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_keychain_operations() {
        use crate::keychain;

        const TEST_SERVICE: &str = "ch.dokassist.app.integration-test";
        const TEST_ACCOUNT: &str = "integration-test-key";

        // Generate a test key
        let key = crypto::generate_key();

        // Store in keychain
        keychain::store_key(TEST_SERVICE, TEST_ACCOUNT, &key).unwrap();

        // Retrieve from keychain
        let retrieved = keychain::retrieve_key(TEST_SERVICE, TEST_ACCOUNT).unwrap();
        assert_eq!(key.to_vec(), retrieved);

        // Delete from keychain
        keychain::delete_key(TEST_SERVICE, TEST_ACCOUNT).unwrap();

        // Verify deletion
        let result = keychain::retrieve_key(TEST_SERVICE, TEST_ACCOUNT);
        assert!(result.is_err());
    }
}
