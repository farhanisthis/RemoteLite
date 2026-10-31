//! Pairing invitation management endpoints.
//!
//! POST /api/v1/pairing/invite — Host creates a short-lived invitation token.
//! POST /api/v1/pairing/complete — Client completes the pairing with the token.

use crate::{
    error::ApiError,
    state::{AppState, PairingInvitation},
};
use axum::{extract::State, Json};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::{Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateInviteRequest {
    pub host_device_id: String,
    pub host_public_key: String,
    pub host_display_name: String,
}

#[derive(Debug, Serialize)]
pub struct CreateInviteResponse {
    pub invitation_token: String,
    pub expires_at: chrono::DateTime<Utc>,
}

/// POST /api/v1/pairing/invite
pub async fn create_invite(
    State(state): State<AppState>,
    Json(req): Json<CreateInviteRequest>,
) -> Result<Json<CreateInviteResponse>, ApiError> {
    // Generate a cryptographically random 32-byte token.
    let mut token_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut token_bytes);
    let token = B64.encode(token_bytes);

    let expires_at = Utc::now() + Duration::minutes(5);

    let invitation = PairingInvitation {
        token: token.clone(),
        host_device_id: req.host_device_id,
        host_public_key: req.host_public_key,
        host_display_name: req.host_display_name,
        expires_at,
        used: false,
    };

    state
        .inner
        .pairing_invitations
        .write()
        .await
        .insert(token.clone(), invitation);

    tracing::info!("pairing invitation created");

    Ok(Json(CreateInviteResponse {
        invitation_token: token,
        expires_at,
    }))
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CompletePairingRequest {
    pub invitation_token: String,
    pub client_device_id: String,
    pub client_public_key: String,
    pub client_display_name: String,
}

#[derive(Debug, Serialize)]
pub struct CompletePairingResponse {
    pub host_device_id: String,
    pub host_public_key: String,
    pub host_display_name: String,
}

/// POST /api/v1/pairing/complete
pub async fn complete_pairing(
    State(state): State<AppState>,
    Json(req): Json<CompletePairingRequest>,
) -> Result<Json<CompletePairingResponse>, ApiError> {
    let mut invitations = state.inner.pairing_invitations.write().await;

    let invitation = invitations
        .get_mut(&req.invitation_token)
        .ok_or_else(|| ApiError::NotFound("invitation not found".into()))?;

    if invitation.used {
        return Err(ApiError::Unauthorized("invitation already used".into()));
    }
    if Utc::now() > invitation.expires_at {
        return Err(ApiError::Unauthorized("invitation expired".into()));
    }

    // Mark as used (single-use).
    invitation.used = true;

    let response = CompletePairingResponse {
        host_device_id: invitation.host_device_id.clone(),
        host_public_key: invitation.host_public_key.clone(),
        host_display_name: invitation.host_display_name.clone(),
    };

    tracing::info!(
        client_id = %req.client_device_id,
        "pairing completed"
    );

    Ok(Json(response))
}
