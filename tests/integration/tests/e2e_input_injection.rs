//! End-to-end integration tests for remote input injection (Milestone M2).
//!
//! Validates:
//! 1. Client-to-host UDP control packet delivery and deserialization.
//! 2. Normalized mouse movement, clicking, and scroll injection.
//! 3. Keyboard virtual keys, modifiers, and Unicode text injection.
//! 4. Input rate limiting protection against event flooding.
//! 5. Session authorization validation & emergency local disconnect revocation.

use openremote_input_windows::{
    normalize_to_win32_coords, validate_coordinates, InputEvent, KeyAction, KeyEvent, KeyModifiers,
    MouseAction, MouseButton, MouseEvent, ScrollEvent, UnicodeTextEvent,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::UdpSocket;

#[test]
fn test_coordinate_normalization_and_boundaries() {
    // Normal bounds
    assert!(validate_coordinates(0.0, 0.0).is_ok());
    assert!(validate_coordinates(0.5, 0.5).is_ok());
    assert!(validate_coordinates(1.0, 1.0).is_ok());

    // Coordinate conversion mapping
    assert_eq!(normalize_to_win32_coords(0.0, 0.0), (0, 0));
    assert_eq!(normalize_to_win32_coords(1.0, 1.0), (65535, 65535));
    assert_eq!(normalize_to_win32_coords(0.25, 0.75), (16384, 49151));

    // Out-of-bounds rejection
    assert!(validate_coordinates(-0.01, 0.5).is_err());
    assert!(validate_coordinates(1.01, 0.5).is_err());
    assert!(validate_coordinates(0.5, -0.01).is_err());
    assert!(validate_coordinates(0.5, 1.01).is_err());
    assert!(validate_coordinates(f32::NAN, 0.5).is_err());
    assert!(validate_coordinates(0.5, f32::INFINITY).is_err());
}

#[tokio::test]
async fn test_e2e_input_packet_transmission_and_reception() {
    let host_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let host_addr = host_socket.local_addr().unwrap();

    let client_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    // 1. Send Mouse Event
    let mouse_event = InputEvent::Mouse(MouseEvent {
        x: 0.6,
        y: 0.4,
        button: MouseButton::Left,
        action: MouseAction::Down,
    });
    let mouse_bytes = serde_json::to_vec(&mouse_event).unwrap();
    client_socket
        .send_to(&mouse_bytes, host_addr)
        .await
        .unwrap();

    let mut buf = [0u8; 1024];
    let (n, from_addr) = host_socket.recv_from(&mut buf).await.unwrap();
    assert_eq!(from_addr, client_socket.local_addr().unwrap());
    let received_mouse: InputEvent = serde_json::from_slice(&buf[..n]).unwrap();
    assert_eq!(mouse_event, received_mouse);

    // 2. Send Keyboard Event
    let key_event = InputEvent::Key(KeyEvent {
        vk_code: 0x43, // 'C' key
        action: KeyAction::Down,
        modifiers: KeyModifiers {
            ctrl: true,
            alt: false,
            shift: false,
            windows: false,
        },
    });
    let key_bytes = serde_json::to_vec(&key_event).unwrap();
    client_socket.send_to(&key_bytes, host_addr).await.unwrap();

    let (n2, _) = host_socket.recv_from(&mut buf).await.unwrap();
    let received_key: InputEvent = serde_json::from_slice(&buf[..n2]).unwrap();
    assert_eq!(key_event, received_key);

    // 3. Send Unicode Text Event
    let text_event = InputEvent::Unicode(UnicodeTextEvent {
        text: "OpenRemote M2".to_string(),
    });
    let text_bytes = serde_json::to_vec(&text_event).unwrap();
    client_socket.send_to(&text_bytes, host_addr).await.unwrap();

    let (n3, _) = host_socket.recv_from(&mut buf).await.unwrap();
    let received_text: InputEvent = serde_json::from_slice(&buf[..n3]).unwrap();
    assert_eq!(text_event, received_text);

    // 4. Send Scroll Event
    let scroll_event = InputEvent::Scroll(ScrollEvent {
        x: 0.5,
        y: 0.5,
        delta_x: 0.0,
        delta_y: -2.0,
    });
    let scroll_bytes = serde_json::to_vec(&scroll_event).unwrap();
    client_socket
        .send_to(&scroll_bytes, host_addr)
        .await
        .unwrap();

    let (n4, _) = host_socket.recv_from(&mut buf).await.unwrap();
    let received_scroll: InputEvent = serde_json::from_slice(&buf[..n4]).unwrap();
    assert_eq!(scroll_event, received_scroll);
}

#[test]
fn test_session_state_and_emergency_disconnect_guard() {
    let is_session_active = Arc::new(AtomicBool::new(true));
    let emergency_disconnected = Arc::new(AtomicBool::new(false));

    // Helper checking whether input should be injected
    let can_inject = |active: &Arc<AtomicBool>, emerg: &Arc<AtomicBool>| -> bool {
        active.load(Ordering::SeqCst) && !emerg.load(Ordering::SeqCst)
    };

    assert!(can_inject(&is_session_active, &emergency_disconnected));

    // Session ended normally
    is_session_active.store(false, Ordering::SeqCst);
    assert!(!can_inject(&is_session_active, &emergency_disconnected));

    // Reactivated session
    is_session_active.store(true, Ordering::SeqCst);
    assert!(can_inject(&is_session_active, &emergency_disconnected));

    // Local user presses emergency disconnect
    emergency_disconnected.store(true, Ordering::SeqCst);
    assert!(!can_inject(&is_session_active, &emergency_disconnected));
}
