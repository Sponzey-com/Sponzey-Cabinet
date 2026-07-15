use std::fs;
use std::path::{Path, PathBuf};

use cabinet_adapters::durable_backup_package_store::{
    LocalBackupPackagePolicy, LocalBackupPackageStore,
};
use cabinet_adapters::local_backup_store::LocalBackupStore;
use cabinet_usecases::backup_package_operation::{
    BackupPackageOperationEvent, BackupPackageOperationLogger, GetBackupPackageOperationInput,
    GetBackupPackageOperationUsecase, RunBackupPackageOperationInput,
    RunBackupPackageOperationUsecase, StartBackupPackageOperationInput,
    StartBackupPackageOperationUsecase,
};

#[test]
fn completed_package_operation_survives_repository_restart() {
    let root = temp_root();
    seed_workspace(&root, "workspace-1");
    let policy = LocalBackupPackagePolicy::new(10_000, 1024 * 1024).unwrap();
    let mut logger = Sink;
    let mut jobs = LocalBackupStore::new(root.clone());
    StartBackupPackageOperationUsecase::new()
        .execute(
            StartBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
            &mut jobs,
            &mut logger,
        )
        .unwrap();
    let mut packages = LocalBackupPackageStore::new(root.clone(), policy);
    RunBackupPackageOperationUsecase::new()
        .execute(
            RunBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
            &mut jobs,
            &mut packages,
            &mut logger,
        )
        .unwrap();

    let restarted = LocalBackupStore::new(root.clone());
    let status = GetBackupPackageOperationUsecase::new()
        .execute(
            GetBackupPackageOperationInput::new("user-1", "workspace-1", "backup-op-1"),
            &restarted,
        )
        .unwrap();
    assert_eq!(format!("{:?}", status.state()), "Completed");
    assert_eq!(status.progress_completed_units(), 1);
    let _ = fs::remove_dir_all(root);
}

struct Sink;
impl BackupPackageOperationLogger for Sink {
    fn write_product(&mut self, _: BackupPackageOperationEvent) {}
}

fn seed_workspace(root: &Path, workspace: &str) {
    let encoded = hex(workspace);
    for relative in [
        "authoring-current",
        "authoring-versions",
        "canvases",
        "assets/metadata",
        "assets/objects",
        "assets/associations",
    ] {
        let directory = root.join(relative).join(&encoded);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("record.data"), relative).unwrap();
    }
}
fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn temp_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "cabinet-backup-operation-restart-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
