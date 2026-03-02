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
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
