//! End-to-end integration test: Device identity, QR pairing data exchange,
//! trusted device registry persistence, and unattended connection authorization.

use chrono::Utc;
use openremote_authorization::evaluator::{AuthDecision, AuthorizationEvaluator};
use openremote_authorization::registry::{InMemoryRegistry, TrustedDeviceRegistry};
use openremote_identity::keypair::DeviceKeypair;
use openremote_protocol::pairing::{DevicePermissions, QrPairingPayload, TrustedDevice};
use openremote_protocol::session::SessionState;
use openremote_protocol::types::{DeviceId, Platform};
use openremote_session::manager::SessionManager;

#[test]
fn e2e_device_pairing_and_unattended_session_lifecycle() {
    // 1. Host initializes identity
    let host_keypair = DeviceKeypair::generate().expect("host keypair generation");
    let host_id = DeviceId::new();
    let host_pub_b64 = host_keypair.public_identity().to_base64();

    // 2. Client initializes identity
    let client_keypair = DeviceKeypair::generate().expect("client keypair generation");
    let client_id = DeviceId::new();
    let client_pub_b64 = client_keypair.public_identity().to_base64();

    // 3. Host generates short-lived QR invitation
    let qr_invitation = QrPairingPayload {
        server_url: "wss://coordination.openremote.dev".into(),
        invitation_token: "single-use-token-xyz".into(),
        host_public_key: host_pub_b64.clone(),
        host_device_id: host_id.clone(),
        host_display_name: "Desktop-Host-PC".into(),
        expires_at: Utc::now() + chrono::Duration::minutes(5),
    };

    // Client verifies invitation has not expired
    assert!(qr_invitation.expires_at > Utc::now());

    // 4. Host receives client public identity and stores trusted device
    let mut registry = InMemoryRegistry::new();
    let trusted_client = TrustedDevice {
        device_id: client_id.clone(),
        display_name: "Pixel 8 Pro".into(),
        platform: Platform::Android,
        public_key: client_pub_b64.clone(),
        paired_at: Utc::now(),
        last_seen_at: None,
        unattended_access_enabled: true, // Owner explicitly enabled unattended access
        permissions: DevicePermissions {
            remote_control: true,
            file_transfer: true,
            remote_reboot: false,
            clipboard_sync: true,
            audio_streaming: false,
        },
    };
    registry
        .insert(trusted_client)
        .expect("insert trusted device");

    // 5. Subsequent connection from client: Host evaluates authorization
    let evaluator = AuthorizationEvaluator::new(registry);
    let server_challenge = b"unique-nonce-challenge-from-server-12345";
    let client_signature = client_keypair
        .sign(server_challenge)
        .expect("client signs challenge");

    let decision = evaluator
        .evaluate_unattended_connect(&client_id, server_challenge, &client_signature)
        .expect("evaluation succeeds");
    assert_eq!(
        decision,
        AuthDecision::Allow,
        "Unattended connection must be authorized"
    );

    // 6. Host starts session via SessionManager
    let mut session_mgr = SessionManager::new();
    let session_id = session_mgr
        .create_session(host_id.clone(), client_id.clone())
        .expect("create session");

    // Transition: Connecting -> Active
    session_mgr
        .transition(&session_id, SessionState::Active)
        .expect("transition to active");
    assert_eq!(
        session_mgr.get(&session_id).unwrap().state,
        SessionState::Active
    );

    // Session termination
    session_mgr.end_session(&session_id).expect("end session");
    assert_eq!(
        session_mgr.get(&session_id).unwrap().state,
        SessionState::Ended
    );
}

#[test]
fn unapproved_device_connection_is_strictly_rejected() {
    let registry = InMemoryRegistry::new();
    let evaluator = AuthorizationEvaluator::new(registry);

    let untrusted_client = DeviceKeypair::generate().unwrap();
    let untrusted_id = DeviceId::new();
    let challenge = b"challenge";
    let sig = untrusted_client.sign(challenge).unwrap();

    let result = evaluator.evaluate_unattended_connect(&untrusted_id, challenge, &sig);
    assert!(
        result.is_err(),
        "Untrusted device must be rejected immediately"
    );
}
