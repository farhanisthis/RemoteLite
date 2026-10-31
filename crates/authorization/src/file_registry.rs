//! Persistent file-based trusted device registry.
//!
//! Stores trusted device records and permissions in JSON format using atomic
//! write-and-rename semantics to guarantee integrity across crashes.

use crate::error::AuthorizationError;
use crate::registry::TrustedDeviceRegistry;
use openremote_protocol::pairing::TrustedDevice;
use openremote_protocol::types::DeviceId;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// File-backed persistent trusted device registry.
pub struct FileRegistry {
    path: PathBuf,
    devices: HashMap<DeviceId, TrustedDevice>,
}

impl FileRegistry {
    /// Open or create a file registry at the given path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, AuthorizationError> {
        let path = path.as_ref().to_path_buf();
        let mut devices = HashMap::new();

        if path.exists() {
            let data = fs::read_to_string(&path).map_err(|e| {
                AuthorizationError::Registry(format!("failed to read registry file: {}", e))
            })?;

            if !data.trim().is_empty() {
                let list: Vec<TrustedDevice> = serde_json::from_str(&data).map_err(|e| {
                    AuthorizationError::Registry(format!("failed to parse registry json: {}", e))
                })?;

                for device in list {
                    devices.insert(device.device_id.clone(), device);
                }
            }
            debug!(path = ?path, count = devices.len(), "Loaded trusted devices from file");
        } else if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AuthorizationError::Registry(format!("failed to create registry directory: {}", e))
            })?;
        }

        Ok(Self { path, devices })
    }

    /// Save the current in-memory device state atomically to disk.
    fn save_to_disk(&self) -> Result<(), AuthorizationError> {
        let list: Vec<&TrustedDevice> = self.devices.values().collect();
        let json = serde_json::to_string_pretty(&list).map_err(|e| {
            AuthorizationError::Registry(format!("failed to serialize devices: {}", e))
        })?;

        let tmp_path = self.path.with_extension("tmp");
        fs::write(&tmp_path, json.as_bytes()).map_err(|e| {
            AuthorizationError::Registry(format!("failed to write temp file: {}", e))
        })?;

        fs::rename(&tmp_path, &self.path).map_err(|e| {
            AuthorizationError::Registry(format!("failed to rename temp file: {}", e))
        })?;

        trace_save(&self.path, self.devices.len());
        Ok(())
    }
}

fn trace_save(path: &Path, count: usize) {
    debug!(path = ?path, count = count, "Saved trusted devices atomically");
}

impl TrustedDeviceRegistry for FileRegistry {
    fn get(&self, device_id: &DeviceId) -> Result<Option<TrustedDevice>, AuthorizationError> {
        Ok(self.devices.get(device_id).cloned())
    }

    fn insert(&mut self, device: TrustedDevice) -> Result<(), AuthorizationError> {
        info!(
            device_id = %device.device_id,
            display_name = %device.display_name,
            unattended = device.unattended_access_enabled,
            "Persisting trusted device"
        );
        self.devices.insert(device.device_id.clone(), device);
        self.save_to_disk()
    }

    fn remove(&mut self, device_id: &DeviceId) -> Result<bool, AuthorizationError> {
        if self.devices.remove(device_id).is_some() {
            info!(device_id = %device_id, "Revoking and removing trusted device");
            self.save_to_disk()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn list(&self) -> Result<Vec<TrustedDevice>, AuthorizationError> {
        Ok(self.devices.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use openremote_protocol::pairing::DevicePermissions;
    use openremote_protocol::types::Platform;

    #[test]
    fn file_registry_persists_across_instances() {
        let temp_dir =
            std::env::temp_dir().join(format!("openremote_test_{}", uuid::Uuid::new_v4()));
        let file_path = temp_dir.join("trusted_devices.json");

        let device_id = DeviceId::new();
        let device = TrustedDevice {
            device_id: device_id.clone(),
            display_name: "Test Android".into(),
            platform: Platform::Android,
            public_key: "pubkey-12345678".into(),
            paired_at: Utc::now(),
            last_seen_at: None,
            unattended_access_enabled: true,
            permissions: DevicePermissions::default(),
        };

        // 1. Insert into first instance
        {
            let mut reg = FileRegistry::open(&file_path).unwrap();
            reg.insert(device.clone()).unwrap();
            assert_eq!(reg.list().unwrap().len(), 1);
        }

        // 2. Open new instance from same file
        {
            let reg2 = FileRegistry::open(&file_path).unwrap();
            let retrieved = reg2.get(&device_id).unwrap().expect("device must exist");
            assert_eq!(retrieved.display_name, "Test Android");
            assert!(retrieved.unattended_access_enabled);
        }

        // 3. Remove device
        {
            let mut reg3 = FileRegistry::open(&file_path).unwrap();
            let removed = reg3.remove(&device_id).unwrap();
            assert!(removed);
        }

        // 4. Verify gone in new instance
        {
            let reg4 = FileRegistry::open(&file_path).unwrap();
            assert_eq!(reg4.list().unwrap().len(), 0);
        }

        let _ = fs::remove_dir_all(temp_dir);
    }
}
