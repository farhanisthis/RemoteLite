use thiserror::Error;

#[derive(Debug, Error)]
pub enum HostServiceError {
    #[error("identity error: {0}")]
    Identity(#[from] openremote_identity::IdentityError),
    #[error("authorization error: {0}")]
    Authorization(#[from] openremote_authorization::AuthorizationError),
    #[error("session error: {0}")]
    Session(#[from] openremote_session::SessionError),
    #[error("transport error: {0}")]
    Transport(#[from] openremote_transport::TransportError),
    #[error("invitation expired or invalid")]
    InvalidInvitation,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("internal error: {0}")]
    Internal(String),
}
