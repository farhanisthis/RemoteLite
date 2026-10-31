//! End-to-end integration tests for Internet Connectivity, Signaling, and Relay Fallback (Milestone M3).
//!
//! Validates:
//! 1. WebSocket signaling exchange between Host and Client through the coordination server.
//! 2. Device registration and authentication.
//! 3. Session connection request/accept handshake.
//! 4. SDP Offer and Answer negotiation.
//! 5. Trickle ICE candidate relay (Host, STUN Server-Reflexive, and TURN Relayed candidates).
//! 6. Negative cases: connecting to offline hosts, unauthorized session injection.
//! 7. Clean session termination.

use openremote_coordination::{build_router, AppState};
use openremote_identity::DeviceKeypair;
use openremote_protocol::messages::{ConnectRejectReason, SessionEndReason, SignalingMessage};
use openremote_protocol::types::{DeviceId, Platform, SessionId};
use openremote_transport::ice::{CandidateType, IceCandidate, TransportProtocol};
use openremote_transport::sdp::{
    generate_h264_sdp_answer, generate_h264_sdp_offer, verify_h264_sdp, SdpSessionParams,
};
use openremote_transport::SignalingClient;
use std::time::Duration;
use tokio::net::TcpListener;

/// Spawn an in-process coordination server on a free port. Returns the server base URL.
async fn spawn_test_server() -> String {
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
async fn test_e2e_signaling_and_sdp_negotiation() {
    let ws_url = spawn_test_server().await;

    let host_keypair = DeviceKeypair::generate().unwrap();
    let client_keypair = DeviceKeypair::generate().unwrap();

    let host_id = DeviceId::new();
    let client_id = DeviceId::new();

    // 1. Connect Host and Client to Coordination Server WebSocket
    let mut host_client = SignalingClient::connect(&ws_url, host_id.clone())
        .await
        .unwrap();
    let mut client = SignalingClient::connect(&ws_url, client_id.clone())
        .await
        .unwrap();

    // 2. Register Both Devices
    host_client
        .register(
            Platform::Windows,
            host_keypair.public_identity().to_base64(),
            "Windows Host PC".to_string(),
        )
        .await
        .unwrap();

    client
        .register(
            Platform::Android,
            client_keypair.public_identity().to_base64(),
            "Android Phone".to_string(),
        )
        .await
        .unwrap();

    // 3. Initiate Session Connect Request (Client -> Host)
    let session_id = SessionId::new();
    let challenge = "session-challenge-42".to_string();
    let auth_sig = client_keypair.sign(challenge.as_bytes()).unwrap();

    client
        .send_connect_request(session_id.clone(), host_id.clone(), auth_sig, challenge)
        .await
        .unwrap();

    // Host receives ConnectRequest
    let host_recv = tokio::time::timeout(Duration::from_secs(3), host_client.recv_envelope())
        .await
        .expect("timeout waiting for ConnectRequest")
        .unwrap();

    match host_recv.payload {
        SignalingMessage::ConnectRequest(req) => {
            assert_eq!(req.session_id, session_id);
            assert_eq!(req.client_device_id, client_id);
            assert_eq!(req.host_device_id, host_id);
        }
        other => panic!("Expected ConnectRequest, got {:?}", other),
    }

    // 4. Host Accepts Session (Host -> Client)
    host_client
        .send_connect_accept(session_id.clone())
        .await
        .unwrap();

    let client_recv = tokio::time::timeout(Duration::from_secs(3), client.recv_envelope())
        .await
        .expect("timeout waiting for ConnectAccept")
        .unwrap();

    match client_recv.payload {
        SignalingMessage::ConnectAccept(acc) => {
            assert_eq!(acc.session_id, session_id);
            assert_eq!(acc.host_device_id, host_id);
        }
        other => panic!("Expected ConnectAccept, got {:?}", other),
    }

    // 5. SDP Offer & Answer Exchange
    let sdp_params = SdpSessionParams {
        session_id: session_id.to_string(),
        host_ip: "192.168.1.100".to_string(),
        video_port: 5004,
        ice_ufrag: "ufrag-test".to_string(),
        ice_pwd: "pwd-test-secret".to_string(),
        dtls_fingerprint_sha256: "AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99".to_string(),
        ssrc: 0x12345678,
    };

    let offer_sdp = generate_h264_sdp_offer(&sdp_params);
    client
        .send_sdp_offer(session_id.clone(), offer_sdp.clone())
        .await
        .unwrap();

    // Host receives SDP Offer
    let host_sdp_env = tokio::time::timeout(Duration::from_secs(3), host_client.recv_envelope())
        .await
        .expect("timeout waiting for SdpOffer")
        .unwrap();

    match host_sdp_env.payload {
        SignalingMessage::SdpOffer(offer) => {
            assert_eq!(offer.session_id, session_id);
            assert!(verify_h264_sdp(&offer.sdp).is_ok());
        }
        other => panic!("Expected SdpOffer, got {:?}", other),
    }

    // Host replies with SDP Answer
    let answer_sdp = generate_h264_sdp_answer(&sdp_params);
    host_client
        .send_sdp_answer(session_id.clone(), answer_sdp)
        .await
        .unwrap();

    // Client receives SDP Answer
    let client_sdp_env = tokio::time::timeout(Duration::from_secs(3), client.recv_envelope())
        .await
        .expect("timeout waiting for SdpAnswer")
        .unwrap();

    match client_sdp_env.payload {
        SignalingMessage::SdpAnswer(ans) => {
            assert_eq!(ans.session_id, session_id);
            assert!(verify_h264_sdp(&ans.sdp).is_ok());
        }
        other => panic!("Expected SdpAnswer, got {:?}", other),
    }

    // 6. Trickle ICE Candidates (Host, STUN srflx, TURN relay)
    let host_candidate = IceCandidate::new(
        "1",
        1,
        TransportProtocol::Udp,
        "192.168.1.100".parse().unwrap(),
        5004,
        CandidateType::Host,
        None,
        None,
    );
    host_client
        .send_ice_candidate(
            session_id.clone(),
            host_candidate.to_sdp_attribute(),
            Some("video".into()),
            Some(0),
        )
        .await
        .unwrap();

    let srflx_candidate = IceCandidate::new(
        "2",
        1,
        TransportProtocol::Udp,
        "203.0.113.10".parse().unwrap(),
        5004,
        CandidateType::Srflx,
        Some("192.168.1.100".parse().unwrap()),
        Some(5004),
    );
    host_client
        .send_ice_candidate(
            session_id.clone(),
            srflx_candidate.to_sdp_attribute(),
            Some("video".into()),
            Some(0),
        )
        .await
        .unwrap();

    let relay_candidate = IceCandidate::new(
        "3",
        1,
        TransportProtocol::Udp,
        "198.51.100.2".parse().unwrap(),
        3478,
        CandidateType::Relay,
        Some("203.0.113.10".parse().unwrap()),
        Some(5004),
    );
    host_client
        .send_ice_candidate(
            session_id.clone(),
            relay_candidate.to_sdp_attribute(),
            Some("video".into()),
            Some(0),
        )
        .await
        .unwrap();

    // Client receives and verifies all 3 candidate types
    for expected_type in [
        CandidateType::Host,
        CandidateType::Srflx,
        CandidateType::Relay,
    ] {
        let ice_env = tokio::time::timeout(Duration::from_secs(3), client.recv_envelope())
            .await
            .expect("timeout waiting for IceCandidate")
            .unwrap();

        match ice_env.payload {
            SignalingMessage::IceCandidate(ice) => {
                assert_eq!(ice.session_id, session_id);
                let parsed = IceCandidate::parse_sdp_attribute(&ice.candidate).unwrap();
                assert_eq!(parsed.candidate_type, expected_type);
            }
            other => panic!("Expected IceCandidate, got {:?}", other),
        }
    }

    // 7. Session Termination
    client
        .send_session_end(session_id.clone(), SessionEndReason::UserDisconnected)
        .await
        .unwrap();

    let end_env = tokio::time::timeout(Duration::from_secs(3), host_client.recv_envelope())
        .await
        .expect("timeout waiting for SessionEnd")
        .unwrap();

    match end_env.payload {
        SignalingMessage::SessionEnd(end) => {
            assert_eq!(end.session_id, session_id);
        }
        other => panic!("Expected SessionEnd, got {:?}", other),
    }

    client.close().await.unwrap();
    host_client.close().await.unwrap();
}

#[tokio::test]
async fn test_offline_host_connect_request_rejected() {
    let ws_url = spawn_test_server().await;
    let client_keypair = DeviceKeypair::generate().unwrap();
    let client_id = DeviceId::new();
    let offline_host_id = DeviceId::new();

    let mut client = SignalingClient::connect(&ws_url, client_id).await.unwrap();
    client
        .register(
            Platform::Android,
            client_keypair.public_identity().to_base64(),
            "Android Client".into(),
        )
        .await
        .unwrap();

    let session_id = SessionId::new();
    client
        .send_connect_request(
            session_id.clone(),
            offline_host_id,
            "sig".into(),
            "challenge".into(),
        )
        .await
        .unwrap();

    let resp = tokio::time::timeout(Duration::from_secs(3), client.recv_envelope())
        .await
        .expect("timeout waiting for reject")
        .unwrap();

    match resp.payload {
        SignalingMessage::ConnectReject(rej) => {
            assert_eq!(rej.session_id, session_id);
            assert!(matches!(rej.reason, ConnectRejectReason::HostOffline));
        }
        other => panic!("Expected ConnectReject, got {:?}", other),
    }

    client.close().await.unwrap();
}

#[tokio::test]
async fn test_unauthorized_session_sdp_offer_rejected() {
    let ws_url = spawn_test_server().await;
    let client_keypair = DeviceKeypair::generate().unwrap();
    let client_id = DeviceId::new();

    let mut client = SignalingClient::connect(&ws_url, client_id).await.unwrap();
    client
        .register(
            Platform::Android,
            client_keypair.public_identity().to_base64(),
            "Android Client".into(),
        )
        .await
        .unwrap();

    // Send SDP offer for a non-existent session
    let dummy_session = SessionId::new();
    client
        .send_sdp_offer(dummy_session, "v=0\r\no=dummy\r\n".into())
        .await
        .unwrap();

    let resp = tokio::time::timeout(Duration::from_secs(3), client.recv_envelope())
        .await
        .expect("timeout waiting for error response")
        .unwrap();

    match resp.payload {
        SignalingMessage::Error(err) => {
            assert_eq!(err.code, "session_not_found");
        }
        other => panic!("Expected Error, got {:?}", other),
    }

    client.close().await.unwrap();
}
