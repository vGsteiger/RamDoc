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

    #[error("Export error: {0}")]
    Export(String),
}

impl AppError {
    /// Machine-readable error code sent to the frontend.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Keychain(_) => "KEYCHAIN_ERROR",
            AppError::Crypto(_) => "CRYPTO_ERROR",
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::Filesystem(_) => "FILESYSTEM_ERROR",
            AppError::Llm(_) => "LLM_ERROR",
            AppError::AuthRequired => "AUTH_REQUIRED",
            AppError::InvalidRecoveryPhrase => "INVALID_RECOVERY_PHRASE",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Validation(_) => "VALIDATION_ERROR",
            AppError::RateLimited(_) => "RATE_LIMITED",
            AppError::Export(_) => "EXPORT_ERROR",
        }
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(err: zip::result::ZipError) -> Self {
        AppError::Export(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Export(err.to_string())
    }
}

/// Serialises as `{ "code": "KEYCHAIN_ERROR", "message": "Keychain error: ..." }`
/// so the frontend can branch on the machine-readable `code` field.
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}
