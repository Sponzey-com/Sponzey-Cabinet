use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_domain::{
    document::DocumentId,
    graph::{GraphNode, GraphProjectionStatus, KnowledgeGraph},
    workspace::WorkspaceId,
};
use cabinet_ports::graph_projection::{
    GraphProjectionRecord, GraphProjectionStore, WorkspaceGraphProjectionReader,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn durable_global_listing_is_workspace_scoped_stable_and_cursor_bounded() {
    let temp = Temp::new();
    let first = WorkspaceId::new("workspace-1").unwrap();
    let second = WorkspaceId::new("workspace-2").unwrap();
    let mut store = DurableLocalGraphProjectionStore::new(temp.path.clone());
    for id in ["doc-c", "doc-a", "doc-b"] {
        store.replace_projection(&first, record(id)).unwrap();
    }
    store.replace_projection(&second, record("doc-z")).unwrap();
    let page = store.list_workspace_projections(&first, None, 2).unwrap();
    assert_eq!(
        page.records()
            .iter()
            .map(|value| value.graph().center_document_id().as_str())
            .collect::<Vec<_>>(),
        ["doc-a", "doc-b"]
    );
    assert_eq!(page.next_cursor(), Some("doc-b"));
    let next = store
        .list_workspace_projections(&first, page.next_cursor(), 2)
        .unwrap();
    assert_eq!(
        next.records()[0].graph().center_document_id().as_str(),
        "doc-c"
    );
    assert_eq!(next.next_cursor(), None);
    assert_eq!(
        store.list_workspace_projections(&first, None, 0),
        Err(cabinet_ports::graph_projection::GraphProjectionError::InvalidInput)
    );
}
fn record(id: &str) -> GraphProjectionRecord {
    let center = DocumentId::new(id).unwrap();
    GraphProjectionRecord::new_with_revision(
        KnowledgeGraph::new_with_center(
            center.clone(),
            vec![GraphNode::new_document(center)],
            vec![],
            GraphProjectionStatus::Clean,
        )
        .unwrap(),
        "v1",
    )
    .unwrap()
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
            "cabinet-global-list-{}-{nonce}",
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
