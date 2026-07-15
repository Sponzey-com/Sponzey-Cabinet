use cabinet_domain::comment::{Comment, CommentThread, CommentThreadId, CommentThreadState};
use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{AccessResource, Permission, PermissionDecision};
use cabinet_domain::user::UserId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;

pub trait CommentRepository {
    fn save_thread(
        &mut self,
        workspace_id: &WorkspaceId,
        thread: CommentThread,
    ) -> Result<(), CommentRepositoryError>;

    fn get_thread(
        &self,
        workspace_id: &WorkspaceId,
        thread_id: &CommentThreadId,
    ) -> Result<Option<CommentThread>, CommentRepositoryError>;

    fn append_comment(
        &mut self,
        workspace_id: &WorkspaceId,
        thread_id: &CommentThreadId,
        comment: Comment,
    ) -> Result<Option<CommentThread>, CommentRepositoryError>;

    fn update_thread_state(
        &mut self,
        workspace_id: &WorkspaceId,
        thread_id: &CommentThreadId,
        state: CommentThreadState,
    ) -> Result<Option<CommentThread>, CommentRepositoryError>;

    fn list_document_threads(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Vec<CommentThread>, CommentRepositoryError>;
}

pub trait CommentPermissionChecker {
    fn check_permission(
        &self,
        actor_user_id: &UserId,
        resource: &AccessResource,
        permission: Permission,
    ) -> Result<PermissionDecision, CommentPermissionCheckError>;
}

pub trait InlineAnchorDocumentLookup {
    fn get_anchor_document_state(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<InlineAnchorDocumentState>, InlineAnchorDocumentLookupError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineAnchorDocumentState {
    current_version_id: VersionId,
    anchored_document_len: usize,
}

impl InlineAnchorDocumentState {
    pub const fn new(current_version_id: VersionId, anchored_document_len: usize) -> Self {
        Self {
            current_version_id,
            anchored_document_len,
        }
    }

    pub fn current_version_id(&self) -> &VersionId {
        &self.current_version_id
    }

    pub const fn anchored_document_len(&self) -> usize {
        self.anchored_document_len
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentRepositoryError {
    StorageUnavailable,
    Conflict,
    CorruptedState,
}

impl CommentRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "comment_repository.storage_unavailable",
            Self::Conflict => "comment_repository.conflict",
            Self::CorruptedState => "comment_repository.corrupted_state",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentPermissionCheckError {
    StorageUnavailable,
}

impl CommentPermissionCheckError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "comment_permission.storage_unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineAnchorDocumentLookupError {
    StorageUnavailable,
    CorruptedVersion,
}

impl InlineAnchorDocumentLookupError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "inline_anchor_lookup.storage_unavailable",
            Self::CorruptedVersion => "inline_anchor_lookup.corrupted_version",
        }
    }
}
