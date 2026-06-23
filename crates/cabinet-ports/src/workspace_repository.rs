use cabinet_domain::workspace::{Workspace, WorkspaceId};

pub trait WorkspaceRepository {
    fn put_workspace(&mut self, workspace: Workspace) -> Result<(), WorkspaceRepositoryError>;

    fn get_workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Option<Workspace>, WorkspaceRepositoryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceRepositoryError {
    Conflict,
    StorageUnavailable,
}

impl WorkspaceRepositoryError {
    pub fn code(self) -> &'static str {
        match self {
            Self::Conflict => "workspace_repository.conflict",
            Self::StorageUnavailable => "workspace_repository.storage_unavailable",
        }
    }
}
