use std::cell::RefCell;
use std::rc::Rc;

use cabinet_domain::backup::{
    BackupDataClass, BackupJobId, BackupManifestEntry, BackupPackageManifest, RestoreState,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_package::{
    BackupPackageStore, BackupPackageStoreError, BackupPackageValidation,
};
use cabinet_ports::backup_restore::{
    BackupRestoreOperationSnapshot, BackupRestoreStore, BackupRestoreStoreError,
    WorkspaceReopenError, WorkspaceReopener,
};
use cabinet_usecases::backup_restore::{
    BackupRestoreProductEvent, BackupRestoreUsecaseLogger, CancelBackupRestoreInput,
    CancelBackupRestoreUsecase, ConfirmBackupRestoreError, ConfirmBackupRestoreInput,
    ConfirmBackupRestoreUsecase, GetBackupRestoreOperationInput, GetBackupRestoreOperationUsecase,
    StartBackupRestoreOperationInput, StartBackupRestoreOperationUsecase,
};

#[test]
fn confirmed_start_validates_package_and_registers_durable_staging_operation() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut packages = FakePorts::new(Rc::clone(&calls));
    let mut restores = packages.clone();
    let mut logger = Logger::default();

    let output = StartBackupRestoreOperationUsecase::new()
        .execute(
            StartBackupRestoreOperationInput::new(
                "user-1",
                "workspace-1",
                "package-1",
                "operation-1",
                true,
            ),
            &mut packages,
            &mut restores,
            &mut logger,
        )
        .expect("start");

    assert_eq!(output.state(), RestoreState::Staging);
    assert_eq!(
        calls.borrow().as_slice(),
        ["inspect", "validate", "request"]
    );
    assert_eq!(logger.events[0].event_name(), "restore.confirmed");
}

#[test]
fn restore_status_reads_durable_snapshot_through_port() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let restores = FakePorts::new(Rc::clone(&calls));
    let output = GetBackupRestoreOperationUsecase::new()
        .execute(
            GetBackupRestoreOperationInput::new("user-1", "workspace-1", "operation-1"),
            &restores,
        )
        .expect("status");
    assert_eq!(output.state(), RestoreState::Staging);
    assert_eq!(output.operation_id(), "operation-1");
    assert_eq!(calls.borrow().as_slice(), ["status"]);
}

#[test]
fn confirmation_is_required_before_any_restore_io() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut ports = FakePorts::new(Rc::clone(&calls));
    let mut restores = ports.clone();
    let mut reopener = ports.clone();
    let mut logger = Logger::default();
    let result = ConfirmBackupRestoreUsecase::new().execute(
        ConfirmBackupRestoreInput::new("user-1", "workspace-1", "package-1", "operation-1", false),
        &mut ports,
        &mut restores,
        &mut reopener,
        &mut logger,
    );
    assert_eq!(result, Err(ConfirmBackupRestoreError::ConfirmationRequired));
    assert!(calls.borrow().is_empty());
}

#[test]
fn confirmed_restore_uses_validated_order_and_completes_only_after_reopen() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut packages = FakePorts::new(Rc::clone(&calls));
    let mut restores = packages.clone();
    let mut reopener = packages.clone();
    let mut logger = Logger::default();

    let output = ConfirmBackupRestoreUsecase::new()
        .execute(
            ConfirmBackupRestoreInput::new(
                "user-1",
                "workspace-1",
                "package-1",
                "operation-1",
                true,
            ),
            &mut packages,
            &mut restores,
            &mut reopener,
            &mut logger,
        )
        .expect("restore completes");

    assert_eq!(output.state(), RestoreState::Completed);
    assert_eq!(
        calls.borrow().as_slice(),
        [
            "inspect", "validate", "prepare", "apply", "reopen", "finalize"
        ]
    );
    assert_eq!(logger.events[0].event_name(), "restore.confirmed");
    assert_eq!(logger.events[1].event_name(), "restore.completed");
}

#[test]
fn reopen_failure_rolls_back_and_never_finalizes() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut packages = FakePorts::new(Rc::clone(&calls));
    let mut restores = packages.clone();
    let mut reopener = FakePorts::with_reopen_failure(Rc::clone(&calls));
    let mut logger = Logger::default();

    let output = ConfirmBackupRestoreUsecase::new()
        .execute(
            ConfirmBackupRestoreInput::new(
                "user-1",
                "workspace-1",
                "package-1",
                "operation-1",
                true,
            ),
            &mut packages,
            &mut restores,
            &mut reopener,
            &mut logger,
        )
        .expect("rollback is a handled result");

    assert_eq!(output.state(), RestoreState::RolledBack);
    assert_eq!(output.error_code(), Some("RESTORE_REOPEN_FAILED"));
    assert_eq!(calls.borrow().last(), Some(&"rollback"));
    assert!(!calls.borrow().contains(&"finalize"));
}

#[test]
fn rollback_failure_is_recorded_as_recovery_required_and_never_finalizes() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut packages = FakePorts::new(Rc::clone(&calls));
    let mut restores = FakePorts::with_rollback_failure(Rc::clone(&calls));
    let mut reopener = FakePorts::with_reopen_failure(Rc::clone(&calls));
    let mut logger = Logger::default();

    let output = ConfirmBackupRestoreUsecase::new()
        .execute(
            ConfirmBackupRestoreInput::new(
                "user-1",
                "workspace-1",
                "package-1",
                "operation-1",
                true,
            ),
            &mut packages,
            &mut restores,
            &mut reopener,
            &mut logger,
        )
        .expect("rollback failure becomes a durable recovery result");

    assert_eq!(output.state(), RestoreState::RecoveryRequired);
    assert_eq!(output.error_code(), Some("RESTORE_ROLLBACK_FAILED"));
    assert_eq!(calls.borrow().last(), Some(&"mark_recovery_required"));
    assert!(!calls.borrow().contains(&"finalize"));
    assert_eq!(
        logger.events.last().unwrap().event_name(),
        "restore.recovery_required"
    );
}

#[test]
fn cancel_usecase_delegates_to_staging_operation_and_logs_cancelled() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut restores = FakePorts::new(Rc::clone(&calls));
    let mut logger = Logger::default();

    let output = CancelBackupRestoreUsecase::new()
        .execute(
            CancelBackupRestoreInput::new("user-1", "workspace-1", "operation-1"),
            &mut restores,
            &mut logger,
        )
        .expect("cancel");

    assert_eq!(output.state(), RestoreState::Cancelled);
    assert_eq!(calls.borrow().as_slice(), ["cancel"]);
    assert_eq!(logger.events[0].event_name(), "restore.cancelled");
}

#[derive(Clone)]
struct FakePorts {
    calls: Rc<RefCell<Vec<&'static str>>>,
    reopen_fails: bool,
    rollback_fails: bool,
}
impl FakePorts {
    fn new(calls: Rc<RefCell<Vec<&'static str>>>) -> Self {
        Self {
            calls,
            reopen_fails: false,
            rollback_fails: false,
        }
    }
    fn with_reopen_failure(calls: Rc<RefCell<Vec<&'static str>>>) -> Self {
        Self {
            calls,
            reopen_fails: true,
            rollback_fails: false,
        }
    }
    fn with_rollback_failure(calls: Rc<RefCell<Vec<&'static str>>>) -> Self {
        Self {
            calls,
            reopen_fails: false,
            rollback_fails: true,
        }
    }
}
impl BackupPackageStore for FakePorts {
    fn build_package(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<BackupPackageManifest, BackupPackageStoreError> {
        unreachable!()
    }
    fn inspect_manifest(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<BackupPackageManifest, BackupPackageStoreError> {
        self.calls.borrow_mut().push("inspect");
        Ok(manifest())
    }
    fn discard_package(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<(), BackupPackageStoreError> {
        self.calls.borrow_mut().push("discard");
        Ok(())
    }
    fn validate_package(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
        _: &BackupPackageManifest,
    ) -> Result<BackupPackageValidation, BackupPackageStoreError> {
        self.calls.borrow_mut().push("validate");
        Ok(BackupPackageValidation::valid())
    }
}
impl BackupRestoreStore for FakePorts {
    fn request_restore(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
        _: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        self.calls.borrow_mut().push("request");
        Ok(snapshot(RestoreState::Staging))
    }
    fn prepare_restore(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
        _: &BackupJobId,
        _: &BackupPackageManifest,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        self.calls.borrow_mut().push("prepare");
        Ok(snapshot(RestoreState::Staging))
    }
    fn apply_restore(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        self.calls.borrow_mut().push("apply");
        Ok(snapshot(RestoreState::Reopening))
    }
    fn rollback_restore(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        self.calls.borrow_mut().push("rollback");
        if self.rollback_fails {
            return Err(BackupRestoreStoreError::StorageUnavailable);
        }
        Ok(snapshot(RestoreState::RolledBack))
    }
    fn finalize_restore(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        self.calls.borrow_mut().push("finalize");
        Ok(snapshot(RestoreState::Completed))
    }
    fn cancel_restore(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        self.calls.borrow_mut().push("cancel");
        Ok(snapshot(RestoreState::Cancelled))
    }
    fn get_restore_status(
        &self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<Option<BackupRestoreOperationSnapshot>, BackupRestoreStoreError> {
        self.calls.borrow_mut().push("status");
        Ok(Some(snapshot(RestoreState::Staging)))
    }
    fn mark_cleanup_required(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        Ok(snapshot(RestoreState::CleanupRequired))
    }
    fn mark_recovery_required(
        &mut self,
        _: &WorkspaceId,
        _: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        self.calls.borrow_mut().push("mark_recovery_required");
        Ok(snapshot(RestoreState::RecoveryRequired))
    }
}
impl WorkspaceReopener for FakePorts {
    fn reopen_workspace(&mut self, _: &WorkspaceId) -> Result<(), WorkspaceReopenError> {
        self.calls.borrow_mut().push("reopen");
        if self.reopen_fails {
            Err(WorkspaceReopenError::ReopenFailed)
        } else {
            Ok(())
        }
    }
}
#[derive(Default)]
struct Logger {
    events: Vec<BackupRestoreProductEvent>,
}
impl BackupRestoreUsecaseLogger for Logger {
    fn write_product(&mut self, event: BackupRestoreProductEvent) {
        self.events.push(event);
    }
}

fn snapshot(state: RestoreState) -> BackupRestoreOperationSnapshot {
    BackupRestoreOperationSnapshot::new(
        WorkspaceId::new("workspace-1").expect("workspace"),
        BackupJobId::new("package-1").expect("package"),
        BackupJobId::new("operation-1").expect("operation"),
        state,
    )
}

fn manifest() -> BackupPackageManifest {
    BackupPackageManifest::new(
        1,
        BackupDataClass::ALL
            .into_iter()
            .map(|class| {
                BackupManifestEntry::new(
                    class,
                    class.expected_ownership(),
                    1,
                    1,
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                )
                .expect("entry")
            })
            .collect(),
    )
    .expect("manifest")
}
