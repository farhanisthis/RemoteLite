//! # OpenRemote Encoder (Windows)
//!
//! H.264 video encoding using Windows Media Foundation and hardware
//! encoder support (NVENC, Quick Sync, VCE via MFT).
//!
//! M0: Stub implementation. Real MFT integration is delivered in M1.

#[cfg(not(windows))]
compile_error!("openremote-encoder-windows only supports Windows targets");

pub mod error;

pub use error::EncoderError;

/// Target encoding profile.
#[derive(Debug, Clone, Copy)]
pub struct EncoderConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    /// Target bitrate in bits per second.
    pub bitrate_bps: u32,
    /// Prefer hardware acceleration.
    pub prefer_hardware: bool,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 30,
            bitrate_bps: 5_000_000,
            prefer_hardware: true,
        }
    }
}

/// Placeholder H.264 encoder for M0.
pub struct H264Encoder {
    config: EncoderConfig,
}

impl H264Encoder {
    pub fn new(config: EncoderConfig) -> Result<Self, EncoderError> {
        tracing::info!(
            width = config.width,
            height = config.height,
            fps = config.fps,
            "H264Encoder created (M0 stub)"
        );
        Ok(Self { config })
    }

    pub fn config(&self) -> &EncoderConfig {
        &self.config
    }

    pub fn encode_frame(&self, _frame_data: &[u8]) -> Result<Vec<u8>, EncoderError> {
        Err(EncoderError::NotImplemented {
            reason: "encoding not implemented until M1".into(),
        })
    }
}
