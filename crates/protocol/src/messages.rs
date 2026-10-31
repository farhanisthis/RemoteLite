//! Wire-format messages exchanged between OpenRemote components.
//!
//! All messages are serialized to JSON for signaling and bincode for
//! high-throughput data channels.

use crate::types::{DeviceId, Platform, ProtocolVersion, SessionId, TransferId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Top-level envelope wrapping every signaling message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Protocol version of the sender.
    pub version: ProtocolVersion,
    /// Unique message ID for deduplication and tracing.
    pub message_id: uuid::Uuid,
    /// Timestamp at which the sender created this message.
    pub timestamp: DateTime<Utc>,
    /// The payload.
    pub payload: SignalingMessage,
}

impl Envelope {
    /// Create a new envelope with the current time and a fresh message ID.
    pub fn new(payload: SignalingMessage) -> Self {
        Self {
            version: ProtocolVersion::CURRENT,
            message_id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            payload,
        }
    }
}

/// Signaling messages sent through the coordination server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalingMessage {
    /// Client registers itself with the coordination server.
    Register(RegisterMessage),
    /// Server acknowledges registration.
    Registered(RegisteredMessage),
    /// Host announces presence so clients can discover it.
    HostAnnounce(HostAnnounceMessage),
    /// Client requests to connect to a host.
    ConnectRequest(ConnectRequestMessage),
    /// Host accepts the connection request.
    ConnectAccept(ConnectAcceptMessage),
    /// Host rejects the connection request.
    ConnectReject(ConnectRejectMessage),
    /// ICE candidate from either peer.
    IceCandidate(IceCandidateMessage),
    /// SDP offer from the connecting peer.
    SdpOffer(SdpMessage),
    /// SDP answer from the receiving peer.
    SdpAnswer(SdpMessage),
    /// Session terminated.
    SessionEnd(SessionEndMessage),
    /// Pairing invitation from host to a new device.
    PairingInvitation(PairingInvitationMessage),
    /// Pairing completion from client after scanning QR.
    PairingComplete(PairingCompleteMessage),
    /// Error response.
    Error(ErrorMessage),
    /// Heartbeat to keep the connection alive.
    Ping,
    /// Heartbeat response.
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterMessage {
    pub device_id: DeviceId,
    pub platform: Platform,
    /// Ed25519 public key bytes, base64-encoded.
    pub public_key: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredMessage {
    pub device_id: DeviceId,
    pub server_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostAnnounceMessage {
    pub host_device_id: DeviceId,
    pub display_name: String,
    pub platform: Platform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectRequestMessage {
    pub session_id: SessionId,
    pub client_device_id: DeviceId,
    pub host_device_id: DeviceId,
    /// Base64-encoded signature proving client holds the registered private key.
    pub auth_signature: String,
    /// The challenge that was signed (provided by server during registration ack).
    pub signed_challenge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectAcceptMessage {
    pub session_id: SessionId,
    pub host_device_id: DeviceId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectRejectMessage {
    pub session_id: SessionId,
    pub reason: ConnectRejectReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectRejectReason {
    NotAuthorized,
    HostBusy,
    UnattendedAccessDisabled,
    HostOffline,
    RateLimited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidateMessage {
    pub session_id: SessionId,
    pub from_device_id: DeviceId,
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_m_line_index: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdpMessage {
    pub session_id: SessionId,
    pub from_device_id: DeviceId,
    pub sdp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEndMessage {
    pub session_id: SessionId,
    pub reason: SessionEndReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndReason {
    UserDisconnected,
    HostDisconnected,
    Revoked,
    Timeout,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingInvitationMessage {
    /// Short-lived, single-use token embedded in the QR code.
    pub invitation_token: String,
    /// Host's public key for the client to verify during pairing.
    pub host_public_key: String,
    pub host_device_id: DeviceId,
    pub host_display_name: String,
    /// Expiry time for this invitation.
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingCompleteMessage {
    pub invitation_token: String,
    pub client_device_id: DeviceId,
    pub client_public_key: String,
    pub client_display_name: String,
    pub platform: Platform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub code: String,
    pub detail: String,
}

/// File transfer control messages sent over the DataChannel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransferMessage {
    /// Initiator announces an upcoming transfer.
    Announce(TransferAnnounce),
    /// Receiver accepts the transfer and provides the destination info.
    Accept(TransferAccept),
    /// Receiver rejects the transfer.
    Reject(TransferReject),
    /// A data chunk.
    Chunk(TransferChunk),
    /// Sender signals all chunks have been sent.
    Complete(TransferComplete),
    /// Either side cancels the transfer.
    Cancel(TransferCancel),
    /// Receiver acknowledges a chunk range (for flow control).
    Ack(TransferAck),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferAnnounce {
    pub transfer_id: TransferId,
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: Option<String>,
    /// SHA-256 hex digest of the complete file.
    pub sha256: String,
    pub direction: TransferDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferDirection {
    HostToClient,
    ClientToHost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferAccept {
    pub transfer_id: TransferId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferReject {
    pub transfer_id: TransferId,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferChunk {
    pub transfer_id: TransferId,
    pub sequence: u32,
    /// Raw bytes are sent separately on the binary DataChannel.
    pub byte_offset: u64,
    pub chunk_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferComplete {
    pub transfer_id: TransferId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferCancel {
    pub transfer_id: TransferId,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferAck {
    pub transfer_id: TransferId,
    pub last_received_sequence: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serializes_and_deserializes() {
        let msg = SignalingMessage::Ping;
        let envelope = Envelope::new(msg);
        let json = serde_json::to_string(&envelope).expect("serialize");
        let back: Envelope = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.version.major, 0);
    }

    #[test]
    fn connect_reject_reason_serializes_correctly() {
        let reason = ConnectRejectReason::NotAuthorized;
        let json = serde_json::to_string(&reason).unwrap();
        assert_eq!(json, r#""not_authorized""#);
    }

    #[test]
    fn transfer_direction_serializes_correctly() {
        let d = TransferDirection::HostToClient;
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, r#""host_to_client""#);
    }
}
