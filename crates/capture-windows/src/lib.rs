//! # OpenRemote Capture (Windows)
//!
//! Windows screen capture with GPU hardware acceleration (DXGI Desktop Duplication)
//! and automatic fallback to GDI BitBlt capture for virtual or legacy environments.
//!
//! Platform restriction: this crate only compiles on Windows.

#[cfg(not(windows))]
compile_error!("openremote-capture-windows only supports Windows targets");

pub mod dxgi;
pub mod error;
pub mod gdi;

pub use dxgi::DxgiCapturer;
pub use error::CaptureError;
pub use gdi::{CapturedFrame, GdiCapturer};

/// The capture backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureBackend {
    /// DXGI Desktop Duplication (GPU hardware accelerated, Direct3D 11).
    DesktopDuplication,
    /// GDI BitBlt software capture (universal fallback).
    Gdi,
}

enum CapturerImpl {
    Dxgi(DxgiCapturer),
    Gdi(GdiCapturer),
}

/// Unified screen capturer that manages desktop frame acquisition.
pub struct ScreenCapturer {
    backend: CaptureBackend,
    inner: CapturerImpl,
}

impl ScreenCapturer {
    /// Create a screen capturer using the specified backend.
    pub fn new(backend: CaptureBackend) -> Result<Self, CaptureError> {
        match backend {
            CaptureBackend::DesktopDuplication => {
                let dxgi = DxgiCapturer::new()?;
                Ok(Self {
                    backend: CaptureBackend::DesktopDuplication,
                    inner: CapturerImpl::Dxgi(dxgi),
                })
            }
            CaptureBackend::Gdi => {
                let gdi = GdiCapturer::new()?;
                Ok(Self {
                    backend: CaptureBackend::Gdi,
                    inner: CapturerImpl::Gdi(gdi),
                })
            }
        }
    }

    /// Automatically select the best available capture backend:
    /// tries DXGI Desktop Duplication first, falls back to GDI BitBlt.
    pub fn auto() -> Result<Self, CaptureError> {
        match DxgiCapturer::new() {
            Ok(dxgi) => {
                tracing::info!("Using DXGI Desktop Duplication for screen capture");
                Ok(Self {
                    backend: CaptureBackend::DesktopDuplication,
                    inner: CapturerImpl::Dxgi(dxgi),
                })
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "DXGI Desktop Duplication unavailable, falling back to GDI BitBlt"
                );
                let gdi = GdiCapturer::new()?;
                Ok(Self {
                    backend: CaptureBackend::Gdi,
                    inner: CapturerImpl::Gdi(gdi),
                })
            }
        }
    }

    /// Return the active backend.
    pub fn backend(&self) -> CaptureBackend {
        self.backend
    }

    /// Return the current desktop screen dimensions (width, height).
    pub fn dimensions(&self) -> (u32, u32) {
        match &self.inner {
            CapturerImpl::Dxgi(dxgi) => dxgi.dimensions(),
            CapturerImpl::Gdi(gdi) => gdi.dimensions(),
        }
    }

    /// Capture a single frame from the desktop.
    pub fn capture_frame(&self) -> Result<CapturedFrame, CaptureError> {
        match &self.inner {
            CapturerImpl::Dxgi(dxgi) => dxgi.capture_frame(),
            CapturerImpl::Gdi(gdi) => gdi.capture_frame(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_capturer_initializes_and_captures() {
        let handle = std::thread::spawn(|| {
            let capturer = ScreenCapturer::auto().expect("screen capturer must initialize");
            let (width, height) = capturer.dimensions();
            assert!(width > 0);
            assert!(height > 0);

            // Capture a frame to verify memory layout
            let frame = capturer
                .capture_frame()
                .expect("capture frame must succeed");
            assert_eq!(frame.width, width);
            assert_eq!(frame.height, height);
            assert_eq!(frame.data.len(), (frame.stride * height) as usize);
        });

        handle.join().expect("capture thread finished cleanly");
    }
}
