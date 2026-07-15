use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_restore::{
    BackupRecoveryReport, BackupRecoveryStore, BackupRestoreStoreError,
};
use cabinet_usecases::backup_recovery::{
    BackupRecoveryProductEvent, BackupRecoveryUsecaseLogger, RecoverBackupStartupInput,
    RecoverBackupStartupUsecase,
};

#[test]
fn startup_recovery_returns_safe_summary_and_product_event() {
    let mut store = FakeRecoveryStore { calls: 0 };
    let mut logger = Logger::default();

    let output = RecoverBackupStartupUsecase::new()
        .execute(
            RecoverBackupStartupInput::new("user-1", "workspace-1"),
            &mut store,
            &mut logger,
        )
        .expect("recovery");

    assert_eq!(store.calls, 1);
    assert_eq!(output.cleaned_staging_count(), 2);
    assert_eq!(output.rolled_back_operation_count(), 1);
    assert_eq!(output.cleanup_required_operation_count(), 1);
    assert_eq!(logger.events[0].event_name(), "backup.recovery.completed");
    let debug = format!("{:?}", logger.events[0]);
    assert!(!debug.contains("/Users/"));
    assert!(!debug.contains("body"));
}

struct FakeRecoveryStore {
    calls: usize,
}
impl BackupRecoveryStore for FakeRecoveryStore {
    fn recover_startup(
        &mut self,
        _: &WorkspaceId,
    ) -> Result<BackupRecoveryReport, BackupRestoreStoreError> {
        self.calls += 1;
        Ok(BackupRecoveryReport::new(
            2,
            vec!["operation-1".into()],
            vec!["operation-2".into()],
        ))
    }
}

#[derive(Default)]
struct Logger {
    events: Vec<BackupRecoveryProductEvent>,
}
impl BackupRecoveryUsecaseLogger for Logger {
    fn write_product(&mut self, event: BackupRecoveryProductEvent) {
        self.events.push(event);
    }
}
