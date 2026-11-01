use crate::crypto::{derive_session_key, SecureChannel};
use crate::encoder::decoder::FrameDecoder;
use crate::net::signaling_client::SignalingClient;
use crate::protocol::{InputPayload, SessionMessage};
use anyhow::Result;
use egui::ColorImage;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ViewerFrameData {
    pub image: ColorImage,
    pub width: u32,
    pub height: u32,
    pub latency_ms: u64,
}

pub struct ViewerSession {
    pub session_id: String,
    pub host_id: String,
    signaling: SignalingClient,
    send_channel: Arc<SecureChannel>,
    recv_channel: Arc<SecureChannel>,
    is_active: Arc<AtomicBool>,
    start_time: Instant,
    last_frame_instant: Instant,
    frame_count: u32,
    current_fps: f32,
}

impl ViewerSession {
    pub fn new(
        session_id: String,
        host_id: String,
        signaling: SignalingClient,
        client_entropy: &[u8],
        host_entropy: &[u8],
    ) -> Self {
        let session_key = derive_session_key(client_entropy, host_entropy);
        let send_channel = Arc::new(SecureChannel::new(&session_key, 0x02));
        let recv_channel = Arc::new(SecureChannel::new(&session_key, 0x01));

        Self {
            session_id,
            host_id,
            signaling,
            send_channel,
            recv_channel,
            is_active: Arc::new(AtomicBool::new(true)),
            start_time: Instant::now(),
            last_frame_instant: Instant::now(),
            frame_count: 0,
            current_fps: 0.0,
        }
    }

    /// Process incoming encrypted payload from host, decrypting and decoding video frame.
    pub fn handle_incoming_payload(&mut self, payload: &[u8]) -> Result<Option<ViewerFrameData>> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Ok(None);
        }

        let decrypted = self.recv_channel.decrypt(payload)?;
        let session_msg: SessionMessage = serde_json::from_slice(&decrypted)?;

        match session_msg {
            SessionMessage::VideoFrame {
                width,
                height,
                timestamp_ms,
                jpeg_data,
                ..
            } => {
                let color_image = FrameDecoder::decode_jpeg(&jpeg_data)?;
                let now_ms = self.start_time.elapsed().as_millis() as u64;
                let latency_ms = now_ms.saturating_sub(timestamp_ms);

                // Update FPS calculation
                self.frame_count += 1;
                let elapsed_since_last = self.last_frame_instant.elapsed();
                if elapsed_since_last.as_secs_f32() >= 1.0 {
                    self.current_fps = self.frame_count as f32 / elapsed_since_last.as_secs_f32();
                    self.frame_count = 0;
                    self.last_frame_instant = Instant::now();
                }

                Ok(Some(ViewerFrameData {
                    image: color_image,
                    width,
                    height,
                    latency_ms,
                }))
            }
            _ => Ok(None),
        }
    }

    /// Send user mouse or keyboard input event to remote host.
    pub fn send_input(&self, payload: InputPayload) -> Result<()> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Ok(());
        }

        let msg = SessionMessage::InputEvent(payload);
        let serialized = serde_json::to_vec(&msg)?;
        let encrypted = self.send_channel.encrypt(&serialized)?;

        self.signaling
            .send_relay_data(self.session_id.clone(), self.host_id.clone(), encrypted)?;

        Ok(())
    }

    pub fn fps(&self) -> f32 {
        self.current_fps
    }

    /// Disconnect from remote host.
    pub fn disconnect(&mut self, reason: &str) {
        if self.is_active.swap(false, Ordering::SeqCst) {
            tracing::info!("Disconnecting Viewer session: {}", reason);
            let _ = self
                .signaling
                .send_disconnect(self.session_id.clone(), reason.to_string());
        }
    }
}

impl Drop for ViewerSession {
    fn drop(&mut self) {
        self.disconnect("Viewer session closed");
    }
}
