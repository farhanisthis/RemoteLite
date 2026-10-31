//! Protocol-level error types.

use thiserror::Error;

/// Errors that can occur during protocol message processing.
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(String),

    #[error("invalid message format: {reason}")]
    InvalidMessage { reason: String },

    #[error("authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    #[error("session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("device not authorized: {device_id}")]
    DeviceNotAuthorized { device_id: String },

    #[error("pairing invitation expired or invalid")]
    InvalidPairingInvitation,

    #[error("transfer error: {reason}")]
    TransferError { reason: String },
}
