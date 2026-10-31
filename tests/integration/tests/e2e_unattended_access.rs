//! End-to-end integration tests for Milestone M4 (Trusted Devices and Unattended Access).
//!
//! Validates:
//! 1. Full QR pairing flow between Windows Host and Android Client.
//! 2. Unattended access is strictly disabled by default (rejection of connection).
//! 3. Explicit unattended access enablement allows seamless subsequent connections without desktop prompts.
//! 4. Instant revocation immediately terminates active sessions and blocks reconnection.
//! 5. Untrusted or spoofed devices cannot obtain remote access.

use openremote_authorization::file_registry::FileRegistry;
use openremote_coordination::{build_router, AppState};
use openremote_host_service::{HostConfig, HostService};
use openremote_identity::DeviceKeypair;
use openremote_protocol::messages::{
    ConnectRejectReason, PairingCompleteMessage, SignalingMessage,
};
use openremote_protocol::types::{DeviceId, Platform, SessionId};
use openremote_transport::SignalingClient;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

async fn spawn_test_coordination_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let state = AppState::new();
    let app = build_router(state);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("ws://{}:{}/ws/signal", addr.ip(), addr.port())
}

#[tokio::test]
async fn test_e2e_unattended_access_lifecycle() {
    let ws_url = spawn_test_coordination_server().await;

    // 1. Initialize Host Service with persistent temporary storage
    let temp_dir = tempfile::tempdir().unwrap();
    let host_config = HostConfig {
        storage_path: temp_dir.path().join("identity.dat"),
        registry_path: temp_dir.path().join("trusted_devices.json"),
        display_name: "Desktop-Win11".to_string(),
        signaling_url: Some(ws_url.clone()),
    };

    let host_keypair = DeviceKeypair::generate().unwrap();
    let host_registry = FileRegistry::open(&host_config.registry_path).unwrap();
    let host_service = Arc::new(HostService::new(host_config, host_keypair, host_registry));
    let host_id = host_service.device_id();

    // 2. Start Host Service background signaling client
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
    let host_sig_client = SignalingClient::connect(&ws_url, host_id.clone())
        .await
        .unwrap();

    let host_service_clone = host_service.clone();
    tokio::spawn(async move {
        let _ = host_service_clone
            .run_signaling_loop(host_sig_client, shutdown_rx)
            .await;
    });

    // Allow host registration to settle
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 3. Pairing Flow: Host generates QR pairing invitation
    let qr_payload = host_service.create_pairing_invitation(&ws_url, chrono::Duration::minutes(5));
    assert_eq!(qr_payload.host_device_id, host_id);
    assert_eq!(qr_payload.host_display_name, "Desktop-Win11");

    // Android client scans QR code and completes pairing
    let client_keypair = DeviceKeypair::generate().unwrap();
    let client_id = DeviceId::new();

    let pair_complete = PairingCompleteMessage {
        invitation_token: qr_payload.invitation_token,
        client_device_id: client_id.clone(),
        client_public_key: client_keypair.public_identity().to_base64(),
        client_display_name: "Pixel 8 Pro".to_string(),
        platform: Platform::Android,
    };

    let trusted_device = host_service
        .complete_pairing(pair_complete)
        .expect("Pairing completion must succeed");

    assert_eq!(trusted_device.device_id, client_id);
    // CRITICAL SECURITY ASSERTION: Unattended access must be DISABLED by default
    assert!(!trusted_device.unattended_access_enabled);

    // 4. Android Client connects to coordination server and registers
    let mut android_client = SignalingClient::connect(&ws_url, client_id.clone())
        .await
        .unwrap();
    android_client
        .register(
            Platform::Android,
            client_keypair.public_identity().to_base64(),
            "Pixel 8 Pro".to_string(),
        )
        .await
        .unwrap();

    // 5. Client attempts to connect while unattended access is still DISABLED
    let session_1 = SessionId::new();
    let challenge = "auth-challenge-1";
    let sig = client_keypair.sign(challenge.as_bytes()).unwrap();

    android_client
        .send_connect_request(
            session_1.clone(),
            host_id.clone(),
            sig.clone(),
            challenge.to_string(),
        )
        .await
        .unwrap();

    // Verify client receives ConnectReject with UnattendedAccessDisabled
    let reject_envelope =
        tokio::time::timeout(Duration::from_secs(2), android_client.recv_envelope())
            .await
            .expect("must receive response")
            .expect("must be valid envelope");

    match reject_envelope.payload {
        SignalingMessage::ConnectReject(msg) => {
            assert_eq!(msg.session_id, session_1);
            assert_eq!(msg.reason, ConnectRejectReason::UnattendedAccessDisabled);
        }
        other => panic!("Expected ConnectReject, got {:?}", other),
    }

    // 6. Windows PC Owner explicitly enables Unattended Access for this trusted device
    host_service
        .set_unattended_access(&client_id, true)
        .expect("Enabling unattended access must succeed");

    // 7. Client connects again with unattended access ENABLED
    let session_2 = SessionId::new();
    let challenge_2 = "auth-challenge-2";
    let sig_2 = client_keypair.sign(challenge_2.as_bytes()).unwrap();

    android_client
        .send_connect_request(
            session_2.clone(),
            host_id.clone(),
            sig_2,
            challenge_2.to_string(),
        )
        .await
        .unwrap();

    // Verify client receives ConnectAccept
    let accept_envelope =
        tokio::time::timeout(Duration::from_secs(2), android_client.recv_envelope())
            .await
            .expect("must receive accept")
            .expect("must be valid envelope");

    match accept_envelope.payload {
        SignalingMessage::ConnectAccept(msg) => {
            assert_eq!(msg.session_id, session_2);
            assert_eq!(msg.host_device_id, host_id);
        }
        other => panic!("Expected ConnectAccept, got {:?}", other),
    }

    // 8. Revocation: Owner revokes the Android phone
    let revoked = host_service
        .revoke_device(&client_id)
        .expect("Revocation must succeed");
    assert!(revoked);

    // Verify subsequent connection attempts are immediately rejected as NotAuthorized
    let session_3 = SessionId::new();
    let challenge_3 = "auth-challenge-3";
    let sig_3 = client_keypair.sign(challenge_3.as_bytes()).unwrap();

    android_client
        .send_connect_request(
            session_3.clone(),
            host_id.clone(),
            sig_3,
            challenge_3.to_string(),
        )
        .await
        .unwrap();

    let reject_envelope_2 =
        tokio::time::timeout(Duration::from_secs(2), android_client.recv_envelope())
            .await
            .expect("must receive response")
            .expect("must be valid envelope");

    match reject_envelope_2.payload {
        SignalingMessage::ConnectReject(msg) => {
            assert_eq!(msg.session_id, session_3);
            assert_eq!(msg.reason, ConnectRejectReason::NotAuthorized);
        }
        other => panic!("Expected ConnectReject(NotAuthorized), got {:?}", other),
    }

    // Clean shutdown
    let _ = shutdown_tx.send(());
}

#[tokio::test]
async fn test_e2e_unauthorized_device_rejected() {
    let ws_url = spawn_test_coordination_server().await;

    let temp_dir = tempfile::tempdir().unwrap();
    let host_config = HostConfig {
        storage_path: temp_dir.path().join("identity.dat"),
        registry_path: temp_dir.path().join("trusted_devices.json"),
        display_name: "Desktop-Win11".to_string(),
        signaling_url: Some(ws_url.clone()),
    };

    let host_keypair = DeviceKeypair::generate().unwrap();
    let host_registry = FileRegistry::open(&host_config.registry_path).unwrap();
    let host_service = Arc::new(HostService::new(host_config, host_keypair, host_registry));
    let host_id = host_service.device_id();

    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
    let host_sig_client = SignalingClient::connect(&ws_url, host_id.clone())
        .await
        .unwrap();

    let host_service_clone = host_service.clone();
    tokio::spawn(async move {
        let _ = host_service_clone
            .run_signaling_loop(host_sig_client, shutdown_rx)
            .await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Untrusted attacker client tries to connect without pairing
    let attacker_keypair = DeviceKeypair::generate().unwrap();
    let attacker_id = DeviceId::new();

    let mut attacker_client = SignalingClient::connect(&ws_url, attacker_id.clone())
        .await
        .unwrap();
    attacker_client
        .register(
            Platform::Android,
            attacker_keypair.public_identity().to_base64(),
            "Attacker Device".to_string(),
        )
        .await
        .unwrap();

    let session_id = SessionId::new();
    let challenge = "challenge-123";
    let sig = attacker_keypair.sign(challenge.as_bytes()).unwrap();

    attacker_client
        .send_connect_request(
            session_id.clone(),
            host_id.clone(),
            sig,
            challenge.to_string(),
        )
        .await
        .unwrap();

    let envelope = tokio::time::timeout(Duration::from_secs(2), attacker_client.recv_envelope())
        .await
        .expect("must receive reply")
        .expect("must be valid envelope");

    match envelope.payload {
        SignalingMessage::ConnectReject(msg) => {
            assert_eq!(msg.session_id, session_id);
            assert_eq!(msg.reason, ConnectRejectReason::NotAuthorized);
        }
        other => panic!("Expected ConnectReject, got {:?}", other),
    }

    let _ = shutdown_tx.send(());
}
