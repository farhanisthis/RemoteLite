//! Session lifecycle management.

use crate::error::SessionError;
use chrono::Utc;
use openremote_protocol::session::{SessionInfo, SessionState};
use openremote_protocol::types::{DeviceId, SessionId};
use std::collections::HashMap;
use tracing::{info, warn};

/// Maximum number of concurrent sessions (MVP: 1).
const MAX_SESSIONS: usize = 1;

/// Manages active remote sessions.
pub struct SessionManager {
    sessions: HashMap<SessionId, SessionInfo>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Create a new session record.
    pub fn create_session(
        &mut self,
        host_device_id: DeviceId,
        client_device_id: DeviceId,
    ) -> Result<SessionId, SessionError> {
        if self.sessions.len() >= MAX_SESSIONS {
            warn!("session limit reached");
            return Err(SessionError::LimitReached);
        }

        let session_id = SessionId::new();
        let info = SessionInfo {
            session_id: session_id.clone(),
            host_device_id,
            client_device_id,
            state: SessionState::Connecting,
            started_at: Utc::now(),
            ended_at: None,
            connection_type: None,
        };

        self.sessions.insert(session_id.clone(), info);
        info!(session_id = %session_id, "session created");
        Ok(session_id)
    }

    /// Transition a session to a new state.
    pub fn transition(
        &mut self,
        session_id: &SessionId,
        new_state: SessionState,
    ) -> Result<(), SessionError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::NotFound {
                id: session_id.to_string(),
            })?;
        info!(
            session_id = %session_id,
            old_state = ?session.state,
            new_state = ?new_state,
            "session state transition"
        );
        session.state = new_state;
        Ok(())
    }

    /// End a session, recording the termination time.
    pub fn end_session(&mut self, session_id: &SessionId) -> Result<(), SessionError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::NotFound {
                id: session_id.to_string(),
            })?;
        session.state = SessionState::Ended;
        session.ended_at = Some(Utc::now());
        info!(session_id = %session_id, "session ended");
        Ok(())
    }

    /// Get session info.
    pub fn get(&self, session_id: &SessionId) -> Option<&SessionInfo> {
        self.sessions.get(session_id)
    }

    /// List all active sessions.
    pub fn active_sessions(&self) -> Vec<&SessionInfo> {
        self.sessions
            .values()
            .filter(|s| s.state == SessionState::Active)
            .collect()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_transition_session() {
        let mut mgr = SessionManager::new();
        let host = DeviceId::new();
        let client = DeviceId::new();
        let sid = mgr.create_session(host, client).unwrap();
        assert!(mgr.get(&sid).is_some());
        mgr.transition(&sid, SessionState::Active).unwrap();
        assert_eq!(mgr.get(&sid).unwrap().state, SessionState::Active);
    }

    #[test]
    fn session_limit_is_enforced() {
        let mut mgr = SessionManager::new();
        let h = DeviceId::new();
        let c = DeviceId::new();
        mgr.create_session(h.clone(), c.clone()).unwrap();
        let result = mgr.create_session(h, c);
        assert!(matches!(result, Err(SessionError::LimitReached)));
    }

    #[test]
    fn end_session_records_time() {
        let mut mgr = SessionManager::new();
        let sid = mgr
            .create_session(DeviceId::new(), DeviceId::new())
            .unwrap();
        mgr.end_session(&sid).unwrap();
        let info = mgr.get(&sid).unwrap();
        assert!(info.ended_at.is_some());
        assert_eq!(info.state, SessionState::Ended);
    }
}
