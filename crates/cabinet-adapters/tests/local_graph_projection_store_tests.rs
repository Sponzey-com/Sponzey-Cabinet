use cabinet_adapters::local_graph_projection::LocalGraphProjectionStore;
use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::graph_projection::{GraphProjectionRecord, GraphProjectionStore};

#[test]
fn local_graph_projection_store_keeps_workspace_projections_separate() {
    let center_document_id = document_id("center-doc");
    let mut store = LocalGraphProjectionStore::new();
    let first_workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let second_workspace = WorkspaceId::new("workspace-2").expect("workspace");

    store
        .replace_projection(
            &first_workspace,
            GraphProjectionRecord::new(graph_with_status(
                center_document_id.clone(),
                "first-neighbor",
                GraphProjectionStatus::Clean,
            ))
            .expect("record"),
        )
        .expect("replace first");
    store
        .replace_projection(
            &second_workspace,
            GraphProjectionRecord::new(graph_with_status(
                center_document_id.clone(),
                "second-neighbor",
                GraphProjectionStatus::Degraded,
            ))
            .expect("record"),
        )
        .expect("replace second");

    let first = store
        .get_projection(&first_workspace, &center_document_id)
        .expect("first")
        .expect("first record");
    let second = store
        .get_projection(&second_workspace, &center_document_id)
        .expect("second")
        .expect("second record");

    assert_eq!(first.graph().status(), GraphProjectionStatus::Clean);
    assert!(
        first
            .graph()
            .nodes()
            .iter()
            .any(|node| node.id() == "first-neighbor")
    );
    assert_eq!(second.graph().status(), GraphProjectionStatus::Degraded);
    assert!(
        second
            .graph()
            .nodes()
            .iter()
            .any(|node| node.id() == "second-neighbor")
    );
}

fn graph_with_status(
    center_document_id: DocumentId,
    neighbor_id: &str,
    status: GraphProjectionStatus,
) -> KnowledgeGraph {
    let center = GraphNode::new_document(center_document_id.clone());
    let neighbor = GraphNode::new_document(document_id(neighbor_id));
    let edge = GraphEdge::new(
        "edge-1",
        center.id().to_string(),
        neighbor.id().to_string(),
        GraphEdgeKind::DocumentLink,
    )
    .expect("edge");
    KnowledgeGraph::new_with_center(
        center_document_id,
        vec![center, neighbor],
        vec![edge],
        status,
    )
    .expect("graph")
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}
