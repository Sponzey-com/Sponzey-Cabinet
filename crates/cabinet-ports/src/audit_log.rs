use cabinet_domain::audit::{AuditEvent, AuditTimestamp};
use cabinet_domain::permission::{Permission, PermissionDecision};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;

pub trait AuditLogStore {
    fn append_audit_event(&mut self, event: AuditEvent) -> Result<(), AuditLogStoreError>;

    fn list_audit_events(
        &self,
        query: AuditListQuery,
    ) -> Result<AuditEventPage, AuditLogStoreError>;
}

pub trait AuditPermissionChecker {
    fn check_workspace_permission(
        &self,
        actor_user_id: &UserId,
        workspace_id: &WorkspaceId,
        permission: Permission,
    ) -> Result<PermissionDecision, AuditPermissionCheckError>;
}

pub trait AuditClock {
    fn now(&self) -> AuditTimestamp;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditListQuery {
    workspace_id: WorkspaceId,
    scope: AuditListScope,
    page: AuditPageRequest,
}

impl AuditListQuery {
    pub fn new(workspace_id: WorkspaceId, scope: AuditListScope, page: AuditPageRequest) -> Self {
        Self {
            workspace_id,
            scope,
            page,
        }
    }

    pub fn workspace(workspace_id: WorkspaceId, page: AuditPageRequest) -> Self {
        Self::new(workspace_id, AuditListScope::Workspace, page)
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub const fn scope(&self) -> &AuditListScope {
        &self.scope
    }

    pub const fn page(&self) -> &AuditPageRequest {
        &self.page
    }

    pub fn matches(&self, event: &AuditEvent) -> bool {
        event.workspace_id() == &self.workspace_id && self.scope.matches(event)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditListScope {
    Workspace,
    Actor {
        actor_user_id: UserId,
    },
    Target {
        target_type: String,
        target_id: String,
    },
}

impl AuditListScope {
    pub fn actor(actor_user_id: UserId) -> Self {
        Self::Actor { actor_user_id }
    }

    pub fn target(target_type: &str, target_id: &str) -> Result<Self, AuditLogStoreError> {
        let target_type = validate_scope_part(target_type)?;
        let target_id = validate_scope_part(target_id)?;
        Ok(Self::Target {
            target_type,
            target_id,
        })
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Actor { .. } => "actor",
            Self::Target { .. } => "target",
        }
    }

    pub fn matches(&self, event: &AuditEvent) -> bool {
        match self {
            Self::Workspace => true,
            Self::Actor { actor_user_id } => event.actor().actor_id() == actor_user_id.as_str(),
            Self::Target {
                target_type,
                target_id,
            } => {
                event.target().target_type() == target_type
                    && event.target().target_id() == target_id
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditPageRequest {
    limit: usize,
    cursor: Option<AuditCursor>,
}

impl AuditPageRequest {
    pub fn new(limit: usize, cursor: Option<AuditCursor>) -> Result<Self, AuditLogStoreError> {
        if !(1..=500).contains(&limit) {
            return Err(AuditLogStoreError::InvalidLimit);
        }
        Ok(Self { limit, cursor })
    }

    pub const fn limit(&self) -> usize {
        self.limit
    }

    pub const fn cursor(&self) -> Option<&AuditCursor> {
        self.cursor.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditCursor {
    value: String,
    offset: usize,
}

impl AuditCursor {
    pub fn new(value: &str) -> Result<Self, AuditLogStoreError> {
        let value = value.trim();
        if value.is_empty() || value.chars().any(char::is_control) {
            return Err(AuditLogStoreError::InvalidCursor);
        }
        let offset = value
            .parse::<usize>()
            .map_err(|_| AuditLogStoreError::InvalidCursor)?;
        Ok(Self {
            value: value.to_string(),
            offset,
        })
    }

    pub fn from_offset(offset: usize) -> Self {
        Self {
            value: offset.to_string(),
            offset,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub const fn offset(&self) -> usize {
        self.offset
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEventPage {
    events: Vec<AuditEvent>,
    next_cursor: Option<AuditCursor>,
}

impl AuditEventPage {
    pub fn new(events: Vec<AuditEvent>, next_cursor: Option<AuditCursor>) -> Self {
        Self {
            events,
            next_cursor,
        }
    }

    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    pub const fn next_cursor(&self) -> Option<&AuditCursor> {
        self.next_cursor.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditLogStoreError {
    InvalidLimit,
    InvalidCursor,
    InvalidScope,
    StorageUnavailable,
    Conflict,
    CorruptedState,
}

impl AuditLogStoreError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimit => "audit_log.invalid_limit",
            Self::InvalidCursor => "audit_log.invalid_cursor",
            Self::InvalidScope => "audit_log.invalid_scope",
            Self::StorageUnavailable => "audit_log.storage_unavailable",
            Self::Conflict => "audit_log.conflict",
            Self::CorruptedState => "audit_log.corrupted_state",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditPermissionCheckError {
    StorageUnavailable,
}

impl AuditPermissionCheckError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "audit_permission.storage_unavailable",
        }
    }
}

fn validate_scope_part(value: &str) -> Result<String, AuditLogStoreError> {
    let value = value.trim();
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(AuditLogStoreError::InvalidScope);
    }
    Ok(value.to_string())
}
