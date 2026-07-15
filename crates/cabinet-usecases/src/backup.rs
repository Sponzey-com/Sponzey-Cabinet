use cabinet_domain::backup::{
    BackupJobEvent, BackupJobId, BackupJobOperation, BackupJobRetryPolicy, BackupJobSnapshot,
    BackupJobState, BackupJobStateMachine, BackupProgress,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_store::{
    BackupAuditRecord, BackupAuditRecorder, BackupAuditRecorderError, BackupStore, BackupStoreError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackupJobPolicy {
    retry_policy: BackupJobRetryPolicy,
    retention_days: u32,
}

impl BackupJobPolicy {
    pub const fn new(
        max_retry_attempts: u16,
        retention_days: u32,
    ) -> Result<Self, BackupJobUsecaseError> {
        if retention_days == 0 {
            return Err(BackupJobUsecaseError::InvalidInput);
        }
        let retry_policy = match BackupJobRetryPolicy::new(max_retry_attempts) {
            Ok(policy) => policy,
            Err(_) => return Err(BackupJobUsecaseError::InvalidInput),
        };
        Ok(Self {
            retry_policy,
            retention_days,
        })
    }

    pub const fn retry_policy(self) -> BackupJobRetryPolicy {
        self.retry_policy
    }

    pub const fn retention_days(self) -> u32 {
        self.retention_days
    }
}

impl Default for BackupJobPolicy {
    fn default() -> Self {
        Self::new(2, 30).expect("default backup policy is valid")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBackupInput {
    actor_user_id: String,
    workspace_id: String,
    job_id: String,
}

impl CreateBackupInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, job_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            job_id: job_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreBackupInput {
    actor_user_id: String,
    workspace_id: String,
    source_backup_job_id: String,
    restore_job_id: String,
}

impl RestoreBackupInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        source_backup_job_id: &str,
        restore_job_id: &str,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            source_backup_job_id: source_backup_job_id.to_string(),
            restore_job_id: restore_job_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetBackupStatusInput {
    actor_user_id: String,
    workspace_id: String,
    job_id: String,
}

impl GetBackupStatusInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, job_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            job_id: job_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportWorkspaceInput {
    actor_user_id: String,
    workspace_id: String,
    job_id: String,
}

impl ExportWorkspaceInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, job_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            job_id: job_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetExportStatusInput {
    actor_user_id: String,
    workspace_id: String,
    job_id: String,
}

impl GetExportStatusInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, job_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            job_id: job_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupJobOutput {
    job_id: String,
    workspace_id: String,
    operation: BackupJobOperation,
    state: BackupJobState,
    retry_count: u16,
    progress_completed_units: u64,
    progress_total_units: u64,
    error_code: Option<&'static str>,
}

impl BackupJobOutput {
    fn from_snapshot(snapshot: &BackupJobSnapshot) -> Self {
        Self {
            job_id: snapshot.job_id().as_str().to_string(),
            workspace_id: snapshot.workspace_id().as_str().to_string(),
            operation: snapshot.operation(),
            state: snapshot.state(),
            retry_count: snapshot.retry_count(),
            progress_completed_units: snapshot.progress().completed_units(),
            progress_total_units: snapshot.progress().total_units(),
            error_code: snapshot.error_code(),
        }
    }

    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub const fn operation(&self) -> BackupJobOperation {
        self.operation
    }

    pub const fn state(&self) -> BackupJobState {
        self.state
    }

    pub const fn retry_count(&self) -> u16 {
        self.retry_count
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
pub enum BackupJobUsecaseError {
    InvalidInput,
    JobNotFound,
    StorageUnavailable,
    Conflict,
    AuditUnavailable,
}

impl BackupJobUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "BACKUP_JOB_INVALID_INPUT",
            Self::JobNotFound => "BACKUP_JOB_NOT_FOUND",
            Self::StorageUnavailable => "BACKUP_JOB_STORAGE_UNAVAILABLE",
            Self::Conflict => "BACKUP_JOB_CONFLICT",
            Self::AuditUnavailable => "BACKUP_JOB_AUDIT_UNAVAILABLE",
        }
    }

    const fn from_store_error(error: BackupStoreError) -> Self {
        match error {
            BackupStoreError::StorageUnavailable
            | BackupStoreError::CorruptedArtifact
            | BackupStoreError::MissingJob => Self::StorageUnavailable,
            BackupStoreError::Conflict => Self::Conflict,
        }
    }

    const fn from_audit_error(_error: BackupAuditRecorderError) -> Self {
        Self::AuditUnavailable
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupJobProductEvent {
    JobCompleted {
        event_name: &'static str,
        job_id: String,
        workspace_id: String,
        operation: &'static str,
        state: &'static str,
        duration_bucket: &'static str,
    },
    JobFailed {
        event_name: &'static str,
        job_id: String,
        workspace_id: String,
        operation: &'static str,
        state: &'static str,
        error_code: &'static str,
        duration_bucket: &'static str,
    },
}

impl BackupJobProductEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::JobCompleted { event_name, .. } | Self::JobFailed { event_name, .. } => {
                event_name
            }
        }
    }

    pub fn completed(event_name: &'static str, job: &BackupJobSnapshot) -> Self {
        Self::JobCompleted {
            event_name,
            job_id: job.job_id().as_str().to_string(),
            workspace_id: job.workspace_id().as_str().to_string(),
            operation: job.operation().as_str(),
            state: job.state().as_str(),
            duration_bucket: "not-measured",
        }
    }

    pub fn failed(
        event_name: &'static str,
        job: &BackupJobSnapshot,
        error_code: &'static str,
    ) -> Self {
        Self::JobFailed {
            event_name,
            job_id: job.job_id().as_str().to_string(),
            workspace_id: job.workspace_id().as_str().to_string(),
            operation: job.operation().as_str(),
            state: job.state().as_str(),
            error_code,
            duration_bucket: "not-measured",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupJobFieldDebugEvent {
    backend_type: String,
    retry_count: u16,
    job_state: &'static str,
    object_key_hash: Option<String>,
}

impl BackupJobFieldDebugEvent {
    pub fn new(
        backend_type: &str,
        retry_count: u16,
        job_state: BackupJobState,
        object_key_hash: Option<String>,
    ) -> Result<Self, BackupJobUsecaseError> {
        let backend_type = validate_text(backend_type)?;
        Ok(Self {
            backend_type,
            retry_count,
            job_state: job_state.as_str(),
            object_key_hash,
        })
    }

    pub fn backend_type(&self) -> &str {
        &self.backend_type
    }

    pub const fn retry_count(&self) -> u16 {
        self.retry_count
    }

    pub const fn job_state(&self) -> &'static str {
        self.job_state
    }

    pub fn object_key_hash(&self) -> Option<&str> {
        self.object_key_hash.as_deref()
    }
}

pub trait BackupJobUsecaseLogger {
    fn write_product(&mut self, event: BackupJobProductEvent);

    fn write_field_debug(&mut self, _event: BackupJobFieldDebugEvent) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateBackupUsecase {
    policy: BackupJobPolicy,
}

impl CreateBackupUsecase {
    pub fn new() -> Self {
        Self {
            policy: BackupJobPolicy::default(),
        }
    }

    pub const fn with_policy(policy: BackupJobPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: CreateBackupInput,
        store: &mut impl BackupStore,
        audit: &mut impl BackupAuditRecorder,
    ) -> Result<BackupJobOutput, BackupJobUsecaseError> {
        let parsed = parse_create_input(input)?;
        let job = queued_job(
            parsed.job_id,
            parsed.workspace_id,
            BackupJobOperation::Backup,
        )?;
        store
            .save_job(job.clone())
            .map_err(BackupJobUsecaseError::from_store_error)?;
        audit
            .record_backup_audit(audit_record(&job, "backup.created", self.policy)?)
            .map_err(BackupJobUsecaseError::from_audit_error)?;
        Ok(BackupJobOutput::from_snapshot(&job))
    }
}

impl Default for CreateBackupUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GetBackupStatusUsecase;

impl GetBackupStatusUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetBackupStatusInput,
        store: &impl BackupStore,
    ) -> Result<BackupJobOutput, BackupJobUsecaseError> {
        let parsed = parse_status_input(input.actor_user_id, input.workspace_id, input.job_id)?;
        let job = store
            .get_job(&parsed.workspace_id, &parsed.job_id)
            .map_err(BackupJobUsecaseError::from_store_error)?
            .ok_or(BackupJobUsecaseError::JobNotFound)?;
        Ok(BackupJobOutput::from_snapshot(&job))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExportWorkspaceUsecase;

impl ExportWorkspaceUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ExportWorkspaceInput,
        store: &mut impl BackupStore,
    ) -> Result<BackupJobOutput, BackupJobUsecaseError> {
        let parsed = parse_status_input(input.actor_user_id, input.workspace_id, input.job_id)?;
        let job = queued_job(
            parsed.job_id,
            parsed.workspace_id,
            BackupJobOperation::Export,
        )?;
        store
            .save_job(job.clone())
            .map_err(BackupJobUsecaseError::from_store_error)?;
        Ok(BackupJobOutput::from_snapshot(&job))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GetExportStatusUsecase;

impl GetExportStatusUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetExportStatusInput,
        store: &impl BackupStore,
    ) -> Result<BackupJobOutput, BackupJobUsecaseError> {
        let parsed = parse_status_input(input.actor_user_id, input.workspace_id, input.job_id)?;
        let job = store
            .get_job(&parsed.workspace_id, &parsed.job_id)
            .map_err(BackupJobUsecaseError::from_store_error)?
            .ok_or(BackupJobUsecaseError::JobNotFound)?;
        Ok(BackupJobOutput::from_snapshot(&job))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoreBackupUsecase {
    policy: BackupJobPolicy,
}

impl RestoreBackupUsecase {
    pub fn new() -> Self {
        Self {
            policy: BackupJobPolicy::default(),
        }
    }

    pub const fn with_policy(policy: BackupJobPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: RestoreBackupInput,
        store: &mut impl BackupStore,
        audit: &mut impl BackupAuditRecorder,
        logger: &mut impl BackupJobUsecaseLogger,
    ) -> Result<BackupJobOutput, BackupJobUsecaseError> {
        let actor_user_id =
            UserId::new(&input.actor_user_id).map_err(|_| BackupJobUsecaseError::InvalidInput)?;
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| BackupJobUsecaseError::InvalidInput)?;
        let source_job_id = BackupJobId::new(&input.source_backup_job_id)
            .map_err(|_| BackupJobUsecaseError::InvalidInput)?;
        let restore_job_id = BackupJobId::new(&input.restore_job_id)
            .map_err(|_| BackupJobUsecaseError::InvalidInput)?;
        let queued = queued_job(
            restore_job_id,
            workspace_id.clone(),
            BackupJobOperation::Restore,
        )?;
        store
            .save_job(queued.clone())
            .map_err(BackupJobUsecaseError::from_store_error)?;
        let running = BackupJobStateMachine::transition(
            &queued,
            BackupJobEvent::Start,
            self.policy.retry_policy(),
        )
        .map_err(|_| BackupJobUsecaseError::InvalidInput)?
        .job()
        .clone();
        store
            .save_job(running.clone())
            .map_err(BackupJobUsecaseError::from_store_error)?;

        let validation = store
            .validate_restore_staging(&workspace_id, &source_job_id)
            .map_err(BackupJobUsecaseError::from_store_error)?;
        if !validation.is_valid() {
            let failed = abandoned_restore_job(
                &running,
                validation
                    .error_code()
                    .unwrap_or("BACKUP_RESTORE_STAGING_INVALID"),
                self.policy,
            )?;
            store
                .save_job(failed.clone())
                .map_err(BackupJobUsecaseError::from_store_error)?;
            write_failure_side_effects(&actor_user_id, &failed, audit, logger)?;
            return Ok(BackupJobOutput::from_snapshot(&failed));
        }

        if let Err(error) = store.apply_restore_staging(&workspace_id, &source_job_id) {
            let failed = abandoned_restore_job(&running, error.code(), self.policy)?;
            store
                .save_job(failed.clone())
                .map_err(BackupJobUsecaseError::from_store_error)?;
            write_failure_side_effects(&actor_user_id, &failed, audit, logger)?;
            return Ok(BackupJobOutput::from_snapshot(&failed));
        }

        let completed = BackupJobStateMachine::transition(
            &running,
            BackupJobEvent::Complete,
            self.policy.retry_policy(),
        )
        .map_err(|_| BackupJobUsecaseError::InvalidInput)?
        .job()
        .clone();
        store
            .save_job(completed.clone())
            .map_err(BackupJobUsecaseError::from_store_error)?;
        audit
            .record_backup_audit(audit_record(&completed, "restore.completed", self.policy)?)
            .map_err(BackupJobUsecaseError::from_audit_error)?;
        logger.write_product(BackupJobProductEvent::completed(
            "restore.completed",
            &completed,
        ));
        Ok(BackupJobOutput::from_snapshot(&completed))
    }
}

impl Default for RestoreBackupUsecase {
    fn default() -> Self {
        Self::new()
    }
}

struct ParsedJobInput {
    workspace_id: WorkspaceId,
    job_id: BackupJobId,
}

struct ParsedCreateInput {
    workspace_id: WorkspaceId,
    job_id: BackupJobId,
}

fn parse_create_input(
    input: CreateBackupInput,
) -> Result<ParsedCreateInput, BackupJobUsecaseError> {
    let _actor =
        UserId::new(&input.actor_user_id).map_err(|_| BackupJobUsecaseError::InvalidInput)?;
    Ok(ParsedCreateInput {
        workspace_id: WorkspaceId::new(&input.workspace_id)
            .map_err(|_| BackupJobUsecaseError::InvalidInput)?,
        job_id: BackupJobId::new(&input.job_id).map_err(|_| BackupJobUsecaseError::InvalidInput)?,
    })
}

fn parse_status_input(
    actor_user_id: String,
    workspace_id: String,
    job_id: String,
) -> Result<ParsedJobInput, BackupJobUsecaseError> {
    let _actor = UserId::new(&actor_user_id).map_err(|_| BackupJobUsecaseError::InvalidInput)?;
    Ok(ParsedJobInput {
        workspace_id: WorkspaceId::new(&workspace_id)
            .map_err(|_| BackupJobUsecaseError::InvalidInput)?,
        job_id: BackupJobId::new(&job_id).map_err(|_| BackupJobUsecaseError::InvalidInput)?,
    })
}

fn queued_job(
    job_id: BackupJobId,
    workspace_id: WorkspaceId,
    operation: BackupJobOperation,
) -> Result<BackupJobSnapshot, BackupJobUsecaseError> {
    BackupJobSnapshot::new(
        job_id,
        workspace_id,
        operation,
        BackupJobState::Queued,
        0,
        BackupProgress::queued(),
        None,
    )
    .map_err(|_| BackupJobUsecaseError::InvalidInput)
}

fn abandoned_restore_job(
    running: &BackupJobSnapshot,
    error_code: &'static str,
    policy: BackupJobPolicy,
) -> Result<BackupJobSnapshot, BackupJobUsecaseError> {
    Ok(
        BackupJobStateMachine::transition(
            running,
            BackupJobEvent::FailFatal,
            policy.retry_policy(),
        )
        .map_err(|_| BackupJobUsecaseError::InvalidInput)?
        .job()
        .clone()
        .with_error_code(error_code),
    )
}

fn audit_record(
    job: &BackupJobSnapshot,
    event_name: &str,
    policy: BackupJobPolicy,
) -> Result<BackupAuditRecord, BackupJobUsecaseError> {
    BackupAuditRecord::new(
        job.workspace_id().clone(),
        job.job_id().clone(),
        event_name,
        vec![
            ("state".to_string(), job.state().as_str().to_string()),
            (
                "retention_days".to_string(),
                policy.retention_days().to_string(),
            ),
        ],
    )
    .map_err(BackupJobUsecaseError::from_audit_error)
}

fn write_failure_side_effects(
    _actor_user_id: &UserId,
    job: &BackupJobSnapshot,
    audit: &mut impl BackupAuditRecorder,
    logger: &mut impl BackupJobUsecaseLogger,
) -> Result<(), BackupJobUsecaseError> {
    let audit_record = BackupAuditRecord::new(
        job.workspace_id().clone(),
        job.job_id().clone(),
        "restore.failed",
        vec![("state".to_string(), job.state().as_str().to_string())],
    )
    .map_err(BackupJobUsecaseError::from_audit_error)?;
    audit
        .record_backup_audit(audit_record)
        .map_err(BackupJobUsecaseError::from_audit_error)?;
    logger.write_product(BackupJobProductEvent::failed(
        "restore.failed",
        job,
        job.error_code().unwrap_or("BACKUP_JOB_FAILED"),
    ));
    Ok(())
}

fn validate_text(value: &str) -> Result<String, BackupJobUsecaseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(BackupJobUsecaseError::InvalidInput);
    }
    Ok(trimmed.to_string())
}
