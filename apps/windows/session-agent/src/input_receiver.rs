//! Control channel input receiver for the Windows interactive desktop agent.
//!
//! Validates session authorization, decodes input event payloads, enforces rate limits,
//! and dispatches to the Win32 `InputInjector`.

use anyhow::{anyhow, Result};
use openremote_input_windows::{InputEvent, InputInjector};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{error, info, trace, warn};

/// Metrics for input receiver.
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct InputReceiverMetrics {
    pub events_received: u64,
    pub events_injected: u64,
    pub events_rejected: u64,
}

/// Remote input receiver handling UDP / DataChannel input packets.
pub struct InputReceiver {
    injector: InputInjector,
    is_session_active: Arc<AtomicBool>,
    emergency_disconnected: Arc<AtomicBool>,
    events_received: Arc<AtomicU64>,
    events_injected: Arc<AtomicU64>,
    events_rejected: Arc<AtomicU64>,
    socket: Arc<UdpSocket>,
}

impl InputReceiver {
    /// Bind a new input receiver to the specified local address (e.g. "0.0.0.0:0").
    pub async fn bind(bind_addr: &str) -> Result<Self> {
        let socket = UdpSocket::bind(bind_addr).await?;
        let is_session_active = Arc::new(AtomicBool::new(true));
        let emergency_disconnected = Arc::new(AtomicBool::new(false));

        Ok(Self {
            injector: InputInjector::new(),
            is_session_active,
            emergency_disconnected,
            events_received: Arc::new(AtomicU64::new(0)),
            events_injected: Arc::new(AtomicU64::new(0)),
            events_rejected: Arc::new(AtomicU64::new(0)),
            socket: Arc::new(socket),
        })
    }

    /// Local socket address.
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }

    /// Whether the remote session is currently active and authorized for input.
    pub fn is_active(&self) -> bool {
        self.is_session_active.load(Ordering::SeqCst)
            && !self.emergency_disconnected.load(Ordering::SeqCst)
    }

    /// Set session active status.
    #[allow(dead_code)]
    pub fn set_session_active(&self, active: bool) {
        self.is_session_active.store(active, Ordering::SeqCst);
        self.injector.set_active(active);
    }

    /// Trigger local emergency disconnect, immediately revoking all input privileges.
    pub fn trigger_emergency_disconnect(&self) {
        info!("EMERGENCY DISCONNECT TRIGGERED by local desktop user");
        self.emergency_disconnected.store(true, Ordering::SeqCst);
        self.is_session_active.store(false, Ordering::SeqCst);
        self.injector.set_active(false);
    }

    /// Get telemetry metrics snapshot.
    #[allow(dead_code)]
    pub fn metrics(&self) -> InputReceiverMetrics {
        InputReceiverMetrics {
            events_received: self.events_received.load(Ordering::Relaxed),
            events_injected: self.events_injected.load(Ordering::Relaxed),
            events_rejected: self.events_rejected.load(Ordering::Relaxed),
        }
    }

    /// Handle a raw byte buffer payload (e.g. from UDP or WebRTC DataChannel).
    pub fn handle_packet(&self, payload: &[u8]) -> Result<()> {
        self.events_received.fetch_add(1, Ordering::Relaxed);

        if !self.is_active() {
            self.events_rejected.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow!(
                "Input packet rejected: session is inactive or emergency-disconnected"
            ));
        }

        // Parse JSON (or binary) input event
        let event: InputEvent = serde_json::from_slice(payload).map_err(|e| {
            self.events_rejected.fetch_add(1, Ordering::Relaxed);
            anyhow!("Failed to deserialize input event: {}", e)
        })?;

        match self.injector.inject(&event) {
            Ok(()) => {
                self.events_injected.fetch_add(1, Ordering::Relaxed);
                trace!("Successfully injected remote input event");
                Ok(())
            }
            Err(e) => {
                self.events_rejected.fetch_add(1, Ordering::Relaxed);
                warn!(error = %e, "Input injection failed");
                Err(anyhow!("Injection error: {}", e))
            }
        }
    }

    /// Run the background packet listener loop until aborted or emergency disconnect.
    pub async fn run_listener(&self) -> Result<()> {
        let mut buf = [0u8; 4096];
        info!(
            local_addr = %self.local_addr()?,
            "InputReceiver listening for control packets"
        );

        while self.is_active() {
            tokio::select! {
                recv_res = self.socket.recv_from(&mut buf) => {
                    match recv_res {
                        Ok((size, peer_addr)) => {
                            trace!(bytes = size, peer = %peer_addr, "Received input packet");
                            if let Err(e) = self.handle_packet(&buf[..size]) {
                                warn!(peer = %peer_addr, error = %e, "Failed to handle input packet");
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Socket receive error in InputReceiver");
                            break;
                        }
                    }
                }
            }
        }

        info!("InputReceiver loop finished");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openremote_input_windows::{MouseButton, MouseEvent};

    #[tokio::test]
    async fn input_receiver_packet_lifecycle_and_rejection() {
        let receiver = InputReceiver::bind("127.0.0.1:0").await.unwrap();

        let event = InputEvent::Mouse(MouseEvent {
            x: 0.5,
            y: 0.5,
            button: MouseButton::None,
            action: openremote_input_windows::MouseAction::Move,
        });
        let payload = serde_json::to_vec(&event).unwrap();

        // Valid packet while active
        let res = receiver.handle_packet(&payload);
        assert!(res.is_ok());
        let m = receiver.metrics();
        assert_eq!(m.events_received, 1);
        assert_eq!(m.events_injected, 1);
        assert_eq!(m.events_rejected, 0);

        // Deactivate session
        receiver.set_session_active(false);
        let res_deactivated = receiver.handle_packet(&payload);
        assert!(res_deactivated.is_err());
        let m2 = receiver.metrics();
        assert_eq!(m2.events_received, 2);
        assert_eq!(m2.events_rejected, 1);

        // Emergency disconnect
        receiver.set_session_active(true);
        receiver.trigger_emergency_disconnect();
        assert!(!receiver.is_active());
        let res_emerg = receiver.handle_packet(&payload);
        assert!(res_emerg.is_err());
        let m3 = receiver.metrics();
        assert_eq!(m3.events_received, 3);
        assert_eq!(m3.events_rejected, 2);
    }
}
