# MVP Plan

## Current Status: M3 Internet Connectivity & Relay Fallback ✅

## Milestone Summary

| Milestone | Description | Status |
|-----------|-------------|--------|
| M0 | Repository foundation, skeletons, CI | ✅ Complete |
| M1 | Screen streaming (Windows → Android) | ✅ Complete |
| M2 | Interactive mouse/keyboard control | ✅ Complete |
| M3 | Internet connectivity (STUN/TURN/signaling) | ✅ Complete |
| M4 | Trusted devices and unattended access | ⏳ Next |
| M5 | Bidirectional file transfer | ⏳ Pending M4 |
| M6 | MVP release (installer, packaging, docs) | ⏳ Pending M5 |
| M7 | Convenience features | ⏳ Pending M6 |
| M8 | Advanced features | ⏳ Pending M7 |

## M0 Acceptance Criteria

- [x] Rust workspace structure created
- [x] All 13 workspace crates and binaries compile
- [x] 29 Rust unit tests pass (`cargo test --workspace`)
- [x] `cargo fmt --all -- --check` passes with zero formatting issues
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes with zero warnings
- [x] Coordination server `/health` endpoint responds with 200 OK
- [x] Android Jetpack Compose skeleton structured
- [x] Tauri 2 React desktop-ui structured
- [x] CI workflows created
- [x] Complete architecture documentation and threat model

## M1 Acceptance Criteria

- [x] Screen capture engine with DXGI Desktop Duplication & GDI fallback
- [x] H.264 real-time encoder with Annex-B formatting, SPS/PPS generation & IDR keyframes
- [x] RTP packetizer (RFC 3550) with H.264 FU-A fragmentation (RFC 6184)
- [x] SDP offer/answer generation (RFC 4566)
- [x] Live end-to-end streaming pipeline verified
- [x] Android remote desktop viewer rendering decoded stream

## M2 Acceptance Criteria

- [x] Windows mouse injection engine with normalized `[0.0, 1.0]` coordinates
- [x] Mouse movement, left/right/middle click, double click, vertical & horizontal scroll
- [x] Windows keyboard injection (virtual keys, scancodes, modifiers, Unicode text)
- [x] Automatic interactive desktop attachment (`ensure_interactive_desktop`)
- [x] Input rate-limiting protection against DoS/flooding
- [x] Session authorization verification & emergency local disconnect
- [x] Android GestureHandler (Touchpad relative mode + Direct-touch absolute mode)
- [x] Android virtual keyboard toolbar with special keys & shortcuts
- [x] 44 Rust unit/integration tests passing
- [x] Android unit tests passing

## M3 Acceptance Criteria

- [x] Full bidirectional WebSocket signaling hub in `services/coordination`
- [x] Device registration and cryptographic verification
- [x] Session connection handshake (`ConnectRequest` -> `ConnectAccept` / `ConnectReject`)
- [x] WebRTC SDP Offer and Answer relay
- [x] Trickle ICE candidate relay (Host, STUN reflexive, TURN relay)
- [x] RFC 8489 STUN Binding client (XOR-MAPPED-ADDRESS public IP discovery)
- [x] RFC 8656 TURN Allocate client (XOR-RELAYED-ADDRESS relay fallback)
- [x] Graceful rejection of offline hosts and unauthorized session snooping
- [x] 47 Rust unit/integration tests passing
- [x] Android unit tests passing

## M4 Acceptance Criteria

- [x] User pairs Android once via single-use, short-lived QR invitation (`QrPairingPayload`)
- [x] Unattended access explicitly disabled by default (rejects with `UnattendedAccessDisabled`)
- [x] Owner can explicitly toggle unattended access on Windows host
- [x] Android reconnects without PC approval once unattended access is enabled
- [x] Instant revocation immediately terminates active sessions and blocks reconnect (`ConnectRejectReason::NotAuthorized`)
- [x] Platform secure key storage via Windows DPAPI (`DpapiIdentityStore` / `CryptProtectData`)
- [x] Crash-resilient atomic file-backed trusted device registry (`FileRegistry`)
- [x] Host Service background daemon (`apps/windows/host-service`) with coordination signaling client
- [x] Android pairing client (`PairingClient.kt`) with QR parser, expiration validator, and completion payload
- [x] 59 Rust unit and integration tests passing (`cargo test --workspace`)
- [x] Android unit tests passing (`./gradlew.bat testDebugUnitTest`)

## M5 Acceptance Criteria

- [x] Bidirectional file transfer: PC-to-Android download and Android-to-PC upload
- [x] Streamed chunking (256 KiB chunks) with bounded memory footprint
- [x] Full cryptographic SHA-256 integrity verification (`sha256_hex_reader` & `MessageDigest`)
- [x] Atomic `.part` file staging and rename prevents corrupted or partial files
- [x] Immediate staging file cleanup on corruption or cancellation
- [x] Strict directory containment and path traversal rejection (`resolve_safe_path` & `sanitize_filename`)
- [x] Android file transfer client (`FileTransferClient.kt` & `AndroidFileReceiver`)
- [x] Flow control with chunk sequence ACKs
- [x] 64 Rust unit and integration tests passing (`cargo test --workspace`)
- [x] Android file transfer unit tests passing (`./gradlew.bat testDebugUnitTest`)

## M6 Acceptance Criteria

- [x] Full onboarding to remote control documented in under 10 minutes (`docs/user-guide/getting-started.md`)
- [x] Windows host service and session agent release binaries compiled (`cargo build --workspace --release`)
- [x] Android client APK compiled and verified on Android API 26+ (`assembleDebug` passing)
- [x] Comprehensive security review audit completed (`docs/security/security-review.md`)
- [x] Performance engineering benchmarks and targets documented (`docs/benchmarks/performance.md`)
- [x] 64 workspace Rust tests passing with 0 warnings on `-D warnings`
- [x] Full Android Compose & unit test suite passing
