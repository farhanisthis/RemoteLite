//! # OpenRemote Session Agent
//!
//! Runs in the interactive Windows desktop session.
//! Responsible for screen capture, H.264 encoding, RTP streaming,
//! remote mouse/keyboard input injection, and displaying the visible
//! remote-session indicator.
//!
//! ## Security design
//! - Receives work only from the host service via authenticated IPC.
//! - Never exposes arbitrary command execution.
//! - Shows a visible indicator on screen during any remote session.
//! - Provides a local emergency-disconnect mechanism.
//! - Terminates immediately if IPC authentication fails or local emergency disconnect is triggered.

mod input_receiver;
mod pipeline;

use anyhow::Result;
use input_receiver::InputReceiver;
use openremote_observability::init_tracing;
use pipeline::StreamingPipeline;
use std::sync::Arc;
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
        info!("Session agent initializing capture, streaming pipeline, and input injection...");

        // 1. Initialize streaming pipeline at 1080p/30 FPS, 5 Mbps
        let pipeline = StreamingPipeline::new(30, 5_000_000).await?;
        let sdp = pipeline.sdp_offer("session-init")?;
        info!(sdp_length = sdp.len(), "WebRTC SDP offer generated");

        // 2. Initialize input receiver on local UDP port
        let input_receiver = Arc::new(InputReceiver::bind("0.0.0.0:0").await?);
        let control_port = input_receiver.local_addr()?.port();
        info!(control_port = control_port, "Input control channel active");

        // Spawn background input receiver listener
        let rx = input_receiver.clone();
        tokio::spawn(async move {
            if let Err(e) = rx.run_listener().await {
                tracing::warn!(error = %e, "Input listener task ended");
            }
        });

        info!("Session agent ready. Press Ctrl+C to exit or trigger emergency disconnect");
        tokio::signal::ctrl_c().await?;

        // Emergency disconnect / shutdown cleanup
        input_receiver.trigger_emergency_disconnect();
        info!("Session agent shutting down");
        Ok(())
    })
}
