//! # OpenRemote Capture (Windows)
//!
//! Windows screen capture using Windows Graphics Capture API.
//!
//! This crate provides the capture abstraction. In M0, the capture
//! pipeline is stubbed; the real WGC and DXGI integration is
//! implemented in M1.
//!
//! Platform restriction: this crate only compiles on Windows.

#[cfg(not(windows))]
compile_error!("openremote-capture-windows only supports Windows targets");

pub mod error;

pub use error::CaptureError;

/// A raw captured frame (placeholder for M0).
pub struct CapturedFrame {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Raw BGRA pixel data.
    pub data: Vec<u8>,
}

/// The capture backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureBackend {
    /// Windows Graphics Capture API (preferred, Windows 10 1903+).
    WindowsGraphicsCapture,
    /// Desktop Duplication API (fallback, requires desktop session).
    DesktopDuplication,
}

/// Placeholder capture handle for M0.
///
/// Real capture implementation is delivered in M1.
pub struct ScreenCapturer {
    backend: CaptureBackend,
}

impl ScreenCapturer {
    /// Create a new screen capturer.
    pub fn new(backend: CaptureBackend) -> Result<Self, CaptureError> {
        tracing::info!(backend = ?backend, "screen capturer created (M0 stub)");
        Ok(Self { backend })
    }

    /// Return the selected backend.
    pub fn backend(&self) -> CaptureBackend {
        self.backend
    }

    /// Capture a frame. Returns a placeholder in M0.
    pub fn capture_frame(&self) -> Result<CapturedFrame, CaptureError> {
        Err(CaptureError::NotImplemented {
            reason: "capture not implemented until M1".into(),
        })
    }
}
