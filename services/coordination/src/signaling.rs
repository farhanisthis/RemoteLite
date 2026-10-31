//! WebSocket signaling handler.
//!
//! Clients connect to /ws/signal and exchange JSON-encoded Envelope messages.
//! The server routes messages between registered peers.
//!
//! M0: Basic WebSocket echo/routing skeleton. Full authenticated routing
//! is implemented in M3.

use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use tracing::{info, warn};

/// Upgrade handler — upgrades HTTP to WebSocket.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, _state: AppState) {
    info!("signaling WebSocket connection established");

    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                // M0: Echo back for connectivity testing.
                // M3 will implement authenticated message routing.
                if let Err(e) = socket.send(Message::Text(text)).await {
                    warn!(error = %e, "failed to send WebSocket message");
                    break;
                }
            }
            Ok(Message::Ping(data)) => {
                if let Err(e) = socket.send(Message::Pong(data)).await {
                    warn!(error = %e, "failed to send pong");
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                info!("signaling WebSocket closed by client");
                break;
            }
            Err(e) => {
                warn!(error = %e, "WebSocket error");
                break;
            }
            _ => {}
        }
    }

    info!("signaling WebSocket connection ended");
}
