use crate::protocol::{ClientMessage, ServerMessage};
use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone)]
pub enum SignalingEvent {
    Connected,
    Registered {
        device_id: String,
    },
    IncomingRequest {
        session_id: String,
        from_id: String,
        key_exchange: Vec<u8>,
    },
    ConnectResponse {
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
    Disconnected,
    Error(String),
}

#[derive(Clone)]
pub struct SignalingClient {
    pub _server_url: String,
    pub _device_id: String,
    outgoing_tx: mpsc::UnboundedSender<ClientMessage>,
    is_connected: Arc<AtomicBool>,
}

impl SignalingClient {
    /// Connect to signaling server, start background loop, and return client handle and event receiver.
    pub fn connect(
        server_url: String,
        device_id: String,
    ) -> (Self, mpsc::UnboundedReceiver<SignalingEvent>) {
        let (out_tx, out_rx) = mpsc::unbounded_channel::<ClientMessage>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<SignalingEvent>();
        let is_connected = Arc::new(AtomicBool::new(false));

        let client = Self {
            _server_url: server_url.clone(),
            _device_id: device_id.clone(),
            outgoing_tx: out_tx,
            is_connected: Arc::clone(&is_connected),
        };

        // Spawn background connection and communication loop
        tokio::spawn(Self::connection_loop(
            server_url,
            device_id,
            out_rx,
            event_tx,
            is_connected,
        ));

        (client, event_rx)
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }

    pub fn send(&self, msg: ClientMessage) -> Result<()> {
        self.outgoing_tx
            .send(msg)
            .map_err(|_| anyhow!("Signaling client channel closed"))
    }

    pub fn send_connect_request(
        &self,
        session_id: String,
        target_id: String,
        key_exchange: Vec<u8>,
    ) -> Result<()> {
        self.send(ClientMessage::ConnectRequest {
            session_id,
            target_id,
            key_exchange,
        })
    }

    pub fn send_connect_response(
        &self,
        session_id: String,
        accepted: bool,
        reason: Option<String>,
        key_exchange: Option<Vec<u8>>,
    ) -> Result<()> {
        self.send(ClientMessage::ConnectResponse {
            session_id,
            accepted,
            reason,
            key_exchange,
        })
    }

    pub fn send_relay_data(
        &self,
        session_id: String,
        target_id: String,
        payload: Vec<u8>,
    ) -> Result<()> {
        self.send(ClientMessage::RelayData {
            session_id,
            target_id,
            payload,
        })
    }

    pub fn send_disconnect(&self, session_id: String, reason: String) -> Result<()> {
        self.send(ClientMessage::Disconnect { session_id, reason })
    }

    async fn connection_loop(
        server_url: String,
        device_id: String,
        out_rx: mpsc::UnboundedReceiver<ClientMessage>,
        event_tx: mpsc::UnboundedSender<SignalingEvent>,
        is_connected: Arc<AtomicBool>,
    ) {
        let out_rx = Arc::new(RwLock::new(out_rx));

        loop {
            tracing::info!("Connecting to signaling server at {}...", server_url);
            match connect_async(&server_url).await {
                Ok((ws_stream, _)) => {
                    tracing::info!("Connected to signaling server!");
                    is_connected.store(true, Ordering::SeqCst);
                    let _ = event_tx.send(SignalingEvent::Connected);

                    let (mut ws_sink, mut ws_stream) = ws_stream.split();

                    // Register our 9-digit device ID immediately
                    let reg_msg = ClientMessage::Register {
                        device_id: device_id.clone(),
                    };
                    if let Ok(json) = serde_json::to_string(&reg_msg) {
                        if let Err(e) = ws_sink.send(Message::Text(json)).await {
                            tracing::warn!("Failed to send registration: {}", e);
                            is_connected.store(false, Ordering::SeqCst);
                            let _ = event_tx.send(SignalingEvent::Disconnected);
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            continue;
                        }
                    }

                    // Task to send outgoing messages
                    let out_rx_clone = Arc::clone(&out_rx);
                    let send_task = tokio::spawn(async move {
                        let mut rx = out_rx_clone.write().await;
                        while let Some(msg) = rx.recv().await {
                            if let Ok(json) = serde_json::to_string(&msg) {
                                if ws_sink.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    });

                    // Read incoming messages
                    while let Some(msg_result) = ws_stream.next().await {
                        match msg_result {
                            Ok(Message::Text(text)) => {
                                if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                                    match server_msg {
                                        ServerMessage::Registered { success, device_id, reason } => {
                                            if success {
                                                let _ = event_tx.send(SignalingEvent::Registered { device_id });
                                            } else {
                                                let _ = event_tx.send(SignalingEvent::Error(reason.unwrap_or_else(|| "Registration failed".into())));
                                            }
                                        }
                                        ServerMessage::IncomingRequest { session_id, from_id, key_exchange } => {
                                            let _ = event_tx.send(SignalingEvent::IncomingRequest {
                                                session_id,
                                                from_id,
                                                key_exchange,
                                            });
                                        }
                                        ServerMessage::RequestResponse { session_id, from_id, accepted, reason, key_exchange } => {
                                            let _ = event_tx.send(SignalingEvent::ConnectResponse {
                                                session_id,
                                                from_id,
                                                accepted,
                                                reason,
                                                key_exchange,
                                            });
                                        }
                                        ServerMessage::RelayedData { session_id, from_id, payload } => {
                                            let _ = event_tx.send(SignalingEvent::RelayedData {
                                                session_id,
                                                from_id,
                                                payload,
                                            });
                                        }
                                        ServerMessage::PeerDisconnected { session_id, reason } => {
                                            let _ = event_tx.send(SignalingEvent::PeerDisconnected {
                                                session_id,
                                                reason,
                                            });
                                        }
                                        ServerMessage::Pong { .. } => {}
                                        ServerMessage::Error { message } => {
                                            let _ = event_tx.send(SignalingEvent::Error(message));
                                        }
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => break,
                            Err(_) => break,
                            _ => {}
                        }
                    }

                    send_task.abort();
                    is_connected.store(false, Ordering::SeqCst);
                    let _ = event_tx.send(SignalingEvent::Disconnected);
                }
                Err(e) => {
                    tracing::debug!("Signaling server connection failed: {}. Retrying in 3s...", e);
                    is_connected.store(false, Ordering::SeqCst);
                    let _ = event_tx.send(SignalingEvent::Disconnected);
                }
            }

            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }
}
