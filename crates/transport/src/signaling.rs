//! Client implementation for OpenRemote coordination server WebSocket signaling.
//!
//! Connects to `/ws/signal` to negotiate WebRTC sessions, exchange SDP offers/answers,
//! and relay trickle ICE candidates.

use crate::TransportError;
use futures::{SinkExt, StreamExt};
use openremote_protocol::messages::{
    ConnectAcceptMessage, ConnectRejectMessage, ConnectRejectReason, ConnectRequestMessage,
    Envelope, IceCandidateMessage, RegisterMessage, SdpMessage, SessionEndMessage,
    SessionEndReason, SignalingMessage,
};
use openremote_protocol::types::{DeviceId, Platform, SessionId};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::debug;

/// Active signaling WebSocket client connection.
pub struct SignalingClient {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    device_id: DeviceId,
}

impl SignalingClient {
    /// Connect to the coordination server WebSocket at the given URL (e.g. "ws://127.0.0.1:8080/ws/signal").
    pub async fn connect(url: &str, device_id: DeviceId) -> Result<Self, TransportError> {
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| TransportError::SendFailed(format!("WebSocket connect failed: {}", e)))?;

        debug!(url = %url, device_id = %device_id, "Connected to coordination signaling server");

        Ok(Self {
            ws_stream,
            device_id,
        })
    }

    /// Client's local DeviceId.
    pub fn device_id(&self) -> DeviceId {
        self.device_id.clone()
    }

    /// Register this client with the coordination server.
    pub async fn register(
        &mut self,
        platform: Platform,
        public_key: String,
        display_name: String,
    ) -> Result<(), TransportError> {
        let msg = SignalingMessage::Register(RegisterMessage {
            device_id: self.device_id.clone(),
            platform,
            public_key,
            display_name,
        });

        self.send_message(msg).await?;

        // Wait for Registered ack
        let ack = self.recv_envelope().await?;
        match ack.payload {
            SignalingMessage::Registered(_) => Ok(()),
            other => Err(TransportError::ReceiveFailed(format!(
                "Expected Registered ack, got {:?}",
                other
            ))),
        }
    }

    /// Send an envelope to the signaling server.
    pub async fn send_envelope(&mut self, envelope: Envelope) -> Result<(), TransportError> {
        let json = serde_json::to_string(&envelope).map_err(|e| {
            TransportError::SendFailed(format!("Failed to serialize envelope: {}", e))
        })?;

        self.ws_stream
            .send(Message::Text(json))
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))
    }

    /// Convenience wrapper to wrap a payload in an Envelope and send it.
    pub async fn send_message(&mut self, payload: SignalingMessage) -> Result<(), TransportError> {
        let envelope = Envelope::new(payload);
        self.send_envelope(envelope).await
    }

    /// Receive the next envelope from the signaling server.
    pub async fn recv_envelope(&mut self) -> Result<Envelope, TransportError> {
        while let Some(msg) = self.ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let envelope: Envelope = serde_json::from_str(&text).map_err(|e| {
                        TransportError::ReceiveFailed(format!("Invalid envelope JSON: {}", e))
                    })?;
                    return Ok(envelope);
                }
                Ok(Message::Ping(data)) => {
                    let _ = self.ws_stream.send(Message::Pong(data)).await;
                }
                Ok(Message::Close(_)) => {
                    return Err(TransportError::Closed);
                }
                Err(e) => {
                    return Err(TransportError::ReceiveFailed(e.to_string()));
                }
                _ => {}
            }
        }
        Err(TransportError::Closed)
    }

    /// Request a remote session connection to target host.
    pub async fn send_connect_request(
        &mut self,
        session_id: SessionId,
        host_device_id: DeviceId,
        auth_signature: String,
        signed_challenge: String,
    ) -> Result<(), TransportError> {
        let msg = SignalingMessage::ConnectRequest(ConnectRequestMessage {
            session_id,
            client_device_id: self.device_id.clone(),
            host_device_id,
            auth_signature,
            signed_challenge,
        });
        self.send_message(msg).await
    }

    /// Accept an incoming connection request.
    pub async fn send_connect_accept(
        &mut self,
        session_id: SessionId,
    ) -> Result<(), TransportError> {
        let msg = SignalingMessage::ConnectAccept(ConnectAcceptMessage {
            session_id,
            host_device_id: self.device_id.clone(),
        });
        self.send_message(msg).await
    }

    /// Reject an incoming connection request.
    pub async fn send_connect_reject(
        &mut self,
        session_id: SessionId,
        reason: ConnectRejectReason,
    ) -> Result<(), TransportError> {
        let msg = SignalingMessage::ConnectReject(ConnectRejectMessage { session_id, reason });
        self.send_message(msg).await
    }

    /// Send WebRTC SDP Offer.
    pub async fn send_sdp_offer(
        &mut self,
        session_id: SessionId,
        sdp: String,
    ) -> Result<(), TransportError> {
        let msg = SignalingMessage::SdpOffer(SdpMessage {
            session_id,
            from_device_id: self.device_id.clone(),
            sdp,
        });
        self.send_message(msg).await
    }

    /// Send WebRTC SDP Answer.
    pub async fn send_sdp_answer(
        &mut self,
        session_id: SessionId,
        sdp: String,
    ) -> Result<(), TransportError> {
        let msg = SignalingMessage::SdpAnswer(SdpMessage {
            session_id,
            from_device_id: self.device_id.clone(),
            sdp,
        });
        self.send_message(msg).await
    }

    /// Send trickle ICE candidate.
    pub async fn send_ice_candidate(
        &mut self,
        session_id: SessionId,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_m_line_index: Option<u16>,
    ) -> Result<(), TransportError> {
        let msg = SignalingMessage::IceCandidate(IceCandidateMessage {
            session_id,
            from_device_id: self.device_id.clone(),
            candidate,
            sdp_mid,
            sdp_m_line_index,
        });
        self.send_message(msg).await
    }

    /// Signal session termination.
    pub async fn send_session_end(
        &mut self,
        session_id: SessionId,
        reason: SessionEndReason,
    ) -> Result<(), TransportError> {
        let msg = SignalingMessage::SessionEnd(SessionEndMessage { session_id, reason });
        self.send_message(msg).await
    }

    /// Close the signaling WebSocket.
    pub async fn close(&mut self) -> Result<(), TransportError> {
        self.ws_stream
            .close(None)
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))
    }
}
