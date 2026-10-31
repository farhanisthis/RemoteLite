# Pairing Protocol

> Status: M0 design. Implementation begins in M4.

## Overview

Pairing establishes a cryptographically verified trust relationship between an Android device and a Windows host. After successful pairing and explicit unattended-access enablement, the Android device can connect without requiring manual approval on every connection.

## Security Properties

- Private keys are never transmitted.
- The pairing token is short-lived (5 minutes) and single-use.
- The host owner must explicitly confirm the pairing.
- Unattended access requires a separate opt-in step.
- Trust is bound to a specific device identity (public key).

## Protocol Steps

### 1. Host Creates Invitation

```
Host → Coordination Server:
  POST /api/v1/pairing/invite
  { host_device_id, host_public_key, host_display_name }

Server → Host:
  { invitation_token, expires_at }
```

The host encodes the token and metadata into a QR code displayed on screen.

### 2. QR Payload

The QR code contains:
```json
{
  "server_url": "wss://coordination.openremote.dev",
  "invitation_token": "<32-byte random token, base64>",
  "host_public_key": "<Ed25519 public key, base64>",
  "host_device_id": "<UUID>",
  "host_display_name": "My Desktop PC",
  "expires_at": "2026-10-31T12:05:00Z"
}
```

**Security note**: No private key material is included. The token does not grant access by itself.

### 3. Client Scans QR

The Android app scans the QR, connects to the coordination server, and completes the pairing:

```
Client → Coordination Server:
  POST /api/v1/pairing/complete
  {
    invitation_token,
    client_device_id,
    client_public_key,
    client_display_name
  }
```

The server:
1. Looks up the invitation by token.
2. Verifies the token has not been used and has not expired.
3. Marks the token as used (single-use enforcement).
4. Returns the host's public key to the client.
5. Notifies the host via signaling WebSocket.

### 4. Host Confirms

The host receives the pairing notification showing:
- Client display name.
- Client platform.
- Client device ID (abbreviated).

The host owner must click "Approve" to complete pairing.

### 5. Trust Stored

After approval, the host stores:
```
TrustedDevice {
  device_id: <client UUID>,
  display_name: <client name>,
  public_key: <client Ed25519 public key>,
  unattended_access_enabled: false,  ← disabled by default
  permissions: DevicePermissions::default(),
}
```

### 6. Unattended Access Opt-In

Unattended access is **disabled by default**. The host owner must separately enable it for each trusted device.

## Subsequent Connection Authentication

On every connection attempt, the host verifies:

1. The client's device ID is in the trusted registry.
2. Unattended access is enabled for this device.
3. The client signs a fresh server-issued challenge with its private key.
4. The signature verifies against the stored public key.

All four checks must pass. Failure at any step results in rejection.
