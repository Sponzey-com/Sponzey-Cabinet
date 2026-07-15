use std::collections::BTreeMap;

use cabinet_domain::backup::{
    BackupArtifactManifest, BackupJobId, BackupJobOperation, BackupJobSnapshot, BackupJobState,
    BackupProgress,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_store::{
    BackupAuditRecord, BackupAuditRecorder, BackupAuditRecorderError, BackupStore,
    BackupStoreError, RestoreValidation,
};

#[test]
fn backup_store_contract_saves_and_reads_job_status_without_artifact_body() {
    let workspace_id = workspace_id();
    let mut store = FakeBackupStore::default();
    let record = job(
        "backup-job-1",
        BackupJobOperation::Backup,
        BackupJobState::Queued,
    );

    store.save_job(record.clone()).expect("save job");
    let fetched = store
        .get_job(&workspace_id, record.job_id())
        .expect("get job")
        .expect("job");

    assert_eq!(fetched.job_id(), record.job_id());
    assert_eq!(fetched.state(), BackupJobState::Queued);
    assert!(!format!("{fetched:?}").contains("document body"));
}

#[test]
fn restore_validation_failure_prevents_apply_call() {
    let workspace_id = workspace_id();
    let mut store = FakeBackupStore {
        validation: RestoreValidation::failed("BACKUP_ARTIFACT_CORRUPTED"),
        ..FakeBackupStore::default()
    };
    let source_job_id = BackupJobId::new("backup-job-1").expect("job id");

    let validation = store
        .validate_restore_staging(&workspace_id, &source_job_id)
        .expect("validation result");
    if validation.is_valid() {
        store
            .apply_restore_staging(&workspace_id, &source_job_id)
            .expect("apply");
    }

    assert!(!validation.is_valid());
    assert_eq!(store.apply_count, 0);
    assert_eq!(validation.error_code(), Some("BACKUP_ARTIFACT_CORRUPTED"));
}

#[test]
fn backup_audit_recorder_accepts_job_event_without_document_body() {
    let mut recorder = FakeAuditRecorder::default();
    let record = BackupAuditRecord::new(
        workspace_id(),
        BackupJobId::new("backup-job-1").expect("job id"),
        "backup.created",
        vec![("state".to_string(), "queued".to_string())],
    )
    .expect("audit record");

    recorder
        .record_backup_audit(record.clone())
        .expect("record audit");

    assert_eq!(recorder.records.len(), 1);
    assert_eq!(recorder.records[0].event_name(), "backup.created");
    assert!(!format!("{record:?}").contains("document body"));
}

#[derive(Default)]
struct FakeBackupStore {
    jobs: BTreeMap<String, BackupJobSnapshot>,
    validation: RestoreValidation,
    apply_count: usize,
}

impl BackupStore for FakeBackupStore {
    fn save_job(&mut self, job: BackupJobSnapshot) -> Result<(), BackupStoreError> {
        self.jobs
            .insert(job_key(job.workspace_id(), job.job_id()), job);
        Ok(())
    }

    fn get_job(
        &self,
        workspace_id: &WorkspaceId,
        job_id: &BackupJobId,
    ) -> Result<Option<BackupJobSnapshot>, BackupStoreError> {
        Ok(self.jobs.get(&job_key(workspace_id, job_id)).cloned())
    }

    fn validate_restore_staging(
        &self,
        _workspace_id: &WorkspaceId,
        _source_job_id: &BackupJobId,
    ) -> Result<RestoreValidation, BackupStoreError> {
        Ok(self.validation.clone())
    }

    fn apply_restore_staging(
        &mut self,
        _workspace_id: &WorkspaceId,
        _source_job_id: &BackupJobId,
    ) -> Result<(), BackupStoreError> {
        self.apply_count += 1;
        Ok(())
    }
}

#[derive(Default)]
struct FakeAuditRecorder {
    records: Vec<BackupAuditRecord>,
}

impl BackupAuditRecorder for FakeAuditRecorder {
    fn record_backup_audit(
        &mut self,
        record: BackupAuditRecord,
    ) -> Result<(), BackupAuditRecorderError> {
        self.records.push(record);
        Ok(())
    }
}

fn workspace_id() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace id")
}

fn job(job_id: &str, operation: BackupJobOperation, state: BackupJobState) -> BackupJobSnapshot {
    BackupJobSnapshot::new(
        BackupJobId::new(job_id).expect("job id"),
        workspace_id(),
        operation,
        state,
        0,
        BackupProgress::new(0, 1).expect("progress"),
        Some(BackupArtifactManifest::new("artifact-1", 1).expect("manifest")),
    )
    .expect("job")
}

fn job_key(workspace_id: &WorkspaceId, job_id: &BackupJobId) -> String {
    format!("{}:{}", workspace_id.as_str(), job_id.as_str())
}
