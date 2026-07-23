use std::fs;
use std::path::{Path, PathBuf};

use cabinet_adapters::durable_projection_work_repository::DurableProjectionWorkRepository;
use cabinet_desktop_shell::{
    DesktopBackupCatalogRequestDto, DesktopBackupPackageRequestDto, DesktopBackupProductEvent,
    DesktopBackupRecoveryRequestDto, DesktopBackupRecoveryRuntime,
    DesktopDocumentMutationRequestDto, DesktopDocumentMutationRuntime,
    DesktopRestoreCancelRequestDto, DesktopRestoreConfirmRequestDto,
};
use cabinet_domain::projection_work::ProjectionChangeKind;
use cabinet_ports::projection_work::ProjectionWorkRepository;

#[test]
fn native_runtime_creates_and_previews_ui_safe_complete_manifest() {
    let root = temp_root("create-preview");
    seed_workspace(&root, "workspace-1");
    let runtime = runtime(root.clone());

    let created = runtime.create(DesktopBackupPackageRequestDto {
        workspace_id: "workspace-1".into(),
        package_id: "package-1".into(),
    });
    let preview = runtime.preview(DesktopBackupPackageRequestDto {
        workspace_id: "workspace-1".into(),
        package_id: "package-1".into(),
    });

    assert!(created.ok);
    assert_eq!(created.state, "Ready");
    let manifest = created.manifest.expect("manifest");
    assert!(manifest.created_at_epoch_ms.is_some_and(|value| value > 0));
    assert_eq!(manifest.entries.len(), 8);
    assert_eq!(manifest.entries[0].data_class, "current_documents");
    assert_eq!(manifest.entries[6].data_class, "graph_rebuild_metadata");
    assert!(preview.ok);
    assert_eq!(preview.state, "AwaitingConfirmation");
    assert_eq!(preview.confirmation_ready, Some(true));

    let json = serde_json::to_string(&preview).unwrap();
    for prohibited in [
        "checksum",
        "documentBody",
        "objectBytes",
        root.to_str().unwrap(),
    ] {
        assert!(!json.contains(prohibited), "response leaked {prohibited}");
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_runtime_lists_bounded_backup_catalog_after_restart() {
    let root = temp_root("catalog");
    seed_workspace(&root, "workspace-1");
    let initial_runtime = runtime(root.clone());
    for package_id in ["package-1", "package-2"] {
        assert!(
            initial_runtime
                .create(DesktopBackupPackageRequestDto {
                    workspace_id: "workspace-1".into(),
                    package_id: package_id.into(),
                })
                .ok
        );
    }
    drop(initial_runtime);

    let page = runtime(root.clone()).list_catalog(DesktopBackupCatalogRequestDto {
        workspace_id: "workspace-1".into(),
        cursor: None,
        limit: 1,
    });
    assert!(page.ok);
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.next_cursor.as_deref(), Some("1"));
    let json = serde_json::to_string(&page).unwrap();
    assert!(!json.contains("checksum"));
    assert!(!json.contains(root.to_str().unwrap()));
}

#[test]
fn confirmation_is_required_before_restore_io_and_confirmed_restore_completes() {
    let root = temp_root("confirm");
    seed_workspace(&root, "workspace-1");
    let runtime = runtime(root.clone());
    let package = DesktopBackupPackageRequestDto {
        workspace_id: "workspace-1".into(),
        package_id: "package-1".into(),
    };
    assert!(runtime.create(package).ok);

    let rejected = runtime.confirm(DesktopRestoreConfirmRequestDto {
        workspace_id: "workspace-1".into(),
        package_id: "package-1".into(),
        operation_id: "operation-rejected".into(),
        confirmed: false,
    });
    assert!(!rejected.ok);
    assert_eq!(
        rejected.error_code.as_deref(),
        Some("RESTORE_CONFIRMATION_REQUIRED")
    );
    assert!(!root.join("restore-operations").exists());

    let completed = runtime.confirm(DesktopRestoreConfirmRequestDto {
        workspace_id: "workspace-1".into(),
        package_id: "package-1".into(),
        operation_id: "operation-1".into(),
        confirmed: true,
    });
    assert!(completed.ok);
    assert_eq!(completed.state, "Completed");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn startup_recovery_and_cancel_return_stable_operation_results() {
    let root = temp_root("recovery");
    seed_workspace(&root, "workspace-1");
    let runtime = runtime(root.clone());

    let recovery = runtime.recover_startup(DesktopBackupRecoveryRequestDto {
        workspace_id: "workspace-1".into(),
    });
    assert!(recovery.ok);
    assert_eq!(recovery.state, "Completed");
    assert_eq!(recovery.recovery.unwrap().cleaned_staging_count, 0);

    let cancelled = runtime.cancel(DesktopRestoreCancelRequestDto {
        workspace_id: "workspace-1".into(),
        operation_id: "missing-operation".into(),
    });
    assert!(!cancelled.ok);
    assert_eq!(
        cancelled.error_code.as_deref(),
        Some("RESTORE_STORAGE_UNAVAILABLE")
    );
    assert!(cancelled.retryable);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn restore_runtime_starts_statuses_and_runs_a_durable_operation() {
    let root = temp_root("async-restore");
    seed_workspace(&root, "workspace-1");
    let runtime = runtime(root.clone());
    let authoring = DesktopDocumentMutationRuntime::new(root.clone(), 1024 * 1024).unwrap();
    assert!(
        authoring
            .execute(DesktopDocumentMutationRequestDto::Create {
                operation_id: "document-create".into(),
                workspace_id: "workspace-1".into(),
                document_id: "doc-1".into(),
                body: "body".into(),
                author: "local-user".into(),
                summary: "created".into(),
            })
            .ok
    );
    assert!(
        runtime
            .create(DesktopBackupPackageRequestDto {
                workspace_id: "workspace-1".into(),
                package_id: "package-1".into(),
            })
            .ok
    );
    let request = DesktopRestoreConfirmRequestDto {
        workspace_id: "workspace-1".into(),
        package_id: "package-1".into(),
        operation_id: "operation-1".into(),
        confirmed: true,
    };

    let started = runtime.start_restore_operation(request.clone());
    assert!(started.ok);
    assert_eq!(started.state, "Staging");
    let status = runtime.restore_operation_status(DesktopRestoreCancelRequestDto {
        workspace_id: "workspace-1".into(),
        operation_id: "operation-1".into(),
    });
    assert_eq!(status.state, "Staging");
    let completed = runtime.run_restore_operation(request);
    assert!(completed.ok);
    assert_eq!(
        completed.state, "Completed",
        "restore error={:?}",
        completed.error_code
    );
    let works = DurableProjectionWorkRepository::new(root.clone())
        .list_resumable(10)
        .unwrap();
    let restored_count = works
        .iter()
        .filter(|work| work.identity().change_kind() == ProjectionChangeKind::Restored)
        .count();
    assert_eq!(restored_count, 0, "resumable works: {works:?}");
    assert!(runtime.product_events().iter().any(|event| matches!(event,
        DesktopBackupProductEvent::Operation { event_name, state, error_code }
            if event_name == "restore.projection_rebuild.requested" && state == "Pending" && error_code.is_none()
    )));
    assert!(runtime.product_events().iter().any(|event| matches!(event,
        DesktopBackupProductEvent::Operation { event_name, state, error_code }
            if event_name == "restore.projection_rebuild.completed" && state == "Completed" && error_code.is_none()
    )));
    let _ = fs::remove_dir_all(root);
}

fn runtime(root: PathBuf) -> DesktopBackupRecoveryRuntime {
    DesktopBackupRecoveryRuntime::new(root, 10_000, 1024 * 1024 * 1024).unwrap()
}

fn seed_workspace(root: &Path, workspace: &str) {
    let encoded = hex(workspace);
    for relative in ["authoring-current", "document-versions"] {
        let directory = root.join(relative).join(workspace);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("record.data"), format!("{relative}-data")).unwrap();
    }
    let pointer_directory = root.join("document-current-pointers").join(&encoded);
    fs::create_dir_all(&pointer_directory).unwrap();
    fs::write(pointer_directory.join("record.data"), "pointer-data").unwrap();
    for relative in [
        "canvases",
        "assets/metadata",
        "assets/objects",
        "assets/associations",
    ] {
        let directory = root.join(relative).join(&encoded);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("record.data"), format!("{relative}-data")).unwrap();
    }
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn temp_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cabinet-backup-runtime-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
