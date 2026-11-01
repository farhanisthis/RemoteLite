use crate::id::DeviceId;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const APP_DIR_NAME: &str = "RemoteLite";
const CONFIG_FILE_NAME: &str = "config.json";
pub const DEFAULT_SIGNALING_URL: &str = "ws://127.0.0.1:9090";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub device_id: DeviceId,
    pub signaling_server_url: String,
    pub target_fps: u32,
    pub jpeg_quality: u8,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            device_id: DeviceId::generate(),
            signaling_server_url: DEFAULT_SIGNALING_URL.to_string(),
            target_fps: 30,
            jpeg_quality: 75,
        }
    }
}

impl AppConfig {
    /// Get the standard config path: %APPDATA%/RemoteLite/config.json
    pub fn config_path() -> PathBuf {
        let base_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_DIR_NAME);
        base_dir.join(CONFIG_FILE_NAME)
    }

    /// Load persistent config from disk, or create and persist default if missing.
    pub fn load_or_create() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<Self>(&content) {
                    Ok(cfg) => return Ok(cfg),
                    Err(e) => {
                        tracing::warn!("Failed to parse config file at {:?}: {}. Generating new.", path, e);
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read config file at {:?}: {}. Generating new.", path, e);
                }
            }
        }

        // Generate and save new config with a fresh persistent device ID
        let new_config = Self::default();
        if let Err(e) = new_config.save() {
            tracing::error!("Failed to save new configuration: {}", e);
        }
        Ok(new_config)
    }

    /// Save configuration atomically to disk.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory {:?}", parent))?;
        }

        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize AppConfig to JSON")?;
        
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, json.as_bytes())
            .with_context(|| format!("Failed to write temporary config {:?}", temp_path))?;
        
        fs::rename(&temp_path, &path)
            .with_context(|| format!("Failed to atomically replace config at {:?}", path))?;

        tracing::info!("Saved configuration with Device ID: {}", self.device_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.device_id, deserialized.device_id);
        assert_eq!(cfg.signaling_server_url, deserialized.signaling_server_url);
        assert_eq!(cfg.target_fps, deserialized.target_fps);
    }
}
