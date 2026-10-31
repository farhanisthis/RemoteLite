use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session not found: {id}")]
    NotFound { id: String },

    #[error("session already exists: {id}")]
    AlreadyExists { id: String },

    #[error("authorization error: {0}")]
    Authorization(#[from] openremote_authorization::AuthorizationError),

    #[error("session limit reached")]
    LimitReached,

    #[error("internal error: {0}")]
    Internal(String),
}
