use cabinet_domain::backup::{BackupJobId, BackupJobSnapshot};
use cabinet_domain::workspace::WorkspaceId;

pub trait BackupStore {
    fn save_job(&mut self, job: BackupJobSnapshot) -> Result<(), BackupStoreError>;

    fn get_job(
        &self,
        workspace_id: &WorkspaceId,
        job_id: &BackupJobId,
    ) -> Result<Option<BackupJobSnapshot>, BackupStoreError>;

    fn validate_restore_staging(
        &self,
        workspace_id: &WorkspaceId,
        source_job_id: &BackupJobId,
    ) -> Result<RestoreValidation, BackupStoreError>;

    fn apply_restore_staging(
        &mut self,
        workspace_id: &WorkspaceId,
        source_job_id: &BackupJobId,
    ) -> Result<(), BackupStoreError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupStoreError {
    StorageUnavailable,
    Conflict,
    CorruptedArtifact,
    MissingJob,
}

impl BackupStoreError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "backup_store.storage_unavailable",
            Self::Conflict => "backup_store.conflict",
            Self::CorruptedArtifact => "backup_store.corrupted_artifact",
            Self::MissingJob => "backup_store.missing_job",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreValidation {
    valid: bool,
    error_code: Option<&'static str>,
}

impl RestoreValidation {
    pub const fn valid() -> Self {
        Self {
            valid: true,
            error_code: None,
        }
    }

    pub const fn failed(error_code: &'static str) -> Self {
        Self {
            valid: false,
            error_code: Some(error_code),
        }
    }

    pub const fn is_valid(&self) -> bool {
        self.valid
    }

    pub const fn error_code(&self) -> Option<&'static str> {
        self.error_code
    }
}

impl Default for RestoreValidation {
    fn default() -> Self {
        Self::valid()
    }
}

pub trait BackupAuditRecorder {
    fn record_backup_audit(
        &mut self,
        record: BackupAuditRecord,
    ) -> Result<(), BackupAuditRecorderError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupAuditRecord {
    workspace_id: WorkspaceId,
    job_id: BackupJobId,
    event_name: String,
    metadata: Vec<(String, String)>,
}

impl BackupAuditRecord {
    pub fn new(
        workspace_id: WorkspaceId,
        job_id: BackupJobId,
        event_name: &str,
        metadata: Vec<(String, String)>,
    ) -> Result<Self, BackupAuditRecorderError> {
        let event_name = validate_safe_text(event_name)?;
        for (key, value) in &metadata {
            validate_safe_text(key)?;
            validate_safe_text(value)?;
            if contains_sensitive_fragment(key) || contains_sensitive_fragment(value) {
                return Err(BackupAuditRecorderError::SensitiveMetadata);
            }
        }
        Ok(Self {
            workspace_id,
            job_id,
            event_name,
            metadata,
        })
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn job_id(&self) -> &BackupJobId {
        &self.job_id
    }

    pub fn event_name(&self) -> &str {
        &self.event_name
    }

    pub fn metadata(&self) -> &[(String, String)] {
        &self.metadata
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupAuditRecorderError {
    InvalidRecord,
    SensitiveMetadata,
    StorageUnavailable,
}

impl BackupAuditRecorderError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidRecord => "backup_audit.invalid_record",
            Self::SensitiveMetadata => "backup_audit.sensitive_metadata",
            Self::StorageUnavailable => "backup_audit.storage_unavailable",
        }
    }
}

fn validate_safe_text(value: &str) -> Result<String, BackupAuditRecorderError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(BackupAuditRecorderError::InvalidRecord);
    }
    Ok(trimmed.to_string())
}

fn contains_sensitive_fragment(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    [
        "body",
        "content",
        "token",
        "secret",
        "credential",
        "password",
    ]
    .iter()
    .any(|fragment| lowered.contains(fragment))
}
