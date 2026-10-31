# Session Protocol

> Status: M0 design. Implementation in M1–M4.

## Session Lifecycle

```
Connecting → Authenticating → Active → Ended
                                    ↘ Suspended → Active (reconnect)
                                    ↘ Failed
```

## WebRTC Tracks and Channels

| Channel | Type | Purpose |
|---------|------|---------|
| Video track | WebRTC video | Desktop screen stream (H.264) |
| Control channel | DataChannel (ordered, reliable) | Input events, session commands |
| Transfer channel | DataChannel (ordered, reliable) | File transfer metadata |
| Transfer data | DataChannel (ordered, reliable) | File transfer payload |

## Input Event Format

Input events are serialized as JSON `InputEvent` structs (see `crates/input-windows/src/events.rs`) and sent over the control DataChannel.

## Session Security

- The WebRTC DTLS-SRTP handshake provides encryption.
- The host verifies the DTLS fingerprint against the client's registered identity to prevent MITM.
- Input events are only accepted during an active, authorized session.
- The session agent processes input only from the authenticated host service connection.

## Reconnection

When the network is briefly interrupted:
1. WebRTC detects ICE failure.
2. Client attempts ICE restart.
3. If ICE restart succeeds, the session resumes.
4. If ICE restart fails, the client reconnects to the coordination server and starts a new signaling exchange.
5. The host service matches the reconnect to the existing session for continuity.
