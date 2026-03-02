## PKG-9 — Backup & Recovery

**Goal**: Built-in backup tooling and recovery flow.

**Depends on**: PKG-1, PKG-3

**Files**:

```
src-tauri/src/
├── backup.rs
src/routes/settings/
├── backup/
│   └── +page.svelte
```

**Public interface**:

```rust
/// Create a backup of the entire DokAssist data directory to a target path.
/// Copies: dokassist.db, recovery.vault, entire vault/ directory.
/// All files are already encrypted — this is a plain file copy.
pub fn create_backup(source_dir: &Path, target_dir: &Path) -> Result<BackupReport, AppError>;

/// Verify a backup: checks all expected files exist and are non-zero.
pub fn verify_backup(backup_dir: &Path) -> Result<BackupReport, AppError>;

/// Restore from backup: copy files to data directory, then trigger recovery flow.
pub fn restore_from_backup(backup_dir: &Path, target_dir: &Path) -> Result<(), AppError>;

#[derive(Serialize)]
pub struct BackupReport {
    pub files_copied: u32,
    pub total_size_bytes: u64,
    pub timestamp: String,
    pub vault_file_count: u32,
    pub db_present: bool,
    pub recovery_vault_present: bool,
}
```

**Tauri commands**:

```rust
/// Trigger backup to user-selected directory (opens native folder picker).
#[tauri::command]
async fn create_backup(state: State<'_, AppState>) -> Result<BackupReport, AppError>;

/// Verify an existing backup directory.
#[tauri::command]
async fn verify_backup(path: String) -> Result<BackupReport, AppError>;
```

**Settings UI**: Backup section shows last backup date, button to "Backup Now" (opens folder picker), button to "Verify Backup".

**Acceptance criteria**:

- [ ] Backup copies all files to selected external drive
- [ ] Backup report shows file count, total size, and verifies completeness
- [ ] Verify detects missing or zero-byte files
- [ ] Restore + recovery passphrase entry results in working app on new Mac
- [ ] Backup does not include any decrypted/temp files

**Effort**: ~8h

-----