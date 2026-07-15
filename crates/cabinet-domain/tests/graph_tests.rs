use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphError, GraphNode, GraphNodeKind, GraphProjectionStatus,
    KnowledgeGraph,
};

#[test]
fn knowledge_graph_rejects_edge_with_missing_node_reference() {
    let source = GraphNode::new_document(document_id("source-doc"));
    let edge = GraphEdge::new(
        "edge-1",
        source.id().to_string(),
        "missing-doc".to_string(),
        GraphEdgeKind::DocumentLink,
    )
    .expect("edge");

    let result = KnowledgeGraph::new(vec![source], vec![edge], GraphProjectionStatus::Clean);

    assert_eq!(result, Err(GraphError::MissingEdgeNode));
}

#[test]
fn knowledge_graph_rejects_duplicate_node_ids() {
    let first = GraphNode::new_document(document_id("same-doc"));
    let second = GraphNode::new_document(document_id("same-doc"));

    let result = KnowledgeGraph::new(
        vec![first, second],
        Vec::new(),
        GraphProjectionStatus::Clean,
    );

    assert_eq!(result, Err(GraphError::DuplicateNodeId));
}

#[test]
fn graph_projection_status_uses_explicit_transitions() {
    let requested = GraphProjectionStatus::Clean
        .transition(GraphProjectionStatus::REQUEST_REINDEX)
        .expect("request reindex");
    let reindexing = requested
        .transition(GraphProjectionStatus::START_REINDEX)
        .expect("start reindex");
    let clean = reindexing
        .transition(GraphProjectionStatus::FINISH_REINDEX)
        .expect("finish reindex");
    let invalid = GraphProjectionStatus::Clean.transition(GraphProjectionStatus::FINISH_REINDEX);

    assert_eq!(requested, GraphProjectionStatus::ReindexRequested);
    assert_eq!(reindexing, GraphProjectionStatus::Reindexing);
    assert_eq!(clean, GraphProjectionStatus::Clean);
    assert_eq!(invalid, Err(GraphError::InvalidStatusTransition));
}

#[test]
fn graph_node_supports_document_unresolved_attachment_and_external_types() {
    let document = GraphNode::new_document(document_id("document-node"));
    let unresolved = GraphNode::new_unresolved("missing-page").expect("unresolved");
    let attachment = GraphNode::new_attachment("asset-1").expect("attachment");
    let external = GraphNode::new_external_link("https://example.test").expect("external");

    assert_eq!(document.kind(), GraphNodeKind::Document);
    assert_eq!(unresolved.kind(), GraphNodeKind::UnresolvedLink);
    assert_eq!(attachment.kind(), GraphNodeKind::Attachment);
    assert_eq!(external.kind(), GraphNodeKind::ExternalLink);
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}
