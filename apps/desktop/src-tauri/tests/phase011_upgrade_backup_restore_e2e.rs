use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_projection_work_repository::DurableProjectionWorkRepository;
use cabinet_adapters::local_document_store_migration::LocalDocumentStoreMigration;
use cabinet_adapters::local_migration::LocalMigrationStore;
use cabinet_core::migration::{MigrationPlan, MigrationRunner, MigrationState};
use cabinet_desktop_shell::{
    DesktopAssetDetailRequestDto, DesktopAssetImportRequestDto, DesktopAssetImportSelectionRuntime,
    DesktopBackupOperationRequestDto, DesktopBackupPackageRequestDto, DesktopBackupRecoveryRuntime,
    DesktopCanvasRequestDto, DesktopCanvasRuntime, DesktopDocumentAssetsRuntime,
    DesktopDocumentAuthoringRequestDto, DesktopDocumentAuthoringRuntime,
    DesktopDocumentQueryRequestDto, DesktopDocumentQueryRuntime, DesktopRestoreConfirmRequestDto,
};
use cabinet_domain::document::DocumentBodyPolicy;
use cabinet_domain::projection_work::ProjectionChangeKind;
use cabinet_ports::projection_work::ProjectionWorkRepository;

const WORKSPACE: &str = "workspace-1";
const DOCUMENT: &str = "phase011-document";
const BODY_LIMIT: usize = 4 * 1024 * 1024;

#[test]
fn phase011_fixture_upgrades_extends_backs_up_restores_and_reopens() {
    let source = TempRoot::new("phase011-source");
    seed_phase011_authoring_fixture(&source.path);
    let source_before = fingerprint(&source.path);

    let runtime_root = TempRoot::new("phase012-upgrade-runtime");
    copy_tree(&source.path, &runtime_root.path);
    assert_eq!(fingerprint(&source.path), source_before);

    let migration = MigrationRunner::new(MigrationPlan::initial());
    let metadata = runtime_root.path.join("metadata");
    let first_migration = migration.run(&mut LocalMigrationStore::new(metadata.clone()));
    let repeated_migration = migration.run(&mut LocalMigrationStore::new(metadata));
    assert_eq!(first_migration.final_state, MigrationState::Completed);
    assert_eq!(repeated_migration.final_state, MigrationState::Completed);
    assert!(repeated_migration.applied_versions.is_empty());
    LocalDocumentStoreMigration::new(
        runtime_root.path.clone(),
        DocumentBodyPolicy::new(BODY_LIMIT).expect("body policy"),
    )
    .execute()
    .expect("authoritative document migration");

    let query = DesktopDocumentQueryRuntime::new(runtime_root.path.clone(), BODY_LIMIT)
        .expect("document query runtime");
    let current = query.execute(DesktopDocumentQueryRequestDto::Current {
        workspace_id: WORKSPACE.into(),
        document_id: DOCUMENT.into(),
    });
    assert!(current.ok, "current error={:?}", current.error_code);
    assert_eq!(
        current
            .data
            .as_ref()
            .and_then(|data| data.current_version_token.as_deref()),
        Some("phase011-v2")
    );
    let history = query.execute(DesktopDocumentQueryRequestDto::History {
        workspace_id: WORKSPACE.into(),
        document_id: DOCUMENT.into(),
        cursor: None,
        limit: 10,
    });
    assert!(history.ok, "history error={:?}", history.error_code);
    assert_eq!(
        history.data.as_ref().map(|data| data.entries.len()),
        Some(2)
    );

    let source_asset = runtime_root.path.join("selected-asset.bin");
    let expected_asset_bytes = b"phase012 sanitized asset object";
    fs::write(&source_asset, expected_asset_bytes).expect("asset fixture");
    let importer = DesktopAssetImportSelectionRuntime::with_app_data_root(
        runtime_root.path.clone(),
        WORKSPACE,
        4,
    )
    .expect("asset importer");
    let selected = importer.register_selected_paths(vec![source_asset]);
    let imported = importer.import(DesktopAssetImportRequestDto {
        workspace_id: WORKSPACE.into(),
        document_id: DOCUMENT.into(),
        handle: selected.data.expect("selection").files[0].handle.clone(),
        label: "Upgrade attachment".into(),
    });
    assert!(imported.ok, "import error={:?}", imported.error_code);
    let asset_id = imported.asset_id.expect("asset identity");

    let canvas = DesktopCanvasRuntime::new(runtime_root.path.clone()).expect("canvas runtime");
    assert!(
        canvas
            .execute(DesktopCanvasRequestDto::Create {
                workspace_id: WORKSPACE.into(),
                canvas_id: "upgrade-canvas".into(),
                title: "Upgrade map".into(),
            })
            .ok
    );
    assert!(
        canvas
            .execute(DesktopCanvasRequestDto::AddDocumentNode {
                workspace_id: WORKSPACE.into(),
                canvas_id: "upgrade-canvas".into(),
                expected_revision: 1,
                node_id: "document-node".into(),
                document_id: DOCUMENT.into(),
                x: 20,
                y: 20,
                width: 320,
                height: 180,
                operation_id: "canvas-document".into(),
            })
            .ok
    );
    assert!(
        canvas
            .execute(DesktopCanvasRequestDto::AddAssetNode {
                workspace_id: WORKSPACE.into(),
                canvas_id: "upgrade-canvas".into(),
                expected_revision: 2,
                node_id: "asset-node".into(),
                asset_id: asset_id.clone(),
                x: 380,
                y: 20,
                width: 320,
                height: 180,
                operation_id: "canvas-asset".into(),
            })
            .ok
    );

    let backup =
        DesktopBackupRecoveryRuntime::new(runtime_root.path.clone(), 100_000, 1024 * 1024 * 1024)
            .expect("backup runtime");
    let backup_request = DesktopBackupOperationRequestDto {
        workspace_id: WORKSPACE.into(),
        operation_id: "upgrade-backup".into(),
    };
    assert_eq!(
        backup.start_operation(backup_request.clone()).state,
        "Queued"
    );
    assert_eq!(backup.run_operation(backup_request).state, "Completed");
    let preview = backup.preview(DesktopBackupPackageRequestDto {
        workspace_id: WORKSPACE.into(),
        package_id: "upgrade-backup".into(),
    });
    assert!(preview.ok && preview.confirmation_ready == Some(true));
    assert_eq!(
        preview.manifest.as_ref().map(|value| value.entries.len()),
        Some(8)
    );

    remove_authoritative_workspace_data(&runtime_root.path);
    assert!(
        !DesktopDocumentQueryRuntime::new(runtime_root.path.clone(), BODY_LIMIT)
            .expect("mutated query")
            .execute(DesktopDocumentQueryRequestDto::Current {
                workspace_id: WORKSPACE.into(),
                document_id: DOCUMENT.into(),
            })
            .ok
    );

    let restore = DesktopRestoreConfirmRequestDto {
        workspace_id: WORKSPACE.into(),
        package_id: "upgrade-backup".into(),
        operation_id: "upgrade-restore".into(),
        confirmed: true,
    };
    assert_eq!(
        backup.start_restore_operation(restore.clone()).state,
        "Staging"
    );
    let restored = backup.run_restore_operation(restore);
    assert!(restored.ok, "restore error={:?}", restored.error_code);
    assert_eq!(restored.state, "Completed");

    let reopened_query = DesktopDocumentQueryRuntime::new(runtime_root.path.clone(), BODY_LIMIT)
        .expect("reopened query");
    let reopened_current = reopened_query.execute(DesktopDocumentQueryRequestDto::Current {
        workspace_id: WORKSPACE.into(),
        document_id: DOCUMENT.into(),
    });
    assert!(reopened_current.ok);
    assert_eq!(
        reopened_current
            .data
            .as_ref()
            .and_then(|data| data.current_version_token.as_deref()),
        Some("phase011-v2")
    );
    let reopened_history = reopened_query.execute(DesktopDocumentQueryRequestDto::History {
        workspace_id: WORKSPACE.into(),
        document_id: DOCUMENT.into(),
        cursor: None,
        limit: 10,
    });
    assert_eq!(
        reopened_history
            .data
            .as_ref()
            .map(|data| data.entries.len()),
        Some(2)
    );

    let reopened_canvas = DesktopCanvasRuntime::new(runtime_root.path.clone())
        .expect("reopened canvas")
        .execute(DesktopCanvasRequestDto::Get {
            workspace_id: WORKSPACE.into(),
            canvas_id: "upgrade-canvas".into(),
        });
    assert!(reopened_canvas.ok);
    let reopened_canvas = reopened_canvas.data.expect("canvas data");
    assert_eq!(reopened_canvas.revision, 3);
    assert!(
        reopened_canvas
            .nodes
            .iter()
            .any(|node| node.target_id == DOCUMENT)
    );
    assert!(
        reopened_canvas
            .nodes
            .iter()
            .any(|node| node.target_id == asset_id)
    );

    let reopened_asset = DesktopDocumentAssetsRuntime::new(runtime_root.path.clone(), BODY_LIMIT)
        .expect("asset runtime")
        .detail(DesktopAssetDetailRequestDto {
            workspace_id: WORKSPACE.into(),
            asset_id: asset_id.clone(),
        });
    assert!(reopened_asset.ok);
    assert_eq!(
        reopened_asset
            .data
            .as_ref()
            .map(|data| data.linked_document_ids.as_slice()),
        Some([DOCUMENT.to_string()].as_slice())
    );
    assert_eq!(
        fs::read(asset_object_path(&runtime_root.path, &asset_id)).expect("restored object"),
        expected_asset_bytes
    );

    let works = DurableProjectionWorkRepository::new(runtime_root.path.clone())
        .list_resumable(20)
        .expect("projection rebuild work");
    assert_eq!(
        works
            .iter()
            .filter(|work| work.identity().change_kind() == ProjectionChangeKind::Restored)
            .count(),
        3
    );
    assert_eq!(fingerprint(&source.path), source_before);
}

fn seed_phase011_authoring_fixture(root: &Path) {
    fs::create_dir_all(root.join("metadata")).expect("phase011 metadata directory");
    let runtime = DesktopDocumentAuthoringRuntime::new(root.to_path_buf(), BODY_LIMIT)
        .expect("phase011 authoring");
    assert!(
        runtime
            .execute(DesktopDocumentAuthoringRequestDto::Create {
                workspace_id: WORKSPACE.into(),
                document_id: DOCUMENT.into(),
                path: "phase011.md".into(),
                body: "# Phase 011".into(),
                version_id: "phase011-v1".into(),
                snapshot_ref: "phase011-snapshot-v1".into(),
                author: "local-user".into(),
                summary: "created".into(),
            })
            .ok
    );
    assert!(
        runtime
            .execute(DesktopDocumentAuthoringRequestDto::Update {
                workspace_id: WORKSPACE.into(),
                document_id: DOCUMENT.into(),
                body: "# Phase 011\nCompatible current document".into(),
                expected_version_id: "phase011-v1".into(),
                version_id: "phase011-v2".into(),
                snapshot_ref: "phase011-snapshot-v2".into(),
                author: "local-user".into(),
                summary: "updated".into(),
            })
            .ok
    );
}

fn remove_authoritative_workspace_data(root: &Path) {
    let workspace_hex = hex(WORKSPACE);
    for path in [
        root.join("authoring-current").join(WORKSPACE),
        root.join("document-current-pointers").join(&workspace_hex),
        root.join("document-versions").join(WORKSPACE),
        root.join("canvases").join(&workspace_hex),
        root.join("assets/metadata").join(&workspace_hex),
        root.join("assets/objects").join(&workspace_hex),
        root.join("assets/associations").join(&workspace_hex),
    ] {
        if path.exists() {
            fs::remove_dir_all(path).expect("destructive fixture mutation");
        }
    }
}

fn asset_object_path(root: &Path, asset_id: &str) -> PathBuf {
    root.join("assets/objects")
        .join(hex(WORKSPACE))
        .join(&asset_id[0..2])
        .join(format!("{asset_id}.bin"))
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("destination");
    for entry in fs::read_dir(source).expect("read source") {
        let entry = entry.expect("source entry");
        let target = destination.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy fixture file");
        }
    }
}

fn fingerprint(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut values = Vec::new();
    collect_files(root, root, &mut values);
    values.sort_by(|left, right| left.0.cmp(&right.0));
    values
}

fn collect_files(root: &Path, base: &Path, values: &mut Vec<(String, Vec<u8>)>) {
    for entry in fs::read_dir(root).expect("read fixture") {
        let path = entry.expect("fixture entry").path();
        if path.is_dir() {
            collect_files(&path, base, values);
        } else {
            values.push((
                path.strip_prefix(base)
                    .expect("relative")
                    .to_string_lossy()
                    .into_owned(),
                fs::read(path).expect("fixture file"),
            ));
        }
    }
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("sponzey-{label}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).expect("temp root");
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
