use std::fs;

use cabinet_adapters::local_workspace_home_projection::LocalWorkspaceHomeProjectionStore;
use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::workspace_home::{
    WorkspaceHomeBackupStatus, WorkspaceHomeChangeProjection, WorkspaceHomeDocumentProjection,
    WorkspaceHomeHealthStatus, WorkspaceHomeProjection, WorkspaceHomeProjectionError,
    WorkspaceHomeProjectionLimits, WorkspaceHomeProjectionPort, WorkspaceHomeSummaryProjection,
    WorkspaceHomeTagProjection, WorkspaceHomeUnfinishedProjection,
};

#[test]
fn local_workspace_home_projection_persists_restarts_and_applies_independent_limits() {
    let root = temp_root("restart-limits");
    let workspace_id = workspace("workspace-1");
    let store = LocalWorkspaceHomeProjectionStore::new(root.clone());
    store
        .replace_projection(&workspace_id, &projection("a"))
        .expect("projection write");

    let restarted = LocalWorkspaceHomeProjectionStore::new(root.clone());
    let loaded = restarted
        .load_workspace_home(
            &workspace_id,
            WorkspaceHomeProjectionLimits::new(1, 1, 1, 1, 1).expect("limits"),
        )
        .expect("projection read");

    assert_eq!(loaded.recent_documents().len(), 1);
    assert_eq!(loaded.recent_documents()[0].document_id(), "doc-a-1");
    assert_eq!(loaded.favorites().len(), 1);
    assert_eq!(loaded.tags().len(), 1);
    assert_eq!(loaded.recent_changes().len(), 1);
    assert_eq!(loaded.unfinished_items().len(), 1);
    assert_eq!(loaded.backup_status(), WorkspaceHomeBackupStatus::Fresh);
    assert_eq!(loaded.health_status(), WorkspaceHomeHealthStatus::Healthy);
    assert_eq!(loaded.summary().document_count(), 10_000);
    assert_eq!(loaded.summary().asset_count(), 2_500);
    assert_eq!(loaded.summary().canvas_count(), 24);

    let files = fs::read_dir(root.join("home-projections"))
        .expect("projection directory")
        .collect::<Result<Vec<_>, _>>()
        .expect("projection entries");
    assert_eq!(files.len(), 1);
    assert!(
        files[0]
            .file_name()
            .to_string_lossy()
            .ends_with(".snapshot")
    );
    assert!(
        !files[0]
            .file_name()
            .to_string_lossy()
            .contains("workspace-1")
    );
    assert!(
        !files
            .iter()
            .any(|entry| entry.file_name().to_string_lossy().contains(".tmp"))
    );
}

#[test]
fn local_workspace_home_projection_returns_empty_for_missing_workspace_and_isolates_workspaces() {
    let root = temp_root("isolation");
    let store = LocalWorkspaceHomeProjectionStore::new(root);
    let workspace_a = workspace("workspace-a");
    let workspace_b = workspace("workspace-b");
    store
        .replace_projection(&workspace_a, &projection("a"))
        .expect("workspace a write");
    store
        .replace_projection(&workspace_b, &projection("b"))
        .expect("workspace b write");

    let limits = WorkspaceHomeProjectionLimits::new(10, 10, 10, 10, 10).expect("limits");
    let loaded_a = store
        .load_workspace_home(&workspace_a, limits)
        .expect("workspace a read");
    let loaded_b = store
        .load_workspace_home(&workspace_b, limits)
        .expect("workspace b read");
    let missing = store
        .load_workspace_home(&workspace("workspace-missing"), limits)
        .expect("missing is empty");

    assert_eq!(loaded_a.recent_documents()[0].document_id(), "doc-a-1");
    assert_eq!(loaded_b.recent_documents()[0].document_id(), "doc-b-1");
    assert_eq!(missing.total_item_count(), 0);
    assert_eq!(
        missing.backup_status(),
        WorkspaceHomeBackupStatus::NeverCreated
    );
    assert_eq!(missing.health_status(), WorkspaceHomeHealthStatus::Healthy);
}

#[test]
fn local_workspace_home_projection_rejects_corrupted_or_unknown_schema_snapshot() {
    let root = temp_root("corruption");
    let workspace_id = workspace("workspace-1");
    let store = LocalWorkspaceHomeProjectionStore::new(root.clone());
    store
        .replace_projection(&workspace_id, &projection("a"))
        .expect("projection write");
    let snapshot = fs::read_dir(root.join("home-projections"))
        .expect("projection directory")
        .next()
        .expect("snapshot entry")
        .expect("snapshot path")
        .path();

    fs::write(&snapshot, "schema\t99\nrecent\tinvalid\n").expect("corrupt snapshot");
    let error = store
        .load_workspace_home(
            &workspace_id,
            WorkspaceHomeProjectionLimits::new(10, 10, 10, 10, 10).expect("limits"),
        )
        .expect_err("corrupt snapshot must fail");

    assert_eq!(error, WorkspaceHomeProjectionError::CorruptedProjection);
    assert_eq!(error.code(), "workspace_home_projection.corrupted");
}

#[test]
fn local_workspace_home_projection_defaults_legacy_summary_and_rejects_malformed_counts() {
    let root = temp_root("summary-compatibility");
    let workspace_id = workspace("workspace-1");
    let store = LocalWorkspaceHomeProjectionStore::new(root.clone());
    store
        .replace_projection(&workspace_id, &projection("a"))
        .expect("projection write");
    let snapshot = fs::read_dir(root.join("home-projections"))
        .expect("projection directory")
        .next()
        .expect("snapshot entry")
        .expect("snapshot path")
        .path();
    let encoded = fs::read_to_string(&snapshot).expect("encoded projection");
    let legacy = encoded
        .lines()
        .filter(|line| !line.starts_with("summary\t"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&snapshot, format!("{legacy}\n")).expect("legacy projection");

    let loaded = store
        .load_workspace_home(
            &workspace_id,
            WorkspaceHomeProjectionLimits::new(10, 10, 10, 10, 10).expect("limits"),
        )
        .expect("legacy projection remains readable");
    assert_eq!(loaded.summary(), WorkspaceHomeSummaryProjection::default());

    fs::write(
        &snapshot,
        encoded.replace("summary\t10000\t2500\t24", "summary\tinvalid\t2500\t24"),
    )
    .expect("malformed projection");
    let error = store
        .load_workspace_home(
            &workspace_id,
            WorkspaceHomeProjectionLimits::new(10, 10, 10, 10, 10).expect("limits"),
        )
        .expect_err("malformed summary must fail");
    assert_eq!(error, WorkspaceHomeProjectionError::CorruptedProjection);
}

#[test]
fn local_workspace_home_projection_maps_invalid_root_to_storage_unavailable() {
    let root = temp_root("write-failure");
    fs::create_dir_all(&root).expect("root");
    let root_file = root.join("not-a-directory");
    fs::write(&root_file, "file blocks directory creation").expect("root file");
    let store = LocalWorkspaceHomeProjectionStore::new(root_file);

    let error = store
        .replace_projection(&workspace("workspace-1"), &projection("a"))
        .expect_err("write must fail");

    assert_eq!(error, WorkspaceHomeProjectionError::StorageUnavailable);
}

#[test]
fn local_workspace_home_projection_codec_does_not_store_document_body_or_absolute_root() {
    let root = temp_root("safe-codec");
    let workspace_id = workspace("workspace-1");
    let store = LocalWorkspaceHomeProjectionStore::new(root.clone());
    store
        .replace_projection(&workspace_id, &projection("a"))
        .expect("projection write");
    let snapshot = fs::read_dir(root.join("home-projections"))
        .expect("projection directory")
        .next()
        .expect("snapshot entry")
        .expect("snapshot path")
        .path();
    let encoded = fs::read_to_string(snapshot).expect("snapshot text");

    assert!(!encoded.contains("raw document body"));
    assert!(!encoded.contains(root.to_string_lossy().as_ref()));
    assert!(!encoded.contains("Private title a 1"));
    assert!(!encoded.contains("notes/a-1.md"));
}

fn projection(prefix: &str) -> WorkspaceHomeProjection {
    WorkspaceHomeProjection::new(
        vec![document(prefix, 1), document(prefix, 2)],
        vec![document(prefix, 2), document(prefix, 1)],
        vec![
            WorkspaceHomeTagProjection::new(&format!("tag-{prefix}-1"), 2).expect("tag"),
            WorkspaceHomeTagProjection::new(&format!("tag-{prefix}-2"), 1).expect("tag"),
        ],
        vec![
            WorkspaceHomeChangeProjection::new(
                DocumentId::new(&format!("doc-{prefix}-1")).expect("id"),
                &format!("change-{prefix}-1"),
            )
            .expect("change"),
            WorkspaceHomeChangeProjection::new(
                DocumentId::new(&format!("doc-{prefix}-2")).expect("id"),
                &format!("change-{prefix}-2"),
            )
            .expect("change"),
        ],
        vec![
            WorkspaceHomeUnfinishedProjection::new(
                DocumentId::new(&format!("doc-{prefix}-1")).expect("id"),
                &format!("unfinished-{prefix}-1"),
            )
            .expect("unfinished"),
            WorkspaceHomeUnfinishedProjection::new(
                DocumentId::new(&format!("doc-{prefix}-2")).expect("id"),
                &format!("unfinished-{prefix}-2"),
            )
            .expect("unfinished"),
        ],
        WorkspaceHomeBackupStatus::Fresh,
        WorkspaceHomeHealthStatus::Healthy,
    )
    .with_summary(WorkspaceHomeSummaryProjection::new(10_000, 2_500, 24))
}

fn document(prefix: &str, index: u8) -> WorkspaceHomeDocumentProjection {
    WorkspaceHomeDocumentProjection::new(
        DocumentId::new(&format!("doc-{prefix}-{index}")).expect("id"),
        DocumentTitle::new(&format!("Private title {prefix} {index}")).expect("title"),
        DocumentPath::new(&format!("notes/{prefix}-{index}.md")).expect("path"),
    )
}

fn workspace(id: &str) -> WorkspaceId {
    WorkspaceId::new(id).expect("workspace id")
}

fn temp_root(name: &str) -> std::path::PathBuf {
    let unique = format!(
        "sponzey-home-projection-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    );
    std::env::temp_dir().join(unique)
}
