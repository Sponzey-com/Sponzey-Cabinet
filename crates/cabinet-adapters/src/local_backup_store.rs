use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use cabinet_domain::backup::{
    BackupArtifactManifest, BackupJobId, BackupJobOperation, BackupJobSnapshot, BackupJobState,
    BackupProgress,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_store::{BackupStore, BackupStoreError, RestoreValidation};

use crate::local_atomic_file::write_text_atomically;

pub const LOCAL_BACKUP_JOBS_DIR: &str = "backup-jobs";
pub const LOCAL_BACKUP_JOBS_BY_ID_DIR: &str = "jobs";
pub const LOCAL_BACKUP_RESTORE_APPLIED_DIR: &str = "restore-applied";
pub const BACKUP_SOURCE_NOT_COMPLETED: &str = "BACKUP_SOURCE_NOT_COMPLETED";
pub const BACKUP_ARTIFACT_MISSING: &str = "BACKUP_ARTIFACT_MISSING";

#[derive(Clone, PartialEq, Eq)]
pub struct LocalBackupStore {
    root: PathBuf,
}

impl fmt::Debug for LocalBackupStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalBackupStore")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl LocalBackupStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn workspace_dir(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.root
            .join(LOCAL_BACKUP_JOBS_DIR)
            .join(hex_encode(workspace_id.as_str()))
    }

    fn job_path(&self, workspace_id: &WorkspaceId, job_id: &BackupJobId) -> PathBuf {
        self.workspace_dir(workspace_id)
            .join(LOCAL_BACKUP_JOBS_BY_ID_DIR)
            .join(format!("{}.job", hex_encode(job_id.as_str())))
    }

    fn restore_marker_path(
        &self,
        workspace_id: &WorkspaceId,
        source_job_id: &BackupJobId,
    ) -> PathBuf {
        self.workspace_dir(workspace_id)
            .join(LOCAL_BACKUP_RESTORE_APPLIED_DIR)
            .join(format!("{}.restore", hex_encode(source_job_id.as_str())))
    }
}

impl BackupStore for LocalBackupStore {
    fn save_job(&mut self, job: BackupJobSnapshot) -> Result<(), BackupStoreError> {
        write_text_atomically(
            &self.job_path(job.workspace_id(), job.job_id()),
            encode_job(&job),
        )
        .map(|_| ())
        .map_err(|_| BackupStoreError::StorageUnavailable)
    }

    fn get_job(
        &self,
        workspace_id: &WorkspaceId,
        job_id: &BackupJobId,
    ) -> Result<Option<BackupJobSnapshot>, BackupStoreError> {
        let path = self.job_path(workspace_id, job_id);
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(BackupStoreError::StorageUnavailable),
        };
        let job = decode_job(&content)?;
        if job.workspace_id() != workspace_id || job.job_id() != job_id {
            return Err(BackupStoreError::CorruptedArtifact);
        }
        Ok(Some(job))
    }

    fn validate_restore_staging(
        &self,
        workspace_id: &WorkspaceId,
        source_job_id: &BackupJobId,
    ) -> Result<RestoreValidation, BackupStoreError> {
        let job = self
            .get_job(workspace_id, source_job_id)?
            .ok_or(BackupStoreError::MissingJob)?;
        if job.state() != BackupJobState::Completed {
            return Ok(RestoreValidation::failed(BACKUP_SOURCE_NOT_COMPLETED));
        }
        if job.artifact_manifest().is_none() {
            return Ok(RestoreValidation::failed(BACKUP_ARTIFACT_MISSING));
        }
        Ok(RestoreValidation::valid())
    }

    fn apply_restore_staging(
        &mut self,
        workspace_id: &WorkspaceId,
        source_job_id: &BackupJobId,
    ) -> Result<(), BackupStoreError> {
        let validation = self.validate_restore_staging(workspace_id, source_job_id)?;
        if !validation.is_valid() {
            return Err(BackupStoreError::CorruptedArtifact);
        }
        write_text_atomically(
            &self.restore_marker_path(workspace_id, source_job_id),
            format!(
                "workspace_id={}\nsource_job_id={}\nstate=applied\n",
                hex_encode(workspace_id.as_str()),
                hex_encode(source_job_id.as_str())
            ),
        )
        .map(|_| ())
        .map_err(|_| BackupStoreError::StorageUnavailable)
    }
}

fn encode_job(job: &BackupJobSnapshot) -> String {
    let artifact_id = job
        .artifact_manifest()
        .map_or("", |artifact| artifact.artifact_id());
    let artifact_object_count = job
        .artifact_manifest()
        .map_or(0, BackupArtifactManifest::object_count);
    let error_code = job.error_code().unwrap_or("");
    format!(
        "job_id={}\nworkspace_id={}\noperation={}\nstate={}\nretry_count={}\nprogress_completed={}\nprogress_total={}\nartifact_present={}\nartifact_id={}\nartifact_object_count={}\nerror_code={}\n",
        hex_encode(job.job_id().as_str()),
        hex_encode(job.workspace_id().as_str()),
        job.operation().as_str(),
        job.state().as_str(),
        job.retry_count(),
        job.progress().completed_units(),
        job.progress().total_units(),
        job.artifact_manifest().is_some(),
        hex_encode(artifact_id),
        artifact_object_count,
        hex_encode(error_code),
    )
}

fn decode_job(content: &str) -> Result<BackupJobSnapshot, BackupStoreError> {
    let fields = parse_fields(content)?;
    let job_id = BackupJobId::new(&required_hex(&fields, "job_id")?)
        .map_err(|_| BackupStoreError::CorruptedArtifact)?;
    let workspace_id = WorkspaceId::new(&required_hex(&fields, "workspace_id")?)
        .map_err(|_| BackupStoreError::CorruptedArtifact)?;
    let operation = decode_operation(required(&fields, "operation")?)?;
    let state = decode_state(required(&fields, "state")?)?;
    let retry_count = required(&fields, "retry_count")?
        .parse::<u16>()
        .map_err(|_| BackupStoreError::CorruptedArtifact)?;
    let progress = BackupProgress::new(
        required(&fields, "progress_completed")?
            .parse::<u64>()
            .map_err(|_| BackupStoreError::CorruptedArtifact)?,
        required(&fields, "progress_total")?
            .parse::<u64>()
            .map_err(|_| BackupStoreError::CorruptedArtifact)?,
    )
    .map_err(|_| BackupStoreError::CorruptedArtifact)?;
    let artifact_manifest = decode_artifact_manifest(&fields)?;
    let mut job = BackupJobSnapshot::new(
        job_id,
        workspace_id,
        operation,
        state,
        retry_count,
        progress,
        artifact_manifest,
    )
    .map_err(|_| BackupStoreError::CorruptedArtifact)?;
    if let Some(error_code) = optional_hex(&fields, "error_code")? {
        job = job.with_error_code(stable_error_code(&error_code)?);
    }
    Ok(job)
}

fn decode_artifact_manifest(
    fields: &BTreeMap<String, String>,
) -> Result<Option<BackupArtifactManifest>, BackupStoreError> {
    match required(fields, "artifact_present")? {
        "true" => Ok(Some(
            BackupArtifactManifest::new(
                &required_hex(fields, "artifact_id")?,
                required(fields, "artifact_object_count")?
                    .parse::<u64>()
                    .map_err(|_| BackupStoreError::CorruptedArtifact)?,
            )
            .map_err(|_| BackupStoreError::CorruptedArtifact)?,
        )),
        "false" => Ok(None),
        _ => Err(BackupStoreError::CorruptedArtifact),
    }
}

fn decode_operation(value: &str) -> Result<BackupJobOperation, BackupStoreError> {
    match value {
        "backup" => Ok(BackupJobOperation::Backup),
        "restore" => Ok(BackupJobOperation::Restore),
        "export" => Ok(BackupJobOperation::Export),
        _ => Err(BackupStoreError::CorruptedArtifact),
    }
}

fn decode_state(value: &str) -> Result<BackupJobState, BackupStoreError> {
    match value {
        "queued" => Ok(BackupJobState::Queued),
        "running" => Ok(BackupJobState::Running),
        "completed" => Ok(BackupJobState::Completed),
        "failed" => Ok(BackupJobState::Failed),
        "retrying" => Ok(BackupJobState::Retrying),
        "abandoned" => Ok(BackupJobState::Abandoned),
        _ => Err(BackupStoreError::CorruptedArtifact),
    }
}

fn parse_fields(content: &str) -> Result<BTreeMap<String, String>, BackupStoreError> {
    let mut fields = BTreeMap::new();
    for line in content.lines().filter(|line| !line.is_empty()) {
        let (key, value) = line
            .split_once('=')
            .ok_or(BackupStoreError::CorruptedArtifact)?;
        if key.is_empty() {
            return Err(BackupStoreError::CorruptedArtifact);
        }
        fields.insert(key.to_string(), value.to_string());
    }
    Ok(fields)
}

fn required<'a>(
    fields: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, BackupStoreError> {
    fields
        .get(key)
        .map(String::as_str)
        .ok_or(BackupStoreError::CorruptedArtifact)
}

fn required_hex(fields: &BTreeMap<String, String>, key: &str) -> Result<String, BackupStoreError> {
    hex_decode(required(fields, key)?)
}

fn optional_hex(
    fields: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<String>, BackupStoreError> {
    let Some(value) = fields.get(key) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    Ok(Some(hex_decode(value)?))
}

fn stable_error_code(value: &str) -> Result<&'static str, BackupStoreError> {
    match value {
        "BACKUP_JOB_RETRY_EXHAUSTED" => Ok("BACKUP_JOB_RETRY_EXHAUSTED"),
        "BACKUP_JOB_RETRYABLE_FAILURE" => Ok("BACKUP_JOB_RETRYABLE_FAILURE"),
        "BACKUP_JOB_FATAL_FAILURE" => Ok("BACKUP_JOB_FATAL_FAILURE"),
        "BACKUP_JOB_ABANDONED" => Ok("BACKUP_JOB_ABANDONED"),
        "BACKUP_OPERATION_CANCELLED" => Ok("BACKUP_OPERATION_CANCELLED"),
        "BACKUP_PACKAGE_STORAGE_UNAVAILABLE" => Ok("BACKUP_PACKAGE_STORAGE_UNAVAILABLE"),
        "BACKUP_PACKAGE_CONFLICT" => Ok("BACKUP_PACKAGE_CONFLICT"),
        "BACKUP_PACKAGE_NOT_FOUND" => Ok("BACKUP_PACKAGE_NOT_FOUND"),
        "BACKUP_PACKAGE_CORRUPTED" => Ok("BACKUP_PACKAGE_CORRUPTED"),
        "BACKUP_ARTIFACT_CORRUPTED" => Ok("BACKUP_ARTIFACT_CORRUPTED"),
        "BACKUP_RESTORE_STAGING_INVALID" => Ok("BACKUP_RESTORE_STAGING_INVALID"),
        "BACKUP_SOURCE_NOT_COMPLETED" => Ok(BACKUP_SOURCE_NOT_COMPLETED),
        "BACKUP_ARTIFACT_MISSING" => Ok(BACKUP_ARTIFACT_MISSING),
        "backup_store.storage_unavailable" => Ok("backup_store.storage_unavailable"),
        "backup_store.conflict" => Ok("backup_store.conflict"),
        "backup_store.corrupted_artifact" => Ok("backup_store.corrupted_artifact"),
        "backup_store.missing_job" => Ok("backup_store.missing_job"),
        _ => Err(BackupStoreError::CorruptedArtifact),
    }
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, BackupStoreError> {
    if !value.len().is_multiple_of(2) {
        return Err(BackupStoreError::CorruptedArtifact);
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| BackupStoreError::CorruptedArtifact)?;
    String::from_utf8(bytes).map_err(|_| BackupStoreError::CorruptedArtifact)
}
