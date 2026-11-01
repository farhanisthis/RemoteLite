use remotelite::crypto::{derive_session_key, generate_entropy, SecureChannel};
use remotelite::net::{SignalingClient, SignalingEvent};
use remotelite::protocol::{InputPayload, SessionMessage};
use remotelite::server::run_signaling_server;
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::test]
async fn test_e2e_signaling_and_encrypted_session() {
    // 1. Pick a random free port for test signaling server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let bind_addr = format!("127.0.0.1:{}", port);
    let server_url = format!("ws://127.0.0.1:{}", port);

    // Spawn server in background
    let server_addr_clone = bind_addr.clone();
    tokio::spawn(async move {
        let _ = run_signaling_server(&server_addr_clone).await;
    });

    // Wait briefly for server startup
    tokio::time::sleep(Duration::from_millis(300)).await;

    // 2. Connect Host (Device A) and Controller (Device B)
    let host_id = "111222333".to_string();
    let controller_id = "444555666".to_string();

    let (host_client, mut host_events) = SignalingClient::connect(server_url.clone(), host_id.clone());
    let (ctrl_client, mut ctrl_events) = SignalingClient::connect(server_url.clone(), controller_id.clone());

    // Wait for both to be connected
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(host_client.is_connected());
    assert!(ctrl_client.is_connected());

    // 3. Controller sends ConnectRequest
    let session_id = "test_sess_100".to_string();
    let client_entropy = generate_entropy();
    ctrl_client
        .send_connect_request(session_id.clone(), host_id.clone(), client_entropy.to_vec())
        .unwrap();

    // 4. Host receives IncomingRequest
    let mut req_received = false;
    let timeout = tokio::time::sleep(Duration::from_secs(3));
    tokio::pin!(timeout);

    let host_entropy = generate_entropy();

    loop {
        tokio::select! {
            _ = &mut timeout => panic!("Timeout waiting for Host to receive IncomingRequest"),
            Some(event) = host_events.recv() => {
                if let SignalingEvent::IncomingRequest { session_id: s_id, from_id, key_exchange } = event {
                    assert_eq!(s_id, session_id);
                    assert_eq!(from_id, controller_id);
                    assert_eq!(key_exchange, client_entropy.to_vec());
                    req_received = true;

                    // Host accepts request
                    host_client.send_connect_response(
                        session_id.clone(),
                        true,
                        None,
                        Some(host_entropy.to_vec())
                    ).unwrap();
                    break;
                }
            }
        }
    }
    assert!(req_received);

    // 5. Controller receives ConnectResponse (Accepted)
    let mut resp_received = false;
    let timeout2 = tokio::time::sleep(Duration::from_secs(3));
    tokio::pin!(timeout2);

    loop {
        tokio::select! {
            _ = &mut timeout2 => panic!("Timeout waiting for Controller to receive ConnectResponse"),
            Some(event) = ctrl_events.recv() => {
                if let SignalingEvent::ConnectResponse { session_id: s_id, from_id, accepted, key_exchange, .. } = event {
                    assert_eq!(s_id, session_id);
                    assert_eq!(from_id, host_id);
                    assert!(accepted);
                    assert_eq!(key_exchange, Some(host_entropy.to_vec()));
                    resp_received = true;
                    break;
                }
            }
        }
    }
    assert!(resp_received);

    // 6. Establish encrypted channels on both ends
    let key = derive_session_key(&client_entropy, &host_entropy);
    let host_send = SecureChannel::new(&key, 0x01);
    let ctrl_recv = SecureChannel::new(&key, 0x01);

    let ctrl_send = SecureChannel::new(&key, 0x02);
    let host_recv = SecureChannel::new(&key, 0x02);

    // 7. Host streams a VideoFrame to Controller
    let test_frame = SessionMessage::VideoFrame {
        width: 1920,
        height: 1080,
        timestamp_ms: 50,
        is_keyframe: true,
        jpeg_data: vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10],
    };
    let serialized_frame = serde_json::to_vec(&test_frame).unwrap();
    let encrypted_frame = host_send.encrypt(&serialized_frame).unwrap();

    host_client
        .send_relay_data(session_id.clone(), controller_id.clone(), encrypted_frame)
        .unwrap();

    // 8. Controller receives and decrypts VideoFrame
    let mut frame_verified = false;
    let timeout3 = tokio::time::sleep(Duration::from_secs(3));
    tokio::pin!(timeout3);

    loop {
        tokio::select! {
            _ = &mut timeout3 => panic!("Timeout waiting for Controller to receive RelayedData"),
            Some(event) = ctrl_events.recv() => {
                if let SignalingEvent::RelayedData { session_id: s_id, from_id, payload } = event {
                    assert_eq!(s_id, session_id);
                    assert_eq!(from_id, host_id);
                    let decrypted = ctrl_recv.decrypt(&payload).unwrap();
                    let msg: SessionMessage = serde_json::from_slice(&decrypted).unwrap();
                    if let SessionMessage::VideoFrame { width, height, is_keyframe, .. } = msg {
                        assert_eq!(width, 1920);
                        assert_eq!(height, 1080);
                        assert!(is_keyframe);
                        frame_verified = true;
                        break;
                    }
                }
            }
        }
    }
    assert!(frame_verified);

    // 9. Controller sends an InputEvent (MouseMove) to Host
    let input_event = SessionMessage::InputEvent(InputPayload::MouseMove {
        norm_x: 0.75,
        norm_y: 0.25,
    });
    let serialized_input = serde_json::to_vec(&input_event).unwrap();
    let encrypted_input = ctrl_send.encrypt(&serialized_input).unwrap();

    ctrl_client
        .send_relay_data(session_id.clone(), host_id.clone(), encrypted_input)
        .unwrap();

    // 10. Host receives and decrypts InputEvent
    let mut input_verified = false;
    let timeout4 = tokio::time::sleep(Duration::from_secs(3));
    tokio::pin!(timeout4);

    loop {
        tokio::select! {
            _ = &mut timeout4 => panic!("Timeout waiting for Host to receive InputEvent"),
            Some(event) = host_events.recv() => {
                if let SignalingEvent::RelayedData { session_id: s_id, from_id, payload } = event {
                    assert_eq!(s_id, session_id);
                    assert_eq!(from_id, controller_id);
                    let decrypted = host_recv.decrypt(&payload).unwrap();
                    let msg: SessionMessage = serde_json::from_slice(&decrypted).unwrap();
                    if let SessionMessage::InputEvent(InputPayload::MouseMove { norm_x, norm_y }) = msg {
                        assert_eq!(norm_x, 0.75);
                        assert_eq!(norm_y, 0.25);
                        input_verified = true;
                        break;
                    }
                }
            }
        }
    }
    assert!(input_verified);

    // 11. Disconnect
    ctrl_client
        .send_disconnect(session_id.clone(), "User ended session".into())
        .unwrap();

    let mut disconnect_verified = false;
    let timeout5 = tokio::time::sleep(Duration::from_secs(3));
    tokio::pin!(timeout5);

    loop {
        tokio::select! {
            _ = &mut timeout5 => panic!("Timeout waiting for Host to receive PeerDisconnected"),
            Some(event) = host_events.recv() => {
                if let SignalingEvent::PeerDisconnected { session_id: s_id, reason } = event {
                    assert_eq!(s_id, session_id);
                    assert_eq!(reason, "User ended session");
                    disconnect_verified = true;
                    break;
                }
            }
        }
    }
    assert!(disconnect_verified);
}
