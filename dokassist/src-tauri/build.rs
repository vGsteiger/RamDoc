fn main() {
    tauri_build::build();

    // Compile the Objective-C Touch ID helper (macOS only).
    // Uses LocalAuthentication framework for app-level biometric prompts,
    // which works without keychain-access-groups entitlements.
    #[cfg(target_os = "macos")]
    {
        cc::Build::new()
            .file("src/touch_id.m")
            .flag("-fobjc-arc")
            .compile("touch_id");

        println!("cargo:rustc-link-lib=framework=LocalAuthentication");
    }
}
