//! WebSocket signaling handler with full authenticated peer routing.
//!
//! Clients and hosts connect to /ws/signal and exchange JSON-encoded Envelope messages.
//! The server routes messages between registered peers:
//! - Device registration & verification
//! - Session connection negotiation (ConnectRequest -> ConnectAccept/Reject)
//! - WebRTC SDP offer & answer relay
//! - Trickle ICE candidate relay
//! - Session termination cleanup

use crate::registry::DeviceRecord;
use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use openremote_protocol::messages::{
    ConnectRejectMessage, ConnectRejectReason, Envelope, ErrorMessage, RegisteredMessage,
    SignalingMessage,
};
use tokio::sync::mpsc;
use tracing::{debug, info, trace, warn};

/// Upgrade handler — upgrades HTTP to WebSocket.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    info!("Signaling WebSocket connection established");

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<String>(128);

    // Background task to send messages from tx queue to WebSocket client
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(Message::Text(msg)).await {
                debug!(error = %e, "Failed to send WebSocket message to client");
                break;
            }
        }
    });

    let mut authenticated_device_id: Option<String> = None;

    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let envelope: Envelope = match serde_json::from_str(&text) {
                    Ok(env) => env,
                    Err(e) => {
                        warn!(error = %e, "Received invalid JSON envelope");
                        let err_env = Envelope::new(SignalingMessage::Error(ErrorMessage {
                            code: "invalid_json".to_string(),
                            detail: format!("Malformed envelope JSON: {}", e),
                        }));
                        if let Ok(err_json) = serde_json::to_string(&err_env) {
                            let _ = tx.send(err_json).await;
                        }
                        continue;
                    }
                };

                let resp_opt =
                    process_message(&envelope, &state, &mut authenticated_device_id, &tx).await;

                if let Some(resp_env) = resp_opt {
                    if let Ok(resp_json) = serde_json::to_string(&resp_env) {
                        let _ = tx.send(resp_json).await;
                    }
                }
            }
            Ok(Message::Ping(_data)) => {
                let pong_env = Envelope::new(SignalingMessage::Pong);
                if let Ok(pong_json) = serde_json::to_string(&pong_env) {
                    let _ = tx.send(pong_json).await;
                }
            }
            Ok(Message::Close(_)) => {
                info!(device_id = ?authenticated_device_id, "Signaling WebSocket closed by client");
                break;
            }
            Err(e) => {
                warn!(error = %e, "Signaling WebSocket read error");
                break;
            }
            _ => {}
        }
    }

    // Clean up device connection on disconnect
    if let Some(ref dev_id) = authenticated_device_id {
        info!(device_id = %dev_id, "Unregistering disconnected device");
        state.remove_connection(dev_id).await;
    }

    send_task.abort();
    info!("Signaling WebSocket session finished");
}

/// Process a single incoming signaling message and return an optional response envelope for the sender.
async fn process_message(
    envelope: &Envelope,
    state: &AppState,
    authenticated_device_id: &mut Option<String>,
    sender_tx: &mpsc::Sender<String>,
) -> Option<Envelope> {
    match &envelope.payload {
        SignalingMessage::Register(reg) => {
            let dev_id_str = reg.device_id.to_string();
            info!(
                device_id = %dev_id_str,
                platform = ?reg.platform,
                display_name = %reg.display_name,
                "Registering device with coordination server"
            );

            // Save to registry
            let record = DeviceRecord {
                device_id: dev_id_str.clone(),
                public_key: reg.public_key.clone(),
                display_name: reg.display_name.clone(),
                platform: format!("{:?}", reg.platform),
                registered_at: Utc::now(),
                last_seen_at: Utc::now(),
            };
            state.register_device(record).await;

            // Associate current WebSocket sender with this device
            state.add_connection(&dev_id_str, sender_tx.clone()).await;
            *authenticated_device_id = Some(dev_id_str.clone());

            Some(Envelope::new(SignalingMessage::Registered(
                RegisteredMessage {
                    device_id: reg.device_id.clone(),
                    server_timestamp: Utc::now(),
                },
            )))
        }

        SignalingMessage::HostAnnounce(ann) => {
            let host_id_str = ann.host_device_id.to_string();
            info!(host_id = %host_id_str, display_name = %ann.display_name, "Host presence announced");
            None
        }

        SignalingMessage::ConnectRequest(req) => {
            let host_id = req.host_device_id.to_string();
            let client_id = req.client_device_id.to_string();
            let session_id = req.session_id.to_string();

            info!(
                session_id = %session_id,
                client_id = %client_id,
                host_id = %host_id,
                "Processing connection request"
            );

            // Verify host is currently online
            if !state.is_online(&host_id).await {
                warn!(host_id = %host_id, "Host is offline; rejecting connect request");
                return Some(Envelope::new(SignalingMessage::ConnectReject(
                    ConnectRejectMessage {
                        session_id: req.session_id.clone(),
                        reason: ConnectRejectReason::HostOffline,
                    },
                )));
            }

            // Record session routing table entry
            state
                .create_session(&session_id, &host_id, &client_id)
                .await;

            // Forward ConnectRequest directly to the target host
            if let Ok(forward_json) = serde_json::to_string(envelope) {
                if !state.send_to_device(&host_id, forward_json).await {
                    warn!(host_id = %host_id, "Failed to deliver ConnectRequest to host");
                    return Some(Envelope::new(SignalingMessage::ConnectReject(
                        ConnectRejectMessage {
                            session_id: req.session_id.clone(),
                            reason: ConnectRejectReason::HostOffline,
                        },
                    )));
                }
            }

            None
        }

        SignalingMessage::ConnectAccept(acc) => {
            let session_id = acc.session_id.to_string();
            let sender_id = authenticated_device_id.as_deref().unwrap_or("");

            if let Some(client_id) = state.get_session_peer(&session_id, sender_id).await {
                info!(session_id = %session_id, client_id = %client_id, "Forwarding ConnectAccept to client");
                if let Ok(json) = serde_json::to_string(envelope) {
                    state.send_to_device(&client_id, json).await;
                }
            } else {
                warn!(session_id = %session_id, "Cannot route ConnectAccept: session not found");
            }
            None
        }

        SignalingMessage::ConnectReject(rej) => {
            let session_id = rej.session_id.to_string();
            let sender_id = authenticated_device_id.as_deref().unwrap_or("");

            if let Some(client_id) = state.get_session_peer(&session_id, sender_id).await {
                info!(session_id = %session_id, client_id = %client_id, "Forwarding ConnectReject to client");
                if let Ok(json) = serde_json::to_string(envelope) {
                    state.send_to_device(&client_id, json).await;
                }
            }
            state.remove_session(&session_id).await;
            None
        }

        SignalingMessage::SdpOffer(offer) => {
            let session_id = offer.session_id.to_string();
            let sender_id = authenticated_device_id.as_deref().unwrap_or("");

            if let Some(target_id) = state.get_session_peer(&session_id, sender_id).await {
                trace!(session_id = %session_id, target = %target_id, "Routing SDP offer");
                if let Ok(json) = serde_json::to_string(envelope) {
                    state.send_to_device(&target_id, json).await;
                }
            } else {
                warn!(session_id = %session_id, sender = %sender_id, "SDP offer for unknown session");
                return Some(Envelope::new(SignalingMessage::Error(ErrorMessage {
                    code: "session_not_found".into(),
                    detail: format!("Session {} not active or unauthorized", session_id),
                })));
            }
            None
        }

        SignalingMessage::SdpAnswer(answer) => {
            let session_id = answer.session_id.to_string();
            let sender_id = authenticated_device_id.as_deref().unwrap_or("");

            if let Some(target_id) = state.get_session_peer(&session_id, sender_id).await {
                trace!(session_id = %session_id, target = %target_id, "Routing SDP answer");
                if let Ok(json) = serde_json::to_string(envelope) {
                    state.send_to_device(&target_id, json).await;
                }
            } else {
                warn!(session_id = %session_id, sender = %sender_id, "SDP answer for unknown session");
            }
            None
        }

        SignalingMessage::IceCandidate(ice) => {
            let session_id = ice.session_id.to_string();
            let sender_id = authenticated_device_id.as_deref().unwrap_or("");

            if let Some(target_id) = state.get_session_peer(&session_id, sender_id).await {
                trace!(session_id = %session_id, target = %target_id, "Routing ICE candidate");
                if let Ok(json) = serde_json::to_string(envelope) {
                    state.send_to_device(&target_id, json).await;
                }
            }
            None
        }

        SignalingMessage::SessionEnd(end) => {
            let session_id = end.session_id.to_string();
            let sender_id = authenticated_device_id.as_deref().unwrap_or("");

            if let Some(target_id) = state.get_session_peer(&session_id, sender_id).await {
                info!(session_id = %session_id, target = %target_id, "Routing SessionEnd and cleaning up");
                if let Ok(json) = serde_json::to_string(envelope) {
                    state.send_to_device(&target_id, json).await;
                }
            }
            state.remove_session(&session_id).await;
            None
        }

        SignalingMessage::Ping => Some(Envelope::new(SignalingMessage::Pong)),

        SignalingMessage::Pong => None,

        _ => None,
    }
}
