use serde::{Deserialize, Serialize};

/// Signaling messages sent from Client to Server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Register {
        device_id: String,
    },
    ConnectRequest {
        session_id: String,
        target_id: String,
        key_exchange: Vec<u8>,
    },
    ConnectResponse {
        session_id: String,
        accepted: bool,
        reason: Option<String>,
        key_exchange: Option<Vec<u8>>,
    },
    RelayData {
        session_id: String,
        target_id: String,
        payload: Vec<u8>,
    },
    Disconnect {
        session_id: String,
        reason: String,
    },
    Ping {
        timestamp: u64,
    },
}

/// Signaling messages sent from Server to Client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    Registered {
        success: bool,
        device_id: String,
        reason: Option<String>,
    },
    IncomingRequest {
        session_id: String,
        from_id: String,
        key_exchange: Vec<u8>,
    },
    RequestResponse {
        session_id: String,
        from_id: String,
        accepted: bool,
        reason: Option<String>,
        key_exchange: Option<Vec<u8>>,
    },
    RelayedData {
        session_id: String,
        from_id: String,
        payload: Vec<u8>,
    },
    PeerDisconnected {
        session_id: String,
        reason: String,
    },
    Pong {
        timestamp: u64,
    },
    Error {
        message: String,
    },
}

/// End-to-End encrypted session messages between Host and Controller.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionMessage {
    VideoFrame {
        width: u32,
        height: u32,
        timestamp_ms: u64,
        is_keyframe: bool,
        jpeg_data: Vec<u8>,
    },
    InputEvent(InputPayload),
    KeepAlive {
        timestamp_ms: u64,
    },
    SessionTerminate {
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum InputPayload {
    /// Normalized coordinates (0.0 to 1.0) on the remote screen.
    MouseMove {
        norm_x: f32,
        norm_y: f32,
    },
    MouseDown {
        button: MouseButton,
        norm_x: f32,
        norm_y: f32,
    },
    MouseUp {
        button: MouseButton,
        norm_x: f32,
        norm_y: f32,
    },
    MouseScroll {
        delta_x: f32,
        delta_y: f32,
    },
    KeyDown {
        virtual_key: u32,
        scancode: u32,
        text: Option<String>,
    },
    KeyUp {
        virtual_key: u32,
        scancode: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signaling_serde() {
        let msg = ClientMessage::ConnectRequest {
            session_id: "sess_1".into(),
            target_id: "987654321".into(),
            key_exchange: vec![1, 2, 3],
        };
        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: ClientMessage = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ClientMessage::ConnectRequest { session_id, .. } => {
                assert_eq!(session_id, "sess_1");
            }
            _ => panic!("Deserialization mismatch"),
        }
    }

    #[test]
    fn test_session_message_serde() {
        let frame = SessionMessage::VideoFrame {
            width: 1920,
            height: 1080,
            timestamp_ms: 12345,
            is_keyframe: true,
            jpeg_data: vec![0xFF, 0xD8, 0xFF],
        };
        let serialized = serde_json::to_string(&frame).unwrap();
        let deserialized: SessionMessage = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            SessionMessage::VideoFrame { width, height, .. } => {
                assert_eq!(width, 1920);
                assert_eq!(height, 1080);
            }
            _ => panic!("Deserialization mismatch"),
        }
    }
}
