use std::collections::BTreeMap;

use cabinet_domain::backup::{BackupJobId, BackupJobOperation, BackupJobSnapshot, BackupJobState};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_store::{
    BackupAuditRecord, BackupAuditRecorder, BackupAuditRecorderError, BackupStore,
    BackupStoreError, RestoreValidation,
};
use cabinet_usecases::backup::{
    BackupJobProductEvent, BackupJobUsecaseError, BackupJobUsecaseLogger, CreateBackupInput,
    CreateBackupUsecase, ExportWorkspaceInput, ExportWorkspaceUsecase, GetBackupStatusInput,
    GetBackupStatusUsecase, GetExportStatusInput, GetExportStatusUsecase, RestoreBackupInput,
    RestoreBackupUsecase,
};

#[test]
fn create_backup_returns_async_queued_job_and_records_audit() {
    let mut store = FakeBackupStore::default();
    let mut audit = FakeBackupAuditRecorder::default();

    let output = CreateBackupUsecase::new()
        .execute(
            CreateBackupInput::new("user-1", "workspace-1", "backup-job-1"),
            &mut store,
            &mut audit,
        )
        .expect("create backup");

    assert_eq!(output.job_id(), "backup-job-1");
    assert_eq!(output.operation(), BackupJobOperation::Backup);
    assert_eq!(output.state(), BackupJobState::Queued);
    assert_eq!(store.save_count, 1);
    assert_eq!(audit.records.len(), 1);
    assert_eq!(audit.records[0].event_name(), "backup.created");
}

#[test]
fn get_backup_status_returns_current_state_without_running_job_work() {
    let mut store = FakeBackupStore::default();
    store
        .save_job(job(
            "backup-job-1",
            BackupJobOperation::Backup,
            BackupJobState::Running,
        ))
        .expect("seed job");

    let output = GetBackupStatusUsecase::new()
        .execute(
            GetBackupStatusInput::new("user-1", "workspace-1", "backup-job-1"),
            &store,
        )
        .expect("status");

    assert_eq!(output.state(), BackupJobState::Running);
    assert_eq!(store.apply_count, 0);
}

#[test]
fn export_workspace_returns_async_queued_job() {
    let mut store = FakeBackupStore::default();

    let output = ExportWorkspaceUsecase::new()
        .execute(
            ExportWorkspaceInput::new("user-1", "workspace-1", "export-job-1"),
            &mut store,
        )
        .expect("export workspace");

    assert_eq!(output.operation(), BackupJobOperation::Export);
    assert_eq!(output.state(), BackupJobState::Queued);

    let status = GetExportStatusUsecase::new()
        .execute(
            GetExportStatusInput::new("user-1", "workspace-1", "export-job-1"),
            &store,
        )
        .expect("export status");
    assert_eq!(status.state(), BackupJobState::Queued);
}

#[test]
fn restore_failure_preserves_workspace_current_data_and_logs_safe_failure() {
    let mut store = FakeBackupStore {
        validation: RestoreValidation::failed("BACKUP_ARTIFACT_CORRUPTED"),
        current_workspace_value: "original".to_string(),
        ..FakeBackupStore::default()
    };
    let mut audit = FakeBackupAuditRecorder::default();
    let mut logger = FakeBackupLogger::default();

    let output = RestoreBackupUsecase::new()
        .execute(
            RestoreBackupInput::new("user-1", "workspace-1", "backup-job-1", "restore-job-1"),
            &mut store,
            &mut audit,
            &mut logger,
        )
        .expect("restore job should return failed job status");

    assert_eq!(output.operation(), BackupJobOperation::Restore);
    assert_eq!(output.state(), BackupJobState::Abandoned);
    assert_eq!(output.error_code(), Some("BACKUP_ARTIFACT_CORRUPTED"));
    assert_eq!(store.apply_count, 0);
    assert_eq!(store.current_workspace_value, "original");
    assert_eq!(logger.product_events.len(), 1);
    assert_eq!(logger.product_events[0].event_name(), "restore.failed");
    assert!(!format!("{:?}", logger.product_events[0]).contains("document body"));
}

#[test]
fn missing_backup_status_returns_stable_error() {
    let store = FakeBackupStore::default();

    let error = GetBackupStatusUsecase::new()
        .execute(
            GetBackupStatusInput::new("user-1", "workspace-1", "missing-job"),
            &store,
        )
        .expect_err("missing job must fail");

    assert_eq!(error, BackupJobUsecaseError::JobNotFound);
    assert_eq!(error.code(), "BACKUP_JOB_NOT_FOUND");
}

#[test]
fn product_log_events_cover_backup_restore_export_without_sensitive_payloads() {
    let backup = job(
        "backup-job-1",
        BackupJobOperation::Backup,
        BackupJobState::Completed,
    );
    let restore = job(
        "restore-job-1",
        BackupJobOperation::Restore,
        BackupJobState::Abandoned,
    )
    .with_error_code("BACKUP_ARTIFACT_CORRUPTED");
    let export = job(
        "export-job-1",
        BackupJobOperation::Export,
        BackupJobState::Completed,
    );

    let events = [
        BackupJobProductEvent::completed("backup.completed", &backup),
        BackupJobProductEvent::failed("restore.failed", &restore, "BACKUP_ARTIFACT_CORRUPTED"),
        BackupJobProductEvent::completed("export.completed", &export),
    ];
    let rendered = format!("{events:?}");

    assert_eq!(events[0].event_name(), "backup.completed");
    assert_eq!(events[1].event_name(), "restore.failed");
    assert_eq!(events[2].event_name(), "export.completed");
    assert!(!rendered.contains("document body"));
    assert!(!rendered.contains("asset content"));
    assert!(!rendered.contains("secret"));
    assert!(!rendered.contains("credential"));
}

#[derive(Default)]
struct FakeBackupStore {
    jobs: BTreeMap<String, BackupJobSnapshot>,
    validation: RestoreValidation,
    save_count: usize,
    apply_count: usize,
    current_workspace_value: String,
}

impl BackupStore for FakeBackupStore {
    fn save_job(&mut self, job: BackupJobSnapshot) -> Result<(), BackupStoreError> {
        self.save_count += 1;
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
        self.current_workspace_value = "restored".to_string();
        Ok(())
    }
}

#[derive(Default)]
struct FakeBackupAuditRecorder {
    records: Vec<BackupAuditRecord>,
}

impl BackupAuditRecorder for FakeBackupAuditRecorder {
    fn record_backup_audit(
        &mut self,
        record: BackupAuditRecord,
    ) -> Result<(), BackupAuditRecorderError> {
        self.records.push(record);
        Ok(())
    }
}

#[derive(Default)]
struct FakeBackupLogger {
    product_events: Vec<BackupJobProductEvent>,
}

impl BackupJobUsecaseLogger for FakeBackupLogger {
    fn write_product(&mut self, event: BackupJobProductEvent) {
        self.product_events.push(event);
    }
}

fn job(job_id: &str, operation: BackupJobOperation, state: BackupJobState) -> BackupJobSnapshot {
    BackupJobSnapshot::new(
        BackupJobId::new(job_id).expect("job id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        operation,
        state,
        0,
        cabinet_domain::backup::BackupProgress::queued(),
        None,
    )
    .expect("job")
}

fn job_key(workspace_id: &WorkspaceId, job_id: &BackupJobId) -> String {
    format!("{}:{}", workspace_id.as_str(), job_id.as_str())
}
