use remotelite::config::AppConfig;
use remotelite::id::{DeviceId, DeviceIdError};
use std::fs;

#[test]
fn test_device_id_rules() {
    // Exactly 9 digits, no leading 0
    let id = DeviceId::generate();
    assert_eq!(id.as_str().len(), 9);
    assert!(!id.as_str().starts_with('0'));

    // Formatting produces "XXX XXX XXX"
    assert_eq!(id.formatted().len(), 11);
    assert_eq!(id.formatted().chars().filter(|c| *c == ' ').count(), 2);

    // Parsing tolerant of spaces and dashes
    let parsed1 = DeviceId::new("987 654 321").unwrap();
    let parsed2 = DeviceId::new("987-654-321").unwrap();
    let parsed3 = DeviceId::new("987654321").unwrap();
    assert_eq!(parsed1, parsed2);
    assert_eq!(parsed2, parsed3);

    // Invalid IDs rejected
    assert_eq!(DeviceId::new("012345678"), Err(DeviceIdError::LeadingZero));
    assert_eq!(DeviceId::new("12345"), Err(DeviceIdError::InvalidLength));
    assert_eq!(DeviceId::new("1234567890"), Err(DeviceIdError::InvalidLength));
}

#[test]
fn test_config_persistence() {
    let temp_dir = std::env::temp_dir().join(format!("remotelite_test_{}", rand::random::<u32>()));
    fs::create_dir_all(&temp_dir).unwrap();
    let config_file = temp_dir.join("config.json");

    let cfg1 = AppConfig::default();
    let original_id = cfg1.device_id.clone();

    // Serialize and save to temp file
    let json = serde_json::to_string_pretty(&cfg1).unwrap();
    fs::write(&config_file, json).unwrap();

    // Reload from file
    let content = fs::read_to_string(&config_file).unwrap();
    let cfg2: AppConfig = serde_json::from_str(&content).unwrap();

    assert_eq!(cfg2.device_id, original_id);
    assert_eq!(cfg2.target_fps, cfg1.target_fps);

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}
