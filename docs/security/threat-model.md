# Threat Model

> Status: M0. Updated as implementation proceeds.

## System Overview

OpenRemote allows an Android device to remotely view and control a Windows PC. The system involves:

- Windows host PC (target of remote access).
- Android client phone (source of remote control).
- Coordination server (signaling relay — cannot read session content).
- TURN relay (traffic relay when P2P fails — cannot read session content).
- Public internet and various NAT configurations.

## Assets to Protect

| Asset | Description | Protection |
|-------|-------------|------------|
| Host private key | Ed25519 signing key, permanent device identity | Windows DPAPI / secure storage |
| Client private key | Ed25519 signing key, permanent device identity | Android Keystore |
| Pairing invitation token | Short-lived, single-use QR token | Server-side expiry + single-use enforcement |
| Session media | Screen content, input events | DTLS-SRTP (WebRTC) end-to-end |
| File transfer content | Files being transferred | DTLS-SRTP (WebRTC DataChannel) |
| Trusted device registry | List of authorized devices | Protected by host service access controls |
| Session metadata | Timing, connection info | Minimize collection |

## Threat Actors

### Unauthorized Remote Attacker
A remote attacker who does not have a trusted device credential.

**Mitigations**:
- All sessions require authenticated device identity.
- Unattended access is disabled by default.
- Authorization is device-specific and cryptographically bound.
- The coordination server verifies device signatures before forwarding connect requests.

### Malicious Device (Untrusted Phone)
An Android device not in the trusted device registry.

**Mitigations**:
- Host service checks device ID against trusted registry before accepting any session.
- Cryptographic signature over a fresh server-issued challenge prevents replay.
- No connection is established until authorization passes.

### Compromised Coordination Server
The coordination server is controlled by an attacker.

**Mitigations**:
- Session content is DTLS-SRTP encrypted end-to-end; the server cannot decrypt it.
- Device identities are verified against per-device public keys stored on the host.
- A compromised server can relay signaling messages but cannot grant unauthorized access.
- Self-hosting eliminates dependency on the managed server.

**Residual risk**: A compromised server could perform denial of service, relay manipulation, or metadata exposure. Documented.

### Compromised TURN Relay
The relay server is controlled by an attacker.

**Mitigations**:
- DTLS-SRTP encrypts all media and DataChannel content; the relay sees only ciphertext.
- Self-hosting eliminates dependency on managed relay.

### Local Attacker (Physical Access to Windows PC)
An attacker with physical or local session access to the Windows host.

**Mitigations**:
- Host service IPC uses OS-level access controls.
- Session agent does not expose command execution.
- Private keys stored in DPAPI-protected storage.
- An attacker with full local access can already control the PC.

### Replay Attack
Reusing a captured pairing token or authentication challenge.

**Mitigations**:
- Pairing tokens are single-use, enforced server-side.
- Pairing tokens expire after 5 minutes.
- Connection challenges are server-issued per-session nonces.

### Lost/Stolen Phone
The authorized Android phone is lost or stolen.

**Mitigations**:
- Device revocation is available from the Windows dashboard.
- Revocation terminates any active session immediately.
- Biometric app unlock (M7) adds a second layer.

## Security Decisions NOT Yet Implemented (M0)

- Platform secure key storage (DPAPI / Android Keystore).
- Windows Service isolation.
- Authenticated IPC between host service and session agent.
- Full WebRTC DTLS peer verification binding (beyond standard DTLS).
- Lock screen behavior research and implementation.
- Rate limiting on coordination server.
- Connection audit logging.

These are tracked in [docs/milestones/mvp-plan.md](../milestones/mvp-plan.md).
