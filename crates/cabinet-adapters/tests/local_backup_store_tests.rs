use std::fs;
use std::path::PathBuf;

use cabinet_adapters::local_backup_store::LocalBackupStore;
use cabinet_domain::backup::{
    BackupArtifactManifest, BackupJobId, BackupJobOperation, BackupJobSnapshot, BackupJobState,
    BackupProgress,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_store::{BackupStore, BackupStoreError};

#[test]
fn local_backup_store_persists_job_snapshot_across_instances() {
    let root = unique_temp_dir("local-backup-store-persist");
    let workspace_id = workspace_id("workspace-1");
    let queued = job(
        "backup-job-1",
        &workspace_id,
        BackupJobOperation::Backup,
        BackupJobState::Queued,
        0,
        BackupProgress::queued(),
        None,
    );
    let running = job(
        "backup-job-1",
        &workspace_id,
        BackupJobOperation::Backup,
        BackupJobState::Running,
        1,
        BackupProgress::new(1, 3).expect("progress"),
        Some(BackupArtifactManifest::new("artifact-1", 3).expect("artifact")),
    )
    .with_error_code("BACKUP_JOB_RETRYABLE_FAILURE");

    {
        let mut store = LocalBackupStore::new(root.clone());
        store.save_job(queued).expect("save queued job");
        store.save_job(running.clone()).expect("replace job");
    }

    let store = LocalBackupStore::new(root.clone());
    let loaded = store
        .get_job(&workspace_id, running.job_id())
        .expect("get job")
        .expect("stored job");

    assert_eq!(loaded.job_id(), running.job_id());
    assert_eq!(loaded.state(), BackupJobState::Running);
    assert_eq!(loaded.retry_count(), 1);
    assert_eq!(loaded.progress().completed_units(), 1);
    assert_eq!(
        loaded.artifact_manifest().expect("artifact").object_count(),
        3
    );
    assert_eq!(loaded.error_code(), Some("BACKUP_JOB_RETRYABLE_FAILURE"));
    assert!(!format!("{store:?}").contains("backup-job-1"));
    cleanup_temp_dir(root);
}

#[test]
fn local_backup_store_validates_and_applies_restore_staging() {
    let root = unique_temp_dir("local-backup-store-restore");
    let workspace_id = workspace_id("workspace-1");
    let completed_source = job(
        "backup-job-1",
        &workspace_id,
        BackupJobOperation::Backup,
        BackupJobState::Completed,
        0,
        BackupProgress::new(1, 1).expect("progress"),
        Some(BackupArtifactManifest::new("artifact-1", 3).expect("artifact")),
    );
    let incomplete_source = job(
        "backup-job-2",
        &workspace_id,
        BackupJobOperation::Backup,
        BackupJobState::Running,
        0,
        BackupProgress::queued(),
        Some(BackupArtifactManifest::new("artifact-2", 3).expect("artifact")),
    );
    let missing_artifact_source = job(
        "backup-job-3",
        &workspace_id,
        BackupJobOperation::Backup,
        BackupJobState::Completed,
        0,
        BackupProgress::new(1, 1).expect("progress"),
        None,
    );
    let mut store = LocalBackupStore::new(root.clone());
    store
        .save_job(completed_source.clone())
        .expect("save source");
    store
        .save_job(incomplete_source.clone())
        .expect("save incomplete source");
    store
        .save_job(missing_artifact_source.clone())
        .expect("save missing artifact source");

    let validation = LocalBackupStore::new(root.clone())
        .validate_restore_staging(&workspace_id, completed_source.job_id())
        .expect("validate completed source");
    let incomplete_validation = store
        .validate_restore_staging(&workspace_id, incomplete_source.job_id())
        .expect("validate incomplete source");
    let missing_artifact_validation = store
        .validate_restore_staging(&workspace_id, missing_artifact_source.job_id())
        .expect("validate missing artifact source");

    assert!(validation.is_valid());
    assert!(!incomplete_validation.is_valid());
    assert_eq!(
        incomplete_validation.error_code(),
        Some("BACKUP_SOURCE_NOT_COMPLETED")
    );
    assert!(!missing_artifact_validation.is_valid());
    assert_eq!(
        missing_artifact_validation.error_code(),
        Some("BACKUP_ARTIFACT_MISSING")
    );

    store
        .apply_restore_staging(&workspace_id, completed_source.job_id())
        .expect("apply restore staging");
    assert!(first_file_under(&root.join("backup-jobs"), "restore").exists());
    assert_eq!(
        store
            .apply_restore_staging(&workspace_id, incomplete_source.job_id())
            .expect_err("invalid source must not apply"),
        BackupStoreError::CorruptedArtifact
    );
    cleanup_temp_dir(root);
}

#[test]
fn local_backup_store_reports_missing_and_corrupted_job_files() {
    let root = unique_temp_dir("local-backup-store-corrupt");
    let workspace_id = workspace_id("workspace-1");
    let job = job(
        "backup-job-1",
        &workspace_id,
        BackupJobOperation::Backup,
        BackupJobState::Queued,
        0,
        BackupProgress::queued(),
        None,
    );
    let mut store = LocalBackupStore::new(root.clone());
    store.save_job(job.clone()).expect("save job");

    let missing = store
        .validate_restore_staging(
            &workspace_id,
            &BackupJobId::new("missing-job").expect("job id"),
        )
        .expect_err("missing source job must fail");
    fs::write(
        first_file_under(&root.join("backup-jobs"), "job"),
        "not-a-backup-job-record",
    )
    .expect("corrupt job file");
    let corrupted = store
        .get_job(&workspace_id, job.job_id())
        .expect_err("corrupted job must fail");

    assert_eq!(missing, BackupStoreError::MissingJob);
    assert_eq!(corrupted, BackupStoreError::CorruptedArtifact);
    cleanup_temp_dir(root);
}

fn job(
    job_id: &str,
    workspace_id: &WorkspaceId,
    operation: BackupJobOperation,
    state: BackupJobState,
    retry_count: u16,
    progress: BackupProgress,
    artifact_manifest: Option<BackupArtifactManifest>,
) -> BackupJobSnapshot {
    BackupJobSnapshot::new(
        BackupJobId::new(job_id).expect("job id"),
        workspace_id.clone(),
        operation,
        state,
        retry_count,
        progress,
        artifact_manifest,
    )
    .expect("job")
}

fn workspace_id(value: &str) -> WorkspaceId {
    WorkspaceId::new(value).expect("workspace id")
}

fn first_file_under(root: &PathBuf, extension: &str) -> PathBuf {
    let mut stack = vec![root.clone()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(path).expect("read dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                return path;
            }
        }
    }
    panic!("file with extension {extension} not found");
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sponzey-cabinet-{name}-{}", std::process::id()));
    cleanup_temp_dir(dir.clone());
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn cleanup_temp_dir(dir: PathBuf) {
    if dir.exists() {
        fs::remove_dir_all(dir).expect("remove temp dir");
    }
}
