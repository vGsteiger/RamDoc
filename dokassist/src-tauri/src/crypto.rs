use crate::error::AppError;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

/// Generate a cryptographically random 256-bit key
pub fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

/// AES-256-GCM encrypt. Returns: [12-byte nonce || ciphertext || 16-byte tag]
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, AppError> {
    let cipher = Aes256Gcm::new(key.into());

    // Generate random 12-byte nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| AppError::Crypto(format!("Encryption failed: {}", e)))?;

    // Format: [nonce || ciphertext+tag]
    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// AES-256-GCM decrypt. Input format: [12-byte nonce || ciphertext || 16-byte tag]
pub fn decrypt(key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>, AppError> {
    if ciphertext.len() < 12 {
        return Err(AppError::Crypto("Ciphertext too short".to_string()));
    }

    let cipher = Aes256Gcm::new(key.into());

    // Extract nonce (first 12 bytes)
    let nonce = Nonce::from_slice(&ciphertext[..12]);

    // Decrypt remainder (ciphertext + tag)
    let plaintext = cipher
        .decrypt(nonce, &ciphertext[12..])
        .map_err(|e| AppError::Crypto(format!("Decryption failed: {}", e)))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key() {
        let key1 = generate_key();
        let key2 = generate_key();
        assert_eq!(key1.len(), 32);
        assert_eq!(key2.len(), 32);
        assert_ne!(key1, key2, "Keys should be random");
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = generate_key();
        let plaintext = b"Hello, World!";

        let ciphertext = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &ciphertext).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_encrypt_decrypt_empty() {
        let key = generate_key();
        let plaintext = b"";

        let ciphertext = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &ciphertext).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_encrypt_decrypt_large() {
        let key = generate_key();
        let plaintext = vec![42u8; 1024 * 1024]; // 1 MB

        let ciphertext = encrypt(&key, &plaintext).unwrap();
        let decrypted = decrypt(&key, &ciphertext).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_decrypt_wrong_key() {
        let key1 = generate_key();
        let key2 = generate_key();
        let plaintext = b"Secret message";

        let ciphertext = encrypt(&key1, plaintext).unwrap();
        let result = decrypt(&key2, &ciphertext);

        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_corrupted() {
        let key = generate_key();
        let plaintext = b"Secret message";

        let mut ciphertext = encrypt(&key, plaintext).unwrap();
        ciphertext[20] ^= 0xFF; // Corrupt one byte

        let result = decrypt(&key, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_too_short() {
        let key = generate_key();
        let ciphertext = vec![1, 2, 3];

        let result = decrypt(&key, &ciphertext);
        assert!(result.is_err());
    }
}
