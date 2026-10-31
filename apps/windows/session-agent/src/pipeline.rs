//! Real-time desktop capture and video streaming pipeline.

use anyhow::Result;
use openremote_capture_windows::ScreenCapturer;
use openremote_encoder_windows::nalu::parse_annex_b_nalus;
use openremote_encoder_windows::{EncoderConfig, H264Encoder};
use openremote_transport::streamer::{RtpVideoStreamer, StreamerMetrics};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Live streaming pipeline controller.
#[allow(dead_code)]
pub struct StreamingPipeline {
    capturer: ScreenCapturer,
    encoder: H264Encoder,
    streamer: RtpVideoStreamer,
    is_streaming: Arc<AtomicBool>,
}

impl StreamingPipeline {
    pub async fn new(target_fps: u32, bitrate_bps: u32) -> Result<Self> {
        let capturer = ScreenCapturer::auto()?;
        let (width, height) = capturer.dimensions();

        let encoder_config = EncoderConfig {
            width,
            height,
            fps: target_fps,
            bitrate_bps,
            keyframe_interval: target_fps, // 1 IDR keyframe per second
            prefer_hardware: true,
        };

        let encoder = H264Encoder::new(encoder_config)?;
        let streamer = RtpVideoStreamer::bind("0.0.0.0:0").await?;

        info!(
            width = width,
            height = height,
            fps = target_fps,
            local_port = streamer.local_addr()?.port(),
            "StreamingPipeline initialized"
        );

        Ok(Self {
            capturer,
            encoder,
            streamer,
            is_streaming: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn sdp_offer(&self, session_id: &str) -> Result<String> {
        Ok(self.streamer.create_offer(session_id)?)
    }

    #[allow(dead_code)]
    pub fn is_streaming(&self) -> bool {
        self.is_streaming.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn stop(&self) {
        self.is_streaming.store(false, Ordering::Relaxed);
    }

    /// Run a timed streaming cycle (e.g. for testing or session execution)
    #[allow(dead_code)]
    pub async fn stream_for_duration(&mut self, duration: Duration) -> Result<StreamerMetrics> {
        self.is_streaming.store(true, Ordering::Relaxed);
        let start = Instant::now();
        let frame_interval = Duration::from_secs_f64(1.0 / self.encoder.config().fps as f64);
        let mut interval = tokio::time::interval(frame_interval);

        info!(
            duration_secs = duration.as_secs(),
            fps = self.encoder.config().fps,
            "Starting screen streaming loop"
        );

        while start.elapsed() < duration && self.is_streaming.load(Ordering::Relaxed) {
            interval.tick().await;

            let cap_start = Instant::now();
            let frame = match self.capturer.capture_frame() {
                Ok(f) => f,
                Err(e) => {
                    warn!(error = %e, "Failed to capture frame, skipping tick");
                    continue;
                }
            };
            let cap_ms = cap_start.elapsed().as_secs_f64() * 1000.0;

            let encoded = match self.encoder.encode_frame(&frame.data) {
                Ok(e) => e,
                Err(e) => {
                    warn!(error = %e, "Failed to encode frame");
                    continue;
                }
            };

            let nalus = parse_annex_b_nalus(&encoded.data);
            let _ = self.streamer.send_h264_frame(&nalus, frame_interval).await;

            if encoded.frame_index > 0 && encoded.frame_index % 30 == 0 {
                let metrics = self.streamer.metrics();
                info!(
                    frame = encoded.frame_index,
                    is_keyframe = encoded.is_keyframe,
                    cap_ms = format!("{:.1}", cap_ms),
                    enc_ms = format!("{:.1}", encoded.encode_time_ms),
                    bytes = encoded.data.len(),
                    total_sent = metrics.packets_sent,
                    "Stream progress"
                );
            }
        }

        self.is_streaming.store(false, Ordering::Relaxed);
        info!("Streaming loop completed");
        Ok(self.streamer.metrics())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn streaming_pipeline_runs_and_emits_frames() {
        let handle = std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("rt");

            rt.block_on(async {
                let mut pipeline = StreamingPipeline::new(30, 2_000_000)
                    .await
                    .expect("pipeline creation");

                let offer = pipeline.sdp_offer("test-sess").expect("sdp offer");
                assert!(offer.contains("m=video"));

                // Stream for 200ms (approx 6 frames at 30 FPS)
                let metrics = pipeline
                    .stream_for_duration(Duration::from_millis(200))
                    .await
                    .expect("stream duration");

                assert!(metrics.frames_sent > 0, "Must capture and stream frames");
                assert!(metrics.packets_sent > 0, "Must packetize RTP packets");
            });
        });

        handle.join().expect("thread join");
    }
}
