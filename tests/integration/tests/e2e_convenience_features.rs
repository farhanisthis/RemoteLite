//! End-to-end integration tests for Milestone M7 (Convenience Features).
//!
//! Validates:
//! 1. Windows clipboard injection and echo suppression.
//! 2. Oversized clipboard payload defense (1 MB limit).
//! 3. Remote reboot permission authorization gate (default-deny vs explicitly granted).
//! 4. Real-time connection quality telemetry exchange.

use openremote_authorization::file_registry::FileRegistry;
use openremote_host_service::{HostConfig, HostService};
use openremote_identity::DeviceKeypair;
use openremote_input_windows::WindowsClipboard;
use openremote_protocol::messages::{
    ConnectionQualityMetrics, Envelope, PairingCompleteMessage, RemoteRebootRequest,
    SignalingMessage,
};
use openremote_protocol::types::{DeviceId, Platform, SessionId};

#[test]
fn test_e2e_windows_clipboard_sync_and_echo_suppression() {
    let clipboard = WindowsClipboard::new();
    let text_from_android = "https://github.com/openremote/openremote";

    // 1. Host receives clipboard text from Android and injects it
    let write_res = clipboard.write_text(text_from_android);
    assert!(write_res.is_ok());

    // 2. Windows clipboard listener reads the clipboard.
    // Echo suppression must ensure this text is NOT sent back to Android!
    let should_sync = clipboard.should_sync_to_remote(text_from_android);
    assert!(
        !should_sync,
        "Echo cancellation must suppress returning same text to peer"
    );

    // 3. When a distinct local string is copied, it MUST be allowed to sync
    let new_local_text = "Distinct text copied on Windows desktop";
    let should_sync_new = clipboard.should_sync_to_remote(new_local_text);
    assert!(
        should_sync_new,
        "Newly copied local text must be allowed to sync to remote peer"
    );
}

#[test]
fn test_e2e_oversized_clipboard_text_rejected() {
    let clipboard = WindowsClipboard::new();
    // 2 MB payload (limit is 1 MB)
    let huge_text = "x".repeat(2 * 1024 * 1024);
    let res = clipboard.write_text(&huge_text);
    assert!(
        res.is_err(),
        "Oversized clipboard payload exceeding 1MB must be rejected"
    );
}

#[test]
fn test_e2e_remote_reboot_permission_gate() {
    let temp_dir = tempfile::tempdir().unwrap();
    let host_config = HostConfig {
        storage_path: temp_dir.path().join("identity.dat"),
        registry_path: temp_dir.path().join("trusted_devices.json"),
        display_name: "Desktop-Win11".to_string(),
        signaling_url: None,
    };

    let host_keypair = DeviceKeypair::generate().unwrap();
    let host_registry = FileRegistry::open(&host_config.registry_path).unwrap();
    let host_service = HostService::new(host_config, host_keypair, host_registry);

    let client_keypair = DeviceKeypair::generate().unwrap();
    let client_id = DeviceId::new();

    // 1. Pair Android device with host
    let inv = host_service.create_pairing_invitation(
        "ws://localhost:8080/ws/signal",
        chrono::Duration::minutes(5),
    );
    let pair_msg = PairingCompleteMessage {
        invitation_token: inv.invitation_token,
        client_device_id: client_id.clone(),
        client_public_key: client_keypair.public_identity().to_base64(),
        client_display_name: "Pixel 8".to_string(),
        platform: Platform::Android,
    };
    host_service.complete_pairing(pair_msg).unwrap();

    let session_id = SessionId::new();
    let reboot_req = RemoteRebootRequest {
        session_id: session_id.clone(),
        client_device_id: client_id.clone(),
        host_device_id: host_service.device_id(),
        reason: Some("System maintenance".to_string()),
    };

    // 2. Attempt reboot when remote_reboot permission is NOT granted (default state)
    let ack = host_service.handle_remote_reboot(&reboot_req).unwrap();
    assert!(
        !ack.accepted,
        "Reboot must be rejected when permission not granted"
    );
    assert!(ack.error.is_some());

    // 3. Host owner explicitly enables remote_reboot permission
    host_service
        .set_device_permission(&client_id, "remote_reboot", true)
        .expect("Permission update must succeed");

    // 4. Client requests reboot again -> Authorized
    let ack_authorized = host_service.handle_remote_reboot(&reboot_req).unwrap();
    assert!(
        ack_authorized.accepted,
        "Reboot must be accepted when authorized"
    );
    assert_eq!(ack_authorized.countdown_seconds, 5);
    assert!(ack_authorized.error.is_none());
}

#[test]
fn test_e2e_connection_quality_telemetry() {
    let session_id = SessionId::new();
    let metrics = ConnectionQualityMetrics {
        session_id: session_id.clone(),
        rtt_ms: 18,
        bitrate_kbps: 3800,
        packet_loss_percent: 0.05,
        fps: 60,
    };

    let env = Envelope::new(SignalingMessage::ConnectionQuality(metrics.clone()));
    let json = serde_json::to_string(&env).unwrap();

    let deserialized: Envelope = serde_json::from_str(&json).unwrap();
    if let SignalingMessage::ConnectionQuality(m) = deserialized.payload {
        assert_eq!(m.session_id, session_id);
        assert_eq!(m.rtt_ms, 18);
        assert_eq!(m.fps, 60);
    } else {
        panic!("Expected ConnectionQuality message payload");
    }
}
