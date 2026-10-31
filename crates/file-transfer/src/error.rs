use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransferError {
    #[error("file not found: {path}")]
    FileNotFound { path: String },

    #[error("path traversal detected: {path}")]
    PathTraversal { path: String },

    #[error("invalid filename: {name}")]
    InvalidFilename { name: String },

    #[error("file too large: {size} bytes (max {max} bytes)")]
    FileTooLarge { size: u64, max: u64 },

    #[error("disk space insufficient: need {needed} bytes, have {available} bytes")]
    InsufficientDiskSpace { needed: u64, available: u64 },

    #[error("integrity check failed: expected {expected}, got {actual}")]
    IntegrityCheckFailed { expected: String, actual: String },

    #[error("transfer cancelled")]
    Cancelled,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("transfer not found: {id}")]
    NotFound { id: String },
}
