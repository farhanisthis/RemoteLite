pub mod gdi;

use anyhow::Result;
use self::gdi::GdiScreenCapture;

pub struct ScreenCapture {
    gdi: GdiScreenCapture,
}

impl ScreenCapture {
    pub fn new() -> Result<Self> {
        let gdi = GdiScreenCapture::new()?;
        Ok(Self { gdi })
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.gdi.dimensions()
    }

    /// Capture frame into BGRA buffer.
    pub fn capture(&mut self, buffer: &mut Vec<u8>) -> Result<()> {
        self.gdi.capture_frame(buffer)
    }
}
