//! # OpenRemote Coordination Server Library
//!
//! Provides the signaling WebSocket, device registry, pairing, and health endpoints.

pub mod error;
pub mod health;
pub mod pairing;
pub mod registry;
pub mod signaling;
pub mod state;

pub use state::AppState;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Build the Axum application router with all routes and middleware.
pub fn build_router(app_state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    Router::new()
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
        .with_state(app_state)
}
