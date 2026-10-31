# MVP Plan

## Current Status: M0 Foundation ✅

## Milestone Summary

| Milestone | Description | Status |
|-----------|-------------|--------|
| M0 | Repository foundation, skeletons, CI | ✅ Complete |
| M1 | Screen streaming (Windows → Android) | ⏳ Pending approval |
| M2 | Interactive mouse/keyboard control | ⏳ Pending M1 |
| M3 | Internet connectivity (STUN/TURN/signaling) | ⏳ Pending M2 |
| M4 | Trusted devices and unattended access | ⏳ Pending M3 |
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

- [ ] Windows screen appears live on Android over LAN
- [ ] 1080p/30 FPS target measured on real hardware
- [ ] 30-minute stability test completed
- [ ] Capture-to-display latency measured
- [ ] Unsupported configurations documented

## M2 Acceptance Criteria

- [ ] User completes predefined Windows tasks from Android
- [ ] Input coordinates correct after scaling
- [ ] Reliable disconnect
- [ ] No unauthorized input accepted

## M3 Acceptance Criteria

- [ ] Android on mobile data controls Windows on home broadband
- [ ] TURN relay fallback verified
- [ ] Connection errors handled gracefully
- [ ] Unauthorized peers rejected

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
