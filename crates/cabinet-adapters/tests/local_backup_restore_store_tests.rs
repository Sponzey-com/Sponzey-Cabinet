use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_backup_package_store::{
    LocalBackupPackagePolicy, LocalBackupPackageStore,
};
use cabinet_adapters::local_backup_restore_store::LocalBackupRestoreStore;
use cabinet_domain::backup::{BackupJobId, RestoreState};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_package::BackupPackageStore;
use cabinet_ports::backup_restore::BackupRecoveryStore;
use cabinet_ports::backup_restore::{BackupRestoreStore, BackupRestoreStoreError};

#[test]
fn request_persists_staging_before_copy_and_prepare_resumes_registered_operation() {
    let fixture = Fixture::new("request-resume");
    fixture.seed_snapshot("backup");
    let manifest = fixture.build_package();
    let mut store = fixture.restore_store();

    let requested = store
        .request_restore(&workspace(), &package(), &operation())
        .expect("request");
    assert_eq!(requested.state(), RestoreState::Staging);
    assert_eq!(
        fixture
            .restore_store()
            .get_restore_status(&workspace(), &operation())
            .unwrap()
            .unwrap()
            .state(),
        RestoreState::Staging
    );
    assert_eq!(
        store.request_restore(&workspace(), &package(), &operation()),
        Err(BackupRestoreStoreError::Conflict)
    );

    let prepared = store
        .prepare_restore(&workspace(), &package(), &operation(), &manifest)
        .expect("resume prepare");
    assert_eq!(prepared.state(), RestoreState::Staging);
}

#[test]
fn prepare_requires_valid_package_and_persists_staging_status_across_restart() {
    let fixture = Fixture::new("prepare");
    fixture.seed_snapshot("backup");
    let manifest = fixture.build_package();
    let package_manifest = fixture.package_root().join("manifest.tsv");
    let package_manifest_before = fs::read(&package_manifest).expect("package manifest");

    let snapshot = fixture
        .restore_store()
        .prepare_restore(&workspace(), &package(), &operation(), &manifest)
        .expect("prepare restore");

    assert_eq!(snapshot.state(), RestoreState::Staging);
    assert_eq!(
        fs::read(package_manifest).expect("package unchanged"),
        package_manifest_before
    );
    let restarted = fixture
        .restore_store()
        .get_restore_status(&workspace(), &operation())
        .expect("read status")
        .expect("operation exists");
    assert_eq!(restarted.state(), RestoreState::Staging);
    assert_eq!(restarted.package_id(), &package());

    let object = fixture.package_root().join("data/asset_objects/asset.bin");
    fs::write(object, b"tampered").expect("tamper");
    assert_eq!(
        fixture.restore_store().prepare_restore(
            &workspace(),
            &package(),
            &BackupJobId::new("operation-2").expect("id"),
            &manifest,
        ),
        Err(BackupRestoreStoreError::PackageInvalid)
    );
}

#[test]
fn apply_restores_authoritative_data_removes_stale_projections_and_rollback_restores_previous_data()
{
    let fixture = Fixture::new("apply-rollback");
    fixture.seed_snapshot("backup");
    let manifest = fixture.build_package();
    fixture.seed_snapshot("live-newer");
    let live_before = fixture.live_values();
    let mut store = fixture.restore_store();
    store
        .prepare_restore(&workspace(), &package(), &operation(), &manifest)
        .expect("prepare");

    let applied = store
        .apply_restore(&workspace(), &operation())
        .expect("apply");

    assert_eq!(applied.state(), RestoreState::Reopening);
    for value in fixture.authoritative_values() {
        assert!(String::from_utf8(value).expect("utf8").contains("backup"));
    }
    assert!(!fixture.graph_projection().exists());
    assert!(!fixture.search_projection().exists());

    let rolled_back = store
        .rollback_restore(&workspace(), &operation())
        .expect("rollback");
    assert_eq!(rolled_back.state(), RestoreState::RolledBack);
    assert_eq!(fixture.live_values(), live_before);
    assert!(fixture.graph_projection().exists());
    assert!(fixture.search_projection().exists());
}

#[test]
fn finalize_requires_applied_state_and_preserves_completed_status() {
    let fixture = Fixture::new("finalize");
    fixture.seed_snapshot("backup");
    let manifest = fixture.build_package();
    let mut store = fixture.restore_store();
    store
        .prepare_restore(&workspace(), &package(), &operation(), &manifest)
        .expect("prepare");
    assert_eq!(
        store.finalize_restore(&workspace(), &operation()),
        Err(BackupRestoreStoreError::Conflict)
    );
    store
        .apply_restore(&workspace(), &operation())
        .expect("apply");

    let completed = store
        .finalize_restore(&workspace(), &operation())
        .expect("finalize");

    assert_eq!(completed.state(), RestoreState::Completed);
    assert_eq!(
        fixture
            .restore_store()
            .get_restore_status(&workspace(), &operation())
            .expect("status")
            .expect("completed")
            .state(),
        RestoreState::Completed
    );
}

#[test]
fn cancel_removes_prepared_payload_and_persists_cancelled_status() {
    let fixture = Fixture::new("cancel");
    fixture.seed_snapshot("backup");
    let manifest = fixture.build_package();
    let mut store = fixture.restore_store();
    store
        .prepare_restore(&workspace(), &package(), &operation(), &manifest)
        .expect("prepare");

    let cancelled = store
        .cancel_restore(&workspace(), &operation())
        .expect("cancel");

    assert_eq!(cancelled.state(), RestoreState::Cancelled);
    assert_eq!(
        fixture
            .restore_store()
            .get_restore_status(&workspace(), &operation())
            .expect("status")
            .expect("cancelled")
            .state(),
        RestoreState::Cancelled
    );
    assert!(
        !fixture
            .root
            .join("restore-operations")
            .join(hex("workspace-1"))
            .join(hex("operation-1"))
            .join("staged")
            .exists()
    );
}

#[test]
fn startup_recovery_rolls_back_interrupted_reopen_cleans_temps_and_is_idempotent() {
    let fixture = Fixture::new("startup-recovery");
    fixture.seed_snapshot("backup");
    let manifest = fixture.build_package();
    fixture.seed_snapshot("live-newer");
    let live_before = fixture.live_values();
    let mut store = fixture.restore_store();
    store
        .prepare_restore(&workspace(), &package(), &operation(), &manifest)
        .expect("prepare");
    store
        .apply_restore(&workspace(), &operation())
        .expect("apply");
    let ws = hex("workspace-1");
    fs::create_dir_all(
        fixture
            .root
            .join("backup-packages")
            .join(&ws)
            .join(".stale.staging"),
    )
    .expect("stale package staging");
    fs::create_dir_all(
        fixture
            .root
            .join("restore-operations")
            .join(&ws)
            .join(".stale.preparing"),
    )
    .expect("stale restore preparing");

    let report = store
        .recover_startup(&workspace())
        .expect("recover startup");

    assert_eq!(report.cleaned_staging_count(), 2);
    assert_eq!(report.rolled_back_operation_ids(), &["operation-1"]);
    assert_eq!(fixture.live_values(), live_before);
    let second = fixture
        .restore_store()
        .recover_startup(&workspace())
        .expect("idempotent recovery");
    assert_eq!(second.cleaned_staging_count(), 0);
    assert!(second.rolled_back_operation_ids().is_empty());
}

#[test]
fn startup_recovery_retries_durable_recovery_required_operation() {
    let fixture = Fixture::new("recovery-required-restart");
    fixture.seed_snapshot("backup");
    let manifest = fixture.build_package();
    fixture.seed_snapshot("live-newer");
    let live_before = fixture.live_values();
    let mut store = fixture.restore_store();
    store
        .prepare_restore(&workspace(), &package(), &operation(), &manifest)
        .expect("prepare");
    store
        .apply_restore(&workspace(), &operation())
        .expect("apply");
    store
        .mark_recovery_required(&workspace(), &operation())
        .expect("persist recovery required");

    let report = fixture
        .restore_store()
        .recover_startup(&workspace())
        .expect("restart recovery");

    assert_eq!(report.rolled_back_operation_ids(), &["operation-1"]);
    assert_eq!(fixture.live_values(), live_before);
    assert_eq!(
        fixture
            .restore_store()
            .get_restore_status(&workspace(), &operation())
            .expect("status")
            .expect("operation")
            .state(),
        RestoreState::RolledBack
    );
}

#[test]
fn startup_recovery_fails_closed_before_mutation_when_expected_rollback_slot_is_missing() {
    let fixture = Fixture::new("missing-rollback-slot");
    fixture.seed_snapshot("backup");
    let manifest = fixture.build_package();
    fixture.seed_snapshot("live-newer");
    let mut store = fixture.restore_store();
    store
        .prepare_restore(&workspace(), &package(), &operation(), &manifest)
        .expect("prepare");
    store
        .apply_restore(&workspace(), &operation())
        .expect("apply");
    store
        .mark_recovery_required(&workspace(), &operation())
        .expect("recovery required");
    let applied_values = fixture.authoritative_values();
    fs::remove_dir_all(fixture.operation_root().join("rollback/current_documents"))
        .expect("remove expected rollback slot");

    let report = fixture
        .restore_store()
        .recover_startup(&workspace())
        .expect("isolate corrupt rollback");

    assert!(report.rolled_back_operation_ids().is_empty());
    assert_eq!(report.cleanup_required_operation_ids(), &["operation-1"]);
    assert_eq!(fixture.authoritative_values(), applied_values);
    assert_eq!(
        fixture
            .restore_store()
            .get_restore_status(&workspace(), &operation())
            .unwrap()
            .unwrap()
            .state(),
        RestoreState::RecoveryRequired
    );
}

#[cfg(unix)]
#[test]
fn startup_recovery_rejects_rollback_symlink_without_touching_outside_or_live_data() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new("rollback-symlink");
    fixture.seed_snapshot("backup");
    let manifest = fixture.build_package();
    fixture.seed_snapshot("live-newer");
    let mut store = fixture.restore_store();
    store
        .prepare_restore(&workspace(), &package(), &operation(), &manifest)
        .expect("prepare");
    store
        .apply_restore(&workspace(), &operation())
        .expect("apply");
    store
        .mark_recovery_required(&workspace(), &operation())
        .expect("recovery required");
    let applied_values = fixture.authoritative_values();
    let outside = fixture.root.join("outside-protected");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(outside.join("sentinel"), b"outside-safe").expect("sentinel");
    let rollback = fixture.operation_root().join("rollback/current_documents");
    fs::remove_dir_all(&rollback).expect("remove rollback");
    symlink(&outside, &rollback).expect("rollback symlink");

    let report = fixture
        .restore_store()
        .recover_startup(&workspace())
        .expect("isolate symlink");

    assert!(report.rolled_back_operation_ids().is_empty());
    assert_eq!(report.cleanup_required_operation_ids(), &["operation-1"]);
    assert_eq!(fixture.authoritative_values(), applied_values);
    assert_eq!(fs::read(outside.join("sentinel")).unwrap(), b"outside-safe");
}

#[test]
fn startup_recovery_isolates_legacy_ambiguous_marker_without_mutation() {
    let fixture = Fixture::new("legacy-rollback-marker");
    fixture.seed_snapshot("backup");
    let manifest = fixture.build_package();
    fixture.seed_snapshot("live-newer");
    let mut store = fixture.restore_store();
    store
        .prepare_restore(&workspace(), &package(), &operation(), &manifest)
        .expect("prepare");
    store
        .apply_restore(&workspace(), &operation())
        .expect("apply");
    store
        .mark_recovery_required(&workspace(), &operation())
        .expect("recovery required");
    let applied_values = fixture.authoritative_values();
    fs::write(
        fixture
            .operation_root()
            .join("journal/current_documents.applied"),
        b"applied\n",
    )
    .expect("legacy marker");

    let report = fixture
        .restore_store()
        .recover_startup(&workspace())
        .expect("isolate legacy marker");

    assert!(report.rolled_back_operation_ids().is_empty());
    assert_eq!(report.cleanup_required_operation_ids(), &["operation-1"]);
    assert_eq!(fixture.authoritative_values(), applied_values);
}

#[test]
fn startup_recovery_preserves_prepared_operation_and_isolates_corrupt_status() {
    let fixture = Fixture::new("recovery-isolation");
    fixture.seed_snapshot("backup");
    let manifest = fixture.build_package();
    let mut store = fixture.restore_store();
    store
        .prepare_restore(&workspace(), &package(), &operation(), &manifest)
        .expect("prepared operation");
    let corrupt_id = BackupJobId::new("corrupt-operation").expect("id");
    let corrupt_root = fixture
        .root
        .join("restore-operations")
        .join(hex("workspace-1"))
        .join(hex(corrupt_id.as_str()));
    fs::create_dir_all(&corrupt_root).expect("corrupt operation root");
    fs::write(corrupt_root.join("status.tsv"), b"broken").expect("corrupt status");

    let report = store
        .recover_startup(&workspace())
        .expect("isolated recovery");

    assert_eq!(
        report.cleanup_required_operation_ids(),
        &["corrupt-operation"]
    );
    assert_eq!(
        store
            .get_restore_status(&workspace(), &operation())
            .expect("status")
            .expect("prepared remains")
            .state(),
        RestoreState::Staging
    );
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cabinet-restore-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("root");
        Self { root }
    }

    fn policy(&self) -> LocalBackupPackagePolicy {
        LocalBackupPackagePolicy::new(100, 1024 * 1024).expect("policy")
    }

    fn package_store(&self) -> LocalBackupPackageStore {
        LocalBackupPackageStore::new(self.root.clone(), self.policy())
    }

    fn restore_store(&self) -> LocalBackupRestoreStore {
        LocalBackupRestoreStore::new(self.root.clone(), self.policy())
    }

    fn operation_root(&self) -> PathBuf {
        self.root
            .join("restore-operations")
            .join(hex("workspace-1"))
            .join(hex("operation-1"))
    }

    fn build_package(&self) -> cabinet_domain::backup::BackupPackageManifest {
        self.package_store()
            .build_package(&workspace(), &package())
            .expect("package")
    }

    fn seed_snapshot(&self, marker: &str) {
        let ws = hex("workspace-1");
        for relative in [
            "authoring-current/workspace-1/document.txt".to_string(),
            format!("document-current-pointers/{ws}/pointer.txt"),
            "document-versions/workspace-1/version.txt".to_string(),
            format!("canvases/{ws}/canvas.txt"),
            format!("assets/metadata/{ws}/asset.asset"),
            format!("assets/objects/{ws}/asset.bin"),
            format!("assets/associations/{ws}/asset.link"),
            format!("graph-projections/{ws}/graph.snapshot"),
            format!("search-projections/{ws}.snapshot"),
        ] {
            let path = self.root.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("dir");
            fs::write(path, format!("{marker}-data")).expect("fixture");
        }
    }

    fn authoritative_values(&self) -> Vec<Vec<u8>> {
        let ws = hex("workspace-1");
        [
            "authoring-current/workspace-1/document.txt".to_string(),
            format!("document-current-pointers/{ws}/pointer.txt"),
            "document-versions/workspace-1/version.txt".to_string(),
            format!("canvases/{ws}/canvas.txt"),
            format!("assets/metadata/{ws}/asset.asset"),
            format!("assets/objects/{ws}/asset.bin"),
            format!("assets/associations/{ws}/asset.link"),
        ]
        .into_iter()
        .map(|path| fs::read(self.root.join(path)).expect("live value"))
        .collect()
    }

    fn live_values(&self) -> Vec<Vec<u8>> {
        let mut values = self.authoritative_values();
        values.push(fs::read(self.graph_projection()).expect("graph"));
        values.push(fs::read(self.search_projection()).expect("search"));
        values
    }

    fn graph_projection(&self) -> PathBuf {
        self.root
            .join("graph-projections")
            .join(hex("workspace-1"))
            .join("graph.snapshot")
    }

    fn search_projection(&self) -> PathBuf {
        self.root
            .join("search-projections")
            .join(format!("{}.snapshot", hex("workspace-1")))
    }

    fn package_root(&self) -> PathBuf {
        self.root
            .join("backup-packages")
            .join(hex("workspace-1"))
            .join(hex("package-1"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace")
}

fn package() -> BackupJobId {
    BackupJobId::new("package-1").expect("package")
}

fn operation() -> BackupJobId {
    BackupJobId::new("operation-1").expect("operation")
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
