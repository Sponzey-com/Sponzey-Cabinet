use std::fmt;

use crate::user::UserId;

#[derive(Clone, PartialEq, Eq)]
pub struct SessionId {
    value: String,
}

impl SessionId {
    pub fn new(value: &str) -> Result<Self, SessionError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(SessionError::EmptySessionId);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(SessionError::InvalidSessionId);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Debug for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionId(<redacted>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionInstant {
    epoch_seconds: u64,
}

impl SessionInstant {
    pub const fn from_epoch_seconds(epoch_seconds: u64) -> Self {
        Self { epoch_seconds }
    }

    pub const fn epoch_seconds(self) -> u64 {
        self.epoch_seconds
    }

    pub fn checked_add_seconds(self, seconds: u32) -> Option<Self> {
        self.epoch_seconds
            .checked_add(u64::from(seconds))
            .map(Self::from_epoch_seconds)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Created,
    Active,
    Expired,
    Revoked,
}

impl SessionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::Active => "Active",
            Self::Expired => "Expired",
            Self::Revoked => "Revoked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    id: SessionId,
    user_id: UserId,
    status: SessionStatus,
    created_at: SessionInstant,
    expires_at: SessionInstant,
}

impl Session {
    pub fn new_created(
        id: SessionId,
        user_id: UserId,
        created_at: SessionInstant,
        expires_at: SessionInstant,
    ) -> Result<Self, SessionError> {
        if expires_at <= created_at {
            return Err(SessionError::InvalidExpiry);
        }
        Ok(Self {
            id,
            user_id,
            status: SessionStatus::Created,
            created_at,
            expires_at,
        })
    }

    pub fn id(&self) -> &SessionId {
        &self.id
    }

    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub const fn status(&self) -> SessionStatus {
        self.status
    }

    pub const fn created_at(&self) -> SessionInstant {
        self.created_at
    }

    pub const fn expires_at(&self) -> SessionInstant {
        self.expires_at
    }

    pub fn activate(&self, now: SessionInstant) -> Result<Self, SessionTransitionError> {
        if self.status != SessionStatus::Created {
            return Err(SessionTransitionError::InvalidTransition);
        }
        if now >= self.expires_at {
            return Err(SessionTransitionError::Expired);
        }
        let mut next = self.clone();
        next.status = SessionStatus::Active;
        Ok(next)
    }

    pub fn expire(&self, now: SessionInstant) -> Result<Self, SessionTransitionError> {
        if matches!(self.status, SessionStatus::Expired | SessionStatus::Revoked) {
            return Err(SessionTransitionError::InvalidTransition);
        }
        if now < self.expires_at {
            return Err(SessionTransitionError::NotExpired);
        }
        let mut next = self.clone();
        next.status = SessionStatus::Expired;
        Ok(next)
    }

    pub fn revoke(&self) -> Result<Self, SessionTransitionError> {
        if matches!(self.status, SessionStatus::Expired | SessionStatus::Revoked) {
            return Err(SessionTransitionError::InvalidTransition);
        }
        let mut next = self.clone();
        next.status = SessionStatus::Revoked;
        Ok(next)
    }

    pub fn validate_at(&self, now: SessionInstant) -> Result<(), SessionValidationFailure> {
        match self.status {
            SessionStatus::Active if now < self.expires_at => Ok(()),
            SessionStatus::Active | SessionStatus::Expired => {
                Err(SessionValidationFailure::Expired)
            }
            SessionStatus::Revoked => Err(SessionValidationFailure::Revoked),
            SessionStatus::Created => Err(SessionValidationFailure::NotActive),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionValidationFailure {
    NotActive,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionTransitionError {
    InvalidTransition,
    NotExpired,
    Expired,
}

impl SessionTransitionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidTransition => "SESSION_INVALID_TRANSITION",
            Self::NotExpired => "SESSION_NOT_EXPIRED",
            Self::Expired => "SESSION_EXPIRED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    EmptySessionId,
    InvalidSessionId,
    InvalidExpiry,
}

impl SessionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptySessionId => "SESSION_EMPTY_ID",
            Self::InvalidSessionId => "SESSION_INVALID_ID",
            Self::InvalidExpiry => "SESSION_INVALID_EXPIRY",
        }
    }
}
