/// Application-level Touch ID / device-passcode authentication.
///
/// Uses `LocalAuthentication` (LAContext) rather than biometric access-control
/// flags on Keychain items.  This approach works in both development and
/// production builds without requiring `keychain-access-groups` entitlements.
///
/// The FFI call blocks the current thread until the user authenticates or
/// dismisses the system sheet.  Call this from a Tauri async command (which
/// runs on a Tokio worker thread) — never from the main thread.
use crate::error::AppError;

#[cfg(target_os = "macos")]
extern "C" {
    fn authenticate_touch_id(reason: *const std::os::raw::c_char) -> std::os::raw::c_int;
}

/// Show the Touch ID / login-password sheet and block until the user responds.
///
/// Returns `Ok(())` on success, `Err(AppError::BiometricCancelled)` when the
/// user dismisses the sheet, and a `Keychain` error for all other failures.
#[cfg(target_os = "macos")]
pub fn authenticate(reason: &str) -> Result<(), AppError> {
    use std::ffi::CString;
    let reason_c = CString::new(reason)
        .map_err(|_| AppError::Keychain("Invalid reason string".to_string()))?;

    // SAFETY: authenticate_touch_id is a pure C function defined in touch_id.m.
    // It does not alias Rust memory.  The CString outlives the call.
    let code = unsafe { authenticate_touch_id(reason_c.as_ptr()) };

    match code {
        0 => Ok(()),
        1 => Err(AppError::BiometricCancelled),
        3 => Err(AppError::Keychain(
            "Touch ID is not available on this device.".to_string(),
        )),
        _ => Err(AppError::Keychain(
            "Touch ID authentication failed. Try again or use your login password.".to_string(),
        )),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn authenticate(_reason: &str) -> Result<(), AppError> {
    Err(AppError::Keychain(
        "Touch ID is only supported on macOS".to_string(),
    ))
}
