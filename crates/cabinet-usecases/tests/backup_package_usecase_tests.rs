use cabinet_domain::backup::{
    BackupDataClass, BackupJobId, BackupManifestEntry, BackupPackageManifest, RestoreState,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_package::{
    BackupPackageStore, BackupPackageStoreError, BackupPackageValidation,
};
use cabinet_usecases::backup_package::{
    BackupPackageProductEvent, BackupPackageUsecaseError, BackupPackageUsecaseLogger,
    CreateBackupPackageInput, CreateBackupPackageUsecase, PreviewBackupRestoreInput,
    PreviewBackupRestoreUsecase,
};

#[test]
fn create_backup_package_returns_manifest_summary_and_safe_product_event() {
    let mut store = FakePackageStore::valid();
    let mut logger = RecordingLogger::default();

    let output = CreateBackupPackageUsecase::new()
        .execute(
            CreateBackupPackageInput::new("user-1", "workspace-1", "package-1"),
            &mut store,
            &mut logger,
        )
        .expect("package created");

    assert_eq!(output.package_id(), "package-1");
    assert_eq!(output.schema_version(), 1);
    assert_eq!(output.entry_count(), 8);
    assert_eq!(
        output.summary().created_at_epoch_ms(),
        Some(1_784_064_000_000)
    );
    assert_eq!(output.authoritative_record_count(), 87);
    assert_eq!(output.rebuildable_record_count(), 20);
    assert_eq!(output.summary().entries().len(), 8);
    assert_eq!(
        output.summary().entries()[0].data_class(),
        BackupDataClass::CurrentDocuments
    );
    assert_eq!(output.summary().entries()[0].record_count(), 10);
    assert_eq!(output.summary().entries()[0].byte_count(), 160);
    assert_eq!(store.calls, vec!["build"]);
    assert_eq!(logger.events.len(), 1);
    assert_eq!(logger.events[0].event_name(), "backup.package.created");
    assert!(!format!("{:?}", logger.events[0]).contains(CHECKSUM));
}

#[test]
fn create_backup_package_rejects_invalid_input_and_maps_storage_failure() {
    let mut store = FakePackageStore::valid();
    let mut logger = RecordingLogger::default();
    assert_eq!(
        CreateBackupPackageUsecase::new().execute(
            CreateBackupPackageInput::new("", "workspace-1", "package-1"),
            &mut store,
            &mut logger,
        ),
        Err(BackupPackageUsecaseError::InvalidInput)
    );
    assert!(store.calls.is_empty());

    store.build_error = Some(BackupPackageStoreError::StorageUnavailable);
    assert_eq!(
        CreateBackupPackageUsecase::new().execute(
            CreateBackupPackageInput::new("user-1", "workspace-1", "package-1"),
            &mut store,
            &mut logger,
        ),
        Err(BackupPackageUsecaseError::StorageUnavailable)
    );
    assert_eq!(logger.events[0].event_name(), "backup.package.failed");
}

#[test]
fn restore_preview_inspects_then_validates_and_becomes_confirmation_ready() {
    let mut store = FakePackageStore::valid();
    let mut logger = RecordingLogger::default();

    let output = PreviewBackupRestoreUsecase::new()
        .execute(
            PreviewBackupRestoreInput::new("user-1", "workspace-1", "package-1"),
            &mut store,
            &mut logger,
        )
        .expect("restore preview");

    assert_eq!(store.calls, vec!["inspect", "validate"]);
    assert_eq!(output.state(), RestoreState::AwaitingConfirmation);
    assert!(output.confirmation_ready());
    assert_eq!(output.summary().entry_count(), 8);
    assert_eq!(logger.events[0].event_name(), "restore.preview.ready");
}

#[test]
fn restore_preview_returns_failed_state_for_checksum_validation_failure() {
    let mut store = FakePackageStore::valid();
    store.validation = BackupPackageValidation::failed("BACKUP_PACKAGE_CHECKSUM_MISMATCH");
    let mut logger = RecordingLogger::default();

    let output = PreviewBackupRestoreUsecase::new()
        .execute(
            PreviewBackupRestoreInput::new("user-1", "workspace-1", "package-1"),
            &mut store,
            &mut logger,
        )
        .expect("invalid package is a preview result");

    assert_eq!(output.state(), RestoreState::Failed);
    assert!(!output.confirmation_ready());
    assert_eq!(
        output.validation_error_code(),
        Some("BACKUP_PACKAGE_CHECKSUM_MISMATCH")
    );
    assert_eq!(logger.events[0].event_name(), "restore.preview.failed");
}

#[test]
fn restore_preview_maps_missing_package_without_attempting_validation() {
    let mut store = FakePackageStore::valid();
    store.inspect_error = Some(BackupPackageStoreError::PackageNotFound);
    let mut logger = RecordingLogger::default();

    let result = PreviewBackupRestoreUsecase::new().execute(
        PreviewBackupRestoreInput::new("user-1", "workspace-1", "missing-package"),
        &mut store,
        &mut logger,
    );

    assert_eq!(result, Err(BackupPackageUsecaseError::PackageNotFound));
    assert_eq!(store.calls, vec!["inspect"]);
    assert_eq!(logger.events[0].event_name(), "restore.preview.failed");
}

const CHECKSUM: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

struct FakePackageStore {
    manifest: BackupPackageManifest,
    validation: BackupPackageValidation,
    build_error: Option<BackupPackageStoreError>,
    inspect_error: Option<BackupPackageStoreError>,
    calls: Vec<&'static str>,
}

impl FakePackageStore {
    fn valid() -> Self {
        Self {
            manifest: manifest(),
            validation: BackupPackageValidation::valid(),
            build_error: None,
            inspect_error: None,
            calls: Vec::new(),
        }
    }
}

impl BackupPackageStore for FakePackageStore {
    fn build_package(
        &mut self,
        _workspace_id: &WorkspaceId,
        _package_id: &BackupJobId,
    ) -> Result<BackupPackageManifest, BackupPackageStoreError> {
        self.calls.push("build");
        self.build_error
            .map_or_else(|| Ok(self.manifest.clone()), Err)
    }

    fn inspect_manifest(
        &mut self,
        _workspace_id: &WorkspaceId,
        _package_id: &BackupJobId,
    ) -> Result<BackupPackageManifest, BackupPackageStoreError> {
        self.calls.push("inspect");
        self.inspect_error
            .map_or_else(|| Ok(self.manifest.clone()), Err)
    }

    fn discard_package(
        &mut self,
        _workspace_id: &WorkspaceId,
        _package_id: &BackupJobId,
    ) -> Result<(), BackupPackageStoreError> {
        self.calls.push("discard");
        Ok(())
    }

    fn validate_package(
        &mut self,
        _workspace_id: &WorkspaceId,
        _package_id: &BackupJobId,
        _manifest: &BackupPackageManifest,
    ) -> Result<BackupPackageValidation, BackupPackageStoreError> {
        self.calls.push("validate");
        Ok(self.validation.clone())
    }
}

#[derive(Default)]
struct RecordingLogger {
    events: Vec<BackupPackageProductEvent>,
}

impl BackupPackageUsecaseLogger for RecordingLogger {
    fn write_product(&mut self, event: BackupPackageProductEvent) {
        self.events.push(event);
    }
}

fn manifest() -> BackupPackageManifest {
    let values = [
        (BackupDataClass::CurrentDocuments, 10),
        (BackupDataClass::VersionHistory, 30),
        (BackupDataClass::CanvasRecords, 23),
        (BackupDataClass::AssetMetadata, 8),
        (BackupDataClass::AssetObjects, 7),
        (BackupDataClass::AssetAssociations, 9),
        (BackupDataClass::GraphRebuildMetadata, 10),
        (BackupDataClass::SearchRebuildMetadata, 10),
    ];
    BackupPackageManifest::new(
        1,
        values
            .into_iter()
            .map(|(data_class, count)| {
                BackupManifestEntry::new(
                    data_class,
                    data_class.expected_ownership(),
                    count,
                    count * 16,
                    CHECKSUM,
                )
                .expect("entry")
            })
            .collect(),
    )
    .expect("manifest")
    .with_created_at_epoch_ms(1_784_064_000_000)
    .expect("creation time")
}
