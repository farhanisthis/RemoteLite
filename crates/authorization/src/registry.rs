//! Trusted device registry interface and in-memory implementation.

use crate::error::AuthorizationError;
use openremote_protocol::pairing::TrustedDevice;
use openremote_protocol::types::DeviceId;
use std::collections::HashMap;

/// Persistent store of trusted devices.
pub trait TrustedDeviceRegistry: Send + Sync {
    fn get(&self, device_id: &DeviceId) -> Result<Option<TrustedDevice>, AuthorizationError>;
    fn insert(&mut self, device: TrustedDevice) -> Result<(), AuthorizationError>;
    fn remove(&mut self, device_id: &DeviceId) -> Result<bool, AuthorizationError>;
    fn list(&self) -> Result<Vec<TrustedDevice>, AuthorizationError>;
}

/// In-memory registry for testing. Not persisted across restarts.
pub struct InMemoryRegistry {
    devices: HashMap<DeviceId, TrustedDevice>,
}

impl InMemoryRegistry {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }
}

impl Default for InMemoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TrustedDeviceRegistry for InMemoryRegistry {
    fn get(&self, device_id: &DeviceId) -> Result<Option<TrustedDevice>, AuthorizationError> {
        Ok(self.devices.get(device_id).cloned())
    }

    fn insert(&mut self, device: TrustedDevice) -> Result<(), AuthorizationError> {
        self.devices.insert(device.device_id.clone(), device);
        Ok(())
    }

    fn remove(&mut self, device_id: &DeviceId) -> Result<bool, AuthorizationError> {
        Ok(self.devices.remove(device_id).is_some())
    }

    fn list(&self) -> Result<Vec<TrustedDevice>, AuthorizationError> {
        Ok(self.devices.values().cloned().collect())
    }
}

impl<T: TrustedDeviceRegistry> TrustedDeviceRegistry for std::sync::Arc<std::sync::Mutex<T>> {
    fn get(&self, device_id: &DeviceId) -> Result<Option<TrustedDevice>, AuthorizationError> {
        self.lock().unwrap().get(device_id)
    }

    fn insert(&mut self, device: TrustedDevice) -> Result<(), AuthorizationError> {
        self.lock().unwrap().insert(device)
    }

    fn remove(&mut self, device_id: &DeviceId) -> Result<bool, AuthorizationError> {
        self.lock().unwrap().remove(device_id)
    }

    fn list(&self) -> Result<Vec<TrustedDevice>, AuthorizationError> {
        self.lock().unwrap().list()
    }
}
