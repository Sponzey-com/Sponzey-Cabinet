use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::graph_projection::{
    GraphProjectionError, GraphProjectionRecord, GraphProjectionStore,
};

#[test]
fn durable_graph_projection_survives_adapter_restart_with_revision() {
    let temp = TempRoot::new("restart");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let center = DocumentId::new("center-doc").expect("center");
    let mut writer = DurableLocalGraphProjectionStore::new(temp.path.clone());
    writer
        .replace_projection(
            &workspace,
            GraphProjectionRecord::new_with_revision(graph_fixture(), "version-7").expect("record"),
        )
        .expect("write");
    drop(writer);

    let reader = DurableLocalGraphProjectionStore::new(temp.path.clone());
    let stored = reader
        .get_projection(&workspace, &center)
        .expect("read")
        .expect("record");

    assert_eq!(stored.freshness_revision(), "version-7");
    assert_eq!(stored.graph().status(), GraphProjectionStatus::Degraded);
    assert_eq!(stored.graph().nodes().len(), 3);
    assert_eq!(stored.graph().edges().len(), 2);
}

#[test]
fn durable_graph_projection_separates_workspace_and_reports_missing() {
    let temp = TempRoot::new("isolation");
    let first = WorkspaceId::new("workspace-1").expect("workspace");
    let second = WorkspaceId::new("workspace-2").expect("workspace");
    let center = DocumentId::new("center-doc").expect("center");
    let mut store = DurableLocalGraphProjectionStore::new(temp.path.clone());
    store
        .replace_projection(
            &first,
            GraphProjectionRecord::new_with_revision(graph_fixture(), "version-1").expect("record"),
        )
        .expect("write");

    assert!(
        store
            .get_projection(&second, &center)
            .expect("read")
            .is_none()
    );
    assert!(
        store
            .get_projection(&first, &DocumentId::new("other-doc").expect("other"))
            .expect("read")
            .is_none()
    );
}

#[test]
fn durable_graph_projection_delete_is_idempotent_and_survives_restart() {
    let temp = TempRoot::new("delete");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let center = DocumentId::new("center-doc").expect("center");
    let mut store = DurableLocalGraphProjectionStore::new(temp.path.clone());
    store
        .replace_projection(
            &workspace,
            GraphProjectionRecord::new_with_revision(graph_fixture(), "version-1").expect("record"),
        )
        .expect("write");
    store
        .delete_projection(&workspace, &center)
        .expect("delete");
    store
        .delete_projection(&workspace, &center)
        .expect("idempotent delete");
    drop(store);

    assert!(
        DurableLocalGraphProjectionStore::new(temp.path.clone())
            .get_projection(&workspace, &center)
            .expect("read")
            .is_none()
    );
}

#[test]
fn durable_graph_projection_rejects_invalid_revision_and_corrupt_schema() {
    assert_eq!(
        GraphProjectionRecord::new_with_revision(graph_fixture(), "\n").unwrap_err(),
        GraphProjectionError::InvalidInput,
    );

    let temp = TempRoot::new("corrupt");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let center = DocumentId::new("center-doc").expect("center");
    let mut store = DurableLocalGraphProjectionStore::new(temp.path.clone());
    store
        .replace_projection(
            &workspace,
            GraphProjectionRecord::new_with_revision(graph_fixture(), "version-1").expect("record"),
        )
        .expect("write");
    let snapshot = find_snapshot(&temp.path);
    fs::write(&snapshot, "schema\t999\nprivate-document-body\n").expect("corrupt");

    let error = store.get_projection(&workspace, &center).unwrap_err();
    assert_eq!(error, GraphProjectionError::CorruptedProjection);
    assert!(!format!("{error:?}").contains("private-document-body"));
    assert!(!format!("{error:?}").contains(&temp.path.display().to_string()));
}

#[test]
fn durable_graph_projection_cache_observes_an_external_atomic_replacement() {
    let temp = TempRoot::new("cache-revision");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let center = DocumentId::new("center-doc").expect("center");
    let mut writer = DurableLocalGraphProjectionStore::new(temp.path.clone());
    writer
        .replace_projection(
            &workspace,
            GraphProjectionRecord::new_with_revision(graph_fixture(), "version-1").expect("record"),
        )
        .expect("first write");
    let reader = DurableLocalGraphProjectionStore::new(temp.path.clone());
    assert_eq!(
        reader
            .get_projection(&workspace, &center)
            .expect("first read")
            .expect("record")
            .freshness_revision(),
        "version-1"
    );
    writer
        .replace_projection(
            &workspace,
            GraphProjectionRecord::new_with_revision(graph_fixture(), "version-2").expect("record"),
        )
        .expect("replacement");
    assert_eq!(
        reader
            .get_projection(&workspace, &center)
            .expect("second read")
            .expect("record")
            .freshness_revision(),
        "version-2"
    );
}

fn graph_fixture() -> KnowledgeGraph {
    let center_id = DocumentId::new("center-doc").expect("center");
    let center = GraphNode::new_document(center_id.clone());
    let neighbor = GraphNode::new_document(DocumentId::new("neighbor-doc").expect("neighbor"));
    let unresolved = GraphNode::new_unresolved("missing-doc").expect("unresolved");
    let edges = vec![
        GraphEdge::new(
            "edge-1",
            center.id().to_string(),
            neighbor.id().to_string(),
            GraphEdgeKind::DocumentLink,
        )
        .expect("edge"),
        GraphEdge::new(
            "edge-2",
            center.id().to_string(),
            unresolved.id().to_string(),
            GraphEdgeKind::DocumentLink,
        )
        .expect("edge"),
    ];
    KnowledgeGraph::new_with_center(
        center_id,
        vec![center, neighbor, unresolved],
        edges,
        GraphProjectionStatus::Degraded,
    )
    .expect("graph")
}

fn find_snapshot(root: &PathBuf) -> PathBuf {
    let workspace_dir = fs::read_dir(root.join("graph-projections"))
        .expect("graph root")
        .next()
        .expect("workspace")
        .expect("workspace entry")
        .path();
    fs::read_dir(workspace_dir)
        .expect("workspace dir")
        .next()
        .expect("snapshot")
        .expect("snapshot entry")
        .path()
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
            "sponzey-cabinet-phase012-graph-{label}-{}-{nonce}",
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
