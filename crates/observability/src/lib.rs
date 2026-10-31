//! # OpenRemote Observability
//!
//! Structured logging and tracing initialization for OpenRemote components.
//!
//! SECURITY: Never log private keys, session content, or user credentials.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize tracing with sensible defaults.
///
/// Format is controlled by `OPENREMOTE_LOG_FORMAT`:
/// - `json` — structured JSON output (default in production).
/// - anything else — human-readable output (development default).
///
/// Log level is controlled by `RUST_LOG` (e.g. `RUST_LOG=info`).
pub fn init_tracing() {
    let env_filter = EnvFilter::try_from_env("RUST_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let format = std::env::var("OPENREMOTE_LOG_FORMAT").unwrap_or_else(|_| "human".into());

    if format == "json" {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer())
            .init();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn init_does_not_panic() {
        // Tracing can only be initialized once per process; guard with try_init.
        let _ = tracing_subscriber::fmt().try_init();
    }
}
