//! Pairing protocol data structures.
//!
//! The pairing flow:
//! 1. Host generates a `PairingInvitation` and encodes it as a QR code.
//! 2. Client scans the QR and constructs a `PairingRequest`.
//! 3. Host validates, displays device info, and owner confirms.
//! 4. Host stores a `TrustedDevice` record.

use crate::types::{DeviceId, Platform};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Data encoded in the QR code shown on the Windows host.
///
/// SECURITY: Private keys are never included. The token is short-lived
/// and single-use. It contains only the public key and a nonce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrPairingPayload {
    /// Coordination server URL so the client knows where to connect.
    pub server_url: String,
    /// Single-use, short-lived token identifying this invitation.
    pub invitation_token: String,
    /// Host's Ed25519 public key (base64), so client can verify server messages.
    pub host_public_key: String,
    /// Host device ID for display and correlation.
    pub host_device_id: DeviceId,
    /// Human-readable host name shown on the Android pairing confirmation screen.
    pub host_display_name: String,
    /// When this invitation expires (UTC).
    pub expires_at: DateTime<Utc>,
}

/// A device that has been trusted by the host owner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedDevice {
    pub device_id: DeviceId,
    pub display_name: String,
    pub platform: Platform,
    /// Ed25519 public key (base64).
    pub public_key: String,
    pub paired_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub unattended_access_enabled: bool,
    pub permissions: DevicePermissions,
}

/// Fine-grained permissions granted to a trusted device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePermissions {
    pub remote_control: bool,
    pub file_transfer: bool,
    pub remote_reboot: bool,
    pub clipboard_sync: bool,
    pub audio_streaming: bool,
}

impl Default for DevicePermissions {
    fn default() -> Self {
        Self {
            remote_control: true,
            file_transfer: false,
            remote_reboot: false,
            clipboard_sync: false,
            audio_streaming: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_permissions_are_minimal() {
        let p = DevicePermissions::default();
        assert!(p.remote_control);
        assert!(!p.file_transfer);
        assert!(!p.remote_reboot);
    }
}
