//! # OpenRemote Session Agent
//!
//! Runs in the interactive Windows desktop session.
//! Responsible for screen capture, H.264 encoding, input injection,
//! and showing the visible remote-session indicator.
//!
//! ## Security design
//! - Receives work only from the host service via authenticated IPC.
//! - Never exposes arbitrary command execution.
//! - Shows a visible indicator on screen during any remote session.
//! - Provides a local emergency-disconnect mechanism.
//! - Terminates immediately if IPC authentication fails.
//!
//! M0: Stub. Real capture, encoding, and IPC are implemented in M1/M2.

use anyhow::Result;
use openremote_observability::init_tracing;
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
        info!("Session agent running (M0 stub — capture not yet implemented)");
        info!("Press Ctrl+C to exit");
        tokio::signal::ctrl_c().await?;
        info!("Session agent shutting down");
        Ok(())
    })
}
