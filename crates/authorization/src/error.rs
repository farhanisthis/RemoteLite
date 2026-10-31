use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("device not found in registry: {device_id}")]
    DeviceNotFound { device_id: String },

    #[error("device is revoked: {device_id}")]
    DeviceRevoked { device_id: String },

    #[error("unattended access is not enabled for device: {device_id}")]
    UnattendedAccessDisabled { device_id: String },

    #[error("permission denied: {permission}")]
    PermissionDenied { permission: String },

    #[error("signature verification failed for device: {device_id}")]
    SignatureVerificationFailed { device_id: String },

    #[error("registry error: {0}")]
    Registry(String),
}
