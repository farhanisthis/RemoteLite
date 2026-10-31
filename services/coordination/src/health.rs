//! Health check endpoints.

use crate::state::AppState;
use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: chrono::DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct ReadinessResponse {
    pub ready: bool,
    pub connections: usize,
}

/// GET /health — liveness check.
pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        timestamp: Utc::now(),
    })
}

/// GET /health/ready — readiness check.
pub async fn readiness_handler(State(state): State<AppState>) -> Json<ReadinessResponse> {
    let connections = state.inner.connections.read().await.len();
    Json(ReadinessResponse {
        ready: true,
        connections,
    })
}
