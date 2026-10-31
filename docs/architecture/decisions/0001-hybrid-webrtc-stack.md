# ADR 0001: WebRTC Integration Strategy for Real-Time Streaming

## Context

OpenRemote requires low-latency screen streaming (1080p, 30fps, <150ms LAN target), bidirectional control, and file transfer between a Windows host and an Android mobile client.

WebRTC provides standard media transport (SRTP), congestion control (TWCC/GCC), NAT traversal (ICE/STUN/TURN), and multiplexed DataChannels (SCTP).

However, selecting a WebRTC implementation across Windows (Rust) and Android (Kotlin) carries architectural risks:
1. `webrtc-rs`: Pure Rust implementation, active ecosystem, but hardware video RTP pipeline integration requires evaluation.
2. `str0m`: Modern synchronous Rust WebRTC implementation with zero external I/O assumptions, suitable for custom video pipelines.
3. Native Google WebRTC (`libwebrtc` C++ / prebuilt JNI on Android): Industry standard, mature MediaCodec HW decoding on Android, but complex C++ build matrix on Windows.

## Decision

1. **Android**: Use standard Android WebRTC library with hardware-assisted `MediaCodec` H.264 decoding and Compose rendering via `SurfaceViewRenderer`.
2. **Windows**: Establish an abstract transport layer (`crates/transport`) and session interface (`crates/session`). Prototype both `webrtc-rs` and `str0m` during the M1 investigation phase with real Windows Graphics Capture frames before locking the final engine.
3. **Signaling**: Independent WebSocket-based JSON signaling server (`services/coordination`), completely decoupled from the media transport.

## Consequences

- Prevents vendor lock-in to a single WebRTC library.
- Android uses mature prebuilt WebRTC binaries with full hardware decoder support.
- Windows host remains pure Rust without painful CMake/Depot_Tools dependencies if pure-Rust transport succeeds.
