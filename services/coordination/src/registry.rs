//! In-memory device registry for the coordination server.
//!
//! In production this will be backed by PostgreSQL.
//! M0 uses an in-memory store suitable for development and testing.

use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct DeviceRecord {
    pub device_id: String,
    pub public_key: String,
    pub display_name: String,
    pub platform: String,
    pub registered_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[allow(dead_code)]
pub struct DeviceRegistry {
    devices: HashMap<String, DeviceRecord>,
}

#[allow(dead_code)]
impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    pub fn register(&mut self, record: DeviceRecord) {
        self.devices.insert(record.device_id.clone(), record);
    }

    pub fn get(&self, device_id: &str) -> Option<&DeviceRecord> {
        self.devices.get(device_id)
    }

    pub fn update_last_seen(&mut self, device_id: &str) {
        if let Some(record) = self.devices.get_mut(device_id) {
            record.last_seen_at = Utc::now();
        }
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
