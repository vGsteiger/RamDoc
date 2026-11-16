use crate::error::AppError;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use aes_gcm_siv::{aead::Payload, Aes256GcmSiv, Nonce as SivNonce};
use rand::RngExt;

const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const CIPHERTEXT_MAGIC: &[u8; 6] = b"RAMDOC";

// Version 1, algorithm 1 (AES-256-GCM-SIV). Besides identifying the format,
// this header keeps new ciphertext distinguishable from legacy AES-256-GCM.
const CIPHERTEXT_HEADER: &[u8; 8] = b"RAMDOC\x01\x01";

/// Generate a cryptographically random 256-bit key
pub fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::rng().fill(&mut key);
    key
}

/// Encrypt using AES-256-GCM-SIV.
///
/// Returns: [8-byte format header || 12-byte nonce || ciphertext || 16-byte tag].
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, AppError> {
    let cipher = Aes256GcmSiv::new(key.into());

    // GCM-SIV remains secure if a nonce is accidentally reused, while random
    // nonces keep repeated plaintexts from producing repeated ciphertexts.
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill(&mut nonce_bytes);
    let nonce = SivNonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad: CIPHERTEXT_HEADER,
            },
        )
        .map_err(|e| AppError::Crypto(format!("Encryption failed: {}", e)))?;

    let mut result = Vec::with_capacity(CIPHERTEXT_HEADER.len() + NONCE_LEN + ciphertext.len());
    result.extend_from_slice(CIPHERTEXT_HEADER);
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Decrypt current AES-256-GCM-SIV ciphertexts and legacy AES-256-GCM data.
pub fn decrypt(key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>, AppError> {
    if ciphertext.starts_with(CIPHERTEXT_HEADER) {
        return decrypt_gcm_siv(key, ciphertext);
    }
    if ciphertext.starts_with(CIPHERTEXT_MAGIC) {
        // Legacy ciphertext begins with a random nonce, which can coincidentally
        // share the reserved magic prefix. Preserve compatibility by accepting
        // it when legacy authentication succeeds.
        return decrypt_legacy_gcm(key, ciphertext)
            .map_err(|_| AppError::Crypto("Unsupported encrypted data format".to_string()));
    }

    decrypt_legacy_gcm(key, ciphertext)
}

fn decrypt_gcm_siv(key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>, AppError> {
    let payload_offset = CIPHERTEXT_HEADER.len() + NONCE_LEN;
    if ciphertext.len() < payload_offset + TAG_LEN {
        return Err(AppError::Crypto("Ciphertext too short".to_string()));
    }

    let cipher = Aes256GcmSiv::new(key.into());
    let nonce_bytes: [u8; NONCE_LEN] = ciphertext[CIPHERTEXT_HEADER.len()..payload_offset]
        .try_into()
        .map_err(|_| AppError::Crypto("Invalid nonce length".to_string()))?;
    let nonce = SivNonce::from(nonce_bytes);

    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: &ciphertext[payload_offset..],
                aad: CIPHERTEXT_HEADER,
            },
        )
        .map_err(|e| AppError::Crypto(format!("Decryption failed: {}", e)))
}

fn decrypt_legacy_gcm(key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>, AppError> {
    if ciphertext.len() < NONCE_LEN + TAG_LEN {
        return Err(AppError::Crypto("Ciphertext too short".to_string()));
    }

    let cipher = Aes256Gcm::new(key.into());

    let nonce = Nonce::try_from(&ciphertext[..NONCE_LEN])
        .map_err(|_| AppError::Crypto("Invalid nonce length".to_string()))?;

    cipher
        .decrypt(&nonce, &ciphertext[NONCE_LEN..])
        .map_err(|e| AppError::Crypto(format!("Decryption failed: {}", e)))
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
        assert!(ciphertext.starts_with(CIPHERTEXT_HEADER));
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

    #[test]
    fn test_decrypt_legacy_aes_gcm_ciphertext() {
        let key = generate_key();
        let plaintext = b"Data encrypted before the GCM-SIV migration";
        let nonce_bytes = [7u8; NONCE_LEN];
        let cipher = Aes256Gcm::new((&key).into());
        let nonce = Nonce::try_from(nonce_bytes.as_slice()).unwrap();
        let encrypted = cipher.encrypt(&nonce, plaintext.as_slice()).unwrap();

        let mut legacy_ciphertext = Vec::with_capacity(NONCE_LEN + encrypted.len());
        legacy_ciphertext.extend_from_slice(&nonce_bytes);
        legacy_ciphertext.extend_from_slice(&encrypted);

        assert_eq!(decrypt(&key, &legacy_ciphertext).unwrap(), plaintext);
    }

    #[test]
    fn test_decrypt_legacy_ciphertext_with_reserved_magic_nonce_prefix() {
        let key = generate_key();
        let plaintext = b"Legacy data whose nonce starts with the format magic";
        let mut nonce_bytes = [7u8; NONCE_LEN];
        nonce_bytes[..CIPHERTEXT_MAGIC.len()].copy_from_slice(CIPHERTEXT_MAGIC);
        assert!(nonce_bytes.starts_with(CIPHERTEXT_MAGIC));
        assert!(!nonce_bytes.starts_with(CIPHERTEXT_HEADER));

        let cipher = Aes256Gcm::new((&key).into());
        let nonce = Nonce::try_from(nonce_bytes.as_slice()).unwrap();
        let encrypted = cipher.encrypt(&nonce, plaintext.as_slice()).unwrap();

        let mut legacy_ciphertext = Vec::with_capacity(NONCE_LEN + encrypted.len());
        legacy_ciphertext.extend_from_slice(&nonce_bytes);
        legacy_ciphertext.extend_from_slice(&encrypted);

        assert_eq!(decrypt(&key, &legacy_ciphertext).unwrap(), plaintext);
    }

    #[test]
    fn test_current_format_header_is_authenticated() {
        let key = generate_key();
        let mut ciphertext = encrypt(&key, b"Secret message").unwrap();
        ciphertext[CIPHERTEXT_HEADER.len() - 1] ^= 1;

        assert!(decrypt(&key, &ciphertext).is_err());
    }
}
