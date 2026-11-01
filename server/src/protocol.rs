use serde::{Deserialize, Serialize};

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
