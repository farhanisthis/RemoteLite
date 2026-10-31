//! # OpenRemote Session Agent
//!
//! Runs in the interactive Windows desktop session.
//! Responsible for live screen capture, low-latency streaming (MJPEG/WebSocket),
//! remote mouse/keyboard input injection via Win32 SendInput, and emergency disconnect.

#[allow(dead_code)]
mod input_receiver;
#[allow(dead_code)]
mod pipeline;

use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use openremote_capture_windows::ScreenCapturer;
use openremote_input_windows::{inject_input, InputEvent};
use openremote_observability::init_tracing;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

struct ServerState {
    frame_rx: watch::Receiver<Option<Vec<u8>>>,
}

fn convert_bgra_to_jpeg(width: u32, height: u32, stride: u32, bgra: &[u8], quality: u8) -> Result<Vec<u8>> {
    let pixel_count = (width * height) as usize;
    let mut rgb = Vec::with_capacity(pixel_count * 3);

    let row_bytes = (width * 4) as usize;
    let stride_bytes = stride as usize;

    for y in 0..height as usize {
        let row_start = y * stride_bytes;
        let row_end = row_start + row_bytes;
        if row_end <= bgra.len() {
            let row = &bgra[row_start..row_end];
            for chunk in row.as_chunks::<4>().0 {
                // BGRA -> RGB
                rgb.push(chunk[2]); // R
                rgb.push(chunk[1]); // G
                rgb.push(chunk[0]); // B
            }
        }
    }

    let mut jpeg_bytes = Vec::with_capacity(128 * 1024);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, quality);
    encoder.encode(&rgb, width, height, image::ExtendedColorType::Rgb8)?;
    Ok(jpeg_bytes)
}

async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "openremote-session-agent",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn get_frame(State(state): State<Arc<ServerState>>) -> Response {
    if let Some(jpeg) = state.frame_rx.borrow().clone() {
        (
            [
                (header::CONTENT_TYPE, "image/jpeg"),
                (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
            ],
            jpeg,
        )
            .into_response()
    } else {
        (StatusCode::NO_CONTENT, "No frame available yet").into_response()
    }
}

async fn handle_post_input(Json(event): Json<InputEvent>) -> Response {
    match inject_input(&event) {
        Ok(()) => (StatusCode::OK, Json(json!({"status": "injected"}))).into_response(),
        Err(e) => {
            warn!(error = %e, "Failed to inject input event");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error", "message": e.to_string()})),
            )
                .into_response()
        }
    }
}

async fn ws_control_handler(
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket: WebSocket| async move {
        info!("WebSocket control channel connected");
        while let Some(msg) = socket.recv().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(event) = serde_json::from_str::<InputEvent>(&text) {
                        if let Err(e) = inject_input(&event) {
                            warn!(error = %e, "Failed to inject WS input");
                        }
                    }
                }
                Ok(Message::Binary(bin)) => {
                    if let Ok(event) = serde_json::from_slice::<InputEvent>(&bin) {
                        if let Err(e) = inject_input(&event) {
                            warn!(error = %e, "Failed to inject WS binary input");
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket control channel closed");
                    break;
                }
                _ => {}
            }
        }
    })
}

async fn ws_stream_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket: WebSocket| async move {
        info!("WebSocket video stream client connected");
        let mut rx = state.frame_rx.clone();

        while rx.changed().await.is_ok() {
            let maybe_jpeg = {
                rx.borrow().clone()
            };
            if let Some(jpeg) = maybe_jpeg {
                if socket.send(Message::Binary(jpeg)).await.is_err() {
                    break;
                }
            }
        }
        info!("WebSocket video stream client disconnected");
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    info!(
        version = env!("CARGO_PKG_VERSION"),
        "OpenRemote Session Agent starting on interactive Windows desktop"
    );

    let (frame_tx, frame_rx) = watch::channel::<Option<Vec<u8>>>(None);

    // Spawn screen capture loop
    std::thread::spawn(move || {
        info!("Starting screen capture worker thread...");
        let capturer = match ScreenCapturer::auto() {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "Failed to initialize screen capturer");
                return;
            }
        };

        let (w, h) = capturer.dimensions();
        info!(width = w, height = h, "Screen capturer initialized");

        loop {
            match capturer.capture_frame() {
                Ok(frame) => {
                    match convert_bgra_to_jpeg(frame.width, frame.height, frame.stride, &frame.data, 70) {
                        Ok(jpeg) => {
                            let _ = frame_tx.send(Some(jpeg));
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to encode frame to JPEG");
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Capture frame error");
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
            // Target ~30 FPS (33ms per frame)
            std::thread::sleep(Duration::from_millis(33));
        }
    });

    let state = Arc::new(ServerState { frame_rx });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/frame", get(get_frame))
        .route("/ws/stream", get(ws_stream_handler))
        .route("/api/input", post(handle_post_input))
        .route("/ws/control", get(ws_control_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let bind_addr = std::env::var("SESSION_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!(bind = %bind_addr, "OpenRemote Session Agent listening for screen stream and input");

    axum::serve(listener, app).await?;
    Ok(())
}
