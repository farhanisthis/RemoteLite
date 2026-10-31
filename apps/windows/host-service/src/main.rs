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
    use openremote_identity::keypair::DeviceKeypair;
    use openremote_session::manager::SessionManager;

    // Verify identity crate is functional.
    let keypair = DeviceKeypair::generate().expect("key generation must succeed on startup");
    let public = keypair.public_identity();
    let challenge = b"startup-self-test";
    let sig = keypair.sign(challenge).expect("signing must succeed");
    public
        .verify(challenge, &sig)
        .expect("signature verification must succeed");

    info!("identity self-test passed");

    // Verify session manager is functional.
    let _mgr = SessionManager::new();
    info!("session manager initialized");

    info!("startup checks complete — host service ready");
}
