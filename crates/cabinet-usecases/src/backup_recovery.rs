use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_restore::{BackupRecoveryStore, BackupRestoreStoreError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoverBackupStartupInput {
    actor_user_id: String,
    workspace_id: String,
}

impl RecoverBackupStartupInput {
    pub fn new(actor_user_id: &str, workspace_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.into(),
            workspace_id: workspace_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoverBackupStartupOutput {
    cleaned_staging_count: u64,
    rolled_back_operation_ids: Vec<String>,
    cleanup_required_operation_ids: Vec<String>,
}

impl RecoverBackupStartupOutput {
    pub const fn cleaned_staging_count(&self) -> u64 {
        self.cleaned_staging_count
    }

    pub fn rolled_back_operation_ids(&self) -> &[String] {
        &self.rolled_back_operation_ids
    }

    pub const fn rolled_back_operation_count(&self) -> usize {
        self.rolled_back_operation_ids.len()
    }

    pub fn cleanup_required_operation_ids(&self) -> &[String] {
        &self.cleanup_required_operation_ids
    }

    pub const fn cleanup_required_operation_count(&self) -> usize {
        self.cleanup_required_operation_ids.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupRecoveryUsecaseError {
    InvalidInput,
    StorageUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupRecoveryProductEvent {
    event_name: &'static str,
    workspace_id: String,
    cleaned_staging_count: u64,
    rolled_back_operation_count: usize,
    cleanup_required_operation_count: usize,
}

impl BackupRecoveryProductEvent {
    pub const fn event_name(&self) -> &'static str {
        self.event_name
    }
}

pub trait BackupRecoveryUsecaseLogger {
    fn write_product(&mut self, event: BackupRecoveryProductEvent);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RecoverBackupStartupUsecase;

impl RecoverBackupStartupUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: RecoverBackupStartupInput,
        store: &mut impl BackupRecoveryStore,
        logger: &mut impl BackupRecoveryUsecaseLogger,
    ) -> Result<RecoverBackupStartupOutput, BackupRecoveryUsecaseError> {
        UserId::new(&input.actor_user_id).map_err(|_| BackupRecoveryUsecaseError::InvalidInput)?;
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| BackupRecoveryUsecaseError::InvalidInput)?;
        let report = store.recover_startup(&workspace).map_err(map_store_error)?;
        let output = RecoverBackupStartupOutput {
            cleaned_staging_count: report.cleaned_staging_count(),
            rolled_back_operation_ids: report.rolled_back_operation_ids().to_vec(),
            cleanup_required_operation_ids: report.cleanup_required_operation_ids().to_vec(),
        };
        logger.write_product(BackupRecoveryProductEvent {
            event_name: "backup.recovery.completed",
            workspace_id: workspace.as_str().into(),
            cleaned_staging_count: output.cleaned_staging_count,
            rolled_back_operation_count: output.rolled_back_operation_count(),
            cleanup_required_operation_count: output.cleanup_required_operation_count(),
        });
        Ok(output)
    }
}

fn map_store_error(_error: BackupRestoreStoreError) -> BackupRecoveryUsecaseError {
    BackupRecoveryUsecaseError::StorageUnavailable
}
