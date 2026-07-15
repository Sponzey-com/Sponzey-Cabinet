use cabinet_domain::document::DocumentId;
use cabinet_domain::document_lock::{DocumentLock, DocumentLockTimestamp};
use cabinet_domain::permission::{Permission, PermissionDecision};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;

pub trait DocumentLockRepository {
    fn get_document_lock(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentLock>, DocumentLockRepositoryError>;

    fn save_document_lock(
        &mut self,
        workspace_id: &WorkspaceId,
        lock: DocumentLock,
    ) -> Result<(), DocumentLockRepositoryError>;

    fn delete_document_lock(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentLock>, DocumentLockRepositoryError>;
}

pub trait DocumentLockPermissionChecker {
    fn check_document_permission(
        &self,
        actor_user_id: &UserId,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        permission: Permission,
    ) -> Result<PermissionDecision, DocumentLockPermissionCheckError>;
}

pub trait DocumentLockClock {
    fn now(&self) -> DocumentLockTimestamp;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLockRepositoryError {
    StorageUnavailable,
    Conflict,
    CorruptedState,
}

impl DocumentLockRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "document_lock_repository.storage_unavailable",
            Self::Conflict => "document_lock_repository.conflict",
            Self::CorruptedState => "document_lock_repository.corrupted_state",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLockPermissionCheckError {
    StorageUnavailable,
}

impl DocumentLockPermissionCheckError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "document_lock_permission.storage_unavailable",
        }
    }
}
