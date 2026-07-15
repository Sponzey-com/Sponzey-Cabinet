use std::collections::BTreeMap;

use cabinet_domain::backup::{
    BackupDataClass, BackupJobEvent, BackupJobId, BackupJobRetryPolicy, BackupJobSnapshot,
    BackupJobState, BackupJobStateMachine, BackupManifestEntry, BackupPackageManifest,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_package::{
    BackupPackageStore, BackupPackageStoreError, BackupPackageValidation,
};
use cabinet_ports::backup_store::{BackupStore, BackupStoreError, RestoreValidation};
use cabinet_usecases::backup_package_operation::{
    BackupPackageOperationEvent, BackupPackageOperationLogger, BackupPackageOperationUsecaseError,
    CancelBackupPackageOperationInput, CancelBackupPackageOperationUsecase,
    GetBackupPackageOperationInput, GetBackupPackageOperationUsecase,
    RunBackupPackageOperationInput, RunBackupPackageOperationUsecase,
    StartBackupPackageOperationInput, StartBackupPackageOperationUsecase,
};

#[test]
fn cancel_abandons_queued_operation_and_run_cannot_overwrite_it() {
    let mut jobs = FakeJobs::default();
    let mut packages = FakePackages::default();
    let mut logger = RecordingLogger::default();
    start(&mut jobs, &mut logger);
    let cancelled = CancelBackupPackageOperationUsecase::new()
        .execute(
            CancelBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
            &mut jobs,
            &mut logger,
        )
        .unwrap();
    assert_eq!(cancelled.state(), BackupJobState::Abandoned);
    assert_eq!(cancelled.error_code(), Some("BACKUP_OPERATION_CANCELLED"));

    let repeated = RunBackupPackageOperationUsecase::new()
        .execute(
            RunBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
            &mut jobs,
            &mut packages,
            &mut logger,
        )
        .unwrap();
    assert_eq!(repeated.state(), BackupJobState::Abandoned);
    assert_eq!(packages.build_calls, 0);
}

#[test]
fn start_queues_without_package_io_and_rejects_duplicate_identity() {
    let mut jobs = FakeJobs::default();
    let mut logger = RecordingLogger::default();
    let packages = FakePackages::default();

    let started = StartBackupPackageOperationUsecase::new()
        .execute(
            StartBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
            &mut jobs,
            &mut logger,
        )
        .unwrap();

    assert_eq!(started.state(), BackupJobState::Queued);
    assert_eq!(started.progress_completed_units(), 0);
    assert_eq!(packages.build_calls, 0);
    assert_eq!(logger.events[0].event_name(), "backup.operation.queued");
    assert_eq!(
        StartBackupPackageOperationUsecase::new().execute(
            StartBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
            &mut jobs,
            &mut logger,
        ),
        Err(BackupPackageOperationUsecaseError::Conflict)
    );
}

#[test]
fn run_persists_running_and_completed_and_is_idempotent_after_completion() {
    let mut jobs = FakeJobs::default();
    let mut packages = FakePackages::default();
    let mut logger = RecordingLogger::default();
    start(&mut jobs, &mut logger);

    let completed = RunBackupPackageOperationUsecase::new()
        .execute(
            RunBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
            &mut jobs,
            &mut packages,
            &mut logger,
        )
        .unwrap();

    assert_eq!(completed.state(), BackupJobState::Completed);
    assert_eq!(completed.progress_completed_units(), 1);
    assert_eq!(packages.build_calls, 1);
    assert_eq!(packages.discard_calls, 0);
    assert_eq!(
        jobs.saved_states,
        vec![
            BackupJobState::Queued,
            BackupJobState::Running,
            BackupJobState::Completed
        ]
    );
    let repeated = RunBackupPackageOperationUsecase::new()
        .execute(
            RunBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
            &mut jobs,
            &mut packages,
            &mut logger,
        )
        .unwrap();
    assert_eq!(repeated.state(), BackupJobState::Completed);
    assert_eq!(packages.build_calls, 1);
}

#[test]
fn cancellation_observed_after_package_publish_discards_the_package() {
    let mut jobs = FakeJobs {
        abandon_running_read: true,
        ..FakeJobs::default()
    };
    let mut packages = FakePackages::default();
    let mut logger = RecordingLogger::default();
    start(&mut jobs, &mut logger);

    let cancelled = RunBackupPackageOperationUsecase::new()
        .execute(
            RunBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
            &mut jobs,
            &mut packages,
            &mut logger,
        )
        .unwrap();

    assert_eq!(cancelled.state(), BackupJobState::Abandoned);
    assert_eq!(packages.build_calls, 1);
    assert_eq!(packages.discard_calls, 1);
    assert_eq!(
        logger.events.last().map(|event| event.event_name()),
        Some("backup.package.discarded")
    );
}

#[test]
fn discard_failure_emits_stable_product_error_without_sensitive_data() {
    let mut jobs = FakeJobs {
        abandon_running_read: true,
        ..FakeJobs::default()
    };
    let mut packages = FakePackages {
        discard_failure: Some(BackupPackageStoreError::StorageUnavailable),
        ..FakePackages::default()
    };
    let mut logger = RecordingLogger::default();
    start(&mut jobs, &mut logger);

    let result = RunBackupPackageOperationUsecase::new().execute(
        RunBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
        &mut jobs,
        &mut packages,
        &mut logger,
    );

    assert_eq!(
        result,
        Err(BackupPackageOperationUsecaseError::StorageUnavailable)
    );
    let event = logger.events.last().expect("discard failure event");
    assert_eq!(event.event_name(), "backup.package.discard_failed");
    assert_eq!(
        event.error_code(),
        Some("BACKUP_PACKAGE_DISCARD_STORAGE_UNAVAILABLE")
    );
    assert!(!format!("{event:?}").contains("workspace-1"));
}

#[test]
fn package_failure_is_durable_and_status_does_not_touch_package_store() {
    let mut jobs = FakeJobs::default();
    let mut packages = FakePackages {
        failure: Some(BackupPackageStoreError::CorruptedPackage),
        ..FakePackages::default()
    };
    let mut logger = RecordingLogger::default();
    start(&mut jobs, &mut logger);

    let failed = RunBackupPackageOperationUsecase::new()
        .execute(
            RunBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
            &mut jobs,
            &mut packages,
            &mut logger,
        )
        .unwrap();
    assert_eq!(failed.state(), BackupJobState::Abandoned);
    assert_eq!(failed.error_code(), Some("BACKUP_PACKAGE_CORRUPTED"));

    let calls = packages.build_calls;
    let status = GetBackupPackageOperationUsecase::new()
        .execute(
            GetBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
            &jobs,
        )
        .unwrap();
    assert_eq!(status.state(), BackupJobState::Abandoned);
    assert_eq!(status.error_code(), Some("BACKUP_PACKAGE_CORRUPTED"));
    assert_eq!(packages.build_calls, calls);
    assert!(!format!("{:?}", logger.events).contains("checksum"));
}

fn start(jobs: &mut FakeJobs, logger: &mut RecordingLogger) {
    StartBackupPackageOperationUsecase::new()
        .execute(
            StartBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
            jobs,
            logger,
        )
        .unwrap();
}

#[derive(Default)]
struct FakeJobs {
    values: BTreeMap<String, BackupJobSnapshot>,
    saved_states: Vec<BackupJobState>,
    abandon_running_read: bool,
}

impl BackupStore for FakeJobs {
    fn save_job(&mut self, job: BackupJobSnapshot) -> Result<(), BackupStoreError> {
        self.saved_states.push(job.state());
        self.values.insert(job.job_id().as_str().to_string(), job);
        Ok(())
    }
    fn get_job(
        &self,
        _: &WorkspaceId,
        id: &BackupJobId,
    ) -> Result<Option<BackupJobSnapshot>, BackupStoreError> {
        let value = self.values.get(id.as_str()).cloned();
        if self.abandon_running_read
            && value.as_ref().map(BackupJobSnapshot::state) == Some(BackupJobState::Running)
        {
            let abandoned = BackupJobStateMachine::transition(
                value.as_ref().expect("running job"),
                BackupJobEvent::Abandon,
                BackupJobRetryPolicy::new(2).expect("retry policy"),
            )
            .expect("running can be abandoned")
            .job()
            .clone()
            .with_error_code("BACKUP_OPERATION_CANCELLED");
            return Ok(Some(abandoned));
        }
        Ok(value)
    }
    fn validate_restore_staging(
        &self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<RestoreValidation, BackupStoreError> {
        Ok(RestoreValidation::valid())
    }
    fn apply_restore_staging(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<(), BackupStoreError> {
        Ok(())
    }
}

#[derive(Default)]
struct FakePackages {
    build_calls: usize,
    discard_calls: usize,
    failure: Option<BackupPackageStoreError>,
    discard_failure: Option<BackupPackageStoreError>,
}

impl BackupPackageStore for FakePackages {
    fn build_package(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<BackupPackageManifest, BackupPackageStoreError> {
        self.build_calls += 1;
        if let Some(error) = self.failure {
            return Err(error);
        }
        Ok(manifest())
    }
    fn inspect_manifest(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<BackupPackageManifest, BackupPackageStoreError> {
        Ok(manifest())
    }
    fn discard_package(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<(), BackupPackageStoreError> {
        self.discard_calls += 1;
        self.discard_failure.map_or(Ok(()), Err)
    }
    fn validate_package(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
        _: &BackupPackageManifest,
    ) -> Result<BackupPackageValidation, BackupPackageStoreError> {
        Ok(BackupPackageValidation::valid())
    }
}

#[derive(Default)]
struct RecordingLogger {
    events: Vec<BackupPackageOperationEvent>,
}
impl BackupPackageOperationLogger for RecordingLogger {
    fn write_product(&mut self, event: BackupPackageOperationEvent) {
        self.events.push(event);
    }
}

fn manifest() -> BackupPackageManifest {
    BackupPackageManifest::new(
        1,
        BackupDataClass::ALL
            .into_iter()
            .map(|class| {
                BackupManifestEntry::new(class, class.expected_ownership(), 1, 10, &"a".repeat(64))
                    .unwrap()
            })
            .collect(),
    )
    .unwrap()
}
