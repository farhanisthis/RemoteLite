use thiserror::Error;

#[derive(Debug, Error)]
pub enum EncoderError {
    #[error("encoder not implemented: {reason}")]
    NotImplemented { reason: String },

    #[error("encoder initialization failed: {reason}")]
    InitFailed { reason: String },

    #[error("encoding frame failed: {reason}")]
    FrameFailed { reason: String },
}
