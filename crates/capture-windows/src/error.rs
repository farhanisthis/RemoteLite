use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("capture not implemented: {reason}")]
    NotImplemented { reason: String },

    #[error("capture backend unavailable: {backend}")]
    BackendUnavailable { backend: String },

    #[error("capture failed: {reason}")]
    CaptureFailed { reason: String },

    #[error("frame encoding failed: {reason}")]
    EncodingFailed { reason: String },
}
