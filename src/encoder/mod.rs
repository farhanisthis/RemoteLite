pub mod decoder;

use anyhow::{Context, Result};
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder};
use std::io::Cursor;

pub struct FrameEncoder {
    rgb_buffer: Vec<u8>,
    last_frame_hash: u64,
    quality: u8,
}

impl FrameEncoder {
    pub fn new(quality: u8) -> Self {
        Self {
            rgb_buffer: Vec::new(),
            last_frame_hash: 0,
            quality: quality.clamp(10, 95),
        }
    }

    /// Fast hash to detect identical/unchanged desktop frames.
    fn sample_hash(bgra: &[u8]) -> u64 {
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let step = 64 * 4;
        let mut i = 0;
        while i < bgra.len() {
            hasher.write_u32(u32::from_ne_bytes([
                bgra[i],
                bgra[i + 1],
                bgra[i + 2],
                bgra[i + 3],
            ]));
            i += step;
        }
        hasher.finish()
    }

    /// Convert BGRA (Windows capture format) to RGB and encode as JPEG.
    /// Returns `Ok(Some(jpeg_bytes))` if encoded, or `Ok(None)` if frame has not changed.
    pub fn encode_frame(
        &mut self,
        bgra: &[u8],
        width: u32,
        height: u32,
        force_keyframe: bool,
    ) -> Result<Option<Vec<u8>>> {
        let current_hash = Self::sample_hash(bgra);
        if !force_keyframe && current_hash == self.last_frame_hash && self.last_frame_hash != 0 {
            // Unchanged frame: skip compression to conserve bandwidth
            return Ok(None);
        }
        self.last_frame_hash = current_hash;

        let pixel_count = (width * height) as usize;
        let rgb_size = pixel_count * 3;
        self.rgb_buffer.resize(rgb_size, 0);

        // Convert BGRA -> RGB
        for i in 0..pixel_count {
            let bgra_idx = i * 4;
            let rgb_idx = i * 3;
            self.rgb_buffer[rgb_idx] = bgra[bgra_idx + 2];     // R
            self.rgb_buffer[rgb_idx + 1] = bgra[bgra_idx + 1]; // G
            self.rgb_buffer[rgb_idx + 2] = bgra[bgra_idx];     // B
        }

        let mut jpeg_bytes = Vec::with_capacity(rgb_size / 8);
        {
            let mut cursor = Cursor::new(&mut jpeg_bytes);
            let encoder = JpegEncoder::new_with_quality(&mut cursor, self.quality);
            encoder
                .write_image(
                    &self.rgb_buffer,
                    width,
                    height,
                    ExtendedColorType::Rgb8,
                )
                .context("Failed to encode frame to JPEG")?;
        }

        Ok(Some(jpeg_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_frame() {
        let mut encoder = FrameEncoder::new(75);
        let width = 64;
        let height = 64;
        let mut bgra = vec![0u8; (width * height * 4) as usize];
        for i in 0..(width * height) as usize {
            bgra[i * 4] = 255; // B
            bgra[i * 4 + 1] = 0; // G
            bgra[i * 4 + 2] = 0; // R
            bgra[i * 4 + 3] = 255; // A
        }

        let encoded = encoder.encode_frame(&bgra, width, height, true).unwrap();
        assert!(encoded.is_some());
        let bytes = encoded.unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0xD8);
    }
}
