# Architecture Overview

> **Status**: M0 Foundation. This document reflects the target architecture.
> Implementation status is noted per component.

## System Components

```
┌─────────────────────────────────────────────────────────────────┐
│                    OpenRemote System                             │
│                                                                  │
│  ┌──────────────────┐         ┌──────────────────────────────┐  │
│  │  Android Client  │         │     Windows Host             │  │
│  │                  │         │                              │  │
│  │  Jetpack Compose │         │  ┌────────────────────────┐  │  │
│  │  WebRTC client   │◄──────►│  │  Host Service          │  │  │
│  │  MediaCodec      │WebRTC  │  │  (Rust, Windows Svc)   │  │  │
│  │  Android Keystore│        │  └────────────┬───────────┘  │  │
│  │                  │         │               │ local IPC    │  │
│  └──────────────────┘         │  ┌────────────▼───────────┐  │  │
│           │                   │  │  Session Agent         │  │  │
│           │ WebSocket         │  │  (capture + input)     │  │  │
│           │ signaling         │  └────────────────────────┘  │  │
│           │                   │                              │  │
│  ┌────────▼──────────┐        │  ┌────────────────────────┐  │  │
│  │ Coordination Svc  │        │  │  Desktop UI (Tauri 2)  │  │  │
│  │ (Rust + Axum)     │◄──────►│  │  (React + TypeScript)  │  │  │
│  │ PostgreSQL        │        │  └────────────────────────┘  │  │
│  └───────────────────┘        └──────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────┐                                          │
│  │  TURN Relay       │                                          │
│  │  (coturn)         │                                          │
│  └───────────────────┘                                          │
└─────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

### Windows Host Service (`apps/windows/host-service`)
**Status**: M0 skeleton

- Persistent device identity and key management.
- Trusted device registry.
- Authorization and permission evaluation.
- Coordination server connectivity and signaling.
- Session lifecycle management.
- Authenticated IPC with the session agent.
- Audit logging.

### Windows Session Agent (`apps/windows/session-agent`)
**Status**: M0 skeleton

- Screen capture via Windows Graphics Capture API or Desktop Duplication API.
- H.264 encoding via Windows Media Foundation.
- Mouse and keyboard input injection via SendInput.
- Visible remote-session indicator on the desktop.
- Local emergency disconnect mechanism.

### Windows Desktop UI (`apps/windows/desktop-ui`)
**Status**: M0 skeleton

- Tauri 2 shell with React/TypeScript frontend.
- Device pairing interface with QR code display.
- Trusted device management.
- Unattended access controls.
- Session activity display.
- File transfer management.

### Android App (`apps/android`)
**Status**: M0 skeleton

- Jetpack Compose UI.
- Device pairing via QR scanner.
- Saved computer management.
- WebRTC session management.
- Touch gesture remote control.
- File transfer interface.
- Biometric unlock (future).

### Coordination Server (`services/coordination`)
**Status**: M0 with health endpoint and WebSocket skeleton

- Device registration and presence.
- Pairing invitation management.
- WebRTC signaling (ICE/SDP relay).
- Session authorization verification.
- Does NOT decrypt any session content.

### Shared Crates

| Crate | Purpose | Status |
|-------|---------|--------|
| `protocol` | Wire format messages, types | M0 ✅ |
| `identity` | Ed25519 keypair management | M0 ✅ |
| `authorization` | Permission evaluation | M0 ✅ |
| `session` | Session lifecycle | M0 ✅ |
| `transport` | Transport abstraction | M0 skeleton |
| `capture-windows` | Screen capture | M0 stub → M1 |
| `encoder-windows` | H.264 encoding | M0 stub → M1 |
| `input-windows` | Input injection | M0 stub → M2 |
| `file-transfer` | File transfer | M0 skeleton → M5 |
| `observability` | Logging/tracing | M0 ✅ |

## Security Boundaries

```
[Android] ←── encrypted WebRTC ──→ [Windows Host Service]
                                            │
                              authenticated local IPC only
                                            │
                                   [Session Agent]
```

The coordination server cannot read session content.
The relay cannot read session content.
The session agent receives commands only via authenticated IPC from the host service.

## Data Flow (Target — M1+)

```
Windows Desktop
     │
     ▼ Windows Graphics Capture (GPU texture)
Frame Buffer (BGRA)
     │
     ▼ H.264 Encoder (Windows Media Foundation / hardware)
Encoded H.264 NAL units
     │
     ▼ WebRTC video track
     │
  (DTLS-SRTP encrypted)
     │
     ▼ Android WebRTC
Decoded frame (MediaCodec hardware)
     │
     ▼ SurfaceView / TextureView
Android Display
```

See [data-flows.md](data-flows.md) for detailed data flow diagrams.
