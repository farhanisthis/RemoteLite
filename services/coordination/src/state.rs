//! Shared application state for the coordination server.

use crate::registry::DeviceRecord;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::trace;

/// Shared, cloneable application state.
#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<Inner>,
}

pub struct Inner {
    /// Active WebSocket signaling connections keyed by device ID string.
    pub connections: RwLock<HashMap<String, SignalingConnection>>,
    /// Active WebRTC sessions keyed by session ID string.
    pub sessions: RwLock<HashMap<String, SessionRoute>>,
    /// Registered devices keyed by device ID string.
    pub devices: RwLock<HashMap<String, DeviceRecord>>,
    /// Active pairing invitations keyed by token.
    pub pairing_invitations: RwLock<HashMap<String, PairingInvitation>>,
}

/// A connected signaling peer.
#[derive(Clone)]
#[allow(dead_code)]
pub struct SignalingConnection {
    pub device_id: String,
    pub sender: mpsc::Sender<String>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// A routed WebRTC session between host and client.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SessionRoute {
    pub host_device_id: String,
    pub client_device_id: String,
    pub established_at: chrono::DateTime<chrono::Utc>,
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
                sessions: RwLock::new(HashMap::new()),
                devices: RwLock::new(HashMap::new()),
                pairing_invitations: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Register or update a device in the coordination registry.
    pub async fn register_device(&self, record: DeviceRecord) {
        let mut devices = self.inner.devices.write().await;
        devices.insert(record.device_id.clone(), record);
    }

    /// Check if a device ID is registered.
    #[allow(dead_code)]
    pub async fn get_device(&self, device_id: &str) -> Option<DeviceRecord> {
        let devices = self.inner.devices.read().await;
        devices.get(device_id).cloned()
    }

    /// Add an active signaling WebSocket connection.
    pub async fn add_connection(&self, device_id: &str, sender: mpsc::Sender<String>) {
        let mut conns = self.inner.connections.write().await;
        conns.insert(
            device_id.to_string(),
            SignalingConnection {
                device_id: device_id.to_string(),
                sender,
                connected_at: chrono::Utc::now(),
            },
        );
    }

    /// Remove a signaling connection on disconnect.
    pub async fn remove_connection(&self, device_id: &str) {
        let mut conns = self.inner.connections.write().await;
        conns.remove(device_id);
    }

    /// Check if a target device is currently connected online.
    pub async fn is_online(&self, device_id: &str) -> bool {
        let conns = self.inner.connections.read().await;
        conns.contains_key(device_id)
    }

    /// Send a JSON string payload to a specific device. Returns true if sent.
    pub async fn send_to_device(&self, device_id: &str, payload: String) -> bool {
        let conns = self.inner.connections.read().await;
        if let Some(conn) = conns.get(device_id) {
            match conn.sender.send(payload).await {
                Ok(_) => {
                    trace!(device_id = %device_id, "Signaling message delivered to peer");
                    true
                }
                Err(_) => {
                    trace!(device_id = %device_id, "Failed to deliver message; channel closed");
                    false
                }
            }
        } else {
            false
        }
    }

    /// Register an active session between host and client.
    pub async fn create_session(&self, session_id: &str, host_id: &str, client_id: &str) {
        let mut sessions = self.inner.sessions.write().await;
        sessions.insert(
            session_id.to_string(),
            SessionRoute {
                host_device_id: host_id.to_string(),
                client_device_id: client_id.to_string(),
                established_at: chrono::Utc::now(),
            },
        );
    }

    /// Look up the peer device ID for a given session and sender.
    pub async fn get_session_peer(&self, session_id: &str, sender_id: &str) -> Option<String> {
        let sessions = self.inner.sessions.read().await;
        if let Some(route) = sessions.get(session_id) {
            if route.host_device_id == sender_id {
                Some(route.client_device_id.clone())
            } else if route.client_device_id == sender_id {
                Some(route.host_device_id.clone())
            } else {
                None // Sender is not a party in this session!
            }
        } else {
            None
        }
    }

    /// End and remove an active session.
    pub async fn remove_session(&self, session_id: &str) {
        let mut sessions = self.inner.sessions.write().await;
        sessions.remove(session_id);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
