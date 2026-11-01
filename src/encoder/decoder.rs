use anyhow::{Context, Result};
use egui::ColorImage;

pub struct FrameDecoder;

impl FrameDecoder {
    /// Decode JPEG bytes to egui::ColorImage.
    pub fn decode_jpeg(jpeg_bytes: &[u8]) -> Result<ColorImage> {
        let dynamic_image = image::load_from_memory_with_format(jpeg_bytes, image::ImageFormat::Jpeg)
            .context("Failed to decode JPEG image from memory")?;
        
        let rgba = dynamic_image.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let pixels = rgba.into_raw();

        Ok(ColorImage::from_rgba_unmultiplied(size, &pixels))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder::FrameEncoder;

    #[test]
    fn test_encode_and_decode_roundtrip() {
        let mut encoder = FrameEncoder::new(80);
        let width = 32;
        let height = 32;
        let mut bgra = vec![0u8; (width * height * 4) as usize];
        for i in 0..(width * height) as usize {
            bgra[i * 4] = 120;     // B
            bgra[i * 4 + 1] = 200; // G
            bgra[i * 4 + 2] = 50;  // R
            bgra[i * 4 + 3] = 255; // A
        }

        let jpeg_data = encoder.encode_frame(&bgra, width, height, true).unwrap().unwrap();
        let color_image = FrameDecoder::decode_jpeg(&jpeg_data).unwrap();

        assert_eq!(color_image.size, [32, 32]);
        assert_eq!(color_image.pixels.len(), 32 * 32);
    }
}
