use thiserror::Error;

#[derive(Debug, Error)]
pub enum InputError {
    #[error("invalid coordinate ({x}, {y}): {reason}")]
    InvalidCoordinate { x: f32, y: f32, reason: String },

    #[error("input injection failed: {reason}")]
    InjectionFailed { reason: String },

    #[error("input rate limit exceeded: {count} events in window (limit: {limit})")]
    RateLimitExceeded { count: u32, limit: u32 },

    #[error("unauthorized session {session_id}: input injection rejected")]
    UnauthorizedSession { session_id: String },

    #[error("emergency disconnect triggered by local user interaction")]
    EmergencyDisconnectTriggered,

    #[error("invalid key code: {code}")]
    InvalidKeyCode { code: u32 },

    #[error("clipboard error: {reason}")]
    ClipboardError { reason: String },
}
