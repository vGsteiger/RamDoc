#[cfg(target_os = "macos")]
use crate::constants::{DB_KEY_ACCOUNT, FS_KEY_ACCOUNT};
use crate::error::AppError;

#[cfg(target_os = "macos")]
use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};
#[cfg(target_os = "macos")]
use security_framework_sys::base::errSecItemNotFound;
#[cfg(target_os = "macos")]
use security_framework_sys::access_control::kSecAttrAccessibleWhenUnlockedThisDeviceOnly;
#[cfg(target_os = "macos")]
use security_framework_sys::item::{
    kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword,
    kSecReturnAttributes, kSecValueData,
};
// kSecAttrAccessible (the dict key for accessibility level) is not exported by
// security_framework_sys, so we declare it directly from Security.framework.
#[cfg(target_os = "macos")]
extern "C" {
    static kSecAttrAccessible: core_foundation_sys::string::CFStringRef;
}
#[cfg(target_os = "macos")]
use security_framework_sys::keychain_item::{SecItemAdd, SecItemCopyMatching, SecItemDelete};
#[cfg(target_os = "macos")]
use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
#[cfg(target_os = "macos")]
use core_foundation::boolean::CFBoolean;
#[cfg(target_os = "macos")]
use core_foundation::data::CFData;
#[cfg(target_os = "macos")]
use core_foundation::dictionary::CFDictionary;
#[cfg(target_os = "macos")]
use core_foundation::string::CFString;

/// Store a key in macOS Keychain with device-bound protection.
///
/// Uses `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`: the item is accessible
/// only while the device is unlocked and is never synced to iCloud.
/// No `SecAccessControl` object is used, so no entitlement is required —
/// `SecAccessControl` (even with no flags) requires a signed app bundle.
///
/// Touch ID enforcement can be added via `LocalAuthentication` once a proper
/// Apple Developer ID signing identity is in place.
#[cfg(target_os = "macos")]
pub fn store_key(service: &str, account: &str, key: &[u8]) -> Result<(), AppError> {
    // Delete any existing item first using a raw SecItemDelete query so we match
    // items regardless of how they were originally stored (the high-level
    // delete_generic_password builds a different query and can miss items stored
    // via SecItemAdd with kSecAttrAccessibleWhenUnlockedThisDeviceOnly).
    let del_query = CFDictionary::<CFString, _>::from_CFType_pairs(&[
        (
            unsafe { CFString::wrap_under_get_rule(kSecClass) },
            unsafe { CFString::wrap_under_get_rule(kSecClassGenericPassword) }.as_CFType(),
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecAttrService) },
            CFString::new(service).as_CFType(),
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecAttrAccount) },
            CFString::new(account).as_CFType(),
        ),
    ]);
    unsafe { SecItemDelete(del_query.as_concrete_TypeRef()) };

    let dict = CFDictionary::<CFString, _>::from_CFType_pairs(&[
        (
            unsafe { CFString::wrap_under_get_rule(kSecClass) },
            unsafe { CFString::wrap_under_get_rule(kSecClassGenericPassword) }.as_CFType(),
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecAttrService) },
            CFString::new(service).as_CFType(),
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecAttrAccount) },
            CFString::new(account).as_CFType(),
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecValueData) },
            CFData::from_buffer(key).as_CFType(),
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecAttrAccessible) },
            unsafe { CFString::wrap_under_get_rule(kSecAttrAccessibleWhenUnlockedThisDeviceOnly) }.as_CFType(),
        ),
    ]);

    let status = unsafe { SecItemAdd(dict.as_concrete_TypeRef(), std::ptr::null_mut()) };
    if status != 0 {
        return Err(AppError::Keychain(format!(
            "Failed to store key (OSStatus {})",
            status
        )));
    }

    Ok(())
}

/// Retrieve a key from Keychain.
///
/// Returns the key whenever the device is unlocked (no auth prompt, since
/// items are stored without a biometric access-control flag).
#[cfg(target_os = "macos")]
pub fn retrieve_key(service: &str, account: &str) -> Result<Vec<u8>, AppError> {
    get_generic_password(service, account)
        .map(|p| p.to_vec())
        .map_err(|e| AppError::Keychain(format!("Failed to retrieve key: {}", e)))
}

/// Delete a key from Keychain.
#[cfg(target_os = "macos")]
pub fn delete_key(service: &str, account: &str) -> Result<(), AppError> {
    delete_generic_password(service, account)
        .map_err(|e| AppError::Keychain(format!("Failed to delete key: {}", e)))
}

/// Store non-sensitive metadata in Keychain **without** biometric protection.
///
/// Uses the standard `set_generic_password` API which stores items with
/// `kSecAttrAccessibleAfterFirstUnlock` accessibility — readable after device boot
/// without Touch ID. Intended for data like recovery attempt counters that must be
/// readable before the user has authenticated.
#[cfg(target_os = "macos")]
pub fn store_metadata(service: &str, account: &str, data: &[u8]) -> Result<(), AppError> {
    set_generic_password(service, account, data)
        .map_err(|e| AppError::Keychain(format!("Failed to store metadata: {}", e)))
}

/// Retrieve non-sensitive metadata from Keychain without triggering Touch ID.
#[cfg(target_os = "macos")]
pub fn retrieve_metadata(service: &str, account: &str) -> Result<Vec<u8>, AppError> {
    get_generic_password(service, account)
        .map(|p| p.to_vec())
        .map_err(|e| AppError::Keychain(format!("Failed to retrieve metadata: {}", e)))
}

/// Delete non-sensitive metadata from Keychain.
#[cfg(target_os = "macos")]
pub fn delete_metadata(service: &str, account: &str) -> Result<(), AppError> {
    delete_generic_password(service, account)
        .map_err(|e| AppError::Keychain(format!("Failed to delete metadata: {}", e)))
}

/// Check if both master keys exist in the Keychain WITHOUT triggering Touch ID.
///
/// Uses a `SecItemCopyMatching` query that requests only item attributes
/// (`kSecReturnAttributes = true`), never the secret data. macOS does not
/// require biometric authentication for attribute-only queries, so this
/// function is safe to call at cold-boot state determination.
#[cfg(target_os = "macos")]
pub fn keys_exist(service: &str) -> Result<bool, AppError> {
    for account in [DB_KEY_ACCOUNT, FS_KEY_ACCOUNT] {
        let query = CFDictionary::<CFString, _>::from_CFType_pairs(&[
            (
                unsafe { CFString::wrap_under_get_rule(kSecClass) },
                unsafe { CFString::wrap_under_get_rule(kSecClassGenericPassword) }.as_CFType(),
            ),
            (
                unsafe { CFString::wrap_under_get_rule(kSecAttrService) },
                CFString::new(service).as_CFType(),
            ),
            (
                unsafe { CFString::wrap_under_get_rule(kSecAttrAccount) },
                CFString::new(account).as_CFType(),
            ),
            (
                unsafe { CFString::wrap_under_get_rule(kSecReturnAttributes) },
                CFBoolean::true_value().as_CFType(),
            ),
        ]);

        let mut result: CFTypeRef = std::ptr::null();
        let status =
            unsafe { SecItemCopyMatching(query.as_concrete_TypeRef(), &mut result) };

        // Release the returned attributes dictionary (if any).
        if !result.is_null() {
            unsafe { CFRelease(result) };
        }

        if status == errSecItemNotFound {
            return Ok(false);
        }
        if status != 0 {
            return Err(AppError::Keychain(format!(
                "Keychain query failed (OSStatus {})",
                status
            )));
        }
    }
    Ok(true)
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
pub fn store_metadata(_service: &str, _account: &str, _data: &[u8]) -> Result<(), AppError> {
    Err(AppError::Keychain(
        "Keychain operations are only supported on macOS".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
pub fn retrieve_metadata(_service: &str, _account: &str) -> Result<Vec<u8>, AppError> {
    Err(AppError::Keychain(
        "Keychain operations are only supported on macOS".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
pub fn delete_metadata(_service: &str, _account: &str) -> Result<(), AppError> {
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

    /// Requires Touch ID hardware and an enrolled fingerprint.
    /// Run manually: `cargo test -- --ignored test_store_retrieve_delete`
    #[test]
    #[ignore = "requires Touch ID hardware"]
    fn test_store_retrieve_delete() {
        let account = "test-key-srd";
        let key = b"test_secret_key_12345678901234567890";

        store_key(TEST_SERVICE, account, key).unwrap();
        let retrieved = retrieve_key(TEST_SERVICE, account).unwrap();
        assert_eq!(key.to_vec(), retrieved);

        delete_key(TEST_SERVICE, account).unwrap();

        let result = retrieve_key(TEST_SERVICE, account);
        assert!(result.is_err());
    }

    /// Requires Touch ID hardware and an enrolled fingerprint.
    /// Run manually: `cargo test -- --ignored test_overwrite_key`
    #[test]
    #[ignore = "requires Touch ID hardware"]
    fn test_overwrite_key() {
        let account = "test-key-ow";
        let key1 = b"first_key_12345678901234567890123";
        let key2 = b"second_key_0987654321098765432109";

        store_key(TEST_SERVICE, account, key1).unwrap();
        store_key(TEST_SERVICE, account, key2).unwrap();

        let retrieved = retrieve_key(TEST_SERVICE, account).unwrap();
        assert_eq!(key2.to_vec(), retrieved);

        let _ = delete_key(TEST_SERVICE, account);
    }

    /// `keys_exist` for absent items must return `false` without prompting Touch ID.
    #[test]
    fn test_keys_exist_nonexistent() {
        let result = keys_exist("ch.dokassist.app.test.nonexistent");
        assert!(matches!(result, Ok(false)));
    }
}
