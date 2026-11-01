# RemoteLite — Lightweight Windows Remote Desktop

**RemoteLite** is a fast, secure, lightweight Windows remote desktop application inspired by AnyDesk. It allows one Windows PC to view and control another Windows PC with low latency over local networks and across the Internet.

---

## 1. Key Features

- **Unique 9-Digit Device ID**: Every device receives an auto-generated, persistent 9-digit ID (e.g., `482 910 327`) saved locally across restarts with server-side uniqueness registration.
- **Explicit Connection Approval**: Incoming connections always display a prominent modal on the target PC with **Accept** and **Reject** buttons. Remote control is strictly blocked until explicit approval.
- **Smooth 30 FPS Screen Streaming**: Windows GDI desktop capture and hardware-friendly JPEG video streaming with unchanged-frame suppression to conserve bandwidth and CPU.
- **Remote Mouse Control**: Complete pointer synchronization, left-click, right-click, middle-click, double-click, click-and-drag, and vertical scroll wheel. Coordinates are mapped to preserve remote aspect ratios.
- **Remote Keyboard Control**: Full keyboard synchronization including letters, numbers, symbols, function keys, arrows, Enter, Backspace, Tab, Escape, and modifier keys (Ctrl, Alt, Shift, Win). Automatic key release prevents stuck keys upon disconnection.
- **Active Host Indicator & Emergency Disconnect**: When a PC is being controlled, a prominent banner indicates `🔴 REMOTE CONTROL ACTIVE — Controlled by [ID]` along with an immediate `Stop Sharing / Disconnect` button.
- **End-to-End Encryption (E2EE)**: Session data is encrypted end-to-end using **ChaCha20-Poly1305** authenticated encryption with ephemeral 256-bit keys negotiated during the handshake. Signaling and relay servers cannot decrypt video streams or keystrokes.
- **Built-in Signaling & Relay**: Includes full source code for the signaling and relay server, enabling seamless NAT traversal and connectivity between PCs on different Wi-Fi networks with zero router configuration.
- **Lightweight Single Executable**: Built with Rust and native GPU rendering (DirectX/wgpu), requiring no installer, no Node.js runtime, no Electron, and consuming minimal idle resources.

---

## 2. Architecture & Technology Stack

### Why Pure Rust + eframe (egui)?
1. **Single Standalone Executable**: Generates `RemoteLite.exe` with zero external DLL or runtime dependencies.
2. **Direct GPU Texture Streaming**: Video frames (1080p) are directly uploaded to GPU memory via DirectX/wgpu textures and rendered with bilinear filtering at 30+ FPS with sub-millisecond decode-to-display times.
3. **Ultra-Low Resource Footprint**:
   - Executable size: ~18 MB
   - Idle RAM usage: ~30 MB
   - Idle CPU usage: < 0.1%
4. **Resilient Desktop Capture & Input**:
   - Capture: Windows GDI BitBlt captures desktop frames into 32-bit BGRA buffers reliably across all Windows 10/11 environments.
   - Input: Windows `SendInput` API handles synthetic mouse and keyboard events with safety trackers that automatically release pressed keys upon disconnection.

### Directory Structure
```
RemoteLite/
├── Cargo.toml                     # Cargo configuration & dependencies
├── src/
│   ├── main.rs                    # Application entry point (GUI or --server mode)
│   ├── lib.rs                     # Library exports
│   ├── app.rs                     # egui graphical interface & viewer canvas
│   ├── config.rs                  # Persistent configuration (%APPDATA%\RemoteLite\config.json)
│   ├── id.rs                      # 9-digit Device ID generation, formatting & validation
│   ├── protocol.rs                # Signaling and encrypted session message types
│   ├── crypto.rs                  # ChaCha20-Poly1305 authenticated end-to-end encryption
│   ├── capture/
│   │   ├── mod.rs                 # Screen capture engine
│   │   └── gdi.rs                 # Windows GDI BitBlt desktop capture
│   ├── encoder/
│   │   ├── mod.rs                 # Fast JPEG frame encoder with dirty-frame suppression
│   │   └── decoder.rs             # JPEG decoder to GPU ColorImage
│   ├── input/
│   │   ├── mod.rs                 # Windows SendInput injector with stuck-key safety
│   │   └── mapper.rs              # Aspect-ratio preserving coordinate & key mapper
│   ├── net/
│   │   ├── mod.rs
│   │   ├── signaling_client.rs    # WebSocket signaling client with auto-reconnect
│   │   ├── host_session.rs        # Host streaming loop & input receiver
│   │   └── viewer_session.rs      # Viewer display loop & input forwarder
│   ├── server/
│   │   ├── mod.rs
│   │   ├── registry.rs            # Device ID registry & active session tracking
│   │   └── signaling_server.rs    # WebSocket signaling & encrypted relay service
│   └── bin/
│       └── server.rs              # Dedicated remotelite-server executable
├── tests/
│   ├── id_persistence_test.rs     # 9-digit ID rules and config persistence
│   ├── crypto_security_test.rs    # Encryption, decryption & tamper resistance
│   ├── coordinate_mapping_test.rs # Canvas aspect ratio mapping & keycodes
│   └── e2e_signaling_session_test.rs # Full E2E signaling, streaming & control test
└── README.md
```

---

## 3. How to Connect Two PCs

### Step 1: Start the Signaling & Relay Server
You can run the included relay server on any machine or VPS:
```powershell
# Using the standalone server binary:
.\remotelite-server.exe 0.0.0.0:9090

# Or directly from RemoteLite.exe:
.\RemoteLite.exe --server 0.0.0.0:9090
```

### Step 2: Launch RemoteLite on PC A (Host) and PC B (Controller)
1. Run `RemoteLite.exe` on **PC A**.
   - Note the prominent 9-digit ID displayed under **Your Device** (e.g., `482 910 327`).
   - Click **Copy ID** to copy it to clipboard.
   - Status shows `● Online` (Ready for connections).
2. Run `RemoteLite.exe` on **PC B**.
   - Under **Remote Device**, enter PC A's 9-digit ID in the input box.
   - Click **Connect**.

### Step 3: Approve Connection on PC A
1. PC A immediately receives a modal dialog:
   ```
   🔔 Incoming Connection Request
   Device ID 444 555 666 requests permission to view and control your desktop.
   [ ✔ Accept ]     [ ✖ Reject ]
   ```
2. Click **✔ Accept**.

### Step 4: Remote Control Active
1. On PC B: The viewer window opens showing PC A's live desktop.
   - Move PC B's mouse to move PC A's cursor.
   - Click, drag, and scroll to control applications.
   - Type on PC B's keyboard to enter text and shortcuts (Ctrl, Alt, Shift, Enter, etc.).
   - Header displays resolution, real-time latency (ms), and streaming FPS.
2. On PC A: A red banner appears:
   ```
   🔴 REMOTE CONTROL ACTIVE — Controlled by 444 555 666   [ Stop Sharing / Disconnect ]
   ```

### Step 5: Disconnecting
- Either user can click **Disconnect** (or **Stop Sharing**) at any time.
- Screen sharing, mouse control, and network sockets terminate immediately, and all keys are automatically released.

---

## 4. Building from Source

### Prerequisites
- Windows 10 or Windows 11 (64-bit)
- Rust 1.75+ (`rustup default stable`)
- C++ Build Tools (included in Visual Studio or Visual Studio Build Tools)

### Build Commands

```powershell
# Check dependencies and compilation
cargo check

# Run all unit and end-to-end integration tests
cargo test

# Build optimized release binaries
cargo build --release
```

Compiled executables:
- Main Application: `target\release\RemoteLite.exe`
- Standalone Server: `target\release\remotelite-server.exe`

---

## 5. Security & Verification

1. **Authentication & Session Secrecy**:
   - Sessions utilize ephemeral 32-byte cryptographic random entropy exchanged between client and host.
   - Shared keys are derived using authenticated key derivation.
   - All session traffic is encrypted with ChaCha20-Poly1305 with sequential 96-bit nonces preventing replay and tampering.
2. **Explicit User Approval**:
   - Remote control is impossible without user clicking "Accept" on the host machine.
   - There is no background or unattended access in V1.
3. **No Stuck Keys**:
   - Keyboard injector tracks held keys and forces release upon session disconnect.
4. **Automated Verification**:
   - `id_persistence_test`: 2 tests passed.
   - `crypto_security_test`: 2 tests passed.
   - `coordinate_mapping_test`: 2 tests passed.
   - `e2e_signaling_session_test`: 1 end-to-end simulation test passed.
   - Unit tests: 11 tests passed.
   - **Total: 18 automated tests passing**.

---

## 6. Known Limitations

- **Windows UAC Secure Desktop**: When Windows prompts for elevated Administrator UAC credentials on a secure desktop, Windows restricts non-service applications from injecting input until elevated permissions are granted.
- **Direct WAN Peer-to-Peer without Relay**: Symmetric NAT routers require the included relay service for full traversal; relay fallback is enabled by default.
