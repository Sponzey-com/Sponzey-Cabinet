use cabinet_domain::backup::{
    BackupDataClass, BackupDataOwnership, BackupJobId, BackupPackageManifest, RestoreEvent,
    RestoreState, RestoreWorkflowStateMachine,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_package::{BackupPackageStore, BackupPackageStoreError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBackupPackageInput {
    actor_user_id: String,
    workspace_id: String,
    package_id: String,
}

impl CreateBackupPackageInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, package_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            package_id: package_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewBackupRestoreInput {
    actor_user_id: String,
    workspace_id: String,
    package_id: String,
}

impl PreviewBackupRestoreInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, package_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            package_id: package_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPackageSummary {
    schema_version: u16,
    created_at_epoch_ms: Option<u64>,
    entries: Vec<BackupPackageEntrySummary>,
    authoritative_record_count: u64,
    rebuildable_record_count: u64,
    byte_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPackageEntrySummary {
    data_class: BackupDataClass,
    record_count: u64,
    byte_count: u64,
}

impl BackupPackageEntrySummary {
    pub const fn data_class(&self) -> BackupDataClass {
        self.data_class
    }

    pub const fn record_count(&self) -> u64 {
        self.record_count
    }

    pub const fn byte_count(&self) -> u64 {
        self.byte_count
    }
}

impl BackupPackageSummary {
    pub(crate) fn from_manifest(manifest: &BackupPackageManifest) -> Self {
        let mut authoritative_record_count = 0_u64;
        let mut rebuildable_record_count = 0_u64;
        let mut byte_count = 0_u64;
        for entry in manifest.entries() {
            match entry.ownership() {
                BackupDataOwnership::Authoritative => {
                    authoritative_record_count =
                        authoritative_record_count.saturating_add(entry.record_count());
                }
                BackupDataOwnership::Rebuildable => {
                    rebuildable_record_count =
                        rebuildable_record_count.saturating_add(entry.record_count());
                }
            }
            byte_count = byte_count.saturating_add(entry.byte_count());
        }
        Self {
            schema_version: manifest.schema_version(),
            created_at_epoch_ms: manifest.created_at_epoch_ms(),
            entries: manifest
                .entries()
                .iter()
                .map(|entry| BackupPackageEntrySummary {
                    data_class: entry.data_class(),
                    record_count: entry.record_count(),
                    byte_count: entry.byte_count(),
                })
                .collect(),
            authoritative_record_count,
            rebuildable_record_count,
            byte_count,
        }
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn created_at_epoch_ms(&self) -> Option<u64> {
        self.created_at_epoch_ms
    }

    pub const fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn entries(&self) -> &[BackupPackageEntrySummary] {
        &self.entries
    }

    pub const fn authoritative_record_count(&self) -> u64 {
        self.authoritative_record_count
    }

    pub const fn rebuildable_record_count(&self) -> u64 {
        self.rebuildable_record_count
    }

    pub const fn byte_count(&self) -> u64 {
        self.byte_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBackupPackageOutput {
    package_id: String,
    summary: BackupPackageSummary,
}

impl CreateBackupPackageOutput {
    pub fn package_id(&self) -> &str {
        &self.package_id
    }

    pub const fn schema_version(&self) -> u16 {
        self.summary.schema_version()
    }

    pub const fn entry_count(&self) -> usize {
        self.summary.entry_count()
    }

    pub const fn authoritative_record_count(&self) -> u64 {
        self.summary.authoritative_record_count()
    }

    pub const fn rebuildable_record_count(&self) -> u64 {
        self.summary.rebuildable_record_count()
    }

    pub const fn summary(&self) -> &BackupPackageSummary {
        &self.summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewBackupRestoreOutput {
    package_id: String,
    state: RestoreState,
    summary: BackupPackageSummary,
    validation_error_code: Option<&'static str>,
}

impl PreviewBackupRestoreOutput {
    pub fn package_id(&self) -> &str {
        &self.package_id
    }

    pub const fn state(&self) -> RestoreState {
        self.state
    }

    pub const fn confirmation_ready(&self) -> bool {
        matches!(self.state, RestoreState::AwaitingConfirmation)
    }

    pub const fn summary(&self) -> &BackupPackageSummary {
        &self.summary
    }

    pub const fn validation_error_code(&self) -> Option<&'static str> {
        self.validation_error_code
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupPackageUsecaseError {
    InvalidInput,
    StorageUnavailable,
    PackageNotFound,
    CorruptedPackage,
    Conflict,
}

impl BackupPackageUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "BACKUP_PACKAGE_INVALID_INPUT",
            Self::StorageUnavailable => "BACKUP_PACKAGE_STORAGE_UNAVAILABLE",
            Self::PackageNotFound => "BACKUP_PACKAGE_NOT_FOUND",
            Self::CorruptedPackage => "BACKUP_PACKAGE_CORRUPTED",
            Self::Conflict => "BACKUP_PACKAGE_CONFLICT",
        }
    }

    const fn from_store(error: BackupPackageStoreError) -> Self {
        match error {
            BackupPackageStoreError::StorageUnavailable => Self::StorageUnavailable,
            BackupPackageStoreError::PackageNotFound => Self::PackageNotFound,
            BackupPackageStoreError::CorruptedPackage => Self::CorruptedPackage,
            BackupPackageStoreError::Conflict => Self::Conflict,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPackageProductEvent {
    event_name: &'static str,
    workspace_id: String,
    package_id: String,
    entry_count: Option<usize>,
    record_count: Option<u64>,
    error_code: Option<&'static str>,
}

impl BackupPackageProductEvent {
    pub const fn event_name(&self) -> &'static str {
        self.event_name
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn package_id(&self) -> &str {
        &self.package_id
    }

    pub const fn error_code(&self) -> Option<&'static str> {
        self.error_code
    }
}

pub trait BackupPackageUsecaseLogger {
    fn write_product(&mut self, event: BackupPackageProductEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CreateBackupPackageUsecase;

impl CreateBackupPackageUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: CreateBackupPackageInput,
        store: &mut impl BackupPackageStore,
        logger: &mut impl BackupPackageUsecaseLogger,
    ) -> Result<CreateBackupPackageOutput, BackupPackageUsecaseError> {
        let parsed = parse_input(input.actor_user_id, input.workspace_id, input.package_id)?;
        let manifest = match store.build_package(&parsed.workspace_id, &parsed.package_id) {
            Ok(manifest) => manifest,
            Err(error) => {
                let error = BackupPackageUsecaseError::from_store(error);
                logger.write_product(product_event(
                    "backup.package.failed",
                    &parsed,
                    None,
                    Some(error.code()),
                ));
                return Err(error);
            }
        };
        let summary = BackupPackageSummary::from_manifest(&manifest);
        logger.write_product(product_event(
            "backup.package.created",
            &parsed,
            Some(&summary),
            None,
        ));
        Ok(CreateBackupPackageOutput {
            package_id: parsed.package_id.as_str().to_string(),
            summary,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PreviewBackupRestoreUsecase;

impl PreviewBackupRestoreUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: PreviewBackupRestoreInput,
        store: &mut impl BackupPackageStore,
        logger: &mut impl BackupPackageUsecaseLogger,
    ) -> Result<PreviewBackupRestoreOutput, BackupPackageUsecaseError> {
        let parsed = parse_input(input.actor_user_id, input.workspace_id, input.package_id)?;
        let previewing = RestoreWorkflowStateMachine::transition(
            RestoreState::Requested,
            RestoreEvent::StartPreview,
        )
        .expect("requested preview transition is fixed");
        let manifest = match store.inspect_manifest(&parsed.workspace_id, &parsed.package_id) {
            Ok(manifest) => manifest,
            Err(error) => {
                let error = BackupPackageUsecaseError::from_store(error);
                logger.write_product(product_event(
                    "restore.preview.failed",
                    &parsed,
                    None,
                    Some(error.code()),
                ));
                return Err(error);
            }
        };
        let validating = RestoreWorkflowStateMachine::transition(
            previewing.next_state(),
            RestoreEvent::PreviewBuilt,
        )
        .expect("preview built transition is fixed");
        let validation = store
            .validate_package(&parsed.workspace_id, &parsed.package_id, &manifest)
            .map_err(BackupPackageUsecaseError::from_store)?;
        let summary = BackupPackageSummary::from_manifest(&manifest);
        let (event, event_name, validation_error_code) = if validation.is_valid() {
            (
                RestoreEvent::ValidationPassed,
                "restore.preview.ready",
                None,
            )
        } else {
            (
                RestoreEvent::ValidationFailed,
                "restore.preview.failed",
                validation.error_code(),
            )
        };
        let final_transition =
            RestoreWorkflowStateMachine::transition(validating.next_state(), event)
                .expect("validation transition is fixed");
        logger.write_product(product_event(
            event_name,
            &parsed,
            Some(&summary),
            validation_error_code,
        ));
        Ok(PreviewBackupRestoreOutput {
            package_id: parsed.package_id.as_str().to_string(),
            state: final_transition.next_state(),
            summary,
            validation_error_code,
        })
    }
}

struct ParsedInput {
    workspace_id: WorkspaceId,
    package_id: BackupJobId,
}

fn parse_input(
    actor_user_id: String,
    workspace_id: String,
    package_id: String,
) -> Result<ParsedInput, BackupPackageUsecaseError> {
    UserId::new(&actor_user_id).map_err(|_| BackupPackageUsecaseError::InvalidInput)?;
    Ok(ParsedInput {
        workspace_id: WorkspaceId::new(&workspace_id)
            .map_err(|_| BackupPackageUsecaseError::InvalidInput)?,
        package_id: BackupJobId::new(&package_id)
            .map_err(|_| BackupPackageUsecaseError::InvalidInput)?,
    })
}

fn product_event(
    event_name: &'static str,
    input: &ParsedInput,
    summary: Option<&BackupPackageSummary>,
    error_code: Option<&'static str>,
) -> BackupPackageProductEvent {
    BackupPackageProductEvent {
        event_name,
        workspace_id: input.workspace_id.as_str().to_string(),
        package_id: input.package_id.as_str().to_string(),
        entry_count: summary.map(BackupPackageSummary::entry_count),
        record_count: summary.map(|summary| {
            summary
                .authoritative_record_count()
                .saturating_add(summary.rebuildable_record_count())
        }),
        error_code,
    }
}
