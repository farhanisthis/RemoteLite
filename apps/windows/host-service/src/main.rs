//! # OpenRemote Host Service
//!
//! Windows background service that maintains device identity, manages
//! trusted device authorization, handles coordination server connectivity,
//! and controls session lifecycle.
//!
//! This binary runs as a Windows Service (or in console mode during
//! development). The actual interactive desktop capture and input
//! injection live in the separate session-agent binary.
//!
//! ## Security design
//! - Runs with minimum required privileges.
//! - Does not perform screen capture or input injection directly.
//! - Communicates with the session agent via authenticated local IPC.
//! - Private key is loaded from platform-secure storage (DPAPI in production).
//! - All authorization decisions are logged.

use anyhow::Result;
use openremote_observability::init_tracing;
use tracing::info;

fn main() -> Result<()> {
    init_tracing();
    info!(
        version = env!("CARGO_PKG_VERSION"),
        "OpenRemote Host Service starting"
    );

    // In M0 we run in console mode.
    // Windows Service wiring is implemented as part of M4
    // (persistent unattended access infrastructure).
    run_console_mode()
}

fn run_console_mode() -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        info!("Host service running in console mode (M0)");
        info!("Press Ctrl+C to exit");

        // M0: Perform startup self-checks and then park.
        // Real coordination server connection is implemented in M3.
        perform_startup_checks();

        tokio::signal::ctrl_c().await?;
        info!("Host service shutting down");
        Ok(())
    })
}

fn perform_startup_checks() {
    use openremote_host_service::{HostConfig, HostService};

    // Initialize the host service state
    let config = HostConfig::default();
    let service = HostService::init(config).expect("Host service initialization must succeed");

    info!(
        host_device_id = %service.device_id(),
        display_name = service.display_name(),
        "Host service initialized with identity and authorization registry"
    );

    // Verify cryptographic self-test
    let challenge = b"startup-self-test";
    let sig = service
        .keypair()
        .sign(challenge)
        .expect("signing must succeed");
    service
        .keypair()
        .public_identity()
        .verify(challenge, &sig)
        .expect("signature verification must succeed");

    info!("Cryptographic identity self-test passed");
    info!("Startup checks complete — host service ready");
}
