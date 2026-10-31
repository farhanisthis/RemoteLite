use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("key generation failed: {0}")]
    KeyGeneration(String),

    #[error("signing failed: {0}")]
    Signing(String),

    #[error("signature verification failed")]
    VerificationFailed,

    #[error("identity storage error: {0}")]
    Storage(String),

    #[error("identity not found at path: {0}")]
    NotFound(String),

    #[error("invalid key format: {0}")]
    InvalidKeyFormat(String),

    #[error("key decoding error: {0}")]
    Decode(String),
}
