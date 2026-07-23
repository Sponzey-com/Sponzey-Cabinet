use crate::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackupJobId {
    value: String,
}

impl BackupJobId {
    pub fn new(value: &str) -> Result<Self, BackupJobError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(BackupJobError::InvalidJobId);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(BackupJobError::InvalidJobId);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupJobOperation {
    Backup,
    Restore,
    Export,
}

impl BackupJobOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Backup => "backup",
            Self::Restore => "restore",
            Self::Export => "export",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupJobState {
    Queued,
    Running,
    Completed,
    Failed,
    Retrying,
    Abandoned,
}

impl BackupJobState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Retrying => "retrying",
            Self::Abandoned => "abandoned",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Abandoned)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupJobEvent {
    Start,
    Complete,
    FailRetryable,
    FailFatal,
    Retry,
    Abandon,
}

impl BackupJobEvent {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Complete => "complete",
            Self::FailRetryable => "fail_retryable",
            Self::FailFatal => "fail_fatal",
            Self::Retry => "retry",
            Self::Abandon => "abandon",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackupJobRetryPolicy {
    max_attempts: u16,
}

impl BackupJobRetryPolicy {
    pub const fn new(max_attempts: u16) -> Result<Self, BackupJobError> {
        if max_attempts == 0 {
            return Err(BackupJobError::InvalidRetryPolicy);
        }
        Ok(Self { max_attempts })
    }

    pub const fn max_attempts(self) -> u16 {
        self.max_attempts
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackupProgress {
    completed_units: u64,
    total_units: u64,
}

impl BackupProgress {
    pub const fn new(completed_units: u64, total_units: u64) -> Result<Self, BackupJobError> {
        if total_units == 0 || completed_units > total_units {
            return Err(BackupJobError::InvalidProgress);
        }
        Ok(Self {
            completed_units,
            total_units,
        })
    }

    pub const fn queued() -> Self {
        Self {
            completed_units: 0,
            total_units: 1,
        }
    }

    pub const fn completed_units(self) -> u64 {
        self.completed_units
    }

    pub const fn total_units(self) -> u64 {
        self.total_units
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupArtifactManifest {
    artifact_id: String,
    object_count: u64,
}

impl BackupArtifactManifest {
    pub fn new(artifact_id: &str, object_count: u64) -> Result<Self, BackupJobError> {
        let artifact_id = validate_required_text(artifact_id)?;
        if object_count == 0 {
            return Err(BackupJobError::InvalidArtifactManifest);
        }
        Ok(Self {
            artifact_id,
            object_count,
        })
    }

    pub fn artifact_id(&self) -> &str {
        &self.artifact_id
    }

    pub const fn object_count(&self) -> u64 {
        self.object_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupJobSnapshot {
    job_id: BackupJobId,
    workspace_id: WorkspaceId,
    operation: BackupJobOperation,
    state: BackupJobState,
    retry_count: u16,
    progress: BackupProgress,
    artifact_manifest: Option<BackupArtifactManifest>,
    error_code: Option<&'static str>,
}

impl BackupJobSnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        job_id: BackupJobId,
        workspace_id: WorkspaceId,
        operation: BackupJobOperation,
        state: BackupJobState,
        retry_count: u16,
        progress: BackupProgress,
        artifact_manifest: Option<BackupArtifactManifest>,
    ) -> Result<Self, BackupJobError> {
        Ok(Self {
            job_id,
            workspace_id,
            operation,
            state,
            retry_count,
            progress,
            artifact_manifest,
            error_code: None,
        })
    }

    pub fn with_error_code(mut self, error_code: &'static str) -> Self {
        self.error_code = Some(error_code);
        self
    }

    pub fn job_id(&self) -> &BackupJobId {
        &self.job_id
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
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

    pub const fn progress(&self) -> BackupProgress {
        self.progress
    }

    pub fn artifact_manifest(&self) -> Option<&BackupArtifactManifest> {
        self.artifact_manifest.as_ref()
    }

    pub const fn error_code(&self) -> Option<&'static str> {
        self.error_code
    }

    fn transition_to(
        &self,
        state: BackupJobState,
        retry_count: u16,
        progress: BackupProgress,
        error_code: Option<&'static str>,
    ) -> Self {
        Self {
            job_id: self.job_id.clone(),
            workspace_id: self.workspace_id.clone(),
            operation: self.operation,
            state,
            retry_count,
            progress,
            artifact_manifest: self.artifact_manifest.clone(),
            error_code,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupJobTransition {
    previous_state: BackupJobState,
    event: BackupJobEvent,
    next: BackupJobSnapshot,
    product_log_event_name: Option<&'static str>,
}

impl BackupJobTransition {
    pub const fn previous_state(&self) -> BackupJobState {
        self.previous_state
    }

    pub const fn event(&self) -> BackupJobEvent {
        self.event
    }

    pub const fn next_state(&self) -> BackupJobState {
        self.next.state()
    }

    pub const fn job(&self) -> &BackupJobSnapshot {
        &self.next
    }

    pub const fn product_log_event_name(&self) -> Option<&'static str> {
        self.product_log_event_name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupJobTransitionError {
    previous_state: BackupJobState,
    event: BackupJobEvent,
    code: &'static str,
}

impl BackupJobTransitionError {
    pub const fn previous_state(&self) -> BackupJobState {
        self.previous_state
    }

    pub const fn event(&self) -> BackupJobEvent {
        self.event
    }

    pub const fn code(&self) -> &'static str {
        self.code
    }
}

pub struct BackupJobStateMachine;

impl BackupJobStateMachine {
    pub fn transition(
        job: &BackupJobSnapshot,
        event: BackupJobEvent,
        policy: BackupJobRetryPolicy,
    ) -> Result<BackupJobTransition, BackupJobTransitionError> {
        if job.state().is_terminal() {
            return Err(transition_error(job, event));
        }

        let transition = match (job.state(), event) {
            (BackupJobState::Queued | BackupJobState::Retrying, BackupJobEvent::Start) => {
                transition(job, event, BackupJobState::Running, job.retry_count(), None)
            }
            (BackupJobState::Running, BackupJobEvent::Complete) => success_transition(
                job,
                event,
                BackupJobState::Completed,
                job.retry_count(),
                None,
                Some(product_event_name(job.operation(), true)),
            ),
            (BackupJobState::Running, BackupJobEvent::FailRetryable) => {
                if job.retry_count() >= policy.max_attempts() {
                    success_transition(
                        job,
                        event,
                        BackupJobState::Abandoned,
                        job.retry_count(),
                        Some("BACKUP_JOB_RETRY_EXHAUSTED"),
                        Some(product_event_name(job.operation(), false)),
                    )
                } else {
                    success_transition(
                        job,
                        event,
                        BackupJobState::Failed,
                        job.retry_count(),
                        Some("BACKUP_JOB_RETRYABLE_FAILURE"),
                        Some(product_event_name(job.operation(), false)),
                    )
                }
            }
            (BackupJobState::Running, BackupJobEvent::FailFatal) => success_transition(
                job,
                event,
                BackupJobState::Abandoned,
                job.retry_count(),
                Some("BACKUP_JOB_FATAL_FAILURE"),
                Some(product_event_name(job.operation(), false)),
            ),
            (BackupJobState::Failed, BackupJobEvent::Retry)
                if job.retry_count() < policy.max_attempts() =>
            {
                success_transition(
                    job,
                    event,
                    BackupJobState::Retrying,
                    job.retry_count() + 1,
                    None,
                    None,
                )
            }
            (
                BackupJobState::Failed
                | BackupJobState::Retrying
                | BackupJobState::Queued
                | BackupJobState::Running,
                BackupJobEvent::Abandon,
            ) => success_transition(
                job,
                event,
                BackupJobState::Abandoned,
                job.retry_count(),
                Some("BACKUP_JOB_ABANDONED"),
                Some(product_event_name(job.operation(), false)),
            ),
            _ => return Err(transition_error(job, event)),
        };

        Ok(transition)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BackupDataClass {
    CurrentDocuments,
    VersionHistory,
    CanvasRecords,
    AssetMetadata,
    AssetObjects,
    AssetAssociations,
    GraphRebuildMetadata,
    SearchRebuildMetadata,
}

impl BackupDataClass {
    pub const ALL: [Self; 8] = [
        Self::CurrentDocuments,
        Self::VersionHistory,
        Self::CanvasRecords,
        Self::AssetMetadata,
        Self::AssetObjects,
        Self::AssetAssociations,
        Self::GraphRebuildMetadata,
        Self::SearchRebuildMetadata,
    ];

    pub const fn expected_ownership(self) -> BackupDataOwnership {
        match self {
            Self::CurrentDocuments
            | Self::VersionHistory
            | Self::CanvasRecords
            | Self::AssetMetadata
            | Self::AssetObjects
            | Self::AssetAssociations => BackupDataOwnership::Authoritative,
            Self::GraphRebuildMetadata | Self::SearchRebuildMetadata => {
                BackupDataOwnership::Rebuildable
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupDataOwnership {
    Authoritative,
    Rebuildable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupManifestEntry {
    data_class: BackupDataClass,
    ownership: BackupDataOwnership,
    record_count: u64,
    byte_count: u64,
    checksum_sha256: String,
}

impl BackupManifestEntry {
    pub fn new(
        data_class: BackupDataClass,
        ownership: BackupDataOwnership,
        record_count: u64,
        byte_count: u64,
        checksum_sha256: &str,
    ) -> Result<Self, BackupPackageError> {
        if ownership != data_class.expected_ownership() {
            return Err(BackupPackageError::InvalidOwnership);
        }
        let checksum_sha256 = checksum_sha256.trim();
        if checksum_sha256.len() != 64
            || !checksum_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(BackupPackageError::InvalidChecksum);
        }
        Ok(Self {
            data_class,
            ownership,
            record_count,
            byte_count,
            checksum_sha256: checksum_sha256.to_ascii_lowercase(),
        })
    }

    pub const fn data_class(&self) -> BackupDataClass {
        self.data_class
    }

    pub const fn ownership(&self) -> BackupDataOwnership {
        self.ownership
    }

    pub const fn record_count(&self) -> u64 {
        self.record_count
    }

    pub const fn byte_count(&self) -> u64 {
        self.byte_count
    }

    pub fn checksum_sha256(&self) -> &str {
        &self.checksum_sha256
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPackageManifest {
    schema_version: u16,
    entries: Vec<BackupManifestEntry>,
    created_at_epoch_ms: Option<u64>,
}

impl BackupPackageManifest {
    pub fn new(
        schema_version: u16,
        mut entries: Vec<BackupManifestEntry>,
    ) -> Result<Self, BackupPackageError> {
        if schema_version != 1 {
            return Err(BackupPackageError::UnsupportedSchemaVersion);
        }
        entries.sort_by_key(BackupManifestEntry::data_class);
        for pair in entries.windows(2) {
            if pair[0].data_class() == pair[1].data_class() {
                return Err(BackupPackageError::DuplicateDataClass(pair[0].data_class()));
            }
        }
        for data_class in BackupDataClass::ALL {
            if !entries.iter().any(|entry| entry.data_class() == data_class) {
                return Err(BackupPackageError::MissingDataClass(data_class));
            }
        }
        Ok(Self {
            schema_version,
            entries,
            created_at_epoch_ms: None,
        })
    }

    pub fn with_created_at_epoch_ms(
        mut self,
        created_at_epoch_ms: u64,
    ) -> Result<Self, BackupPackageError> {
        if created_at_epoch_ms == 0 {
            return Err(BackupPackageError::InvalidCreatedAt);
        }
        self.created_at_epoch_ms = Some(created_at_epoch_ms);
        Ok(self)
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub fn entries(&self) -> &[BackupManifestEntry] {
        &self.entries
    }

    pub const fn created_at_epoch_ms(&self) -> Option<u64> {
        self.created_at_epoch_ms
    }

    pub fn entry(&self, data_class: BackupDataClass) -> Option<&BackupManifestEntry> {
        self.entries
            .iter()
            .find(|entry| entry.data_class() == data_class)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupPackageError {
    UnsupportedSchemaVersion,
    MissingDataClass(BackupDataClass),
    DuplicateDataClass(BackupDataClass),
    InvalidChecksum,
    InvalidOwnership,
    InvalidCreatedAt,
}

impl BackupPackageError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "BACKUP_PACKAGE_UNSUPPORTED_SCHEMA",
            Self::MissingDataClass(_) => "BACKUP_PACKAGE_DATA_CLASS_MISSING",
            Self::DuplicateDataClass(_) => "BACKUP_PACKAGE_DATA_CLASS_DUPLICATE",
            Self::InvalidChecksum => "BACKUP_PACKAGE_CHECKSUM_INVALID",
            Self::InvalidOwnership => "BACKUP_PACKAGE_OWNERSHIP_INVALID",
            Self::InvalidCreatedAt => "BACKUP_PACKAGE_CREATED_AT_INVALID",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreState {
    Requested,
    Previewing,
    Validating,
    AwaitingConfirmation,
    Staging,
    Applying,
    Reopening,
    CleanupRequired,
    RollbackRequired,
    RecoveryRequired,
    Completed,
    Failed,
    Cancelled,
    RolledBack,
}

impl RestoreState {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::RolledBack
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreEvent {
    StartPreview,
    PreviewBuilt,
    PreviewFailed,
    ValidationPassed,
    ValidationFailed,
    Confirm,
    Cancel,
    StageCompleted,
    StageFailed,
    ApplyCompleted,
    ApplyFailed,
    ReopenCompleted,
    ReopenFailed,
    CleanupCompleted,
    RollbackCompleted,
    RollbackFailed,
    RecoveryRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreSideEffectRequest {
    BuildPreview,
    ValidatePackage,
    StagePackage,
    ApplyAtomically,
    ReopenWorkspace,
    CleanupStaging,
    RollbackApply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoreTransition {
    previous_state: RestoreState,
    event: RestoreEvent,
    next_state: RestoreState,
    side_effect_request: Option<RestoreSideEffectRequest>,
    product_log_event_name: Option<&'static str>,
}

impl RestoreTransition {
    pub const fn previous_state(&self) -> RestoreState {
        self.previous_state
    }

    pub const fn event(&self) -> RestoreEvent {
        self.event
    }

    pub const fn next_state(&self) -> RestoreState {
        self.next_state
    }

    pub const fn side_effect_request(&self) -> Option<RestoreSideEffectRequest> {
        self.side_effect_request
    }

    pub const fn product_log_event_name(&self) -> Option<&'static str> {
        self.product_log_event_name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoreTransitionError {
    previous_state: RestoreState,
    event: RestoreEvent,
}

impl RestoreTransitionError {
    pub const fn previous_state(&self) -> RestoreState {
        self.previous_state
    }

    pub const fn event(&self) -> RestoreEvent {
        self.event
    }

    pub const fn code(&self) -> &'static str {
        "RESTORE_INVALID_TRANSITION"
    }
}

pub struct RestoreWorkflowStateMachine;

impl RestoreWorkflowStateMachine {
    pub fn transition(
        state: RestoreState,
        event: RestoreEvent,
    ) -> Result<RestoreTransition, RestoreTransitionError> {
        if state.is_terminal() {
            return Err(RestoreTransitionError {
                previous_state: state,
                event,
            });
        }
        let (next_state, side_effect_request, product_log_event_name) = match (state, event) {
            (RestoreState::Requested, RestoreEvent::StartPreview) => (
                RestoreState::Previewing,
                Some(RestoreSideEffectRequest::BuildPreview),
                Some("restore.preview.started"),
            ),
            (RestoreState::Previewing, RestoreEvent::PreviewBuilt) => (
                RestoreState::Validating,
                Some(RestoreSideEffectRequest::ValidatePackage),
                None,
            ),
            (RestoreState::Previewing, RestoreEvent::PreviewFailed)
            | (RestoreState::Validating, RestoreEvent::ValidationFailed) => {
                (RestoreState::Failed, None, Some("restore.failed"))
            }
            (RestoreState::Validating, RestoreEvent::ValidationPassed) => {
                (RestoreState::AwaitingConfirmation, None, None)
            }
            (RestoreState::AwaitingConfirmation, RestoreEvent::Confirm) => (
                RestoreState::Staging,
                Some(RestoreSideEffectRequest::StagePackage),
                Some("restore.confirmed"),
            ),
            (
                RestoreState::Requested
                | RestoreState::Previewing
                | RestoreState::Validating
                | RestoreState::AwaitingConfirmation,
                RestoreEvent::Cancel,
            ) => (RestoreState::Cancelled, None, Some("restore.cancelled")),
            (RestoreState::Staging, RestoreEvent::StageCompleted) => (
                RestoreState::Applying,
                Some(RestoreSideEffectRequest::ApplyAtomically),
                None,
            ),
            (RestoreState::Staging, RestoreEvent::StageFailed) => (
                RestoreState::CleanupRequired,
                Some(RestoreSideEffectRequest::CleanupStaging),
                Some("restore.failed"),
            ),
            (RestoreState::Applying, RestoreEvent::ApplyCompleted) => (
                RestoreState::Reopening,
                Some(RestoreSideEffectRequest::ReopenWorkspace),
                None,
            ),
            (RestoreState::Applying, RestoreEvent::ApplyFailed)
            | (RestoreState::Reopening, RestoreEvent::ReopenFailed) => (
                RestoreState::RollbackRequired,
                Some(RestoreSideEffectRequest::RollbackApply),
                Some("restore.failed"),
            ),
            (RestoreState::Reopening, RestoreEvent::ReopenCompleted) => {
                (RestoreState::Completed, None, Some("restore.completed"))
            }
            (RestoreState::CleanupRequired, RestoreEvent::CleanupCompleted) => {
                (RestoreState::Failed, None, None)
            }
            (RestoreState::RollbackRequired, RestoreEvent::RollbackCompleted) => {
                (RestoreState::RolledBack, None, Some("restore.rolled_back"))
            }
            (RestoreState::RollbackRequired, RestoreEvent::RollbackFailed) => (
                RestoreState::RecoveryRequired,
                None,
                Some("restore.recovery_required"),
            ),
            (RestoreState::RecoveryRequired, RestoreEvent::RecoveryRequested) => (
                RestoreState::RollbackRequired,
                Some(RestoreSideEffectRequest::RollbackApply),
                Some("restore.recovery.started"),
            ),
            _ => {
                return Err(RestoreTransitionError {
                    previous_state: state,
                    event,
                });
            }
        };
        Ok(RestoreTransition {
            previous_state: state,
            event,
            next_state,
            side_effect_request,
            product_log_event_name,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupJobError {
    InvalidJobId,
    InvalidRetryPolicy,
    InvalidProgress,
    InvalidArtifactManifest,
}

impl BackupJobError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidJobId => "BACKUP_JOB_INVALID_ID",
            Self::InvalidRetryPolicy => "BACKUP_JOB_INVALID_RETRY_POLICY",
            Self::InvalidProgress => "BACKUP_JOB_INVALID_PROGRESS",
            Self::InvalidArtifactManifest => "BACKUP_JOB_INVALID_ARTIFACT_MANIFEST",
        }
    }
}

fn transition(
    job: &BackupJobSnapshot,
    event: BackupJobEvent,
    state: BackupJobState,
    retry_count: u16,
    product_log_event_name: Option<&'static str>,
) -> BackupJobTransition {
    success_transition(job, event, state, retry_count, None, product_log_event_name)
}

fn success_transition(
    job: &BackupJobSnapshot,
    event: BackupJobEvent,
    state: BackupJobState,
    retry_count: u16,
    error_code: Option<&'static str>,
    product_log_event_name: Option<&'static str>,
) -> BackupJobTransition {
    let progress = if state == BackupJobState::Completed {
        BackupProgress::new(1, 1).expect("static completed progress is valid")
    } else {
        job.progress()
    };
    BackupJobTransition {
        previous_state: job.state(),
        event,
        next: job.transition_to(state, retry_count, progress, error_code),
        product_log_event_name,
    }
}

fn transition_error(job: &BackupJobSnapshot, event: BackupJobEvent) -> BackupJobTransitionError {
    BackupJobTransitionError {
        previous_state: job.state(),
        event,
        code: "BACKUP_JOB_INVALID_TRANSITION",
    }
}

fn product_event_name(operation: BackupJobOperation, completed: bool) -> &'static str {
    match (operation, completed) {
        (BackupJobOperation::Backup, true) => "backup.completed",
        (BackupJobOperation::Backup, false) => "backup.failed",
        (BackupJobOperation::Restore, true) => "restore.completed",
        (BackupJobOperation::Restore, false) => "restore.failed",
        (BackupJobOperation::Export, true) => "export.completed",
        (BackupJobOperation::Export, false) => "export.failed",
    }
}

fn validate_required_text(value: &str) -> Result<String, BackupJobError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(BackupJobError::InvalidArtifactManifest);
    }
    Ok(trimmed.to_string())
}
