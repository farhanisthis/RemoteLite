use crate::protocol::{ClientMessage, ServerMessage};
use crate::server::registry::DeviceRegistry;
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

pub async fn run_signaling_server(bind_addr: &str) -> Result<()> {
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("Failed to bind signaling server on {}", bind_addr))?;

    let local_addr = listener.local_addr()?;
    tracing::info!("RemoteLite Signaling & Relay Server listening on ws://{}", local_addr);

    let registry = Arc::new(DeviceRegistry::new());

    loop {
        let (stream, peer_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!("Failed to accept incoming TCP connection: {}", e);
                continue;
            }
        };

        let registry = Arc::clone(&registry);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, peer_addr, registry).await {
                tracing::debug!("Connection closed from {}: {}", peer_addr, e);
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    registry: Arc<DeviceRegistry>,
) -> Result<()> {
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .context("WebSocket handshake error")?;

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Forward messages from internal channel to WebSocket client
    let forward_task = tokio::spawn(async move {
        while let Some(server_msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&server_msg) {
                if ws_sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut registered_device_id: Option<String> = None;

    while let Some(msg_result) = ws_receiver.next().await {
        let msg = match msg_result {
            Ok(m) => m,
            Err(_) => break,
        };

        let text = match msg {
            Message::Text(t) => t,
            Message::Binary(b) => match String::from_utf8(b) {
                Ok(s) => s,
                Err(_) => continue,
            },
            Message::Ping(_) => continue,
            Message::Pong(_) => continue,
            Message::Close(_) => break,
            Message::Frame(_) => continue,
        };

        let client_msg: ClientMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Failed to deserialize client message: {}", e);
                continue;
            }
        };

        match client_msg {
            ClientMessage::Register { device_id } => {
                tracing::info!("Registering Device ID: {} from {}", device_id, peer_addr);
                registry.register(device_id.clone(), tx.clone()).await.ok();
                registered_device_id = Some(device_id.clone());
                let _ = tx.send(ServerMessage::Registered {
                    success: true,
                    device_id,
                    reason: None,
                });
            }

            ClientMessage::ConnectRequest {
                session_id,
                target_id,
                key_exchange,
            } => {
                let from_id = registered_device_id.clone().unwrap_or_default();
                tracing::info!(
                    "Connection request for target {} from {} (Session: {})",
                    target_id,
                    from_id,
                    session_id
                );

                if !registry.is_online(&target_id).await {
                    let _ = tx.send(ServerMessage::RequestResponse {
                        session_id,
                        from_id: target_id,
                        accepted: false,
                        reason: Some("Target device is currently offline".to_string()),
                        key_exchange: None,
                    });
                } else {
                    // Record session request
                    registry
                        .create_session(session_id.clone(), from_id.clone(), target_id.clone())
                        .await;

                    let delivered = registry
                        .send_to(
                            &target_id,
                            ServerMessage::IncomingRequest {
                                session_id: session_id.clone(),
                                from_id: from_id.clone(),
                                key_exchange,
                            },
                        )
                        .await;

                    if !delivered {
                        registry.remove_session(&session_id).await;
                        let _ = tx.send(ServerMessage::RequestResponse {
                            session_id,
                            from_id: target_id,
                            accepted: false,
                            reason: Some("Failed to deliver request to target device".to_string()),
                            key_exchange: None,
                        });
                    }
                }
            }

            ClientMessage::ConnectResponse {
                session_id,
                accepted,
                reason,
                key_exchange,
            } => {
                let host_id = registered_device_id.clone().unwrap_or_default();
                tracing::info!(
                    "Host {} responded to session {}: accepted={}",
                    host_id,
                    session_id,
                    accepted
                );

                if let Some((controller_id, _)) = registry.get_session(&session_id).await {
                    registry
                        .send_to(
                            &controller_id,
                            ServerMessage::RequestResponse {
                                session_id: session_id.clone(),
                                from_id: host_id,
                                accepted,
                                reason,
                                key_exchange,
                            },
                        )
                        .await;

                    if !accepted {
                        registry.remove_session(&session_id).await;
                    }
                }
            }

            ClientMessage::RelayData {
                session_id,
                target_id,
                payload,
            } => {
                let from_id = registered_device_id.clone().unwrap_or_default();
                registry
                    .send_to(
                        &target_id,
                        ServerMessage::RelayedData {
                            session_id,
                            from_id,
                            payload,
                        },
                    )
                    .await;
            }

            ClientMessage::Disconnect { session_id, reason } => {
                if let Some((ctrl, host)) = registry.remove_session(&session_id).await {
                    let from_id = registered_device_id.as_deref().unwrap_or("");
                    let peer_id = if from_id == ctrl { host } else { ctrl };
                    registry
                        .send_to(
                            &peer_id,
                            ServerMessage::PeerDisconnected {
                                session_id,
                                reason: reason.clone(),
                            },
                        )
                        .await;
                }
            }

            ClientMessage::Ping { timestamp } => {
                let _ = tx.send(ServerMessage::Pong { timestamp });
            }
        }
    }

    if let Some(device_id) = registered_device_id {
        tracing::info!("Unregistering device: {}", device_id);
        registry.unregister(&device_id).await;
    }

    forward_task.abort();
    Ok(())
}
