use cabinet_domain::projection_repair::{
    ProjectionRepairOperation, ProjectionRepairOperationId, ProjectionRepairState,
};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionRepairCreateOutcome {
    Created,
    AlreadyExists,
}

pub trait ProjectionRepairRepository {
    fn create(
        &mut self,
        operation: ProjectionRepairOperation,
    ) -> Result<ProjectionRepairCreateOutcome, ProjectionRepairRepositoryError>;

    fn get(
        &self,
        operation_id: &ProjectionRepairOperationId,
    ) -> Result<Option<ProjectionRepairOperation>, ProjectionRepairRepositoryError>;

    fn replace(
        &mut self,
        operation: ProjectionRepairOperation,
        expected_state: ProjectionRepairState,
    ) -> Result<(), ProjectionRepairRepositoryError>;

    fn list_active(
        &self,
        workspace_id: &WorkspaceId,
        limit: usize,
    ) -> Result<Vec<ProjectionRepairOperation>, ProjectionRepairRepositoryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionRepairRepositoryError {
    InvalidLimit,
    NotFound,
    Conflict,
    StorageUnavailable,
    CorruptedRecord,
    UnsupportedSchema,
}

impl ProjectionRepairRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimit => "projection_repair_repository.invalid_limit",
            Self::NotFound => "projection_repair_repository.not_found",
            Self::Conflict => "projection_repair_repository.conflict",
            Self::StorageUnavailable => "projection_repair_repository.storage_unavailable",
            Self::CorruptedRecord => "projection_repair_repository.corrupted",
            Self::UnsupportedSchema => "projection_repair_repository.unsupported_schema",
        }
    }
}
