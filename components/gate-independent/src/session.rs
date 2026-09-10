use crate::{GateError, IdentitySubject};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(value: impl Into<String>) -> Result<Self, GateError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(GateError::InvalidSessionId);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Created,
    Authenticated(IdentitySubject),
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateSession {
    pub id: SessionId,
    state: SessionState,
}

impl GateSession {
    pub fn new(id: SessionId) -> Self {
        Self {
            id,
            state: SessionState::Created,
        }
    }

    pub fn state(&self) -> &SessionState {
        &self.state
    }

    pub fn authenticate(&mut self, subject: IdentitySubject) -> Result<(), GateError> {
        if matches!(self.state, SessionState::Closed) {
            return Err(GateError::SessionClosed);
        }
        self.state = SessionState::Authenticated(subject);
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), GateError> {
        if matches!(self.state, SessionState::Closed) {
            return Err(GateError::SessionClosed);
        }
        self.state = SessionState::Closed;
        Ok(())
    }

    pub fn authenticated_subject(&self) -> Option<&IdentitySubject> {
        match &self.state {
            SessionState::Authenticated(subject) => Some(subject),
            SessionState::Created | SessionState::Closed => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_transitions_created_to_authenticated_to_closed() {
        let id = SessionId::new("s-1").unwrap();
        let subject = IdentitySubject::new("user-1").unwrap();
        let mut session = GateSession::new(id);

        assert_eq!(session.state(), &SessionState::Created);
        session.authenticate(subject.clone()).unwrap();
        assert_eq!(session.authenticated_subject(), Some(&subject));
        session.close().unwrap();
        assert_eq!(session.state(), &SessionState::Closed);
    }

    #[test]
    fn closed_session_cannot_reauthenticate() {
        let id = SessionId::new("s-1").unwrap();
        let mut session = GateSession::new(id);
        session.close().unwrap();

        let err = session
            .authenticate(IdentitySubject::new("user-1").unwrap())
            .unwrap_err();
        assert_eq!(err, GateError::SessionClosed);
    }
}
