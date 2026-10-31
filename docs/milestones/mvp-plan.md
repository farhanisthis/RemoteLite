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

- [ ] User pairs Android once
- [ ] Unattended access explicitly enabled
- [ ] Android reconnects without PC approval
- [ ] Revocation immediately blocks access
- [ ] Session indicators visible

## M5 Acceptance Criteria

- [ ] 1 GB file transfer in each direction
- [ ] SHA-256 integrity verified
- [ ] Clean cancellation
- [ ] Path traversal rejected
- [ ] Disconnection handled safely

## M6 Acceptance Criteria

- [ ] New user can complete full onboarding to remote control in under 10 minutes
- [ ] Windows installer works on clean Windows 10 and 11
- [ ] Android APK installs and runs on API 26+
- [ ] Security review completed
- [ ] Performance benchmarks documented
