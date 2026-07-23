use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_workspace_home_projection::LocalWorkspaceHomeProjectionStore;
use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::workspace_home::{
    WorkspaceHomeBackupStatus, WorkspaceHomeChangeProjection, WorkspaceHomeDocumentMutation,
    WorkspaceHomeDocumentMutationPort, WorkspaceHomeDocumentProjection, WorkspaceHomeHealthStatus,
    WorkspaceHomeProjection, WorkspaceHomeProjectionError, WorkspaceHomeProjectionLimits,
    WorkspaceHomeProjectionPort, WorkspaceHomeSummaryProjection, WorkspaceHomeTagProjection,
    WorkspaceHomeUnfinishedProjection,
};

#[test]
fn missing_upsert_creates_bounded_projection_and_restart_restores_it() {
    let temp = TempRoot::new("missing-upsert");
    let workspace = workspace();
    let mut store = LocalWorkspaceHomeProjectionStore::new(temp.path.clone());

    store
        .apply_document_mutation(
            &workspace,
            WorkspaceHomeDocumentMutation::UpsertRecent {
                document: document("doc-1", "Source", "notes/source.md"),
                change_summary: "Created document".to_string(),
            },
            50,
        )
        .expect("upsert");

    let restarted = LocalWorkspaceHomeProjectionStore::new(temp.path.clone());
    let loaded = restarted
        .load_workspace_home(&workspace, limits())
        .expect("restart load");
    assert_eq!(loaded.recent_documents().len(), 1);
    assert_eq!(loaded.recent_documents()[0].document_id(), "doc-1");
    assert_eq!(loaded.recent_changes()[0].summary(), "Created document");
    assert_eq!(
        loaded.backup_status(),
        WorkspaceHomeBackupStatus::NeverCreated
    );
    assert_eq!(loaded.health_status(), WorkspaceHomeHealthStatus::Healthy);
}

#[test]
fn upsert_deduplicates_moves_to_front_and_applies_independent_capacity() {
    let temp = TempRoot::new("upsert-order");
    let workspace = workspace();
    let mut store = LocalWorkspaceHomeProjectionStore::new(temp.path.clone());
    store
        .replace_projection(
            &workspace,
            &WorkspaceHomeProjection::new(
                vec![
                    document("doc-1", "Old source", "notes/old.md"),
                    document("doc-2", "Second", "notes/second.md"),
                ],
                vec![document("favorite-1", "Favorite", "notes/favorite.md")],
                vec![WorkspaceHomeTagProjection::new("rust", 2).expect("tag")],
                vec![
                    change("doc-1", "Old change"),
                    change("doc-2", "Second change"),
                ],
                vec![unfinished("draft-1", "Review")],
                WorkspaceHomeBackupStatus::Fresh,
                WorkspaceHomeHealthStatus::Degraded,
            )
            .with_summary(WorkspaceHomeSummaryProjection::new(10_000, 2_500, 24)),
        )
        .expect("seed");

    store
        .apply_document_mutation(
            &workspace,
            WorkspaceHomeDocumentMutation::UpsertRecent {
                document: document("doc-1", "Updated source", "notes/source.md"),
                change_summary: "Updated document".to_string(),
            },
            1,
        )
        .expect("upsert");

    let loaded = store
        .load_workspace_home(&workspace, limits())
        .expect("load");
    assert_eq!(loaded.recent_documents().len(), 1);
    assert_eq!(loaded.recent_documents()[0].title(), "Updated source");
    assert_eq!(loaded.recent_changes().len(), 1);
    assert_eq!(loaded.recent_changes()[0].summary(), "Updated document");
    assert_eq!(loaded.favorites()[0].document_id(), "favorite-1");
    assert_eq!(loaded.tags()[0].label(), "rust");
    assert_eq!(loaded.unfinished_items()[0].document_id(), "draft-1");
    assert_eq!(loaded.backup_status(), WorkspaceHomeBackupStatus::Fresh);
    assert_eq!(loaded.health_status(), WorkspaceHomeHealthStatus::Degraded);
    assert_eq!(loaded.summary().document_count(), 10_000);
    assert_eq!(loaded.summary().asset_count(), 2_500);
    assert_eq!(loaded.summary().canvas_count(), 24);
}

#[test]
fn remove_cleans_document_categories_preserves_tags_and_is_idempotent() {
    let temp = TempRoot::new("remove");
    let workspace = workspace();
    let mut store = LocalWorkspaceHomeProjectionStore::new(temp.path.clone());
    store
        .replace_projection(
            &workspace,
            &WorkspaceHomeProjection::new(
                vec![document("doc-1", "Source", "notes/source.md")],
                vec![document("doc-1", "Source", "notes/source.md")],
                vec![WorkspaceHomeTagProjection::new("rust", 1).expect("tag")],
                vec![change("doc-1", "Updated document")],
                vec![unfinished("doc-1", "Review")],
                WorkspaceHomeBackupStatus::Fresh,
                WorkspaceHomeHealthStatus::Healthy,
            ),
        )
        .expect("seed");

    for _ in 0..2 {
        store
            .apply_document_mutation(
                &workspace,
                WorkspaceHomeDocumentMutation::RemoveDocument {
                    document_id: DocumentId::new("doc-1").expect("id"),
                },
                50,
            )
            .expect("idempotent remove");
    }

    let loaded = store
        .load_workspace_home(&workspace, limits())
        .expect("load");
    assert!(loaded.recent_documents().is_empty());
    assert!(loaded.favorites().is_empty());
    assert!(loaded.recent_changes().is_empty());
    assert!(loaded.unfinished_items().is_empty());
    assert_eq!(loaded.tags()[0].label(), "rust");
}

#[test]
fn invalid_capacity_and_corruption_fail_closed_without_overwrite() {
    let temp = TempRoot::new("fail-closed");
    let workspace = workspace();
    let mut store = LocalWorkspaceHomeProjectionStore::new(temp.path.clone());

    for capacity in [0, 101] {
        let error = store
            .apply_document_mutation(
                &workspace,
                WorkspaceHomeDocumentMutation::RemoveDocument {
                    document_id: DocumentId::new("doc-1").expect("id"),
                },
                capacity,
            )
            .expect_err("invalid capacity");
        assert_eq!(error, WorkspaceHomeProjectionError::InvalidLimit);
    }

    store
        .replace_projection(
            &workspace,
            &WorkspaceHomeProjection::empty(
                WorkspaceHomeBackupStatus::Fresh,
                WorkspaceHomeHealthStatus::Healthy,
            ),
        )
        .expect("seed");
    let snapshot = fs::read_dir(temp.path.join("home-projections"))
        .expect("snapshot dir")
        .next()
        .expect("entry")
        .expect("path")
        .path();
    fs::write(&snapshot, "schema\t999\nprivate-body\n").expect("corrupt");
    let before = fs::read(&snapshot).expect("before");

    let error = store
        .apply_document_mutation(
            &workspace,
            WorkspaceHomeDocumentMutation::UpsertRecent {
                document: document("doc-1", "Source", "notes/source.md"),
                change_summary: "Updated document".to_string(),
            },
            50,
        )
        .expect_err("corruption fails");

    assert_eq!(error, WorkspaceHomeProjectionError::CorruptedProjection);
    assert_eq!(fs::read(snapshot).expect("after"), before);
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace")
}

fn limits() -> WorkspaceHomeProjectionLimits {
    WorkspaceHomeProjectionLimits::new(100, 100, 100, 100, 100).expect("limits")
}

fn document(id: &str, title: &str, path: &str) -> WorkspaceHomeDocumentProjection {
    WorkspaceHomeDocumentProjection::new(
        DocumentId::new(id).expect("id"),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
    )
}

fn change(id: &str, summary: &str) -> WorkspaceHomeChangeProjection {
    WorkspaceHomeChangeProjection::new(DocumentId::new(id).expect("id"), summary).expect("change")
}

fn unfinished(id: &str, label: &str) -> WorkspaceHomeUnfinishedProjection {
    WorkspaceHomeUnfinishedProjection::new(DocumentId::new(id).expect("id"), label)
        .expect("unfinished")
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
        let path = std::env::temp_dir().join(format!(
            "sponzey-cabinet-phase011-home-mutation-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp root");
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
