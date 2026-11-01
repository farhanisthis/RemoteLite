use crate::capture::ScreenCapture;
use crate::crypto::{derive_session_key, generate_entropy, SecureChannel};
use crate::encoder::FrameEncoder;
use crate::input::InputInjector;
use crate::net::signaling_client::SignalingClient;
use crate::protocol::{InputPayload, SessionMessage};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct HostSession {
    pub session_id: String,
    pub controller_id: String,
    signaling: SignalingClient,
    _send_channel: Arc<SecureChannel>,
    recv_channel: Arc<SecureChannel>,
    is_active: Arc<AtomicBool>,
    capture_handle: Option<JoinHandle<()>>,
    input_tx: mpsc::UnboundedSender<InputPayload>,
}

impl HostSession {
    /// Start a new host session after explicit user acceptance.
    /// Returns the session handle and host entropy to be sent in ConnectResponse.
    pub fn start(
        session_id: String,
        controller_id: String,
        client_entropy: &[u8],
        signaling: SignalingClient,
        target_fps: u32,
        jpeg_quality: u8,
    ) -> Result<(Self, Vec<u8>)> {
        let host_entropy = generate_entropy();
        let session_key = derive_session_key(client_entropy, &host_entropy);

        let send_channel = Arc::new(SecureChannel::new(&session_key, 0x01));
        let recv_channel = Arc::new(SecureChannel::new(&session_key, 0x02));

        let is_active = Arc::new(AtomicBool::new(true));
        let (input_tx, mut input_rx) = mpsc::unbounded_channel::<InputPayload>();

        // Spawn input injector task
        let is_active_input = Arc::clone(&is_active);
        tokio::spawn(async move {
            let mut injector = InputInjector::new();
            while is_active_input.load(Ordering::Relaxed) {
                match tokio::time::timeout(Duration::from_millis(500), input_rx.recv()).await {
                    Ok(Some(payload)) => {
                        let _ = injector.handle_input(&payload);
                    }
                    Ok(None) => break,
                    Err(_) => {}
                }
            }
            injector.release_all_keys();
        });

        // Spawn screen capture & streaming task
        let is_active_capture = Arc::clone(&is_active);
        let signaling_clone = signaling.clone();
        let session_id_clone = session_id.clone();
        let controller_id_clone = controller_id.clone();
        let send_channel_clone = Arc::clone(&send_channel);

        let capture_handle = tokio::spawn(async move {
            let mut capture = match ScreenCapture::new() {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to initialize screen capture: {}", e);
                    return;
                }
            };

            let mut encoder = FrameEncoder::new(jpeg_quality);
            let mut raw_buffer = Vec::new();
            let frame_duration = Duration::from_millis((1000 / target_fps.clamp(5, 60)) as u64);
            let mut keyframe_interval = 0u32;
            let start_time = Instant::now();

            tracing::info!(
                "Screen streaming started for session {} to controller {}",
                session_id_clone,
                controller_id_clone
            );

            while is_active_capture.load(Ordering::Relaxed) {
                let frame_start = Instant::now();
                keyframe_interval += 1;
                let force_keyframe = keyframe_interval >= (target_fps * 2);

                if capture.capture(&mut raw_buffer).is_ok() {
                    let (width, height) = capture.dimensions();
                    if let Ok(Some(jpeg_data)) =
                        encoder.encode_frame(&raw_buffer, width, height, force_keyframe)
                    {
                        if force_keyframe {
                            keyframe_interval = 0;
                        }

                        let timestamp_ms = start_time.elapsed().as_millis() as u64;
                        let session_msg = SessionMessage::VideoFrame {
                            width,
                            height,
                            timestamp_ms,
                            is_keyframe: force_keyframe,
                            jpeg_data,
                        };

                        if let Ok(serialized) = serde_json::to_vec(&session_msg) {
                            if let Ok(encrypted) = send_channel_clone.encrypt(&serialized) {
                                let _ = signaling_clone.send_relay_data(
                                    session_id_clone.clone(),
                                    controller_id_clone.clone(),
                                    encrypted,
                                );
                            }
                        }
                    }
                }

                let elapsed = frame_start.elapsed();
                if elapsed < frame_duration {
                    tokio::time::sleep(frame_duration - elapsed).await;
                }
            }

            tracing::info!("Screen streaming ended for session {}", session_id_clone);
        });

        let session = Self {
            session_id,
            controller_id,
            signaling,
            _send_channel: send_channel,
            recv_channel,
            is_active,
            capture_handle: Some(capture_handle),
            input_tx,
        };

        Ok((session, host_entropy.to_vec()))
    }

    /// Process incoming encrypted payload from controller.
    pub fn handle_incoming_payload(&self, payload: &[u8]) -> Result<()> {
        let decrypted = self.recv_channel.decrypt(payload)?;
        let msg: SessionMessage = serde_json::from_slice(&decrypted)?;
        if let SessionMessage::InputEvent(input_payload) = msg {
            let _ = self.input_tx.send(input_payload);
        }
        Ok(())
    }

    /// Stop sharing and terminate session.
    pub fn stop(&mut self, reason: &str) {
        if self.is_active.swap(false, Ordering::SeqCst) {
            tracing::info!("Stopping Host session: {}", reason);
            if let Some(handle) = self.capture_handle.take() {
                handle.abort();
            }
            let _ = self
                .signaling
                .send_disconnect(self.session_id.clone(), reason.to_string());
        }
    }
}

impl Drop for HostSession {
    fn drop(&mut self) {
        self.stop("Host session terminated");
    }
}
