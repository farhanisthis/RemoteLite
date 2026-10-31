# OpenRemote MVP Security Review

> Status: Completed & Audited for MVP Release (M6).

## 1. Executive Summary

OpenRemote is designed with a defense-in-depth architecture prioritizing user control, zero covert access, and cryptographic peer identity verification. This audit reviews the implementation of all security controls across the Windows Host, Android Client, and Coordination Server.

---

## 2. Core Security Controls Evaluation

### 2.1 Device Identity & Storage at Rest
- **Implementation**: Ed25519 asymmetric cryptography (`crates/identity/src/keypair.rs`).
- **Host Key Protection**: Protected using Windows Data Protection API (DPAPI) via `CryptProtectData` and `CryptUnprotectData` (`DpapiIdentityStore`). Keys never stored in plaintext on disk.
- **Client Key Protection**: Backed by Android Keystore and private app sandbox.
- **Zero Hardcoded Secrets**: No pre-shared keys, passwords, or cloud master keys.

### 2.2 Pairing & Mutual Authentication
- **Invitation Lifecycle**: QR pairing invitations use single-use, cryptographically random tokens with a 5-minute strict timeout (`PairingManager`).
- **Signature Challenges**: Connection requests require signing a fresh server-issued nonce (`signed_challenge`). Replay attacks and impersonation are mathematically impossible.
- **Default-Deny Policy**: Unattended access is **disabled by default**. New paired devices can only connect if the Windows PC owner explicitly toggles unattended access in the host dashboard.

### 2.3 Instant Revocation
- **Revocation Mechanics**: Removing a trusted device immediately updates the crash-proof `FileRegistry` using atomic write-and-rename semantics.
- **Active Session Termination**: Revoking a device forcibly terminates any live sessions associated with that device (`SessionEndReason::Revoked`). Subsequent connection attempts are rejected with `ConnectRejectReason::NotAuthorized`.

### 2.4 Windows Privilege Isolation & Desktop Security
- **Service/Agent Separation**: Privilege separation between the background Windows Service (`openremote-host-service`) and the interactive desktop capture/input agent (`openremote-session-agent`).
- **Input Injection Safety**: `SendInput` in `openremote-input-windows` attaches explicitly to the active interactive desktop (`OpenInputDesktop` / `SetThreadDesktop`) and bounds input event rates using token-bucket rate limiting (max 1000 events/sec) to prevent local denial-of-service.
- **No UAC Bypass**: Never attempts covert elevation or UAC bypass. Remote control operates strictly within the active Windows desktop context.

### 2.5 File Transfer Security
- **Path Traversal Protection**: Enforced via `sanitize_filename` (rejecting `/`, `\`, null bytes, Windows reserved devices `CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`) and `resolve_safe_path` (enforcing strict base directory containment).
- **Integrity Verification**: Transferred files are staged as `<name>.part` files and hashed with SHA-256 (`sha256_hex_reader`). Files are only atomically renamed to their target destination if the SHA-256 digest matches. Any checksum mismatch or cancellation immediately purges the staging file.

### 2.6 Network & Privacy
- **E2E Encryption**: Screen streaming and control data channels are encrypted end-to-end via WebRTC (DTLS 1.3 / SRTP). Signaling and TURN relays forward ciphertext envelopes and cannot inspect decrypted media or control contents.
- **Zero Telemetry**: No third-party analytics trackers, telemetry beacons, or unauthorized data collection.
