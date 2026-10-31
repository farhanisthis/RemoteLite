//! System reboot execution and permission verification.

use crate::error::HostServiceError;
use tracing::{info, warn};

/// Execute a system reboot with a delay in seconds.
///
/// In testing or when `OPENREMOTE_SIMULATE_REBOOT=1` is set, the shutdown
/// command execution is simulated rather than executing real PC shutdown.
pub fn trigger_system_reboot(countdown_seconds: u32) -> Result<(), HostServiceError> {
    info!(
        countdown_seconds = countdown_seconds,
        "Initiating system reboot"
    );

    // SAFETY: Never execute real Windows OS shutdown during tests or normal dev runs
    // unless explicitly enabled via OPENREMOTE_ALLOW_REAL_REBOOT=1.
    if std::env::var("OPENREMOTE_ALLOW_REAL_REBOOT").unwrap_or_default() != "1" {
        info!("Reboot simulation mode active (default) — skipping real OS shutdown execution");
        return Ok(());
    }

    #[cfg(windows)]
    {
        let comment = "OpenRemote authorized remote reboot";
        let status = std::process::Command::new("shutdown.exe")
            .args(["/r", "/t", &countdown_seconds.to_string(), "/c", comment])
            .status()
            .map_err(|e| {
                HostServiceError::Internal(format!("Failed to execute shutdown.exe: {e}"))
            })?;

        if !status.success() {
            warn!(exit_code = ?status.code(), "shutdown.exe returned non-zero exit code");
            return Err(HostServiceError::Internal(format!(
                "shutdown.exe failed with exit code {:?}",
                status.code()
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reboot_simulation_succeeds() {
        assert!(trigger_system_reboot(5).is_ok());
    }
}
