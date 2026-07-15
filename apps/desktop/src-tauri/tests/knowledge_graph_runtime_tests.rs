use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_desktop_shell::{
    DesktopKnowledgeGraphRuntime, DesktopLocalCommandPayloadDto, DesktopLocalCommandRequestDto,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::graph_projection::{GraphProjectionRecord, GraphProjectionStore};

#[test]
fn native_graph_runtime_returns_bounded_camel_case_data_from_durable_projection() {
    let temp = TempRoot::new("ready");
    seed(&temp.path);
    let runtime = DesktopKnowledgeGraphRuntime::new(temp.path.clone());

    let response = runtime.execute(request("outgoing", false, 10, 10));
    let json = serde_json::to_string(&response).expect("json");

    assert!(response.ok);
    let data = response.data.expect("data");
    assert_eq!(data.center_document_id, "center-doc");
    assert_eq!(data.status, "degraded");
    assert_eq!(data.nodes.len(), 2);
    assert_eq!(data.edges.len(), 1);
    assert_eq!(data.freshness_revision, "version-9");
    assert!(json.contains("\"centerDocumentId\""));
    assert!(json.contains("\"freshnessRevision\""));
    assert!(!json.contains("center_document_id"));
    assert!(!json.contains(&temp.path.display().to_string()));
    assert!(!json.contains("raw document body"));
}

#[test]
fn native_graph_runtime_returns_stable_missing_and_invalid_failures() {
    let temp = TempRoot::new("missing-invalid");
    let runtime = DesktopKnowledgeGraphRuntime::new(temp.path.clone());

    let missing = runtime.execute(request("both", true, 10, 10));
    let invalid = runtime.execute(request("sideways", true, 0, 10));

    assert!(!missing.ok);
    assert_eq!(
        missing.error_code.as_deref(),
        Some("GRAPH_PROJECTION_NOT_FOUND")
    );
    assert!(!missing.retryable);
    assert!(!invalid.ok);
    assert_eq!(invalid.error_code.as_deref(), Some("GRAPH_INVALID_INPUT"));
    assert!(!invalid.retryable);
}

#[test]
fn native_graph_runtime_returns_sanitized_corruption_failure() {
    let temp = TempRoot::new("corrupt");
    seed(&temp.path);
    let snapshot = find_snapshot(&temp.path);
    fs::write(snapshot, "schema\t999\nprivate-document-body\n").expect("corrupt");
    let runtime = DesktopKnowledgeGraphRuntime::new(temp.path.clone());

    let response = runtime.execute(request("both", true, 10, 10));
    let debug = format!("{response:?}");

    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("GRAPH_PROJECTION_CORRUPTED")
    );
    assert!(!response.retryable);
    assert!(!debug.contains("private-document-body"));
    assert!(!debug.contains(&temp.path.display().to_string()));
}

fn request(
    direction: &str,
    include_unresolved: bool,
    node_limit: u16,
    edge_limit: u16,
) -> DesktopLocalCommandRequestDto {
    DesktopLocalCommandRequestDto {
        command_name: "get_graph_projection".to_string(),
        payload: DesktopLocalCommandPayloadDto::GraphProjection {
            workspace_id: "workspace-1".to_string(),
            document_id: "center-doc".to_string(),
            depth: 1,
            direction: direction.to_string(),
            include_unresolved,
            include_assets: false,
            node_limit,
            edge_limit,
        },
    }
}

fn seed(root: &PathBuf) {
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let mut store = DurableLocalGraphProjectionStore::new(root.clone());
    store
        .replace_projection(
            &workspace,
            GraphProjectionRecord::new_with_revision(graph(), "version-9").expect("record"),
        )
        .expect("seed");
}

fn graph() -> KnowledgeGraph {
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
    let workspace = fs::read_dir(root.join("graph-projections"))
        .expect("root")
        .next()
        .expect("workspace")
        .expect("workspace entry")
        .path();
    fs::read_dir(workspace)
        .expect("workspace")
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
            "sponzey-cabinet-phase012-native-graph-{label}-{}-{nonce}",
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
