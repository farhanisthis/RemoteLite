//! Fundamental identifier and newtype wrappers used across the protocol.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique, stable identifier for a device (host or client).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(Uuid);

impl DeviceId {
    /// Generate a new random device ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from a known UUID (e.g. loaded from persistent storage).
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Return the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for DeviceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an active remote session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a file transfer operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransferId(Uuid);

impl TransferId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for TransferId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TransferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Version of the OpenRemote protocol wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl ProtocolVersion {
    /// Current protocol version implemented by this build.
    pub const CURRENT: Self = Self {
        major: 0,
        minor: 1,
        patch: 0,
    };

    /// Returns true if this version is compatible with `other` for
    /// basic signaling handshake purposes (same major).
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Platform identifier for a connected device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Windows,
    Android,
    Ios,
    MacOs,
    Linux,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windows => write!(f, "Windows"),
            Self::Android => write!(f, "Android"),
            Self::Ios => write!(f, "iOS"),
            Self::MacOs => write!(f, "macOS"),
            Self::Linux => write!(f, "Linux"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_is_unique() {
        let a = DeviceId::new();
        let b = DeviceId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn session_id_is_unique() {
        let a = SessionId::new();
        let b = SessionId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn protocol_version_compatibility() {
        let v0 = ProtocolVersion::CURRENT;
        let v1 = ProtocolVersion {
            major: 0,
            minor: 2,
            patch: 0,
        };
        let v2 = ProtocolVersion {
            major: 1,
            minor: 0,
            patch: 0,
        };
        assert!(v0.is_compatible_with(&v1));
        assert!(!v0.is_compatible_with(&v2));
    }

    #[test]
    fn device_id_round_trips_via_json() {
        let id = DeviceId::new();
        let json = serde_json::to_string(&id).expect("serialize");
        let id2: DeviceId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, id2);
    }
}
