use cabinet_domain::field_debug::{FieldDebugSession, FieldDebugSessionId, FieldDebugTimestamp};
use cabinet_domain::permission::{Permission, PermissionDecision};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;

pub trait FieldDebugSessionRepository {
    fn save_field_debug_session(
        &mut self,
        session: FieldDebugSession,
    ) -> Result<(), FieldDebugSessionRepositoryError>;

    fn get_field_debug_session(
        &self,
        workspace_id: &WorkspaceId,
        session_id: &FieldDebugSessionId,
    ) -> Result<Option<FieldDebugSession>, FieldDebugSessionRepositoryError>;
}

pub trait FieldDebugPermissionChecker {
    fn check_workspace_permission(
        &self,
        actor_user_id: &UserId,
        workspace_id: &WorkspaceId,
        permission: Permission,
    ) -> Result<PermissionDecision, FieldDebugPermissionCheckError>;
}

pub trait FieldDebugClock {
    fn now(&self) -> FieldDebugTimestamp;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldDebugSessionRepositoryError {
    StorageUnavailable,
    Conflict,
    CorruptedState,
}

impl FieldDebugSessionRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "field_debug_session.storage_unavailable",
            Self::Conflict => "field_debug_session.conflict",
            Self::CorruptedState => "field_debug_session.corrupted_state",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldDebugPermissionCheckError {
    StorageUnavailable,
}

impl FieldDebugPermissionCheckError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "field_debug_permission.storage_unavailable",
        }
    }
}
