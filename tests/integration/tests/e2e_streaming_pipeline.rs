//! End-to-end integration test: Desktop capture -> H.264 encode -> RTP packetization -> SDP negotiation.

use openremote_capture_windows::ScreenCapturer;
use openremote_encoder_windows::nalu::parse_annex_b_nalus;
use openremote_encoder_windows::{EncoderConfig, H264Encoder};
use openremote_transport::streamer::RtpVideoStreamer;
use std::time::{Duration, Instant};

#[tokio::test]
async fn e2e_live_desktop_streaming_and_sdp_negotiation() {
    let handle = std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");

        rt.block_on(async {
            // 1. Desktop Screen Capture
            let capturer = ScreenCapturer::auto().expect("screen capturer must initialize");
            let (width, height) = capturer.dimensions();
            assert!(width > 0 && height > 0);

            let t_cap = Instant::now();
            let frame = capturer.capture_frame().expect("capture real desktop frame");
            let cap_time_ms = t_cap.elapsed().as_secs_f64() * 1000.0;
            assert_eq!(frame.width, width);
            assert_eq!(frame.height, height);

            // 2. H.264 Encoding
            let mut encoder = H264Encoder::new(EncoderConfig {
                width,
                height,
                fps: 30,
                bitrate_bps: 4_000_000,
                keyframe_interval: 30,
                prefer_hardware: true,
            })
            .expect("encoder init");

            let t_enc = Instant::now();
            let encoded = encoder.encode_frame(&frame.data).expect("encode frame");
            let enc_time_ms = t_enc.elapsed().as_secs_f64() * 1000.0;

            assert!(encoded.is_keyframe);
            assert!(!encoded.data.is_empty());

            // 3. WebRTC SDP Negotiation
            let mut streamer = RtpVideoStreamer::bind("127.0.0.1:0")
                .await
                .expect("bind streamer");

            let sdp_offer = streamer.create_offer("test-sess-stream").expect("sdp offer");
            assert!(sdp_offer.contains("m=video"));
            assert!(sdp_offer.contains("H264/90000"));

            // Client answer
            let client_addr = "127.0.0.1:9876".parse().unwrap();
            let client_sdp_answer = "v=0\r\nm=video 9876 UDP/TLS/RTP/SAVPF 96\r\na=rtpmap:96 H264/90000\r\na=recvonly\r\n";
            let ans = streamer.apply_answer(client_sdp_answer, client_addr);
            assert!(ans.is_ok());

            // 4. Packetize and Stream
            let nalus = parse_annex_b_nalus(&encoded.data);
            assert!(nalus.len() >= 3, "IDR frame must have SPS, PPS, IDR NALUs");

            let t_stream = Instant::now();
            let pkts = streamer
                .send_h264_frame(&nalus, Duration::from_millis(33))
                .await
                .expect("stream frame");
            let stream_time_ms = t_stream.elapsed().as_secs_f64() * 1000.0;

            assert!(pkts >= 3);
            let metrics = streamer.metrics();
            assert_eq!(metrics.frames_sent, 1);
            assert!(metrics.packets_sent >= 3);
            assert!(metrics.bytes_sent > 0);

            // Total local pipeline latency
            let total_latency_ms = cap_time_ms + enc_time_ms + stream_time_ms;
            println!(
                "Latency breakdown: capture={:.2}ms, encode={:.2}ms, packetize={:.2}ms, total={:.2}ms",
                cap_time_ms, enc_time_ms, stream_time_ms, total_latency_ms
            );
            assert!(total_latency_ms < 150.0, "Capture-to-network pipeline latency must be under 150ms");
        });
    });

    handle.join().expect("thread join cleanly");
}
