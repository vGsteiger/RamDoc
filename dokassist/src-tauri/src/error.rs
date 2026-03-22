use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Keychain error: {0}")]
    Keychain(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Filesystem error: {0}")]
    Filesystem(#[from] std::io::Error),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Auth required")]
    AuthRequired,

    #[error("Invalid recovery passphrase")]
    InvalidRecoveryPhrase,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Too many recovery attempts. Try again in {0} seconds.")]
    RateLimited(u64),

    #[error("Update error: {0}")]
    Update(String),

    /// Touch ID sheet was dismissed by the user (not an error — just a cancel).
    #[error("Touch ID authentication was cancelled")]
    BiometricCancelled,
}

impl AppError {
    /// Machine-readable error code sent to the frontend.
    /// Format: CATEGORY_SPECIFIC_DETAIL for easier debugging and support.
    pub fn code(&self) -> String {
        match self {
            AppError::Keychain(_) => "KEYCHAIN_ERROR".to_string(),
            AppError::Crypto(_) => "CRYPTO_ERROR".to_string(),
            AppError::Database(e) => {
                // Generate specific database error codes based on the error message
                let msg = e.to_string().to_lowercase();
                if msg.contains("unique") {
                    "DB_UNIQUE_CONSTRAINT".to_string()
                } else if msg.contains("foreign key") {
                    "DB_FOREIGN_KEY".to_string()
                } else if msg.contains("not null") {
                    "DB_NOT_NULL".to_string()
                } else {
                    "DATABASE_ERROR".to_string()
                }
            }
            AppError::Filesystem(_) => "FILESYSTEM_ERROR".to_string(),
            AppError::Llm(_) => "LLM_ERROR".to_string(),
            AppError::AuthRequired => "AUTH_REQUIRED".to_string(),
            AppError::InvalidRecoveryPhrase => "INVALID_RECOVERY_PHRASE".to_string(),
            AppError::NotFound(resource) => {
                // Generate specific NOT_FOUND codes based on the resource
                if resource.to_lowercase().contains("report") {
                    "REPORT_NOT_FOUND".to_string()
                } else if resource.to_lowercase().contains("patient") {
                    "PATIENT_NOT_FOUND".to_string()
                } else if resource.to_lowercase().contains("session") {
                    "SESSION_NOT_FOUND".to_string()
                } else if resource.to_lowercase().contains("file") {
                    "FILE_NOT_FOUND".to_string()
                } else {
                    "NOT_FOUND".to_string()
                }
            }
            AppError::Validation(msg) => {
                // Generate specific validation error codes
                let lower = msg.to_lowercase();
                if lower.contains("report") {
                    "REPORT_VALIDATION_ERROR".to_string()
                } else if lower.contains("patient") {
                    "PATIENT_VALIDATION_ERROR".to_string()
                } else {
                    "VALIDATION_ERROR".to_string()
                }
            }
            AppError::RateLimited(_) => "RATE_LIMITED".to_string(),
            AppError::Update(_) => "UPDATE_ERROR".to_string(),
            AppError::BiometricCancelled => "BIOMETRIC_CANCELLED".to_string(),
        }
    }

    /// Generate a unique error reference ID for tracking purposes.
    /// Format: ERR-{TIMESTAMP}-{CODE_HASH}
    pub fn error_ref(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let code = self.code();
        let msg = self.to_string();

        // Create a simple hash from the error code and message
        let hash: u32 = code
            .bytes()
            .chain(msg.bytes())
            .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));

        format!("ERR-{}-{:08X}", timestamp, hash)
    }
}

/// Serialises as `{ "code": "REPORT_NOT_FOUND", "message": "Not found: report", "ref": "ERR-1234567890-ABCD1234" }`
/// so the frontend can branch on the machine-readable `code` field and users can share the `ref` for support.
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 3)?;
        s.serialize_field("code", &self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.serialize_field("ref", &self.error_ref())?;
        s.end()
    }
}
