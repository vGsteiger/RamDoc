use crate::constants::{DB_KEY_ACCOUNT, FS_KEY_ACCOUNT};
use crate::error::AppError;

#[cfg(target_os = "macos")]
use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};

/// Store a key in macOS Keychain.
///
/// Note: This uses the default security attributes provided by `set_generic_password`.
/// It does not explicitly set `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` or
/// `kSecAccessControlBiometryCurrentSet` flags, as these require lower-level APIs
/// not exposed by the current version of the security-framework crate.
#[cfg(target_os = "macos")]
pub fn store_key(service: &str, account: &str, key: &[u8]) -> Result<(), AppError> {
    // Delete existing key if present
    let _ = delete_generic_password(service, account);

    // Store new key
    set_generic_password(service, account, key)
        .map_err(|e| AppError::Keychain(format!("Failed to store key: {}", e)))?;

    Ok(())
}

/// Retrieve a key from Keychain.
///
/// Note: This may trigger authentication (Touch ID or password prompt) depending on
/// the keychain item's access control settings and the current device state.
#[cfg(target_os = "macos")]
pub fn retrieve_key(service: &str, account: &str) -> Result<Vec<u8>, AppError> {
    let password = get_generic_password(service, account)
        .map_err(|e| AppError::Keychain(format!("Failed to retrieve key: {}", e)))?;

    Ok(password.to_vec())
}

/// Delete a key from Keychain.
#[cfg(target_os = "macos")]
pub fn delete_key(service: &str, account: &str) -> Result<(), AppError> {
    delete_generic_password(service, account)
        .map_err(|e| AppError::Keychain(format!("Failed to delete key: {}", e)))?;

    Ok(())
}

/// Check if keys exist in the Keychain.
///
/// NOTE: This helper may trigger biometric/password authentication because
/// it uses `get_generic_password` under the hood, which can require user
/// authentication for access-control-protected items. It also reads the key
/// material into memory even though only existence is checked.
///
/// If you need a metadata-only existence check that never prompts the user
/// and does not read key material, you should implement it using lower-level
/// Keychain APIs (e.g. `SecItemCopyMatching` with `kSecReturnData = false`).
#[cfg(target_os = "macos")]
pub fn keys_exist(service: &str) -> Result<bool, AppError> {
    // Try to check if keys exist by attempting to retrieve them
    // This is a simplified implementation; see the note above for a
    // production-ready, metadata-only approach.

    let db_exists = get_generic_password(service, DB_KEY_ACCOUNT).is_ok();
    if !db_exists {
        // If the DB key is missing, we can short-circuit without checking
        // the FS key.
        return Ok(false);
    }

    let fs_exists = get_generic_password(service, FS_KEY_ACCOUNT).is_ok();

    Ok(fs_exists)
}

// Non-macOS stubs
#[cfg(not(target_os = "macos"))]
pub fn store_key(_service: &str, _account: &str, _key: &[u8]) -> Result<(), AppError> {
    Err(AppError::Keychain(
        "Keychain operations are only supported on macOS".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
pub fn retrieve_key(_service: &str, _account: &str) -> Result<Vec<u8>, AppError> {
    Err(AppError::Keychain(
        "Keychain operations are only supported on macOS".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
pub fn delete_key(_service: &str, _account: &str) -> Result<(), AppError> {
    Err(AppError::Keychain(
        "Keychain operations are only supported on macOS".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
pub fn keys_exist(_service: &str) -> Result<bool, AppError> {
    Err(AppError::Keychain(
        "Keychain operations are only supported on macOS".to_string(),
    ))
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    const TEST_SERVICE: &str = "ch.dokassist.app.test";
    const TEST_ACCOUNT: &str = "test-key";

    #[test]
    fn test_store_retrieve_delete() {
        let key = b"test_secret_key_12345678901234567890";

        // Store
        store_key(TEST_SERVICE, TEST_ACCOUNT, key).unwrap();

        // Retrieve
        let retrieved = retrieve_key(TEST_SERVICE, TEST_ACCOUNT).unwrap();
        assert_eq!(key.to_vec(), retrieved);

        // Delete
        delete_key(TEST_SERVICE, TEST_ACCOUNT).unwrap();

        // Verify deleted
        let result = retrieve_key(TEST_SERVICE, TEST_ACCOUNT);
        assert!(result.is_err());
    }

    #[test]
    fn test_overwrite_key() {
        let key1 = b"first_key_12345678901234567890123";
        let key2 = b"second_key_0987654321098765432109";

        // Store first key
        store_key(TEST_SERVICE, TEST_ACCOUNT, key1).unwrap();

        // Overwrite with second key
        store_key(TEST_SERVICE, TEST_ACCOUNT, key2).unwrap();

        // Retrieve should get second key
        let retrieved = retrieve_key(TEST_SERVICE, TEST_ACCOUNT).unwrap();
        assert_eq!(key2.to_vec(), retrieved);

        // Cleanup
        let _ = delete_key(TEST_SERVICE, TEST_ACCOUNT);
    }
}
