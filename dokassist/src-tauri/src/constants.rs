/// Keychain service identifier
pub const KEYCHAIN_SERVICE: &str = "ch.dokassist.app";

/// Keychain account name for database encryption key
pub const DB_KEY_ACCOUNT: &str = "db.master-key";

/// Keychain account name for filesystem encryption key
pub const FS_KEY_ACCOUNT: &str = "fs.master-key";

/// Recovery vault filename
pub const RECOVERY_FILENAME: &str = "recovery.vault";

/// Keychain account name for the recovery attempt counter (no biometric protection).
pub const RECOVERY_ATTEMPTS_ACCOUNT: &str = "recovery.attempts";
