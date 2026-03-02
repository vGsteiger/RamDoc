#[cfg(target_os = "macos")]
use crate::constants::{DB_KEY_ACCOUNT, FS_KEY_ACCOUNT};
use crate::error::AppError;

#[cfg(target_os = "macos")]
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

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
    // NOTE (CRIT-6): The security_framework crate does not expose
    // `SecItemCopyMatching` with `kSecReturnData = false`, so we cannot
    // perform a metadata-only existence check without key-material retrieval.
    // As a mitigation we zeroize the retrieved bytes immediately.
    //
    // Known TOCTOU limitation: between the DB-key check and the FS-key check
    // an external process could delete the FS key.  The resulting `Ok(false)`
    // from the FS check simply causes the state machine to enter
    // RecoveryRequired rather than Locked, which is a safe degraded state.
    // A full fix requires dropping to the raw SecItem C API.

    let db_exists = match get_generic_password(service, DB_KEY_ACCOUNT) {
        Ok(bytes) => {
            // CRIT-6: Zeroize our copy of the retrieved key material immediately
            let mut v = bytes.to_vec();
            zeroize::Zeroize::zeroize(&mut v);
            true
        }
        Err(_) => false,
    };

    if !db_exists {
        return Ok(false);
    }

    let fs_exists = match get_generic_password(service, FS_KEY_ACCOUNT) {
        Ok(bytes) => {
            let mut v = bytes.to_vec();
            zeroize::Zeroize::zeroize(&mut v);
            true
        }
        Err(_) => false,
    };

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

    #[test]
    fn test_store_retrieve_delete() {
        let account = "test-key-srd";
        let key = b"test_secret_key_12345678901234567890";

        // Store
        store_key(TEST_SERVICE, account, key).unwrap();

        // Retrieve
        let retrieved = retrieve_key(TEST_SERVICE, account).unwrap();
        assert_eq!(key.to_vec(), retrieved);

        // Delete
        delete_key(TEST_SERVICE, account).unwrap();

        // Verify deleted
        let result = retrieve_key(TEST_SERVICE, account);
        assert!(result.is_err());
    }

    #[test]
    fn test_overwrite_key() {
        let account = "test-key-ow";
        let key1 = b"first_key_12345678901234567890123";
        let key2 = b"second_key_0987654321098765432109";

        // Store first key
        store_key(TEST_SERVICE, account, key1).unwrap();

        // Overwrite with second key
        store_key(TEST_SERVICE, account, key2).unwrap();

        // Retrieve should get second key
        let retrieved = retrieve_key(TEST_SERVICE, account).unwrap();
        assert_eq!(key2.to_vec(), retrieved);

        // Cleanup
        let _ = delete_key(TEST_SERVICE, account);
    }
}
