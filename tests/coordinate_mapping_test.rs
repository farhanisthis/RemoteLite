use remotelite::input::mapper::{egui_key_to_vk, CoordinateMapper};

#[test]
fn test_coordinate_mapping_aspect_ratios() {
    // 1920x1080 (16:9) remote displayed on 1000x1000 square viewer window
    let fitted = CoordinateMapper::calculate_fitted_rect(1000.0, 1000.0, 1920.0, 1080.0);
    assert_eq!(fitted.width, 1000.0);
    // Letterbox bars on top and bottom
    assert!(fitted.y > 0.0);

    // Pointer inside letterbox bar should return None
    let outside_bar = (500.0, 50.0);
    assert!(CoordinateMapper::canvas_to_remote_normalized(outside_bar, &fitted).is_none());

    // Top-left of remote desktop
    let top_left = (fitted.x, fitted.y);
    let norm_tl = CoordinateMapper::canvas_to_remote_normalized(top_left, &fitted).unwrap();
    assert_eq!(norm_tl, (0.0, 0.0));

    let pixel_tl = CoordinateMapper::normalized_to_remote_pixel(norm_tl, 1920, 1080);
    assert_eq!(pixel_tl, (0, 0));

    // Bottom-right of remote desktop
    let bottom_right = (fitted.x + fitted.width, fitted.y + fitted.height);
    let norm_br = CoordinateMapper::canvas_to_remote_normalized(bottom_right, &fitted).unwrap();
    assert_eq!(norm_br, (1.0, 1.0));

    let pixel_br = CoordinateMapper::normalized_to_remote_pixel(norm_br, 1920, 1080);
    assert_eq!(pixel_br, (1919, 1079));
}

#[test]
fn test_keyboard_scancode_and_vk_mappings() {
    // Check standard keys
    assert_eq!(egui_key_to_vk(egui::Key::A), Some(0x41));
    assert_eq!(egui_key_to_vk(egui::Key::Z), Some(0x5A));
    assert_eq!(egui_key_to_vk(egui::Key::Num0), Some(0x30));
    assert_eq!(egui_key_to_vk(egui::Key::Enter), Some(0x0D));
    assert_eq!(egui_key_to_vk(egui::Key::Backspace), Some(0x08));
    assert_eq!(egui_key_to_vk(egui::Key::Tab), Some(0x09));
    assert_eq!(egui_key_to_vk(egui::Key::Space), Some(0x20));
    assert_eq!(egui_key_to_vk(egui::Key::Escape), Some(0x1B));
    assert_eq!(egui_key_to_vk(egui::Key::F5), Some(0x74));
}
