//! Real-time H.264 video RTP streaming session engine.

use crate::rtp::{packetize_h264_nalu, RtpHeader};
use crate::sdp::{
    generate_h264_sdp_answer, generate_h264_sdp_offer, verify_h264_sdp, SdpSessionParams,
};
use crate::TransportError;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;

/// Real-time streaming metrics and performance telemetry.
#[derive(Debug, Default, Clone)]
pub struct StreamerMetrics {
    pub frames_sent: u64,
    pub packets_sent: u64,
    pub bytes_sent: u64,
    pub last_frame_time_ms: f64,
}

/// RTP Video Streamer managing the live desktop feed.
pub struct RtpVideoStreamer {
    socket: Arc<UdpSocket>,
    target_addr: Option<SocketAddr>,
    header_template: RtpHeader,
    next_sequence: u16,
    timestamp_accumulator: u32,
    frames_sent: AtomicU64,
    packets_sent: AtomicU64,
    bytes_sent: AtomicU64,
}

impl RtpVideoStreamer {
    /// Bind a new RTP video streamer to a local port (or 0 for ephemeral).
    pub async fn bind(local_bind: &str) -> Result<Self, TransportError> {
        let socket = UdpSocket::bind(local_bind)
            .await
            .map_err(|e| TransportError::SendFailed(format!("failed to bind UDP socket: {e}")))?;

        let header_template = RtpHeader {
            version: 2,
            padding: false,
            extension: false,
            marker: false,
            payload_type: 96,
            sequence_number: 0,
            timestamp: 0,
            ssrc: 0x4F52454D, // 'OREM' (OpenRemote)
        };

        Ok(Self {
            socket: Arc::new(socket),
            target_addr: None,
            header_template,
            next_sequence: 1,
            timestamp_accumulator: 0,
            frames_sent: AtomicU64::new(0),
            packets_sent: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
        })
    }

    /// Set the destination peer address (e.g. Android client LAN IP:port).
    pub fn set_target(&mut self, addr: SocketAddr) {
        self.target_addr = Some(addr);
    }

    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        self.socket
            .local_addr()
            .map_err(|e| TransportError::SendFailed(e.to_string()))
    }

    /// Generate an SDP offer for this streaming session.
    pub fn create_offer(&self, session_id: &str) -> Result<String, TransportError> {
        let local = self.local_addr()?;
        let params = SdpSessionParams {
            session_id: session_id.to_string(),
            host_ip: local.ip().to_string(),
            video_port: local.port(),
            ssrc: self.header_template.ssrc,
            ..Default::default()
        };
        Ok(generate_h264_sdp_offer(&params))
    }

    /// Validate and apply an SDP answer from the client.
    pub fn apply_answer(
        &mut self,
        sdp: &str,
        client_addr: SocketAddr,
    ) -> Result<String, TransportError> {
        verify_h264_sdp(sdp)?;
        self.set_target(client_addr);
        Ok(generate_h264_sdp_answer(&SdpSessionParams {
            host_ip: client_addr.ip().to_string(),
            video_port: client_addr.port(),
            ..Default::default()
        }))
    }

    /// Packetize and stream a frame of NAL units at the target frame duration (e.g. 33.3ms for 30 FPS).
    pub async fn send_h264_frame(
        &mut self,
        nalus: &[&[u8]],
        frame_duration: Duration,
    ) -> Result<usize, TransportError> {
        if nalus.is_empty() {
            return Ok(0);
        }

        // Increment timestamp at 90 kHz: 90,000 ticks / sec * duration
        let ticks = (frame_duration.as_secs_f64() * 90_000.0) as u32;
        self.timestamp_accumulator = self.timestamp_accumulator.wrapping_add(ticks);
        self.header_template.timestamp = self.timestamp_accumulator;

        let total_nalus = nalus.len();
        let mut packets = Vec::new();

        for (idx, &nalu) in nalus.iter().enumerate() {
            let is_last = idx == total_nalus - 1;
            let nalu_packets = packetize_h264_nalu(
                nalu,
                &self.header_template,
                &mut self.next_sequence,
                is_last,
            );
            packets.extend(nalu_packets);
        }

        let mut total_bytes = 0;

        for packet in &packets {
            let serialized = packet.serialize();
            total_bytes += serialized.len();

            if let Some(target) = self.target_addr {
                let _ = self.socket.send_to(&serialized, target).await;
            }
        }

        self.frames_sent.fetch_add(1, Ordering::Relaxed);
        self.packets_sent
            .fetch_add(packets.len() as u64, Ordering::Relaxed);
        self.bytes_sent
            .fetch_add(total_bytes as u64, Ordering::Relaxed);

        Ok(packets.len())
    }

    /// Read current streaming performance metrics.
    pub fn metrics(&self) -> StreamerMetrics {
        StreamerMetrics {
            frames_sent: self.frames_sent.load(Ordering::Relaxed),
            packets_sent: self.packets_sent.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            last_frame_time_ms: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rtp_video_streamer_lifecycle_and_packetization() {
        let mut streamer = RtpVideoStreamer::bind("127.0.0.1:0")
            .await
            .expect("streamer bind");

        let offer = streamer.create_offer("test-sess-1").expect("create offer");
        assert!(offer.contains("m=video"));

        // Synthetic NAL units: SPS, PPS, IDR slice
        let sps = vec![0x67, 0x42, 0x80, 0x1F];
        let pps = vec![0x68, 0xCE, 0x3C, 0x80];
        let idr = vec![0x65, 0x88, 0x84, 0x00, 0xAA, 0xBB];
        let nalus = vec![sps.as_slice(), pps.as_slice(), idr.as_slice()];

        let pkt_count = streamer
            .send_h264_frame(&nalus, Duration::from_millis(33))
            .await
            .expect("send frame");
        assert_eq!(pkt_count, 3, "Each small NALU should produce 1 RTP packet");

        let metrics = streamer.metrics();
        assert_eq!(metrics.frames_sent, 1);
        assert_eq!(metrics.packets_sent, 3);
        assert!(metrics.bytes_sent > 0);
    }
}
