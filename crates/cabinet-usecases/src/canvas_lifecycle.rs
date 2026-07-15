use cabinet_domain::canvas::{
    Canvas, CanvasId, CanvasLifecycleEvent, CanvasLifecycleState, CanvasRevision, CanvasTitle,
    CanvasViewport, transition_canvas_lifecycle,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository, CanvasRepositoryError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCanvasRecordInput {
    workspace: String,
    canvas: String,
    title: String,
}
impl CreateCanvasRecordInput {
    pub fn new(workspace: &str, canvas: &str, title: &str) -> Self {
        Self {
            workspace: workspace.into(),
            canvas: canvas.into(),
            title: title.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetCanvasRecordInput {
    workspace: String,
    canvas: String,
}
impl GetCanvasRecordInput {
    pub fn new(workspace: &str, canvas: &str) -> Self {
        Self {
            workspace: workspace.into(),
            canvas: canvas.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameCanvasInput {
    workspace: String,
    canvas: String,
    expected_revision: u64,
    title: String,
}
impl RenameCanvasInput {
    pub fn new(workspace: &str, canvas: &str, expected_revision: u64, title: &str) -> Self {
        Self {
            workspace: workspace.into(),
            canvas: canvas.into(),
            expected_revision,
            title: title.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveCanvasInput {
    workspace: String,
    canvas: String,
    expected_revision: u64,
}
impl ArchiveCanvasInput {
    pub fn new(workspace: &str, canvas: &str, expected_revision: u64) -> Self {
        Self {
            workspace: workspace.into(),
            canvas: canvas.into(),
            expected_revision,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasRecordOutput {
    record: CanvasRecord,
}
impl CanvasRecordOutput {
    pub fn record(&self) -> &CanvasRecord {
        &self.record
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasLifecycleProductEvent {
    Created { canvas_id: String, revision: u64 },
    Renamed { canvas_id: String, revision: u64 },
    Archived { canvas_id: String, revision: u64 },
}
pub trait CanvasLifecycleProductLogger {
    fn write_product(&mut self, event: CanvasLifecycleProductEvent);
}

pub struct CreateCanvasRecordUsecase;
impl CreateCanvasRecordUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: CanvasRepository, L: CanvasLifecycleProductLogger>(
        &self,
        input: CreateCanvasRecordInput,
        repository: &mut R,
        logger: &mut L,
    ) -> Result<CanvasRecordOutput, CanvasLifecycleUsecaseError> {
        let workspace = workspace(&input.workspace)?;
        let canvas_id = canvas_id(&input.canvas)?;
        let title = CanvasTitle::new(&input.title)
            .map_err(|_| CanvasLifecycleUsecaseError::InvalidInput)?;
        let canvas = Canvas::new(
            canvas_id.clone(),
            vec![],
            vec![],
            CanvasLifecycleState::Draft,
        )
        .map_err(|_| CanvasLifecycleUsecaseError::InvalidInput)?;
        let record = CanvasRecord::with_metadata(
            canvas,
            title,
            CanvasRevision::new(1).map_err(|_| CanvasLifecycleUsecaseError::InvalidInput)?,
            CanvasViewport::default(),
        );
        repository
            .create_canvas(&workspace, record.clone())
            .map_err(map_repository)?;
        logger.write_product(CanvasLifecycleProductEvent::Created {
            canvas_id: canvas_id.as_str().into(),
            revision: 1,
        });
        Ok(CanvasRecordOutput { record })
    }
}

pub struct GetCanvasRecordUsecase;
impl GetCanvasRecordUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: CanvasRepository>(
        &self,
        input: GetCanvasRecordInput,
        repository: &R,
    ) -> Result<CanvasRecordOutput, CanvasLifecycleUsecaseError> {
        let record = repository
            .get_canvas(&workspace(&input.workspace)?, &canvas_id(&input.canvas)?)
            .map_err(map_repository)?
            .ok_or(CanvasLifecycleUsecaseError::NotFound)?;
        Ok(CanvasRecordOutput { record })
    }
}

pub struct RenameCanvasUsecase;
impl RenameCanvasUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: CanvasRepository, L: CanvasLifecycleProductLogger>(
        &self,
        input: RenameCanvasInput,
        repository: &mut R,
        logger: &mut L,
    ) -> Result<CanvasRecordOutput, CanvasLifecycleUsecaseError> {
        let workspace = workspace(&input.workspace)?;
        let canvas_id = canvas_id(&input.canvas)?;
        let expected = revision(input.expected_revision)?;
        let current = repository
            .get_canvas(&workspace, &canvas_id)
            .map_err(map_repository)?
            .ok_or(CanvasLifecycleUsecaseError::NotFound)?;
        if current.revision() != expected {
            return Err(CanvasLifecycleUsecaseError::VersionConflict);
        }
        if current.canvas().state() == CanvasLifecycleState::Archived {
            return Err(CanvasLifecycleUsecaseError::InvalidState);
        }
        let next = current
            .revised(
                current.canvas().clone(),
                CanvasTitle::new(&input.title)
                    .map_err(|_| CanvasLifecycleUsecaseError::InvalidInput)?,
                current.viewport(),
            )
            .map_err(map_repository)?;
        repository
            .replace_canvas(&workspace, expected, next.clone())
            .map_err(map_repository)?;
        logger.write_product(CanvasLifecycleProductEvent::Renamed {
            canvas_id: canvas_id.as_str().into(),
            revision: next.revision().value(),
        });
        Ok(CanvasRecordOutput { record: next })
    }
}

pub struct ArchiveCanvasUsecase;
impl ArchiveCanvasUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: CanvasRepository, L: CanvasLifecycleProductLogger>(
        &self,
        input: ArchiveCanvasInput,
        repository: &mut R,
        logger: &mut L,
    ) -> Result<CanvasRecordOutput, CanvasLifecycleUsecaseError> {
        let workspace = workspace(&input.workspace)?;
        let canvas_id = canvas_id(&input.canvas)?;
        let expected = revision(input.expected_revision)?;
        let current = repository
            .get_canvas(&workspace, &canvas_id)
            .map_err(map_repository)?
            .ok_or(CanvasLifecycleUsecaseError::NotFound)?;
        if current.revision() != expected {
            return Err(CanvasLifecycleUsecaseError::VersionConflict);
        }
        let state =
            transition_canvas_lifecycle(current.canvas().state(), CanvasLifecycleEvent::Archive)
                .map_err(|_| CanvasLifecycleUsecaseError::InvalidState)?;
        let canvas = Canvas::new(
            current.canvas().id().clone(),
            current.canvas().nodes().to_vec(),
            current.canvas().edges().to_vec(),
            state,
        )
        .map_err(|_| CanvasLifecycleUsecaseError::InvalidState)?;
        let next = current.next(canvas).map_err(map_repository)?;
        repository
            .replace_canvas(&workspace, expected, next.clone())
            .map_err(map_repository)?;
        logger.write_product(CanvasLifecycleProductEvent::Archived {
            canvas_id: canvas_id.as_str().into(),
            revision: next.revision().value(),
        });
        Ok(CanvasRecordOutput { record: next })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasLifecycleUsecaseError {
    InvalidInput,
    NotFound,
    AlreadyExists,
    VersionConflict,
    InvalidState,
    StorageUnavailable,
    RecoveryRequired,
}
impl CanvasLifecycleUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "CANVAS_INVALID_INPUT",
            Self::NotFound => "CANVAS_NOT_FOUND",
            Self::AlreadyExists => "CANVAS_ALREADY_EXISTS",
            Self::VersionConflict => "CANVAS_VERSION_CONFLICT",
            Self::InvalidState => "CANVAS_INVALID_STATE",
            Self::StorageUnavailable => "CANVAS_STORAGE_UNAVAILABLE",
            Self::RecoveryRequired => "CANVAS_RECOVERY_REQUIRED",
        }
    }
}

fn workspace(value: &str) -> Result<WorkspaceId, CanvasLifecycleUsecaseError> {
    WorkspaceId::new(value).map_err(|_| CanvasLifecycleUsecaseError::InvalidInput)
}
fn canvas_id(value: &str) -> Result<CanvasId, CanvasLifecycleUsecaseError> {
    CanvasId::new(value).map_err(|_| CanvasLifecycleUsecaseError::InvalidInput)
}
fn revision(value: u64) -> Result<CanvasRevision, CanvasLifecycleUsecaseError> {
    CanvasRevision::new(value).map_err(|_| CanvasLifecycleUsecaseError::InvalidInput)
}
fn map_repository(error: CanvasRepositoryError) -> CanvasLifecycleUsecaseError {
    match error {
        CanvasRepositoryError::InvalidInput => CanvasLifecycleUsecaseError::InvalidInput,
        CanvasRepositoryError::StorageUnavailable => {
            CanvasLifecycleUsecaseError::StorageUnavailable
        }
        CanvasRepositoryError::CorruptedCanvas | CanvasRepositoryError::UnsupportedSchema => {
            CanvasLifecycleUsecaseError::RecoveryRequired
        }
        CanvasRepositoryError::AlreadyExists => CanvasLifecycleUsecaseError::AlreadyExists,
        CanvasRepositoryError::VersionConflict => CanvasLifecycleUsecaseError::VersionConflict,
    }
}
