use cabinet_domain::projection_work::{
    ProjectionWork, ProjectionWorkIdentity, ProjectionWorkState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionEnqueueOutcome {
    Enqueued,
    AlreadyExists,
}

pub trait ProjectionWorkRepository {
    fn enqueue(
        &mut self,
        work: ProjectionWork,
    ) -> Result<ProjectionEnqueueOutcome, ProjectionWorkRepositoryError>;

    fn get(
        &self,
        identity: &ProjectionWorkIdentity,
    ) -> Result<Option<ProjectionWork>, ProjectionWorkRepositoryError>;

    fn replace(
        &mut self,
        work: ProjectionWork,
        expected_state: ProjectionWorkState,
    ) -> Result<(), ProjectionWorkRepositoryError>;

    fn list_resumable(
        &self,
        limit: usize,
    ) -> Result<Vec<ProjectionWork>, ProjectionWorkRepositoryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionWorkRepositoryError {
    InvalidLimit,
    NotFound,
    Conflict,
    StorageUnavailable,
    CorruptedRecord,
    UnsupportedSchema,
}

impl ProjectionWorkRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimit => "projection_work.invalid_limit",
            Self::NotFound => "projection_work.not_found",
            Self::Conflict => "projection_work.conflict",
            Self::StorageUnavailable => "projection_work.storage_unavailable",
            Self::CorruptedRecord => "projection_work.corrupted",
            Self::UnsupportedSchema => "projection_work.unsupported_schema",
        }
    }
}
