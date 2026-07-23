use cabinet_domain::backup::{BackupJobId, BackupPackageManifest, RestoreState};
use cabinet_domain::workspace::WorkspaceId;

pub trait BackupRestoreStore {
    fn request_restore(
        &mut self,
        workspace_id: &WorkspaceId,
        package_id: &BackupJobId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError>;

    fn prepare_restore(
        &mut self,
        workspace_id: &WorkspaceId,
        package_id: &BackupJobId,
        operation_id: &BackupJobId,
        manifest: &BackupPackageManifest,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError>;

    fn apply_restore(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError>;

    fn rollback_restore(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError>;

    fn finalize_restore(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError>;

    fn cancel_restore(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError>;

    fn get_restore_status(
        &self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<Option<BackupRestoreOperationSnapshot>, BackupRestoreStoreError>;

    fn mark_cleanup_required(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError>;

    fn mark_recovery_required(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError>;
}

pub trait WorkspaceReopener {
    fn reopen_workspace(&mut self, workspace_id: &WorkspaceId) -> Result<(), WorkspaceReopenError>;
}

pub trait BackupRecoveryStore {
    fn recover_startup(
        &mut self,
        workspace_id: &WorkspaceId,
    ) -> Result<BackupRecoveryReport, BackupRestoreStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackupRecoveryReport {
    cleaned_staging_count: u64,
    rolled_back_operation_ids: Vec<String>,
    cleanup_required_operation_ids: Vec<String>,
}

impl BackupRecoveryReport {
    pub fn new(
        cleaned_staging_count: u64,
        rolled_back_operation_ids: Vec<String>,
        cleanup_required_operation_ids: Vec<String>,
    ) -> Self {
        Self {
            cleaned_staging_count,
            rolled_back_operation_ids,
            cleanup_required_operation_ids,
        }
    }

    pub const fn cleaned_staging_count(&self) -> u64 {
        self.cleaned_staging_count
    }

    pub fn rolled_back_operation_ids(&self) -> &[String] {
        &self.rolled_back_operation_ids
    }

    pub fn cleanup_required_operation_ids(&self) -> &[String] {
        &self.cleanup_required_operation_ids
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceReopenError {
    ReopenFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupRestoreOperationSnapshot {
    workspace_id: WorkspaceId,
    package_id: BackupJobId,
    operation_id: BackupJobId,
    state: RestoreState,
}

impl BackupRestoreOperationSnapshot {
    pub fn new(
        workspace_id: WorkspaceId,
        package_id: BackupJobId,
        operation_id: BackupJobId,
        state: RestoreState,
    ) -> Self {
        Self {
            workspace_id,
            package_id,
            operation_id,
            state,
        }
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn package_id(&self) -> &BackupJobId {
        &self.package_id
    }

    pub fn operation_id(&self) -> &BackupJobId {
        &self.operation_id
    }

    pub const fn state(&self) -> RestoreState {
        self.state
    }

    pub fn with_state(&self, state: RestoreState) -> Self {
        Self::new(
            self.workspace_id.clone(),
            self.package_id.clone(),
            self.operation_id.clone(),
            state,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupRestoreStoreError {
    StorageUnavailable,
    OperationNotFound,
    PackageInvalid,
    CorruptedOperation,
    Conflict,
}

impl BackupRestoreStoreError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "backup_restore.storage_unavailable",
            Self::OperationNotFound => "backup_restore.operation_not_found",
            Self::PackageInvalid => "backup_restore.package_invalid",
            Self::CorruptedOperation => "backup_restore.operation_corrupted",
            Self::Conflict => "backup_restore.conflict",
        }
    }
}
