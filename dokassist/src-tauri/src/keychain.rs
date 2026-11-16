#[cfg(target_os = "macos")]
use crate::constants::{DB_KEY_ACCOUNT, FS_KEY_ACCOUNT};
use crate::error::AppError;

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
#[cfg(target_os = "macos")]
use security_framework::passwords::{delete_generic_password, get_generic_password};
#[cfg(target_os = "macos")]
use security_framework_sys::access_control::kSecAttrAccessibleWhenUnlockedThisDeviceOnly;
#[cfg(target_os = "macos")]
use security_framework_sys::base::{errSecAuthFailed, errSecItemNotFound};
#[cfg(target_os = "macos")]
use security_framework_sys::item::{
    kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword, kSecReturnAttributes,
    kSecValueData,
};
#[cfg(target_os = "macos")]
use security_framework_sys::keychain_item::{SecItemAdd, SecItemCopyMatching, SecItemUpdate};

#[cfg(target_os = "macos")]
extern "C" {
    // `kSecAttrAccessible` is not exported by security_framework_sys.
    static kSecAttrAccessible: core_foundation_sys::string::CFStringRef;
}

/// `errSecUserCanceled`: the user dismissed the Keychain access prompt.
#[cfg(target_os = "macos")]
const ERR_SEC_USER_CANCELED: i32 = -128;
/// `errSecInteractionNotAllowed`: the keychain is locked and may not prompt.
#[cfg(target_os = "macos")]
const ERR_SEC_INTERACTION_NOT_ALLOWED: i32 = -25308;
/// `errSecNoSuchKeychain`: the default (login) keychain cannot be found.
#[cfg(target_os = "macos")]
const ERR_SEC_NO_SUCH_KEYCHAIN: i32 = -25294;

/// Map an `OSStatus` from a Keychain call to an [`AppError`].
///
/// Access-denial statuses get their own variant. The app is not signed with a
/// stable Developer ID, so login-keychain item ACLs can stop trusting the binary
/// after an update and macOS asks the user to allow access again. Denying that
/// prompt, or a login keychain whose password no longer matches the account
/// password, used to surface as an opaque Keychain error with no way forward.
#[cfg(target_os = "macos")]
fn map_status(status: i32, operation: &str) -> AppError {
    match status {
        s if s == errSecItemNotFound => AppError::KeychainItemMissing,
        s if s == errSecAuthFailed
            || s == ERR_SEC_USER_CANCELED
            || s == ERR_SEC_INTERACTION_NOT_ALLOWED
            || s == ERR_SEC_NO_SUCH_KEYCHAIN =>
        {
            AppError::KeychainAccessDenied(format!("{operation} (OSStatus {status})"))
        }
        _ => AppError::Keychain(format!("{operation} (OSStatus {status})")),
    }
}

/// Store a master key in macOS Keychain with device-bound protection.
///
/// The key is accessible only while the device is unlocked and is never synced
/// to iCloud. Touch ID/device-password authentication is enforced by
/// `touch_id::authenticate` before this key is read. A Keychain
/// `SecAccessControl` policy is deliberately not used here: it returns
/// `errSecMissingEntitlement` (-34018) for this application's current signing
/// configuration.
#[cfg(target_os = "macos")]
pub fn store_key(service: &str, account: &str, key: &[u8]) -> Result<(), AppError> {
    let match_query = CFDictionary::<CFString, _>::from_CFType_pairs(&[
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

    // Updating in place is atomic from the caller's perspective and avoids a
    // delete/add window where a crash could temporarily remove an audit checkpoint.
    let update = CFDictionary::<CFString, _>::from_CFType_pairs(&[
        (
            unsafe { CFString::wrap_under_get_rule(kSecValueData) },
            CFData::from_buffer(key).as_CFType(),
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecAttrAccessible) },
            unsafe { CFString::wrap_under_get_rule(kSecAttrAccessibleWhenUnlockedThisDeviceOnly) }
                .as_CFType(),
        ),
    ]);
    let update_status = unsafe {
        SecItemUpdate(
            match_query.as_concrete_TypeRef(),
            update.as_concrete_TypeRef(),
        )
    };
    if update_status == 0 {
        return Ok(());
    }
    if update_status != errSecItemNotFound {
        return Err(map_status(update_status, "Failed to update key"));
    }

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
            unsafe { CFString::wrap_under_get_rule(kSecAttrAccessibleWhenUnlockedThisDeviceOnly) }
                .as_CFType(),
        ),
    ]);

    let status = unsafe { SecItemAdd(dict.as_concrete_TypeRef(), std::ptr::null_mut()) };
    if status != 0 {
        return Err(map_status(status, "Failed to store key"));
    }

    Ok(())
}

/// Store a key, replacing an existing item whose access list no longer trusts
/// this build.
///
/// Only for callers that hold the authoritative key material (mnemonic
/// recovery): deleting and re-adding recreates the item with an access list
/// for the current binary. If the add fails after the delete, the item is
/// missing and the next unlock routes to recovery again, which can recreate it.
#[cfg(target_os = "macos")]
pub fn replace_key(service: &str, account: &str, key: &[u8]) -> Result<(), AppError> {
    match store_key(service, account, key) {
        Err(AppError::KeychainAccessDenied(reason)) => {
            log::warn!("Keychain update denied ({reason}); recreating item");
            match delete_key(service, account) {
                Ok(()) | Err(AppError::KeychainItemMissing) => {}
                Err(err) => return Err(err),
            }
            store_key(service, account, key)
        }
        other => other,
    }
}

/// Retrieve a master key from Keychain.
///
/// This function itself does not show Touch ID — biometric authentication is
/// handled by `touch_id::authenticate` in the caller (`unlock_app`) before
/// this function is invoked.
#[cfg(target_os = "macos")]
pub fn retrieve_key(service: &str, account: &str) -> Result<Vec<u8>, AppError> {
    get_generic_password(service, account)
        .map(|p| p.to_vec())
        .map_err(|e| map_status(e.code(), "Failed to retrieve key"))
}

/// Delete a key from Keychain.
#[cfg(target_os = "macos")]
pub fn delete_key(service: &str, account: &str) -> Result<(), AppError> {
    delete_generic_password(service, account)
        .map_err(|e| map_status(e.code(), "Failed to delete key"))
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
        let status = unsafe { SecItemCopyMatching(query.as_concrete_TypeRef(), &mut result) };

        // Release the returned attributes dictionary (if any).
        if !result.is_null() {
            unsafe { CFRelease(result) };
        }

        if status == errSecItemNotFound {
            return Ok(false);
        }
        if status != 0 {
            return Err(map_status(status, "Keychain query failed"));
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
pub fn replace_key(_service: &str, _account: &str, _key: &[u8]) -> Result<(), AppError> {
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

    #[test]
    fn access_denial_statuses_are_distinguished_from_missing_items() {
        assert!(matches!(
            map_status(errSecItemNotFound, "op"),
            AppError::KeychainItemMissing
        ));
        for status in [
            errSecAuthFailed,
            ERR_SEC_USER_CANCELED,
            ERR_SEC_INTERACTION_NOT_ALLOWED,
            ERR_SEC_NO_SUCH_KEYCHAIN,
        ] {
            assert!(
                matches!(map_status(status, "op"), AppError::KeychainAccessDenied(_)),
                "status {status} should map to KeychainAccessDenied"
            );
        }
        assert!(matches!(map_status(-34018, "op"), AppError::Keychain(_)));
    }

    /// `keys_exist` for absent items must return `false` without prompting Touch ID.
    #[test]
    fn test_keys_exist_nonexistent() {
        let result = keys_exist("ch.dokassist.app.test.nonexistent");
        assert!(matches!(result, Ok(false)), "unexpected result: {result:?}");
    }
}
