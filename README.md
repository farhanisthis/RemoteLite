# OpenRemote

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Build Status](https://github.com/openremote/openremote/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/openremote/openremote/actions)

**OpenRemote** is a free, open-source alternative to AnyDesk and TeamViewer for personal remote access.

Connect from your Android phone to your Windows PC — view the desktop, control the mouse and keyboard, and transfer files, without paying for a subscription.

> **Status:** M0 Foundation — not yet functional for remote control. See [milestone progress](docs/milestones/mvp-plan.md).

## Features (Target)

- 🖥️ Low-latency Windows desktop streaming to Android
- 🖱️ Full mouse and keyboard control
- 📁 Bidirectional file transfer
- 🔐 Secure device pairing with cryptographic identity
- 🔓 Trusted unattended access (connect without approval every time)
- 🌐 Peer-to-peer connections; TURN relay fallback
- 🏠 Self-hostable infrastructure
- 💸 Free forever for personal use

## Supported Platforms

| Platform | Role | Status |
|----------|------|--------|
| Windows 11 x64 | Host | In development (M0) |
| Windows 10 x64 | Host | Compatibility target |
| Android | Remote controller | In development (M0) |

## Quick Start (Development)

### Prerequisites

- Rust 1.82+ (install via [rustup](https://rustup.rs))
- Node.js 18+
- Android Studio with SDK API 34+
- Windows 10/11 x64 (for Windows components)

### Build

```bash
# Clone
git clone https://github.com/openremote/openremote.git
cd openremote

# Rust workspace
cargo build --workspace

# Coordination server
cargo run --bin openremote-coordination

# Host service (Windows only)
cargo run --bin openremote-host-service

# Desktop UI (requires Node.js + Tauri CLI)
cd apps/windows/desktop-ui
npm install
npm run tauri dev
```

### Android

```bash
cd apps/android
./gradlew assembleDebug
```

## Architecture

See [docs/architecture/overview.md](docs/architecture/overview.md) for the full architecture.

```
Android App ←──WebRTC──→ Windows Host Service
                               │
                               │ local IPC
                               ▼
                      Windows Session Agent
                      (capture + input)
```

## Security

Security is a core design principle. See:

- [Threat Model](docs/security/threat-model.md)
- [Pairing Protocol](docs/security/pairing.md)
- [Security Policy](SECURITY.md)

## Self-Hosting

Run your own coordination server and TURN relay:

```bash
cd infrastructure
cp env.example .env
docker compose up -d
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache-2.0. See [LICENSE](LICENSE).

Third-party dependency licenses are reviewed in [docs/deployment/licenses.md](docs/deployment/licenses.md).
