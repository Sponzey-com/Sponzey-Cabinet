use cabinet_domain::canvas::{CanvasId, CanvasRevision};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_recovery::{CanvasRecoveryRepository, CanvasRecoveryRepositoryError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoverCanvasInput {
    workspace_id: String,
    canvas_id: String,
}

impl RecoverCanvasInput {
    pub fn new(workspace_id: &str, canvas_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            canvas_id: canvas_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoverCanvasOutput {
    revision: CanvasRevision,
}

impl RecoverCanvasOutput {
    pub const fn revision(&self) -> CanvasRevision {
        self.revision
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasRecoveryEvent {
    event_name: &'static str,
    canvas_id: String,
    revision: Option<u64>,
    error_code: Option<&'static str>,
}

impl CanvasRecoveryEvent {
    pub const fn event_name(&self) -> &'static str {
        self.event_name
    }
    pub fn canvas_id(&self) -> &str {
        &self.canvas_id
    }
    pub const fn revision(&self) -> Option<u64> {
        self.revision
    }
    pub const fn error_code(&self) -> Option<&'static str> {
        self.error_code
    }
}

pub trait CanvasRecoveryLogger {
    fn write_product(&mut self, event: CanvasRecoveryEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasRecoveryError {
    InvalidInput,
    NoValidRevision,
    StorageUnavailable,
    CorruptedCatalog,
    CandidateLimitExceeded,
}

impl CanvasRecoveryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "CANVAS_RECOVERY_INVALID_INPUT",
            Self::NoValidRevision => "CANVAS_RECOVERY_NO_VALID_REVISION",
            Self::StorageUnavailable => "CANVAS_RECOVERY_STORAGE_UNAVAILABLE",
            Self::CorruptedCatalog => "CANVAS_RECOVERY_CATALOG_CORRUPTED",
            Self::CandidateLimitExceeded => "CANVAS_RECOVERY_CANDIDATE_LIMIT_EXCEEDED",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RecoverCanvasUsecase {
    candidate_limit: usize,
}

impl RecoverCanvasUsecase {
    pub const fn new(candidate_limit: usize) -> Result<Self, CanvasRecoveryError> {
        if candidate_limit == 0 {
            return Err(CanvasRecoveryError::InvalidInput);
        }
        Ok(Self { candidate_limit })
    }

    pub fn execute(
        &self,
        input: RecoverCanvasInput,
        repository: &mut impl CanvasRecoveryRepository,
        logger: &mut impl CanvasRecoveryLogger,
    ) -> Result<RecoverCanvasOutput, CanvasRecoveryError> {
        let workspace =
            WorkspaceId::new(&input.workspace_id).map_err(|_| CanvasRecoveryError::InvalidInput)?;
        let canvas =
            CanvasId::new(&input.canvas_id).map_err(|_| CanvasRecoveryError::InvalidInput)?;
        let candidates =
            match repository.list_valid_revisions(&workspace, &canvas, self.candidate_limit) {
                Ok(candidates) => candidates,
                Err(error) => {
                    let error = map_repository(error);
                    log_failure(logger, &canvas, error);
                    return Err(error);
                }
            };
        let revision = match candidates
            .into_iter()
            .max_by_key(|revision| revision.value())
        {
            Some(revision) => revision,
            None => {
                let error = CanvasRecoveryError::NoValidRevision;
                log_failure(logger, &canvas, error);
                return Err(error);
            }
        };
        if let Err(error) = repository.activate_revision(&workspace, &canvas, revision) {
            let error = map_repository(error);
            log_failure(logger, &canvas, error);
            return Err(error);
        }
        logger.write_product(CanvasRecoveryEvent {
            event_name: "canvas.recovery.completed",
            canvas_id: canvas.as_str().into(),
            revision: Some(revision.value()),
            error_code: None,
        });
        Ok(RecoverCanvasOutput { revision })
    }
}

fn log_failure(
    logger: &mut impl CanvasRecoveryLogger,
    canvas: &CanvasId,
    error: CanvasRecoveryError,
) {
    logger.write_product(CanvasRecoveryEvent {
        event_name: "canvas.recovery.failed",
        canvas_id: canvas.as_str().into(),
        revision: None,
        error_code: Some(error.code()),
    });
}

fn map_repository(error: CanvasRecoveryRepositoryError) -> CanvasRecoveryError {
    match error {
        CanvasRecoveryRepositoryError::InvalidInput => CanvasRecoveryError::InvalidInput,
        CanvasRecoveryRepositoryError::StorageUnavailable => {
            CanvasRecoveryError::StorageUnavailable
        }
        CanvasRecoveryRepositoryError::CorruptedCatalog => CanvasRecoveryError::CorruptedCatalog,
        CanvasRecoveryRepositoryError::CandidateLimitExceeded => {
            CanvasRecoveryError::CandidateLimitExceeded
        }
        CanvasRecoveryRepositoryError::RevisionNotFound => CanvasRecoveryError::NoValidRevision,
    }
}
