//! Shared application state for the coordination server.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared, cloneable application state.
#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<Inner>,
}

pub struct Inner {
    /// Active WebSocket signaling connections keyed by device ID string.
    pub connections: RwLock<HashMap<String, SignalingConnection>>,
    /// Active pairing invitations keyed by token.
    pub pairing_invitations: RwLock<HashMap<String, PairingInvitation>>,
}

/// A connected signaling peer.
#[allow(dead_code)]
pub struct SignalingConnection {
    pub device_id: String,
    pub sender: tokio::sync::mpsc::Sender<String>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// A pending pairing invitation.
#[allow(dead_code)]
pub struct PairingInvitation {
    pub token: String,
    pub host_device_id: String,
    pub host_public_key: String,
    pub host_display_name: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub used: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                connections: RwLock::new(HashMap::new()),
                pairing_invitations: RwLock::new(HashMap::new()),
            }),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
