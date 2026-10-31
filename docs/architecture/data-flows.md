# Data Flows

> Status: M0. Describes the target data flows implemented in M1–M5.

## Session Establishment Flow

```
Android           Coordination Server       Windows Host
   │                      │                      │
   │──── WS connect ─────►│                      │
   │──── Register ────────►│                      │
   │                      │◄── WS connect ────────│
   │                      │◄── Register ──────────│
   │                      │◄── HostAnnounce ──────│
   │                      │                      │
   │──── ConnectRequest ──►│                      │
   │                      │──── ConnectRequest ──►│
   │                      │◄─── ConnectAccept ────│
   │◄── ConnectAccept ────│                      │
   │                      │                      │
   │──── SDP Offer ───────►│──── SDP Offer ───────►│
   │◄─── SDP Answer ───────│◄─── SDP Answer ───────│
   │                      │                      │
   │── ICE Candidates ────►│── ICE Candidates ───►│
   │◄─ ICE Candidates ─────│◄─ ICE Candidates ────│
   │                      │                      │
   │◄──────────────── Direct P2P or TURN Relay ───│
   │                      │                      │
   │    (DTLS-SRTP encrypted, server cannot read) │
```

## Pairing Flow

```
Android (QR Scanner)      Coordination Server      Windows Host
         │                        │                      │
         │                        │◄── CreateInvite ─────│
         │                        │──── token ──────────►│
         │                        │          QR generated │
         │                        │           displayed   │
         │                        │                      │
  [User scans QR]                 │                      │
         │                        │                      │
         │──── CompletePairing ──►│                      │
         │◄─── host public key ───│                      │
         │                        │                      │
         │──── WS: PairingComplete ►│─── forwarded ──────►│
         │                        │                      │
         │                        │         [Host shows  │
         │                        │          device info] │
         │                        │         [Owner confirms]
         │                        │◄── TrustedDevice saved│
```

## File Transfer Flow (Target — M5)

```
Android                              Windows Host
   │                                      │
   │── TransferAnnounce ─────────────────►│
   │◄─ TransferAccept ────────────────────│
   │                                      │
   │── TransferChunk(0) ─────────────────►│
   │── TransferChunk(1) ─────────────────►│
   │── ... ──────────────────────────────►│
   │── TransferComplete ─────────────────►│
   │◄─ (host verifies SHA-256) ───────────│
   │                                      │
   (via WebRTC DataChannel)
```
