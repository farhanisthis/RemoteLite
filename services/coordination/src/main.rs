//! # OpenRemote Coordination Server
//!
//! Axum-based coordination and signaling server.
//!
//! Provides:
//! - Device registration and presence.
//! - Authenticated WebSocket signaling for ICE/SDP exchange.
//! - Pairing invitation management.
//! - Session signaling.
//! - Health and diagnostic endpoints.
//!
//! SECURITY:
//! - The server never decrypts session media content.
//! - Device authentication is verified before any signaling is forwarded.
//! - Pairing tokens are short-lived and single-use.
//! - Rate limiting is applied to all public endpoints.

mod error;
mod health;
mod pairing;
mod registry;
mod signaling;
mod state;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use openremote_observability::init_tracing;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "OpenRemote Coordination Server starting"
    );

    let app_state = state::AppState::new();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        // Health check — unauthenticated, used by load balancers and CI.
        .route("/health", get(health::health_handler))
        .route("/health/ready", get(health::readiness_handler))
        // Signaling WebSocket endpoint.
        .route("/ws/signal", get(signaling::ws_handler))
        // Pairing endpoints.
        .route("/api/v1/pairing/invite", post(pairing::create_invite))
        .route("/api/v1/pairing/complete", post(pairing::complete_pairing))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let addr: SocketAddr = bind_addr.parse()?;

    info!(addr = %addr, "coordination server listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
