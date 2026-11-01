use crate::protocol::ServerMessage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Clone, Default)]
pub struct DeviceRegistry {
    devices: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<ServerMessage>>>>,
    sessions: Arc<RwLock<HashMap<String, (String, String)>>>, // session_id -> (controller_id, host_id)
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a device with its 9-digit ID and message sender.
    pub async fn register(
        &self,
        device_id: String,
        sender: mpsc::UnboundedSender<ServerMessage>,
    ) -> Result<(), String> {
        let mut devices = self.devices.write().await;
        if devices.contains_key(&device_id) {
            tracing::warn!("Device ID {} re-registering, replacing previous connection", device_id);
        }
        devices.insert(device_id, sender);
        Ok(())
    }

    /// Unregister a device upon disconnect.
    pub async fn unregister(&self, device_id: &str) {
        let mut devices = self.devices.write().await;
        devices.remove(device_id);

        let mut sessions = self.sessions.write().await;
        let mut sessions_to_drop = Vec::new();
        for (session_id, (ctrl, host)) in sessions.iter() {
            if ctrl == device_id || host == device_id {
                let peer_id = if ctrl == device_id { host } else { ctrl };
                if let Some(peer_tx) = devices.get(peer_id) {
                    let _ = peer_tx.send(ServerMessage::PeerDisconnected {
                        session_id: session_id.clone(),
                        reason: "Remote peer disconnected".to_string(),
                    });
                }
                sessions_to_drop.push(session_id.clone());
            }
        }
        for s in sessions_to_drop {
            sessions.remove(&s);
        }
    }

    /// Record a requested or active session.
    pub async fn create_session(&self, session_id: String, controller_id: String, host_id: String) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, (controller_id, host_id));
    }

    /// Retrieve the session parties (controller_id, host_id).
    pub async fn get_session(&self, session_id: &str) -> Option<(String, String)> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Remove an active session.
    pub async fn remove_session(&self, session_id: &str) -> Option<(String, String)> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id)
    }

    /// Send a message to a specific registered device.
    pub async fn send_to(&self, device_id: &str, msg: ServerMessage) -> bool {
        let devices = self.devices.read().await;
        if let Some(tx) = devices.get(device_id) {
            tx.send(msg).is_ok()
        } else {
            false
        }
    }

    /// Check if a device is currently registered and online.
    pub async fn is_online(&self, device_id: &str) -> bool {
        let devices = self.devices.read().await;
        devices.contains_key(device_id)
    }
}
