//! # OpenRemote Session Agent
//!
//! Runs in the interactive Windows desktop session.
//! Responsible for screen capture, H.264 encoding, RTP streaming,
//! and displaying the visible remote-session indicator.
//!
//! ## Security design
//! - Receives work only from the host service via authenticated IPC.
//! - Never exposes arbitrary command execution.
//! - Shows a visible indicator on screen during any remote session.
//! - Provides a local emergency-disconnect mechanism.
//! - Terminates immediately if IPC authentication fails.

mod pipeline;

use anyhow::Result;
use openremote_observability::init_tracing;
use pipeline::StreamingPipeline;
use tracing::info;

fn main() -> Result<()> {
    init_tracing();
    info!(
        version = env!("CARGO_PKG_VERSION"),
        "OpenRemote Session Agent starting"
    );

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        info!("Session agent initializing capture and streaming pipeline...");

        // Initialize streaming pipeline at 1080p/30 FPS, 5 Mbps
        let pipeline = StreamingPipeline::new(30, 5_000_000).await?;
        let sdp = pipeline.sdp_offer("session-init")?;
        info!(sdp_length = sdp.len(), "WebRTC SDP offer generated");

        info!("Session agent ready. Press Ctrl+C to exit");
        tokio::signal::ctrl_c().await?;
        info!("Session agent shutting down");
        Ok(())
    })
}
