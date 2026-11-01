/// Maps coordinates between the viewer canvas and the remote desktop screen,
/// taking into account aspect-ratio letterboxing / pillarboxing.
pub struct CoordinateMapper;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FittedRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl CoordinateMapper {
    /// Calculate the centered rectangle with aspect ratio preservation.
    pub fn calculate_fitted_rect(
        canvas_width: f32,
        canvas_height: f32,
        remote_width: f32,
        remote_height: f32,
    ) -> FittedRect {
        if remote_width <= 0.0 || remote_height <= 0.0 || canvas_width <= 0.0 || canvas_height <= 0.0 {
            return FittedRect {
                x: 0.0,
                y: 0.0,
                width: canvas_width,
                height: canvas_height,
            };
        }

        let canvas_aspect = canvas_width / canvas_height;
        let remote_aspect = remote_width / remote_height;

        let (fitted_w, fitted_h) = if canvas_aspect > remote_aspect {
            // Pillarbox (bars on left/right)
            (canvas_height * remote_aspect, canvas_height)
        } else {
            // Letterbox (bars on top/bottom)
            (canvas_width, canvas_width / remote_aspect)
        };

        let offset_x = (canvas_width - fitted_w) / 2.0;
        let offset_y = (canvas_height - fitted_h) / 2.0;

        FittedRect {
            x: offset_x,
            y: offset_y,
            width: fitted_w,
            height: fitted_h,
        }
    }

    /// Map canvas pointer coordinates (x, y) to normalized remote coordinates (0.0 to 1.0).
    /// Returns None if pointer is outside the fitted display area.
    pub fn canvas_to_remote_normalized(
        canvas_pos: (f32, f32),
        fitted: &FittedRect,
    ) -> Option<(f32, f32)> {
        let (cx, cy) = canvas_pos;
        if cx < fitted.x || cx > fitted.x + fitted.width || cy < fitted.y || cy > fitted.y + fitted.height {
            return None;
        }

        let norm_x = ((cx - fitted.x) / fitted.width).clamp(0.0, 1.0);
        let norm_y = ((cy - fitted.y) / fitted.height).clamp(0.0, 1.0);

        Some((norm_x, norm_y))
    }

    /// Map normalized (0.0..1.0) to remote pixel coordinates.
    pub fn normalized_to_remote_pixel(
        norm_pos: (f32, f32),
        remote_width: u32,
        remote_height: u32,
    ) -> (u32, u32) {
        let (nx, ny) = norm_pos;
        let px = (nx * (remote_width.saturating_sub(1) as f32)).round() as u32;
        let py = (ny * (remote_height.saturating_sub(1) as f32)).round() as u32;
        (px.min(remote_width.saturating_sub(1)), py.min(remote_height.saturating_sub(1)))
    }
}

/// Map egui Key to Windows Virtual Key Code.
pub fn egui_key_to_vk(key: egui::Key) -> Option<u32> {
    use egui::Key;
    match key {
        Key::ArrowDown => Some(0x28),
        Key::ArrowLeft => Some(0x25),
        Key::ArrowRight => Some(0x27),
        Key::ArrowUp => Some(0x26),
        Key::Escape => Some(0x1B),
        Key::Tab => Some(0x09),
        Key::Backspace => Some(0x08),
        Key::Enter => Some(0x0D),
        Key::Space => Some(0x20),
        Key::Insert => Some(0x2D),
        Key::Delete => Some(0x2E),
        Key::Home => Some(0x24),
        Key::End => Some(0x23),
        Key::PageUp => Some(0x21),
        Key::PageDown => Some(0x22),
        Key::Num0 => Some(0x30),
        Key::Num1 => Some(0x31),
        Key::Num2 => Some(0x32),
        Key::Num3 => Some(0x33),
        Key::Num4 => Some(0x34),
        Key::Num5 => Some(0x35),
        Key::Num6 => Some(0x36),
        Key::Num7 => Some(0x37),
        Key::Num8 => Some(0x38),
        Key::Num9 => Some(0x39),
        Key::A => Some(0x41),
        Key::B => Some(0x42),
        Key::C => Some(0x43),
        Key::D => Some(0x44),
        Key::E => Some(0x45),
        Key::F => Some(0x46),
        Key::G => Some(0x47),
        Key::H => Some(0x48),
        Key::I => Some(0x49),
        Key::J => Some(0x4A),
        Key::K => Some(0x4B),
        Key::L => Some(0x4C),
        Key::M => Some(0x4D),
        Key::N => Some(0x4E),
        Key::O => Some(0x4F),
        Key::P => Some(0x50),
        Key::Q => Some(0x51),
        Key::R => Some(0x52),
        Key::S => Some(0x53),
        Key::T => Some(0x54),
        Key::U => Some(0x55),
        Key::V => Some(0x56),
        Key::W => Some(0x57),
        Key::X => Some(0x58),
        Key::Y => Some(0x59),
        Key::Z => Some(0x5A),
        Key::F1 => Some(0x70),
        Key::F2 => Some(0x71),
        Key::F3 => Some(0x72),
        Key::F4 => Some(0x73),
        Key::F5 => Some(0x74),
        Key::F6 => Some(0x75),
        Key::F7 => Some(0x76),
        Key::F8 => Some(0x77),
        Key::F9 => Some(0x78),
        Key::F10 => Some(0x79),
        Key::F11 => Some(0x7A),
        Key::F12 => Some(0x7B),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_letterbox_and_mapping() {
        let fitted = CoordinateMapper::calculate_fitted_rect(1000.0, 1000.0, 1920.0, 1080.0);
        assert_eq!(fitted.width, 1000.0);
        assert!((fitted.height - 562.5).abs() < 0.1);
        assert!((fitted.y - (1000.0 - 562.5) / 2.0).abs() < 0.1);

        let canvas_center = (500.0, 500.0);
        let norm = CoordinateMapper::canvas_to_remote_normalized(canvas_center, &fitted).unwrap();
        assert!((norm.0 - 0.5).abs() < 0.01);
        assert!((norm.1 - 0.5).abs() < 0.01);

        let pixel = CoordinateMapper::normalized_to_remote_pixel(norm, 1920, 1080);
        assert!((pixel.0 as i32 - 960).abs() <= 1);
        assert!((pixel.1 as i32 - 540).abs() <= 1);
    }

    #[test]
    fn test_key_mapping() {
        assert_eq!(egui_key_to_vk(egui::Key::A), Some(0x41));
        assert_eq!(egui_key_to_vk(egui::Key::Enter), Some(0x0D));
        assert_eq!(egui_key_to_vk(egui::Key::Escape), Some(0x1B));
    }
}
