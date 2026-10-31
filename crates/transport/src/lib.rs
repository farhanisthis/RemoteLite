//! # OpenRemote Transport
//!
//! Real-Time Transport Protocol (RTP), Session Description Protocol (SDP),
//! and WebRTC media streaming for OpenRemote.

pub mod rtp;
pub mod sdp;
pub mod streamer;

pub use rtp::{packetize_h264_nalu, RtpHeader, RtpPacket, MAX_RTP_PAYLOAD_SIZE, RTP_HEADER_SIZE};
pub use sdp::{
    generate_h264_sdp_answer, generate_h264_sdp_offer, verify_h264_sdp, SdpSessionParams,
};
pub use streamer::{RtpVideoStreamer, StreamerMetrics};

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
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, data: Bytes) -> Result<(), TransportError>;
    async fn recv(&self) -> Result<Bytes, TransportError>;
    fn is_open(&self) -> bool;
    async fn close(&self) -> Result<(), TransportError>;
}
