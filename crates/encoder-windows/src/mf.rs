//! Windows Media Foundation H.264 Encoder integration.

use crate::error::EncoderError;
use windows::Win32::Media::MediaFoundation::{
    MFShutdown, MFStartup, MFSTARTUP_NOSOCKET, MF_VERSION,
};

/// Guard to initialize and shutdown Media Foundation.
pub struct MfPlatformGuard;

impl MfPlatformGuard {
    pub fn new() -> Result<Self, EncoderError> {
        unsafe {
            MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET).map_err(|e| EncoderError::InitFailed {
                reason: format!("MFStartup failed: {e}"),
            })?;
            Ok(Self)
        }
    }
}

impl Drop for MfPlatformGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = MFShutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mf_startup_and_shutdown_succeeds() {
        let guard = MfPlatformGuard::new();
        assert!(guard.is_ok(), "MFStartup must succeed on Windows 10/11");
    }
}
