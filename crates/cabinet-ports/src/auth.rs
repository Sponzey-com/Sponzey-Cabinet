use std::fmt;

use cabinet_domain::session::{Session, SessionInstant};
use cabinet_domain::user::{User, UserLogin};

#[derive(Clone, PartialEq, Eq)]
pub struct CredentialSecret {
    value: String,
}

impl CredentialSecret {
    pub fn new(value: &str) -> Result<Self, AuthPortValueError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(AuthPortValueError::EmptySecret);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(AuthPortValueError::InvalidSecret);
        }
        Ok(Self {
            value: value.to_string(),
        })
    }

    pub fn expose_secret(&self) -> &str {
        &self.value
    }
}

impl fmt::Debug for CredentialSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CredentialSecret(<redacted>)")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct PresentedSessionToken {
    value: String,
}

impl PresentedSessionToken {
    pub fn new(value: &str) -> Result<Self, AuthPortValueError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(AuthPortValueError::EmptyToken);
        }
        if trimmed.chars().any(char::is_control) || trimmed.chars().any(char::is_whitespace) {
            return Err(AuthPortValueError::InvalidToken);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn expose_secret(&self) -> &str {
        &self.value
    }
}

impl fmt::Debug for PresentedSessionToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PresentedSessionToken(<redacted>)")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SessionLookupKey {
    value: String,
}

impl SessionLookupKey {
    pub fn new(value: &str) -> Result<Self, AuthPortValueError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(AuthPortValueError::EmptyLookupKey);
        }
        if trimmed.chars().any(char::is_control) || trimmed.chars().any(char::is_whitespace) {
            return Err(AuthPortValueError::InvalidLookupKey);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Debug for SessionLookupKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionLookupKey(<redacted>)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedSessionToken {
    token: PresentedSessionToken,
    lookup_key: SessionLookupKey,
}

impl IssuedSessionToken {
    pub fn new(token: PresentedSessionToken, lookup_key: SessionLookupKey) -> Self {
        Self { token, lookup_key }
    }

    pub fn token(&self) -> &PresentedSessionToken {
        &self.token
    }

    pub fn into_token(self) -> PresentedSessionToken {
        self.token
    }

    pub fn lookup_key(&self) -> &SessionLookupKey {
        &self.lookup_key
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthPortValueError {
    EmptySecret,
    InvalidSecret,
    EmptyToken,
    InvalidToken,
    EmptyLookupKey,
    InvalidLookupKey,
}

pub trait CredentialVerifier {
    fn verify(
        &self,
        login: &UserLogin,
        credential: &CredentialSecret,
    ) -> Result<Option<User>, CredentialVerifierError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialVerifierError {
    Unavailable,
}

pub trait TokenIssuer {
    fn issue_token(&mut self) -> Result<IssuedSessionToken, TokenIssuerError>;

    fn lookup_key_for(
        &self,
        token: &PresentedSessionToken,
    ) -> Result<SessionLookupKey, TokenIssuerError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenIssuerError {
    Unavailable,
    InvalidToken,
}

pub trait SessionStore {
    fn create_session(
        &mut self,
        lookup_key: SessionLookupKey,
        session: Session,
    ) -> Result<(), SessionStoreError>;

    fn get_session(
        &self,
        lookup_key: &SessionLookupKey,
    ) -> Result<Option<Session>, SessionStoreError>;

    fn update_session(
        &mut self,
        lookup_key: &SessionLookupKey,
        session: Session,
    ) -> Result<(), SessionStoreError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStoreError {
    Conflict,
    NotFound,
    Unavailable,
}

pub trait SessionClock {
    fn now(&self) -> SessionInstant;
}

pub trait SessionIdGenerator {
    fn generate_session_id(&mut self) -> String;
}
