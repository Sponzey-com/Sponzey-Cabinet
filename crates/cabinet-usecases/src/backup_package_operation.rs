use cabinet_domain::backup::{
    BackupJobEvent, BackupJobId, BackupJobOperation, BackupJobRetryPolicy, BackupJobSnapshot,
    BackupJobState, BackupJobStateMachine, BackupProgress,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_package::{BackupPackageStore, BackupPackageStoreError};
use cabinet_ports::backup_store::{BackupStore, BackupStoreError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartBackupPackageOperationInput {
    actor_user_id: String,
    workspace_id: String,
    operation_id: String,
}

impl StartBackupPackageOperationInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, operation_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.into(),
            workspace_id: workspace_id.into(),
            operation_id: operation_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunBackupPackageOperationInput {
    actor_user_id: String,
    workspace_id: String,
    operation_id: String,
}

impl RunBackupPackageOperationInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, operation_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.into(),
            workspace_id: workspace_id.into(),
            operation_id: operation_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetBackupPackageOperationInput {
    actor_user_id: String,
    workspace_id: String,
    operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelBackupPackageOperationInput {
    actor_user_id: String,
    workspace_id: String,
    operation_id: String,
}

impl CancelBackupPackageOperationInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, operation_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.into(),
            workspace_id: workspace_id.into(),
            operation_id: operation_id.into(),
        }
    }
}

impl GetBackupPackageOperationInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, operation_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.into(),
            workspace_id: workspace_id.into(),
            operation_id: operation_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPackageOperationOutput {
    operation_id: String,
    state: BackupJobState,
    progress_completed_units: u64,
    progress_total_units: u64,
    error_code: Option<&'static str>,
}

impl BackupPackageOperationOutput {
    fn from_job(job: &BackupJobSnapshot) -> Self {
        Self {
            operation_id: job.job_id().as_str().into(),
            state: job.state(),
            progress_completed_units: job.progress().completed_units(),
            progress_total_units: job.progress().total_units(),
            error_code: job.error_code(),
        }
    }
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
    pub const fn state(&self) -> BackupJobState {
        self.state
    }
    pub const fn progress_completed_units(&self) -> u64 {
        self.progress_completed_units
    }
    pub const fn progress_total_units(&self) -> u64 {
        self.progress_total_units
    }
    pub const fn error_code(&self) -> Option<&'static str> {
        self.error_code
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupPackageOperationUsecaseError {
    InvalidInput,
    NotFound,
    Conflict,
    StorageUnavailable,
}

impl BackupPackageOperationUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "BACKUP_OPERATION_INVALID_INPUT",
            Self::NotFound => "BACKUP_OPERATION_NOT_FOUND",
            Self::Conflict => "BACKUP_OPERATION_CONFLICT",
            Self::StorageUnavailable => "BACKUP_OPERATION_STORAGE_UNAVAILABLE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPackageOperationEvent {
    event_name: &'static str,
    state: BackupJobState,
    error_code: Option<&'static str>,
}

impl BackupPackageOperationEvent {
    pub const fn event_name(&self) -> &'static str {
        self.event_name
    }
    pub const fn state(&self) -> BackupJobState {
        self.state
    }
    pub const fn error_code(&self) -> Option<&'static str> {
        self.error_code
    }
}

pub trait BackupPackageOperationLogger {
    fn write_product(&mut self, event: BackupPackageOperationEvent);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StartBackupPackageOperationUsecase;

impl StartBackupPackageOperationUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute(
        &self,
        input: StartBackupPackageOperationInput,
        jobs: &mut impl BackupStore,
        logger: &mut impl BackupPackageOperationLogger,
    ) -> Result<BackupPackageOperationOutput, BackupPackageOperationUsecaseError> {
        let parsed = parse(input.actor_user_id, input.workspace_id, input.operation_id)?;
        if jobs
            .get_job(&parsed.workspace, &parsed.operation)
            .map_err(map_job_error)?
            .is_some()
        {
            return Err(BackupPackageOperationUsecaseError::Conflict);
        }
        let job = BackupJobSnapshot::new(
            parsed.operation,
            parsed.workspace,
            BackupJobOperation::Backup,
            BackupJobState::Queued,
            0,
            BackupProgress::queued(),
            None,
        )
        .map_err(|_| BackupPackageOperationUsecaseError::InvalidInput)?;
        jobs.save_job(job.clone()).map_err(map_job_error)?;
        logger.write_product(event("backup.operation.queued", &job));
        Ok(BackupPackageOperationOutput::from_job(&job))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RunBackupPackageOperationUsecase {
    retry_policy: BackupJobRetryPolicy,
}

impl RunBackupPackageOperationUsecase {
    pub fn new() -> Self {
        Self {
            retry_policy: BackupJobRetryPolicy::new(2).expect("fixed operation retry policy"),
        }
    }

    pub fn execute(
        &self,
        input: RunBackupPackageOperationInput,
        jobs: &mut impl BackupStore,
        packages: &mut impl BackupPackageStore,
        logger: &mut impl BackupPackageOperationLogger,
    ) -> Result<BackupPackageOperationOutput, BackupPackageOperationUsecaseError> {
        let parsed = parse(input.actor_user_id, input.workspace_id, input.operation_id)?;
        let current = jobs
            .get_job(&parsed.workspace, &parsed.operation)
            .map_err(map_job_error)?
            .ok_or(BackupPackageOperationUsecaseError::NotFound)?;
        if current.operation() != BackupJobOperation::Backup {
            return Err(BackupPackageOperationUsecaseError::Conflict);
        }
        if current.state().is_terminal() {
            return Ok(BackupPackageOperationOutput::from_job(&current));
        }
        let running = match current.state() {
            BackupJobState::Queued | BackupJobState::Retrying => BackupJobStateMachine::transition(
                &current,
                BackupJobEvent::Start,
                self.retry_policy,
            )
            .map_err(|_| BackupPackageOperationUsecaseError::Conflict)?
            .job()
            .clone(),
            BackupJobState::Running => current,
            _ => return Err(BackupPackageOperationUsecaseError::Conflict),
        };
        jobs.save_job(running.clone()).map_err(map_job_error)?;
        logger.write_product(event("backup.operation.running", &running));
        match packages.build_package(&parsed.workspace, &parsed.operation) {
            Ok(_) => {
                let latest = jobs
                    .get_job(&parsed.workspace, &parsed.operation)
                    .map_err(map_job_error)?
                    .ok_or(BackupPackageOperationUsecaseError::NotFound)?;
                if latest.state() == BackupJobState::Abandoned {
                    if let Err(error) =
                        packages.discard_package(&parsed.workspace, &parsed.operation)
                    {
                        let error_code = package_cleanup_failure(error);
                        logger.write_product(event_with_error(
                            "backup.package.discard_failed",
                            &latest,
                            error_code,
                        ));
                        return Err(BackupPackageOperationUsecaseError::StorageUnavailable);
                    }
                    logger.write_product(event("backup.package.discarded", &latest));
                    return Ok(BackupPackageOperationOutput::from_job(&latest));
                }
                let completed = BackupJobStateMachine::transition(
                    &running,
                    BackupJobEvent::Complete,
                    self.retry_policy,
                )
                .map_err(|_| BackupPackageOperationUsecaseError::Conflict)?
                .job()
                .clone();
                jobs.save_job(completed.clone()).map_err(map_job_error)?;
                logger.write_product(event("backup.operation.completed", &completed));
                Ok(BackupPackageOperationOutput::from_job(&completed))
            }
            Err(error) => {
                let (transition_event, error_code) = package_failure(error);
                let failed = BackupJobStateMachine::transition(
                    &running,
                    transition_event,
                    self.retry_policy,
                )
                .map_err(|_| BackupPackageOperationUsecaseError::Conflict)?
                .job()
                .clone()
                .with_error_code(error_code);
                jobs.save_job(failed.clone()).map_err(map_job_error)?;
                logger.write_product(event("backup.operation.failed", &failed));
                Ok(BackupPackageOperationOutput::from_job(&failed))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CancelBackupPackageOperationUsecase;

impl CancelBackupPackageOperationUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: CancelBackupPackageOperationInput,
        jobs: &mut impl BackupStore,
        logger: &mut impl BackupPackageOperationLogger,
    ) -> Result<BackupPackageOperationOutput, BackupPackageOperationUsecaseError> {
        let parsed = parse(input.actor_user_id, input.workspace_id, input.operation_id)?;
        let current = jobs
            .get_job(&parsed.workspace, &parsed.operation)
            .map_err(map_job_error)?
            .ok_or(BackupPackageOperationUsecaseError::NotFound)?;
        if current.operation() != BackupJobOperation::Backup {
            return Err(BackupPackageOperationUsecaseError::Conflict);
        }
        if current.state().is_terminal() {
            return Ok(BackupPackageOperationOutput::from_job(&current));
        }
        let cancelled = BackupJobStateMachine::transition(
            &current,
            BackupJobEvent::Abandon,
            BackupJobRetryPolicy::new(2).expect("fixed operation retry policy"),
        )
        .map_err(|_| BackupPackageOperationUsecaseError::Conflict)?
        .job()
        .clone()
        .with_error_code("BACKUP_OPERATION_CANCELLED");
        jobs.save_job(cancelled.clone()).map_err(map_job_error)?;
        logger.write_product(event("backup.operation.cancelled", &cancelled));
        Ok(BackupPackageOperationOutput::from_job(&cancelled))
    }
}

impl Default for RunBackupPackageOperationUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GetBackupPackageOperationUsecase;

impl GetBackupPackageOperationUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute(
        &self,
        input: GetBackupPackageOperationInput,
        jobs: &impl BackupStore,
    ) -> Result<BackupPackageOperationOutput, BackupPackageOperationUsecaseError> {
        let parsed = parse(input.actor_user_id, input.workspace_id, input.operation_id)?;
        let job = jobs
            .get_job(&parsed.workspace, &parsed.operation)
            .map_err(map_job_error)?
            .ok_or(BackupPackageOperationUsecaseError::NotFound)?;
        if job.operation() != BackupJobOperation::Backup {
            return Err(BackupPackageOperationUsecaseError::Conflict);
        }
        Ok(BackupPackageOperationOutput::from_job(&job))
    }
}

struct ParsedInput {
    workspace: WorkspaceId,
    operation: BackupJobId,
}

fn parse(
    actor: String,
    workspace: String,
    operation: String,
) -> Result<ParsedInput, BackupPackageOperationUsecaseError> {
    UserId::new(&actor).map_err(|_| BackupPackageOperationUsecaseError::InvalidInput)?;
    Ok(ParsedInput {
        workspace: WorkspaceId::new(&workspace)
            .map_err(|_| BackupPackageOperationUsecaseError::InvalidInput)?,
        operation: BackupJobId::new(&operation)
            .map_err(|_| BackupPackageOperationUsecaseError::InvalidInput)?,
    })
}

fn map_job_error(error: BackupStoreError) -> BackupPackageOperationUsecaseError {
    match error {
        BackupStoreError::Conflict => BackupPackageOperationUsecaseError::Conflict,
        BackupStoreError::StorageUnavailable
        | BackupStoreError::CorruptedArtifact
        | BackupStoreError::MissingJob => BackupPackageOperationUsecaseError::StorageUnavailable,
    }
}

fn package_failure(error: BackupPackageStoreError) -> (BackupJobEvent, &'static str) {
    match error {
        BackupPackageStoreError::StorageUnavailable => (
            BackupJobEvent::FailRetryable,
            "BACKUP_PACKAGE_STORAGE_UNAVAILABLE",
        ),
        BackupPackageStoreError::Conflict => (BackupJobEvent::FailFatal, "BACKUP_PACKAGE_CONFLICT"),
        BackupPackageStoreError::PackageNotFound => {
            (BackupJobEvent::FailFatal, "BACKUP_PACKAGE_NOT_FOUND")
        }
        BackupPackageStoreError::CorruptedPackage => {
            (BackupJobEvent::FailFatal, "BACKUP_PACKAGE_CORRUPTED")
        }
    }
}

fn package_cleanup_failure(error: BackupPackageStoreError) -> &'static str {
    match error {
        BackupPackageStoreError::StorageUnavailable => "BACKUP_PACKAGE_DISCARD_STORAGE_UNAVAILABLE",
        BackupPackageStoreError::PackageNotFound => "BACKUP_PACKAGE_DISCARD_NOT_FOUND",
        BackupPackageStoreError::CorruptedPackage => "BACKUP_PACKAGE_DISCARD_CORRUPTED",
        BackupPackageStoreError::Conflict => "BACKUP_PACKAGE_DISCARD_CONFLICT",
    }
}

fn event(event_name: &'static str, job: &BackupJobSnapshot) -> BackupPackageOperationEvent {
    BackupPackageOperationEvent {
        event_name,
        state: job.state(),
        error_code: job.error_code(),
    }
}

fn event_with_error(
    event_name: &'static str,
    job: &BackupJobSnapshot,
    error_code: &'static str,
) -> BackupPackageOperationEvent {
    BackupPackageOperationEvent {
        event_name,
        state: job.state(),
        error_code: Some(error_code),
    }
}
