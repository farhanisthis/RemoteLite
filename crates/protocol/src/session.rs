//! Session state and lifecycle types.

use crate::types::{DeviceId, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The current lifecycle state of a remote session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Signaling in progress; ICE candidates being exchanged.
    Connecting,
    /// ICE connected; WebRTC DTLS handshake in progress.
    Authenticating,
    /// Session fully established; media and data flowing.
    Active,
    /// Client is temporarily disconnected; host is waiting for reconnect.
    Suspended,
    /// Session has ended normally.
    Ended,
    /// Session ended due to an error.
    Failed,
}

/// Metadata about an active or historical session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: SessionId,
    pub host_device_id: DeviceId,
    pub client_device_id: DeviceId,
    pub state: SessionState,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub connection_type: Option<ConnectionType>,
}

/// How the session's media traffic is routed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionType {
    /// Direct peer-to-peer via ICE.
    Direct,
    /// Relayed via TURN server.
    Relayed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_state_serializes() {
        let state = SessionState::Active;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, r#""active""#);
    }
}
