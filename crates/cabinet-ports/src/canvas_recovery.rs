use cabinet_domain::canvas::{CanvasId, CanvasRevision};
use cabinet_domain::workspace::WorkspaceId;

pub trait CanvasRecoveryRepository {
    fn list_valid_revisions(
        &mut self,
        workspace_id: &WorkspaceId,
        canvas_id: &CanvasId,
        limit: usize,
    ) -> Result<Vec<CanvasRevision>, CanvasRecoveryRepositoryError>;

    fn activate_revision(
        &mut self,
        workspace_id: &WorkspaceId,
        canvas_id: &CanvasId,
        revision: CanvasRevision,
    ) -> Result<(), CanvasRecoveryRepositoryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasRecoveryRepositoryError {
    InvalidInput,
    StorageUnavailable,
    CorruptedCatalog,
    CandidateLimitExceeded,
    RevisionNotFound,
}

impl CanvasRecoveryRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "canvas_recovery.invalid_input",
            Self::StorageUnavailable => "canvas_recovery.storage_unavailable",
            Self::CorruptedCatalog => "canvas_recovery.corrupted_catalog",
            Self::CandidateLimitExceeded => "canvas_recovery.candidate_limit_exceeded",
            Self::RevisionNotFound => "canvas_recovery.revision_not_found",
        }
    }
}
