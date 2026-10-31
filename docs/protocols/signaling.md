# Signaling Protocol

> Status: M0 design. Full implementation in M3.

## Overview

The signaling protocol runs over a WebSocket connection to the coordination server. All messages are JSON-encoded `Envelope` structures (see `crates/protocol/src/messages.rs`).

## Message Flow

See [data-flows.md](../architecture/data-flows.md) for sequence diagrams.

## Message Types

| Message | Direction | Description |
|---------|-----------|-------------|
| `register` | Client → Server | Device announces presence |
| `registered` | Server → Client | Server acknowledges |
| `host_announce` | Host → Server | Host signals availability |
| `connect_request` | Client → Server → Host | Client requests session |
| `connect_accept` | Host → Server → Client | Host accepts |
| `connect_reject` | Host → Server → Client | Host rejects |
| `ice_candidate` | Either → Server → Either | ICE candidate exchange |
| `sdp_offer` | Client → Server → Host | WebRTC SDP offer |
| `sdp_answer` | Host → Server → Client | WebRTC SDP answer |
| `session_end` | Either → Server → Either | Session terminated |
| `pairing_invitation` | Server → Client | Pairing notification |
| `pairing_complete` | Client → Server | Pairing completion |
| `ping` / `pong` | Either | Keepalive |
| `error` | Server → Client | Error response |

## Authentication

Every `connect_request` message must include:
- `auth_signature`: Ed25519 signature over a server-issued challenge.
- `signed_challenge`: The challenge value that was signed.

The server verifies the signature against the device's registered public key before forwarding the request to the host.

## Envelope Format

```json
{
  "version": { "major": 0, "minor": 1, "patch": 0 },
  "message_id": "uuid-v4",
  "timestamp": "2026-10-31T12:00:00Z",
  "payload": {
    "type": "register",
    "device_id": "...",
    "platform": "android",
    "public_key": "base64...",
    "display_name": "My Phone"
  }
}
```
