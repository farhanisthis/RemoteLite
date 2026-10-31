use thiserror::Error;

#[derive(Debug, Error)]
pub enum InputError {
    #[error("input not implemented: {reason}")]
    NotImplemented { reason: String },

    #[error("input injection failed: {reason}")]
    InjectionFailed { reason: String },

    #[error("invalid key code: {code}")]
    InvalidKeyCode { code: u32 },
}
