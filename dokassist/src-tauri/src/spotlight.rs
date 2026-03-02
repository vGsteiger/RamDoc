use crate::error::AppError;
use std::path::Path;

/// Add directory to Spotlight privacy exclusions via `.metadata_never_index`
/// and optionally using `mdutil` command on macOS.
///
/// This prevents Spotlight from indexing encrypted files in the vault directory.
pub fn exclude_from_spotlight(dir: &Path) -> Result<(), AppError> {
    // Ensure directory exists
    if !dir.exists() {
        return Err(AppError::Filesystem(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Directory does not exist: {}", dir.display()),
        )));
    }

    // Create .metadata_never_index file (works on all macOS versions)
    let metadata_file = dir.join(".metadata_never_index");
    std::fs::write(&metadata_file, "")?;

    // On macOS, also try to disable indexing via mdutil
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Try to disable indexing for this specific directory
        // This may require admin privileges, so we don't fail if it doesn't work
        let _ = Command::new("mdutil")
            .arg("-i")
            .arg("off")
            .arg(dir)
            .output();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_exclude_creates_metadata_file() {
        let temp_dir = TempDir::new().unwrap();
        let vault_dir = temp_dir.path().join("vault");
        std::fs::create_dir(&vault_dir).unwrap();

        exclude_from_spotlight(&vault_dir).unwrap();

        // Check that .metadata_never_index file was created
        let metadata_file = vault_dir.join(".metadata_never_index");
        assert!(metadata_file.exists());
    }

    #[test]
    fn test_exclude_nonexistent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("does_not_exist");

        let result = exclude_from_spotlight(&nonexistent);
        assert!(result.is_err());
    }
}
