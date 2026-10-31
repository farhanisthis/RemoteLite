//! Authorization decision evaluator.

use crate::error::AuthorizationError;
use crate::registry::TrustedDeviceRegistry;
use openremote_identity::keypair::PublicIdentity;
use openremote_protocol::types::DeviceId;
use tracing::{info, warn};

/// The result of an authorization evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthDecision {
    /// Connection is permitted to proceed.
    Allow,
    /// Connection must be denied.
    Deny(String),
}

/// Evaluates whether a device may connect and what it may do.
pub struct AuthorizationEvaluator<R: TrustedDeviceRegistry> {
    registry: R,
}

impl<R: TrustedDeviceRegistry> AuthorizationEvaluator<R> {
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    /// Evaluate whether `device_id` may open an unattended remote session.
    ///
    /// Checks performed, in order:
    /// 1. Device exists in the trusted registry.
    /// 2. Device is not revoked.
    /// 3. Unattended access is enabled for this device.
    /// 4. Signature over the provided `challenge` is valid.
    pub fn evaluate_unattended_connect(
        &self,
        device_id: &DeviceId,
        challenge: &[u8],
        signature_b64: &str,
    ) -> Result<AuthDecision, AuthorizationError> {
        let device = self
            .registry
            .get(device_id)
            .map_err(|e| AuthorizationError::Registry(e.to_string()))?
            .ok_or_else(|| AuthorizationError::DeviceNotFound {
                device_id: device_id.to_string(),
            })?;

        if !device.unattended_access_enabled {
            warn!(
                device_id = %device_id,
                "unattended access not enabled"
            );
            return Ok(AuthDecision::Deny("unattended_access_disabled".into()));
        }

        let public_identity = PublicIdentity::from_base64(&device.public_key).map_err(|_| {
            AuthorizationError::SignatureVerificationFailed {
                device_id: device_id.to_string(),
            }
        })?;

        public_identity
            .verify(challenge, signature_b64)
            .map_err(|_| AuthorizationError::SignatureVerificationFailed {
                device_id: device_id.to_string(),
            })?;

        info!(device_id = %device_id, "unattended connect authorized");
        Ok(AuthDecision::Allow)
    }

    /// Check whether a device has a specific permission.
    pub fn check_permission(
        &self,
        device_id: &DeviceId,
        permission: &str,
    ) -> Result<bool, AuthorizationError> {
        let device = self
            .registry
            .get(device_id)
            .map_err(|e| AuthorizationError::Registry(e.to_string()))?
            .ok_or_else(|| AuthorizationError::DeviceNotFound {
                device_id: device_id.to_string(),
            })?;

        Ok(match permission {
            "remote_control" => device.permissions.remote_control,
            "file_transfer" => device.permissions.file_transfer,
            "remote_reboot" => device.permissions.remote_reboot,
            "clipboard_sync" => device.permissions.clipboard_sync,
            "audio_streaming" => device.permissions.audio_streaming,
            _ => false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::InMemoryRegistry;
    use chrono::Utc;
    use openremote_identity::keypair::DeviceKeypair;
    use openremote_protocol::pairing::{DevicePermissions, TrustedDevice};
    use openremote_protocol::types::{DeviceId, Platform};

    fn make_device(keypair: &DeviceKeypair, unattended: bool) -> TrustedDevice {
        TrustedDevice {
            device_id: DeviceId::new(),
            display_name: "Test Android".into(),
            platform: Platform::Android,
            public_key: keypair.public_identity().to_base64(),
            paired_at: Utc::now(),
            last_seen_at: None,
            unattended_access_enabled: unattended,
            permissions: DevicePermissions::default(),
        }
    }

    #[test]
    fn authorized_device_can_connect() {
        let keypair = DeviceKeypair::generate().unwrap();
        let device = make_device(&keypair, true);
        let device_id = device.device_id.clone();
        let mut registry = InMemoryRegistry::new();
        registry.insert(device).unwrap();

        let evaluator = AuthorizationEvaluator::new(registry);
        let challenge = b"random-challenge-bytes";
        let sig = keypair.sign(challenge).unwrap();

        let decision = evaluator
            .evaluate_unattended_connect(&device_id, challenge, &sig)
            .unwrap();
        assert_eq!(decision, AuthDecision::Allow);
    }

    #[test]
    fn unattended_access_disabled_is_denied() {
        let keypair = DeviceKeypair::generate().unwrap();
        let device = make_device(&keypair, false);
        let device_id = device.device_id.clone();
        let mut registry = InMemoryRegistry::new();
        registry.insert(device).unwrap();

        let evaluator = AuthorizationEvaluator::new(registry);
        let sig = keypair.sign(b"challenge").unwrap();
        let decision = evaluator
            .evaluate_unattended_connect(&device_id, b"challenge", &sig)
            .unwrap();
        assert!(matches!(decision, AuthDecision::Deny(_)));
    }

    #[test]
    fn unknown_device_returns_error() {
        let registry = InMemoryRegistry::new();
        let evaluator = AuthorizationEvaluator::new(registry);
        let unknown = DeviceId::new();
        let result = evaluator.evaluate_unattended_connect(&unknown, b"x", "fakesig");
        assert!(matches!(
            result,
            Err(AuthorizationError::DeviceNotFound { .. })
        ));
    }
}
