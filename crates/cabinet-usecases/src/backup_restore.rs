use cabinet_domain::backup::{BackupJobId, RestoreState};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_package::{BackupPackageStore, BackupPackageStoreError};
use cabinet_ports::backup_restore::{
    BackupRestoreStore, BackupRestoreStoreError, WorkspaceReopener,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmBackupRestoreInput {
    actor_user_id: String,
    workspace_id: String,
    package_id: String,
    operation_id: String,
    confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelBackupRestoreInput {
    actor_user_id: String,
    workspace_id: String,
    operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartBackupRestoreOperationInput(ConfirmBackupRestoreInput);

impl StartBackupRestoreOperationInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        package_id: &str,
        operation_id: &str,
        confirmed: bool,
    ) -> Self {
        Self(ConfirmBackupRestoreInput::new(
            actor_user_id,
            workspace_id,
            package_id,
            operation_id,
            confirmed,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetBackupRestoreOperationInput {
    actor_user_id: String,
    workspace_id: String,
    operation_id: String,
}

impl GetBackupRestoreOperationInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, operation_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.into(),
            workspace_id: workspace_id.into(),
            operation_id: operation_id.into(),
        }
    }
}

impl CancelBackupRestoreInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, operation_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.into(),
            workspace_id: workspace_id.into(),
            operation_id: operation_id.into(),
        }
    }
}

impl ConfirmBackupRestoreInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        package_id: &str,
        operation_id: &str,
        confirmed: bool,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.into(),
            workspace_id: workspace_id.into(),
            package_id: package_id.into(),
            operation_id: operation_id.into(),
            confirmed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmBackupRestoreOutput {
    operation_id: String,
    state: RestoreState,
    error_code: Option<&'static str>,
}

impl ConfirmBackupRestoreOutput {
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
    pub const fn state(&self) -> RestoreState {
        self.state
    }
    pub const fn error_code(&self) -> Option<&'static str> {
        self.error_code
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmBackupRestoreError {
    ConfirmationRequired,
    InvalidInput,
    PackageInvalid,
    StorageUnavailable,
    Conflict,
}

impl ConfirmBackupRestoreError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::ConfirmationRequired => "RESTORE_CONFIRMATION_REQUIRED",
            Self::InvalidInput => "RESTORE_INVALID_INPUT",
            Self::PackageInvalid => "RESTORE_PACKAGE_INVALID",
            Self::StorageUnavailable => "RESTORE_STORAGE_UNAVAILABLE",
            Self::Conflict => "RESTORE_CONFLICT",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupRestoreProductEvent {
    event_name: &'static str,
    workspace_id: String,
    package_id: String,
    operation_id: String,
    state: RestoreState,
    error_code: Option<&'static str>,
}

impl BackupRestoreProductEvent {
    pub const fn event_name(&self) -> &'static str {
        self.event_name
    }
    pub const fn state(&self) -> RestoreState {
        self.state
    }
    pub const fn error_code(&self) -> Option<&'static str> {
        self.error_code
    }
}

pub trait BackupRestoreUsecaseLogger {
    fn write_product(&mut self, event: BackupRestoreProductEvent);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ConfirmBackupRestoreUsecase;

#[derive(Debug, Default, Clone, Copy)]
pub struct StartBackupRestoreOperationUsecase;

impl StartBackupRestoreOperationUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: StartBackupRestoreOperationInput,
        packages: &mut impl BackupPackageStore,
        restores: &mut impl BackupRestoreStore,
        logger: &mut impl BackupRestoreUsecaseLogger,
    ) -> Result<ConfirmBackupRestoreOutput, ConfirmBackupRestoreError> {
        let input = input.0;
        if !input.confirmed {
            return Err(ConfirmBackupRestoreError::ConfirmationRequired);
        }
        let _actor = UserId::new(&input.actor_user_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let package = BackupJobId::new(&input.package_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let operation = BackupJobId::new(&input.operation_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let manifest = packages
            .inspect_manifest(&workspace, &package)
            .map_err(map_package_error)?;
        let validation = packages
            .validate_package(&workspace, &package, &manifest)
            .map_err(map_package_error)?;
        if !validation.is_valid() {
            return Err(ConfirmBackupRestoreError::PackageInvalid);
        }
        let requested = restores
            .request_restore(&workspace, &package, &operation)
            .map_err(map_restore_error)?;
        write_event(
            logger,
            "restore.confirmed",
            &workspace,
            &package,
            &operation,
            requested.state(),
            None,
        );
        Ok(ConfirmBackupRestoreOutput {
            operation_id: operation.as_str().into(),
            state: requested.state(),
            error_code: None,
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GetBackupRestoreOperationUsecase;

impl GetBackupRestoreOperationUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetBackupRestoreOperationInput,
        restores: &impl BackupRestoreStore,
    ) -> Result<ConfirmBackupRestoreOutput, ConfirmBackupRestoreError> {
        let _actor = UserId::new(&input.actor_user_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let operation = BackupJobId::new(&input.operation_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let snapshot = restores
            .get_restore_status(&workspace, &operation)
            .map_err(map_restore_error)?
            .ok_or(ConfirmBackupRestoreError::StorageUnavailable)?;
        Ok(ConfirmBackupRestoreOutput {
            operation_id: operation.as_str().into(),
            state: snapshot.state(),
            error_code: None,
        })
    }
}

impl ConfirmBackupRestoreUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ConfirmBackupRestoreInput,
        packages: &mut impl BackupPackageStore,
        restores: &mut impl BackupRestoreStore,
        reopener: &mut impl WorkspaceReopener,
        logger: &mut impl BackupRestoreUsecaseLogger,
    ) -> Result<ConfirmBackupRestoreOutput, ConfirmBackupRestoreError> {
        if !input.confirmed {
            return Err(ConfirmBackupRestoreError::ConfirmationRequired);
        }
        let _actor = UserId::new(&input.actor_user_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let package = BackupJobId::new(&input.package_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let operation = BackupJobId::new(&input.operation_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let manifest = packages
            .inspect_manifest(&workspace, &package)
            .map_err(map_package_error)?;
        let validation = packages
            .validate_package(&workspace, &package, &manifest)
            .map_err(map_package_error)?;
        if !validation.is_valid() {
            write_event(
                logger,
                "restore.failed",
                &workspace,
                &package,
                &operation,
                RestoreState::Failed,
                validation.error_code(),
            );
            return Err(ConfirmBackupRestoreError::PackageInvalid);
        }
        write_event(
            logger,
            "restore.confirmed",
            &workspace,
            &package,
            &operation,
            RestoreState::Staging,
            None,
        );
        restores
            .prepare_restore(&workspace, &package, &operation, &manifest)
            .map_err(map_restore_error)?;
        let applied = restores
            .apply_restore(&workspace, &operation)
            .map_err(map_restore_error)?;
        if reopener.reopen_workspace(&workspace).is_err() {
            let rolled_back = restores
                .rollback_restore(&workspace, &operation)
                .map_err(map_restore_error)?;
            write_event(
                logger,
                "restore.rolled_back",
                &workspace,
                &package,
                &operation,
                rolled_back.state(),
                Some("RESTORE_REOPEN_FAILED"),
            );
            return Ok(ConfirmBackupRestoreOutput {
                operation_id: operation.as_str().into(),
                state: rolled_back.state(),
                error_code: Some("RESTORE_REOPEN_FAILED"),
            });
        }
        let completed = restores
            .finalize_restore(&workspace, &operation)
            .map_err(map_restore_error)?;
        debug_assert_eq!(applied.state(), RestoreState::Reopening);
        write_event(
            logger,
            "restore.completed",
            &workspace,
            &package,
            &operation,
            completed.state(),
            None,
        );
        Ok(ConfirmBackupRestoreOutput {
            operation_id: operation.as_str().into(),
            state: completed.state(),
            error_code: None,
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CancelBackupRestoreUsecase;

impl CancelBackupRestoreUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: CancelBackupRestoreInput,
        restores: &mut impl BackupRestoreStore,
        logger: &mut impl BackupRestoreUsecaseLogger,
    ) -> Result<ConfirmBackupRestoreOutput, ConfirmBackupRestoreError> {
        let _actor = UserId::new(&input.actor_user_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let operation = BackupJobId::new(&input.operation_id)
            .map_err(|_| ConfirmBackupRestoreError::InvalidInput)?;
        let cancelled = restores
            .cancel_restore(&workspace, &operation)
            .map_err(map_restore_error)?;
        write_event(
            logger,
            "restore.cancelled",
            &workspace,
            cancelled.package_id(),
            &operation,
            cancelled.state(),
            None,
        );
        Ok(ConfirmBackupRestoreOutput {
            operation_id: operation.as_str().into(),
            state: cancelled.state(),
            error_code: None,
        })
    }
}

fn write_event(
    logger: &mut impl BackupRestoreUsecaseLogger,
    event_name: &'static str,
    workspace: &WorkspaceId,
    package: &BackupJobId,
    operation: &BackupJobId,
    state: RestoreState,
    error_code: Option<&'static str>,
) {
    logger.write_product(BackupRestoreProductEvent {
        event_name,
        workspace_id: workspace.as_str().into(),
        package_id: package.as_str().into(),
        operation_id: operation.as_str().into(),
        state,
        error_code,
    });
}

fn map_package_error(error: BackupPackageStoreError) -> ConfirmBackupRestoreError {
    match error {
        BackupPackageStoreError::PackageNotFound | BackupPackageStoreError::CorruptedPackage => {
            ConfirmBackupRestoreError::PackageInvalid
        }
        BackupPackageStoreError::StorageUnavailable => {
            ConfirmBackupRestoreError::StorageUnavailable
        }
        BackupPackageStoreError::Conflict => ConfirmBackupRestoreError::Conflict,
    }
}

fn map_restore_error(error: BackupRestoreStoreError) -> ConfirmBackupRestoreError {
    match error {
        BackupRestoreStoreError::PackageInvalid | BackupRestoreStoreError::CorruptedOperation => {
            ConfirmBackupRestoreError::PackageInvalid
        }
        BackupRestoreStoreError::StorageUnavailable
        | BackupRestoreStoreError::OperationNotFound => {
            ConfirmBackupRestoreError::StorageUnavailable
        }
        BackupRestoreStoreError::Conflict => ConfirmBackupRestoreError::Conflict,
    }
}
