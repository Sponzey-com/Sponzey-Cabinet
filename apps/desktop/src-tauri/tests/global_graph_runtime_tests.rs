use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_desktop_shell::{
    DesktopGlobalKnowledgeGraphRequestDto, DesktopGlobalKnowledgeGraphRuntime,
};
use cabinet_domain::{
    document::DocumentId,
    graph::{GraphNode, GraphProjectionStatus, KnowledgeGraph},
    workspace::WorkspaceId,
};
use cabinet_ports::graph_projection::{GraphProjectionRecord, GraphProjectionStore};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn native_global_graph_returns_bounded_camel_case_page_without_fake_center() {
    let temp = Temp::new();
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let mut store = DurableLocalGraphProjectionStore::new(temp.path.clone());
    for id in ["doc-a", "doc-b"] {
        let center = DocumentId::new(id).unwrap();
        let graph = KnowledgeGraph::new_with_center(
            center.clone(),
            vec![GraphNode::new_document(center)],
            vec![],
            GraphProjectionStatus::Clean,
        )
        .unwrap();
        store
            .replace_projection(
                &workspace,
                GraphProjectionRecord::new_with_revision(graph, "v1").unwrap(),
            )
            .unwrap();
    }
    let response = DesktopGlobalKnowledgeGraphRuntime::new(temp.path.clone()).execute(
        DesktopGlobalKnowledgeGraphRequestDto {
            workspace_id: "workspace-1".into(),
            cursor: None,
            projection_limit: 1,
            node_limit: 10,
            edge_limit: 10,
        },
    );
    assert!(response.ok);
    let data = response.data.unwrap();
    assert_eq!(data.nodes.len(), 1);
    assert_eq!(data.next_cursor.as_deref(), Some("doc-a"));
    let json = serde_json::to_string(&data).unwrap();
    assert!(json.contains("nextCursor"));
    assert!(!json.contains("centerDocumentId"));
    assert!(!json.contains(&temp.path.display().to_string()));
}

#[test]
fn native_global_graph_rejects_zero_limit_safely() {
    let temp = Temp::new();
    let response = DesktopGlobalKnowledgeGraphRuntime::new(temp.path.clone()).execute(
        DesktopGlobalKnowledgeGraphRequestDto {
            workspace_id: "workspace-1".into(),
            cursor: None,
            projection_limit: 0,
            node_limit: 10,
            edge_limit: 10,
        },
    );
    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("GLOBAL_GRAPH_INVALID_INPUT")
    );
}
struct Temp {
    path: PathBuf,
}
impl Temp {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cabinet-global-runtime-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
