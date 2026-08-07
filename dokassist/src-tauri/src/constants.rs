/// Keychain service identifier
pub const KEYCHAIN_SERVICE: &str = "ch.dokassist.app";

/// Keychain account name for database encryption key
pub const DB_KEY_ACCOUNT: &str = "db.master-key";

/// Keychain account name for filesystem encryption key
pub const FS_KEY_ACCOUNT: &str = "fs.master-key";

/// Alternating Keychain slots for the independently stored audit-chain head.
/// Two slots ensure an interrupted Keychain replacement leaves a prior checkpoint.
pub const AUDIT_CHECKPOINT_A_ACCOUNT: &str = "audit.chain-head.v1.a";
pub const AUDIT_CHECKPOINT_B_ACCOUNT: &str = "audit.chain-head.v1.b";

/// Recovery vault filename
pub const RECOVERY_FILENAME: &str = "recovery.vault";
