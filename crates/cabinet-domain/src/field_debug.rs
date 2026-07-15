use crate::user::UserId;
use crate::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDebugSession {
    session_id: FieldDebugSessionId,
    workspace_id: WorkspaceId,
    requested_by: UserId,
    approved_by: Option<UserId>,
    scope: Option<FieldDebugScope>,
    ttl: Option<FieldDebugTtl>,
    state: FieldDebugSessionState,
    requested_at: FieldDebugTimestamp,
    approved_at: Option<FieldDebugTimestamp>,
    activated_at: Option<FieldDebugTimestamp>,
    expires_at: Option<FieldDebugTimestamp>,
    ended_at: Option<FieldDebugTimestamp>,
}

impl FieldDebugSession {
    pub fn requested(
        session_id: FieldDebugSessionId,
        workspace_id: WorkspaceId,
        requested_by: UserId,
        scope: Option<FieldDebugScope>,
        ttl: Option<FieldDebugTtl>,
        requested_at: FieldDebugTimestamp,
    ) -> Self {
        Self {
            session_id,
            workspace_id,
            requested_by,
            approved_by: None,
            scope,
            ttl,
            state: FieldDebugSessionState::Requested,
            requested_at,
            approved_at: None,
            activated_at: None,
            expires_at: None,
            ended_at: None,
        }
    }

    pub fn session_id(&self) -> &FieldDebugSessionId {
        &self.session_id
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn requested_by(&self) -> &UserId {
        &self.requested_by
    }

    pub fn approved_by(&self) -> Option<&UserId> {
        self.approved_by.as_ref()
    }

    pub fn scope(&self) -> Option<&FieldDebugScope> {
        self.scope.as_ref()
    }

    pub const fn ttl(&self) -> Option<FieldDebugTtl> {
        self.ttl
    }

    pub const fn state(&self) -> FieldDebugSessionState {
        self.state
    }

    pub const fn requested_at(&self) -> FieldDebugTimestamp {
        self.requested_at
    }

    pub const fn approved_at(&self) -> Option<FieldDebugTimestamp> {
        self.approved_at
    }

    pub const fn activated_at(&self) -> Option<FieldDebugTimestamp> {
        self.activated_at
    }

    pub const fn expires_at(&self) -> Option<FieldDebugTimestamp> {
        self.expires_at
    }

    pub const fn ended_at(&self) -> Option<FieldDebugTimestamp> {
        self.ended_at
    }

    pub fn is_active_at(&self, at: FieldDebugTimestamp) -> bool {
        self.state == FieldDebugSessionState::Active
            && self.expires_at.is_some_and(|expires_at| at <= expires_at)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldDebugSessionState {
    Requested,
    Approved,
    Denied,
    Active,
    Expired,
    Revoked,
}

impl FieldDebugSessionState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldDebugEvent {
    Approve {
        admin_user_id: UserId,
        at: FieldDebugTimestamp,
    },
    Deny {
        admin_user_id: UserId,
        at: FieldDebugTimestamp,
    },
    Activate {
        at: FieldDebugTimestamp,
    },
    Expire {
        at: FieldDebugTimestamp,
    },
    Revoke {
        admin_user_id: UserId,
        at: FieldDebugTimestamp,
    },
}

impl FieldDebugEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::Approve { .. } => "approve",
            Self::Deny { .. } => "deny",
            Self::Activate { .. } => "activate",
            Self::Expire { .. } => "expire",
            Self::Revoke { .. } => "revoke",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDebugTransition {
    session: FieldDebugSession,
    product_log_event_name: &'static str,
}

impl FieldDebugTransition {
    pub fn session(&self) -> &FieldDebugSession {
        &self.session
    }

    pub const fn product_log_event_name(&self) -> &'static str {
        self.product_log_event_name
    }

    pub fn into_session(self) -> FieldDebugSession {
        self.session
    }
}

pub fn transition_field_debug_session(
    session: &FieldDebugSession,
    event: FieldDebugEvent,
) -> Result<FieldDebugTransition, FieldDebugError> {
    match event {
        FieldDebugEvent::Approve { admin_user_id, at } => approve(session, admin_user_id, at),
        FieldDebugEvent::Deny { admin_user_id, at } => deny(session, admin_user_id, at),
        FieldDebugEvent::Activate { at } => activate(session, at),
        FieldDebugEvent::Expire { at } => expire(session, at),
        FieldDebugEvent::Revoke { admin_user_id, at } => revoke(session, admin_user_id, at),
    }
}

fn approve(
    session: &FieldDebugSession,
    admin_user_id: UserId,
    at: FieldDebugTimestamp,
) -> Result<FieldDebugTransition, FieldDebugError> {
    if session.state != FieldDebugSessionState::Requested {
        return Err(FieldDebugError::InvalidTransition);
    }
    if session.scope.is_none() {
        return Err(FieldDebugError::MissingScope);
    }
    if session.ttl.is_none() {
        return Err(FieldDebugError::MissingTtl);
    }

    let mut next = session.clone();
    next.state = FieldDebugSessionState::Approved;
    next.approved_by = Some(admin_user_id);
    next.approved_at = Some(at);
    Ok(FieldDebugTransition {
        session: next,
        product_log_event_name: "field_debug.approved",
    })
}

fn deny(
    session: &FieldDebugSession,
    admin_user_id: UserId,
    at: FieldDebugTimestamp,
) -> Result<FieldDebugTransition, FieldDebugError> {
    if session.state != FieldDebugSessionState::Requested {
        return Err(FieldDebugError::InvalidTransition);
    }

    let mut next = session.clone();
    next.state = FieldDebugSessionState::Denied;
    next.approved_by = Some(admin_user_id);
    next.ended_at = Some(at);
    Ok(FieldDebugTransition {
        session: next,
        product_log_event_name: "field_debug.denied",
    })
}

fn activate(
    session: &FieldDebugSession,
    at: FieldDebugTimestamp,
) -> Result<FieldDebugTransition, FieldDebugError> {
    if session.state != FieldDebugSessionState::Approved {
        return Err(FieldDebugError::InvalidTransition);
    }
    let ttl = session.ttl.ok_or(FieldDebugError::MissingTtl)?;
    let mut next = session.clone();
    next.state = FieldDebugSessionState::Active;
    next.activated_at = Some(at);
    next.expires_at = Some(at.plus_seconds(ttl.as_seconds()));
    Ok(FieldDebugTransition {
        session: next,
        product_log_event_name: "field_debug.active",
    })
}

fn expire(
    session: &FieldDebugSession,
    at: FieldDebugTimestamp,
) -> Result<FieldDebugTransition, FieldDebugError> {
    if session.state != FieldDebugSessionState::Active {
        return Err(FieldDebugError::InvalidTransition);
    }
    let expires_at = session.expires_at.ok_or(FieldDebugError::MissingTtl)?;
    if at < expires_at {
        return Err(FieldDebugError::NotExpired);
    }

    let mut next = session.clone();
    next.state = FieldDebugSessionState::Expired;
    next.ended_at = Some(at);
    Ok(FieldDebugTransition {
        session: next,
        product_log_event_name: "field_debug.expired",
    })
}

fn revoke(
    session: &FieldDebugSession,
    admin_user_id: UserId,
    at: FieldDebugTimestamp,
) -> Result<FieldDebugTransition, FieldDebugError> {
    if session.state != FieldDebugSessionState::Active {
        return Err(FieldDebugError::InvalidTransition);
    }

    let mut next = session.clone();
    next.state = FieldDebugSessionState::Revoked;
    next.approved_by = Some(admin_user_id);
    next.ended_at = Some(at);
    Ok(FieldDebugTransition {
        session: next,
        product_log_event_name: "field_debug.revoked",
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldDebugSessionId {
    value: String,
}

impl FieldDebugSessionId {
    pub fn new(value: &str) -> Result<Self, FieldDebugError> {
        let value = validate_id(value, FieldDebugError::EmptySessionId)?;
        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDebugScope {
    value: String,
}

impl FieldDebugScope {
    pub fn new(value: &str) -> Result<Self, FieldDebugError> {
        let value = validate_id(value, FieldDebugError::EmptyScope)?;
        if contains_sensitive_fragment(&value) {
            return Err(FieldDebugError::SensitiveScope);
        }
        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldDebugTtl {
    seconds: u32,
}

impl FieldDebugTtl {
    pub const fn seconds(seconds: u32) -> Result<Self, FieldDebugError> {
        if seconds == 0 {
            return Err(FieldDebugError::MissingTtl);
        }
        Ok(Self { seconds })
    }

    pub const fn as_seconds(self) -> u32 {
        self.seconds
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldDebugTimestamp {
    millis_since_epoch: u64,
}

impl FieldDebugTimestamp {
    pub const fn from_millis(millis_since_epoch: u64) -> Self {
        Self { millis_since_epoch }
    }

    pub const fn as_millis(self) -> u64 {
        self.millis_since_epoch
    }

    pub const fn plus_seconds(self, seconds: u32) -> Self {
        Self {
            millis_since_epoch: self.millis_since_epoch + seconds as u64 * 1_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldDebugError {
    EmptySessionId,
    EmptyScope,
    SensitiveScope,
    MissingScope,
    MissingTtl,
    InvalidTransition,
    NotExpired,
}

fn validate_id(value: &str, empty_error: FieldDebugError) -> Result<String, FieldDebugError> {
    let value = value.trim();
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(empty_error);
    }
    Ok(value.to_string())
}

fn contains_sensitive_fragment(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "password",
        "token",
        "secret",
        "credential",
        "document_body",
        "comment_body",
        "document body",
        "comment body",
        "asset content",
        "asset bytes",
        "request body",
        "response body",
    ]
    .iter()
    .any(|fragment| lower.contains(fragment))
}
