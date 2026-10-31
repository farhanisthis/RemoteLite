//! # OpenRemote Transport
//!
//! Abstract transport interface used by the session layer.
//! Concrete implementations will use WebRTC DataChannels.

use bytes::Bytes;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("transport closed")]
    Closed,
    #[error("send failed: {0}")]
    SendFailed(String),
    #[error("receive failed: {0}")]
    ReceiveFailed(String),
    #[error("connection timeout")]
    Timeout,
}

/// Abstraction over a bidirectional byte channel.
///
/// In production this is backed by WebRTC DataChannels.
/// In tests it can be backed by an in-memory channel.
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, data: Bytes) -> Result<(), TransportError>;
    async fn recv(&self) -> Result<Bytes, TransportError>;
    fn is_open(&self) -> bool;
    async fn close(&self) -> Result<(), TransportError>;
}
