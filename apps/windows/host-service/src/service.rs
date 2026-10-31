//! Core Windows Host Service state and business logic.

use crate::error::HostServiceError;
use crate::pairing::PairingManager;
use chrono::Duration;
use openremote_authorization::evaluator::{AuthDecision, AuthorizationEvaluator};
use openremote_authorization::file_registry::FileRegistry;
use openremote_authorization::registry::TrustedDeviceRegistry;
use openremote_identity::keypair::DeviceKeypair;
use openremote_identity::store::IdentityStore;
use openremote_protocol::messages::{
    ConnectRejectReason, ConnectRequestMessage, PairingCompleteMessage, SignalingMessage,
};
use openremote_protocol::pairing::{DevicePermissions, QrPairingPayload, TrustedDevice};
use openremote_protocol::session::SessionState;
use openremote_protocol::types::{DeviceId, Platform, SessionId};
use openremote_session::manager::SessionManager;
use openremote_transport::signaling::SignalingClient;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

/// Configuration for the host service.
#[derive(Debug, Clone)]
pub struct HostConfig {
    pub storage_path: PathBuf,
    pub registry_path: PathBuf,
    pub display_name: String,
    pub signaling_url: Option<String>,
}

impl Default for HostConfig {
    fn default() -> Self {
        let base_dir = std::env::var("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("OpenRemote");

        let hostname = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "OpenRemote-Host".into());

        Self {
            storage_path: base_dir.join("identity.dat"),
            registry_path: base_dir.join("trusted_devices.json"),
            display_name: hostname,
            signaling_url: None,
        }
    }
}

/// The core OpenRemote Windows host service manager.
pub struct HostService {
    config: HostConfig,
    device_id: DeviceId,
    keypair: Arc<DeviceKeypair>,
    registry: Arc<Mutex<FileRegistry>>,
    pairing_manager: Arc<Mutex<PairingManager>>,
    session_manager: Arc<Mutex<SessionManager>>,
}

impl HostService {
    /// Initialize with existing keypair and registry (useful for testing and custom runners).
    pub fn new(config: HostConfig, keypair: DeviceKeypair, registry: FileRegistry) -> Self {
        let device_id = DeviceId::new();
        Self {
            config,
            device_id,
            keypair: Arc::new(keypair),
            registry: Arc::new(Mutex::new(registry)),
            pairing_manager: Arc::new(Mutex::new(PairingManager::new())),
            session_manager: Arc::new(Mutex::new(SessionManager::new())),
        }
    }

    /// Load or initialize the host service from disk using DPAPI (or File store fallback).
    pub fn init(config: HostConfig) -> Result<Self, HostServiceError> {
        #[cfg(windows)]
        let store = openremote_identity::store::DpapiIdentityStore::new(&config.storage_path);
        #[cfg(not(windows))]
        let store = openremote_identity::store::FileIdentityStore::new(&config.storage_path);

        let keypair = match store.load_secret()? {
            Some(secret) => {
                info!("Loaded existing host device identity from secure storage");
                DeviceKeypair::from_bytes(&secret)?
            }
            None => {
                info!("Generating new host device identity");
                let kp = DeviceKeypair::generate()?;
                store.save_secret(&*kp.secret_bytes())?;
                kp
            }
        };

        let registry = FileRegistry::open(&config.registry_path)?;

        Ok(Self::new(config, keypair, registry))
    }

    /// Host device identifier.
    pub fn device_id(&self) -> DeviceId {
        self.device_id.clone()
    }

    /// Host Ed25519 public key base64.
    pub fn public_key(&self) -> String {
        self.keypair.public_identity().to_base64()
    }

    /// Host human-readable display name.
    pub fn display_name(&self) -> &str {
        &self.config.display_name
    }

    /// Reference to the device keypair.
    pub fn keypair(&self) -> Arc<DeviceKeypair> {
        self.keypair.clone()
    }

    /// Create a new QR pairing invitation.
    pub fn create_pairing_invitation(
        &self,
        server_url: &str,
        validity: Duration,
    ) -> QrPairingPayload {
        let mut mgr = self.pairing_manager.lock().unwrap();
        mgr.create_invitation(
            server_url,
            &self.device_id,
            &self.public_key(),
            self.display_name(),
            validity,
        )
    }

    /// Complete a pairing request from a client device.
    ///
    /// SECURITY: Unattended access is explicitly disabled by default.
    pub fn complete_pairing(
        &self,
        msg: PairingCompleteMessage,
    ) -> Result<TrustedDevice, HostServiceError> {
        // 1. Validate and consume single-use invitation token
        {
            let mut mgr = self.pairing_manager.lock().unwrap();
            mgr.validate_and_consume(&msg.invitation_token)?;
        }

        // 2. Build trusted device record
        let device = TrustedDevice {
            device_id: msg.client_device_id.clone(),
            display_name: msg.client_display_name,
            platform: msg.platform,
            public_key: msg.client_public_key,
            paired_at: chrono::Utc::now(),
            last_seen_at: None,
            unattended_access_enabled: false, // Must be explicitly toggled by user!
            permissions: DevicePermissions::default(),
        };

        // 3. Persist to registry
        {
            let mut reg = self.registry.lock().unwrap();
            reg.insert(device.clone())?;
        }

        info!(
            client_id = %msg.client_device_id,
            display_name = %device.display_name,
            "Successfully completed pairing new device"
        );

        Ok(device)
    }

    /// Explicitly enable or disable unattended access for a trusted device.
    pub fn set_unattended_access(
        &self,
        device_id: &DeviceId,
        enabled: bool,
    ) -> Result<(), HostServiceError> {
        let mut reg = self.registry.lock().unwrap();
        let mut device = reg.get(device_id)?.ok_or_else(|| {
            openremote_authorization::AuthorizationError::DeviceNotFound {
                device_id: device_id.to_string(),
            }
        })?;

        device.unattended_access_enabled = enabled;
        reg.insert(device)?;

        info!(
            device_id = %device_id,
            enabled = enabled,
            "Updated unattended access setting for device"
        );

        Ok(())
    }

    /// Revoke a trusted device and immediately terminate any active session.
    pub fn revoke_device(&self, device_id: &DeviceId) -> Result<bool, HostServiceError> {
        let removed = {
            let mut reg = self.registry.lock().unwrap();
            reg.remove(device_id)?
        };

        if removed {
            // Check for and terminate active sessions
            let mut sm = self.session_manager.lock().unwrap();
            let active_to_kill: Vec<SessionId> = sm
                .active_sessions()
                .iter()
                .filter(|s| s.client_device_id == *device_id)
                .map(|s| s.session_id.clone())
                .collect();

            for sid in active_to_kill {
                warn!(
                    session_id = %sid,
                    device_id = %device_id,
                    "Terminating active session due to device revocation"
                );
                let _ = sm.end_session(&sid);
            }
        }

        Ok(removed)
    }

    /// Evaluate an incoming connection request.
    ///
    /// Returns the session ID if permitted, or the reason if rejected.
    pub fn evaluate_connect_request(
        &self,
        req: &ConnectRequestMessage,
    ) -> Result<SessionId, ConnectRejectReason> {
        let evaluator = AuthorizationEvaluator::new(self.registry.clone());

        match evaluator.evaluate_unattended_connect(
            &req.client_device_id,
            req.signed_challenge.as_bytes(),
            &req.auth_signature,
        ) {
            Ok(AuthDecision::Allow) => {
                let mut sm = self.session_manager.lock().unwrap();
                let sid = match sm.create_session_with_id(
                    req.session_id.clone(),
                    self.device_id.clone(),
                    req.client_device_id.clone(),
                ) {
                    Ok(id) => id,
                    Err(_) => return Err(ConnectRejectReason::HostBusy),
                };
                let _ = sm.transition(&sid, SessionState::Active);
                info!(
                    session_id = %sid,
                    client_id = %req.client_device_id,
                    "Connection accepted and session activated"
                );
                Ok(sid)
            }
            Ok(AuthDecision::Deny(reason)) => {
                warn!(
                    client_id = %req.client_device_id,
                    reason = %reason,
                    "Connection denied by authorization policy"
                );
                if reason == "unattended_access_disabled" {
                    Err(ConnectRejectReason::UnattendedAccessDisabled)
                } else {
                    Err(ConnectRejectReason::NotAuthorized)
                }
            }
            Err(e) => {
                warn!(
                    client_id = %req.client_device_id,
                    error = %e,
                    "Connection evaluation failed"
                );
                Err(ConnectRejectReason::NotAuthorized)
            }
        }
    }

    /// End an active session.
    pub fn end_session(&self, session_id: &SessionId) -> Result<(), HostServiceError> {
        let mut sm = self.session_manager.lock().unwrap();
        sm.end_session(session_id)?;
        Ok(())
    }

    /// List all trusted devices.
    pub fn list_trusted_devices(&self) -> Result<Vec<TrustedDevice>, HostServiceError> {
        let reg = self.registry.lock().unwrap();
        Ok(reg.list()?)
    }

    /// Run the signaling loop until a shutdown notification is received.
    pub async fn run_signaling_loop(
        &self,
        mut client: SignalingClient,
        mut shutdown: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<(), HostServiceError> {
        client
            .register(
                Platform::Windows,
                self.public_key(),
                self.display_name().to_string(),
            )
            .await?;

        info!("Host service registered with coordination server signaling hub");

        loop {
            tokio::select! {
                _ = shutdown.recv() => {
                    info!("Signaling loop received shutdown signal");
                    let _ = client.close().await;
                    break;
                }
                envelope = client.recv_envelope() => {
                    match envelope {
                        Ok(env) => self.handle_signaling_envelope(&mut client, env.payload).await?,
                        Err(openremote_transport::TransportError::Closed) => {
                            info!("Signaling connection closed by server");
                            break;
                        }
                        Err(e) => {
                            error!(error = %e, "Error receiving signaling envelope");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_signaling_envelope(
        &self,
        client: &mut SignalingClient,
        msg: SignalingMessage,
    ) -> Result<(), HostServiceError> {
        match msg {
            SignalingMessage::ConnectRequest(req) => match self.evaluate_connect_request(&req) {
                Ok(session_id) => {
                    client.send_connect_accept(session_id).await?;
                }
                Err(reason) => {
                    client.send_connect_reject(req.session_id, reason).await?;
                }
            },
            SignalingMessage::SessionEnd(end_msg) => {
                let _ = self.end_session(&end_msg.session_id);
            }
            SignalingMessage::PairingComplete(pm) => {
                if let Err(e) = self.complete_pairing(pm) {
                    warn!(error = %e, "Failed to complete pairing request");
                }
            }
            SignalingMessage::Ping => {
                client.send_message(SignalingMessage::Pong).await?;
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openremote_protocol::types::Platform;

    fn setup_test_host() -> (HostService, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = HostConfig {
            storage_path: temp_dir.path().join("identity.dat"),
            registry_path: temp_dir.path().join("trusted_devices.json"),
            display_name: "Test-Host".to_string(),
            signaling_url: None,
        };

        let keypair = DeviceKeypair::generate().unwrap();
        let registry = FileRegistry::open(&config.registry_path).unwrap();
        let service = HostService::new(config, keypair, registry);

        (service, temp_dir)
    }

    #[test]
    fn test_pairing_and_unattended_access_flow() {
        let (host, _dir) = setup_test_host();
        let client_keypair = DeviceKeypair::generate().unwrap();
        let client_id = DeviceId::new();

        // 1. Generate invitation
        let inv =
            host.create_pairing_invitation("ws://localhost:8080/ws/signal", Duration::minutes(5));

        // 2. Client completes pairing
        let pair_complete = PairingCompleteMessage {
            invitation_token: inv.invitation_token,
            client_device_id: client_id.clone(),
            client_public_key: client_keypair.public_identity().to_base64(),
            client_display_name: "Pixel 8".to_string(),
            platform: Platform::Android,
        };

        let trusted = host
            .complete_pairing(pair_complete)
            .expect("pairing must succeed");
        assert_eq!(trusted.device_id, client_id);
        // By default, unattended access must be FALSE
        assert!(!trusted.unattended_access_enabled);

        // 3. Client attempts to connect without unattended access enabled
        let challenge = "server-challenge-string";
        let sig = client_keypair.sign(challenge.as_bytes()).unwrap();

        let req = ConnectRequestMessage {
            session_id: SessionId::new(),
            client_device_id: client_id.clone(),
            host_device_id: host.device_id(),
            auth_signature: sig.clone(),
            signed_challenge: challenge.to_string(),
        };

        let res = host.evaluate_connect_request(&req);
        assert_eq!(res, Err(ConnectRejectReason::UnattendedAccessDisabled));

        // 4. Host owner explicitly enables unattended access
        host.set_unattended_access(&client_id, true)
            .expect("must enable unattended");

        // 5. Client attempts to connect again -> Allowed
        let res2 = host.evaluate_connect_request(&req);
        assert!(res2.is_ok());
    }

    #[test]
    fn test_revocation_terminates_active_session_and_blocks_reconnect() {
        let (host, _dir) = setup_test_host();
        let client_keypair = DeviceKeypair::generate().unwrap();
        let client_id = DeviceId::new();

        // Pair and enable unattended
        let inv =
            host.create_pairing_invitation("ws://localhost:8080/ws/signal", Duration::minutes(5));
        let pair_complete = PairingCompleteMessage {
            invitation_token: inv.invitation_token,
            client_device_id: client_id.clone(),
            client_public_key: client_keypair.public_identity().to_base64(),
            client_display_name: "Android Phone".to_string(),
            platform: Platform::Android,
        };
        host.complete_pairing(pair_complete).unwrap();
        host.set_unattended_access(&client_id, true).unwrap();

        // Connect
        let challenge = "auth-challenge";
        let sig = client_keypair.sign(challenge.as_bytes()).unwrap();
        let req = ConnectRequestMessage {
            session_id: SessionId::new(),
            client_device_id: client_id.clone(),
            host_device_id: host.device_id(),
            auth_signature: sig,
            signed_challenge: challenge.to_string(),
        };

        let session_id = host.evaluate_connect_request(&req).expect("must connect");
        assert_eq!(
            host.session_manager.lock().unwrap().active_sessions().len(),
            1
        );

        // Revoke device
        let revoked = host
            .revoke_device(&client_id)
            .expect("revocation must succeed");
        assert!(revoked);

        // Active session must be terminated
        assert_eq!(
            host.session_manager.lock().unwrap().active_sessions().len(),
            0
        );
        let s = host
            .session_manager
            .lock()
            .unwrap()
            .get(&session_id)
            .unwrap()
            .clone();
        assert_eq!(s.state, openremote_protocol::session::SessionState::Ended);

        // Subsequent reconnect must be rejected as NotAuthorized
        let res = host.evaluate_connect_request(&req);
        assert_eq!(res, Err(ConnectRejectReason::NotAuthorized));
    }

    #[test]
    fn test_tampered_signature_rejected() {
        let (host, _dir) = setup_test_host();
        let client_keypair = DeviceKeypair::generate().unwrap();
        let client_id = DeviceId::new();

        let inv =
            host.create_pairing_invitation("ws://localhost:8080/ws/signal", Duration::minutes(5));
        let pair_complete = PairingCompleteMessage {
            invitation_token: inv.invitation_token,
            client_device_id: client_id.clone(),
            client_public_key: client_keypair.public_identity().to_base64(),
            client_display_name: "Tablet".to_string(),
            platform: Platform::Android,
        };
        host.complete_pairing(pair_complete).unwrap();
        host.set_unattended_access(&client_id, true).unwrap();

        // Sign different message
        let bad_sig = client_keypair.sign(b"other-message").unwrap();
        let req = ConnectRequestMessage {
            session_id: SessionId::new(),
            client_device_id: client_id.clone(),
            host_device_id: host.device_id(),
            auth_signature: bad_sig,
            signed_challenge: "expected-challenge".to_string(),
        };

        let res = host.evaluate_connect_request(&req);
        assert_eq!(res, Err(ConnectRejectReason::NotAuthorized));
    }
}
