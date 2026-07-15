use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::graph_projection::{
    GraphProjectionError, GraphProjectionRecord, GraphProjectionStore,
};
use cabinet_usecases::graph::{
    GetLocalKnowledgeGraphError, GetLocalKnowledgeGraphInput, GetLocalKnowledgeGraphUsecase,
    LocalGraphDirection,
};

#[test]
fn local_graph_query_filters_direction_and_node_kinds_without_dangling_edges() {
    let store = FakeStore::with_graph(graph_fixture());
    let output = GetLocalKnowledgeGraphUsecase::new()
        .execute(
            GetLocalKnowledgeGraphInput::new(
                "workspace-1",
                "center-doc",
                1,
                LocalGraphDirection::Outgoing,
                false,
                false,
                10,
                10,
            ),
            &store,
        )
        .expect("bounded graph");

    assert_eq!(output.graph().nodes().len(), 2);
    assert_eq!(output.graph().edges().len(), 1);
    assert_eq!(output.graph().edges()[0].target_id(), "outgoing-doc");
    assert!(output.graph().nodes().iter().all(|node| {
        !matches!(
            node.kind(),
            cabinet_domain::graph::GraphNodeKind::UnresolvedLink
                | cabinet_domain::graph::GraphNodeKind::Attachment
        )
    }));
    assert_eq!(output.candidate_count(), 5);
    assert_eq!(output.filtered_count(), 3);
}

#[test]
fn local_graph_query_applies_node_and_edge_limits_deterministically() {
    let store = FakeStore::with_graph(graph_fixture());
    let output = GetLocalKnowledgeGraphUsecase::new()
        .execute(
            GetLocalKnowledgeGraphInput::new(
                "workspace-1",
                "center-doc",
                2,
                LocalGraphDirection::Both,
                true,
                true,
                3,
                1,
            ),
            &store,
        )
        .expect("limited graph");

    assert!(output.graph().nodes().len() <= 3);
    assert!(output.graph().edges().len() <= 1);
    assert!(output.graph().edges().iter().all(|edge| {
        output
            .graph()
            .nodes()
            .iter()
            .any(|node| node.id() == edge.source_id())
            && output
                .graph()
                .nodes()
                .iter()
                .any(|node| node.id() == edge.target_id())
    }));
}

#[test]
fn local_graph_query_maps_invalid_missing_unavailable_and_corrupt_results() {
    let invalid = GetLocalKnowledgeGraphUsecase::new().execute(
        GetLocalKnowledgeGraphInput::new(
            "workspace-1",
            "center-doc",
            3,
            LocalGraphDirection::Both,
            true,
            true,
            10,
            10,
        ),
        &FakeStore::default(),
    );
    let missing = execute_default(&FakeStore::default());
    let unavailable = execute_default(&FakeStore::failing(
        GraphProjectionError::StorageUnavailable,
    ));
    let corrupt = execute_default(&FakeStore::failing(
        GraphProjectionError::CorruptedProjection,
    ));

    assert_eq!(
        invalid.unwrap_err(),
        GetLocalKnowledgeGraphError::InvalidInput
    );
    assert_eq!(
        missing.unwrap_err(),
        GetLocalKnowledgeGraphError::ProjectionNotFound
    );
    assert_eq!(
        unavailable.unwrap_err(),
        GetLocalKnowledgeGraphError::ProjectionUnavailable
    );
    assert_eq!(
        corrupt.unwrap_err(),
        GetLocalKnowledgeGraphError::CorruptedProjection
    );
    assert!(GetLocalKnowledgeGraphError::ProjectionUnavailable.retryable());
    assert!(!GetLocalKnowledgeGraphError::CorruptedProjection.retryable());
}

fn execute_default(
    store: &FakeStore,
) -> Result<cabinet_usecases::graph::GetLocalKnowledgeGraphOutput, GetLocalKnowledgeGraphError> {
    GetLocalKnowledgeGraphUsecase::new().execute(
        GetLocalKnowledgeGraphInput::new(
            "workspace-1",
            "center-doc",
            1,
            LocalGraphDirection::Both,
            true,
            true,
            100,
            200,
        ),
        store,
    )
}

#[derive(Default)]
struct FakeStore {
    record: Option<GraphProjectionRecord>,
    error: Option<GraphProjectionError>,
}

impl FakeStore {
    fn with_graph(graph: KnowledgeGraph) -> Self {
        Self {
            record: Some(GraphProjectionRecord::new(graph).expect("record")),
            error: None,
        }
    }

    fn failing(error: GraphProjectionError) -> Self {
        Self {
            record: None,
            error: Some(error),
        }
    }
}

impl GraphProjectionStore for FakeStore {
    fn replace_projection(
        &mut self,
        _workspace_id: &WorkspaceId,
        _record: GraphProjectionRecord,
    ) -> Result<(), GraphProjectionError> {
        unreachable!("read test")
    }

    fn get_projection(
        &self,
        _workspace_id: &WorkspaceId,
        _center_document_id: &DocumentId,
    ) -> Result<Option<GraphProjectionRecord>, GraphProjectionError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        Ok(self.record.clone())
    }
}

fn graph_fixture() -> KnowledgeGraph {
    let center_id = DocumentId::new("center-doc").expect("center");
    let center = GraphNode::new_document(center_id.clone());
    let incoming = GraphNode::new_document(DocumentId::new("incoming-doc").expect("incoming"));
    let outgoing = GraphNode::new_document(DocumentId::new("outgoing-doc").expect("outgoing"));
    let unresolved = GraphNode::new_unresolved("missing-doc").expect("unresolved");
    let attachment = GraphNode::new_attachment("asset-1").expect("attachment");
    let edges = vec![
        edge(
            "incoming-edge",
            incoming.id(),
            center.id(),
            GraphEdgeKind::DocumentLink,
        ),
        edge(
            "outgoing-edge",
            center.id(),
            outgoing.id(),
            GraphEdgeKind::DocumentLink,
        ),
        edge(
            "unresolved-edge",
            center.id(),
            unresolved.id(),
            GraphEdgeKind::DocumentLink,
        ),
        edge(
            "asset-edge",
            center.id(),
            attachment.id(),
            GraphEdgeKind::AttachmentReference,
        ),
    ];
    KnowledgeGraph::new_with_center(
        center_id,
        vec![center, incoming, outgoing, unresolved, attachment],
        edges,
        GraphProjectionStatus::Clean,
    )
    .expect("graph")
}

fn edge(id: &str, source: &str, target: &str, kind: GraphEdgeKind) -> GraphEdge {
    GraphEdge::new(id, source.to_string(), target.to_string(), kind).expect("edge")
}
