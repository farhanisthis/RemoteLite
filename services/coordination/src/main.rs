//! # OpenRemote Coordination Server
//!
//! Axum-based coordination and signaling server binary.

use anyhow::Result;
use openremote_coordination::{build_router, AppState};
use openremote_observability::init_tracing;
use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "OpenRemote Coordination Server starting"
    );

    let app_state = AppState::new();
    let app = build_router(app_state);

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let addr: SocketAddr = bind_addr.parse()?;

    info!(addr = %addr, "Coordination server listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
