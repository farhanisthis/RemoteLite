# Technical Risks

> Status: M0 initial assessment. Updated as implementation proceeds.

## High Priority Risks

### R1: Rust WebRTC Library Choice
**Risk**: No Rust WebRTC library has verified Windows-to-Android interoperability for our use case.
**Impact**: Could block M1/M3 entirely or require significant rework.
**Mitigation**: Prototype WebRTC integration before M1 commit. Candidates: `webrtc-rs`, `str0m`, or wrapping libwebrtc via FFI.
**Status**: Investigation required before M1.

### R2: Windows Graphics Capture API Compatibility
**Risk**: WGC requires Windows 10 1903+ and may have restrictions in certain session types (lock screen, UAC prompts).
**Impact**: Affects streaming reliability in edge cases.
**Mitigation**: Research WGC limitations. Implement Desktop Duplication API fallback. Document limitations.
**Status**: Investigation required before M1.

### R3: H.264 Hardware Encoder Availability
**Risk**: Not all Windows systems have hardware H.264 encoders (NVENC, Quick Sync, VCE).
**Impact**: CPU encoding fallback may not meet latency targets on lower-end hardware.
**Mitigation**: Implement software encoder fallback. Document minimum hardware requirements.
**Status**: Investigation required before M1.

### R4: NAT Traversal Success Rate
**Risk**: ICE may fail in symmetric NAT configurations, requiring TURN.
**Impact**: Increased relay bandwidth cost and latency.
**Mitigation**: Implement TURN fallback. Document expected relay usage.
**Status**: By design — addressed in M3.

### R5: Android MediaCodec H.264 Compatibility
**Risk**: Hardware H.264 decoder availability varies across Android devices.
**Impact**: Decoding performance on older or low-end Android devices.
**Mitigation**: Software decoder fallback. Test on multiple device classes.
**Status**: Investigation required before M1.

## Medium Priority Risks

### R6: Windows Service Permission Model
**Risk**: Running capture from a Windows Service (Session 0) vs. interactive session has OS restrictions.
**Impact**: Requires session agent architecture.
**Mitigation**: Separate host service and session agent (already designed). Use CreateProcessAsUser or similar.
**Status**: Architecture accounts for this — implementation in M4.

### R7: Lock Screen Capture
**Risk**: Capturing content from the Windows lock screen may require additional privileges or may not be possible via WGC.
**Impact**: Cannot remotely unlock Windows via this mechanism.
**Mitigation**: Document this limitation clearly. Research WinLogon session capture separately.
**Status**: Deferred until M4. Will not be implemented without verified mechanism.

### R8: DPAPI Key Protection on Headless Systems
**Risk**: Windows DPAPI key protection is tied to the user session, which may not be available on headless server setups.
**Impact**: Affects key persistence and security model on non-standard configurations.
**Mitigation**: Document the requirement for a logged-in Windows user session. Research service account DPAPI.
**Status**: Investigation required before M4.

## Low Priority Risks

### R9: Android Battery Impact
**Risk**: Continuous video decoding may significantly drain Android battery.
**Impact**: User experience during long sessions.
**Mitigation**: Adaptive frame rate. Hardware decoder usage. Quality controls.
**Status**: Measurement required in M1.

### R10: Dependency License Compatibility
**Risk**: Some crates may have licenses incompatible with Apache-2.0 distribution.
**Impact**: Licensing compliance issues.
**Mitigation**: Review all transitive dependencies before M6 release.
**Status**: Ongoing — initial review pending.
