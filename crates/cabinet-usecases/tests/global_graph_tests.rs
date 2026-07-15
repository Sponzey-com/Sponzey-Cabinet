use cabinet_domain::{
    document::DocumentId,
    graph::{GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph},
    workspace::WorkspaceId,
};
use cabinet_ports::graph_projection::{
    GraphProjectionError, GraphProjectionRecord, WorkspaceGraphProjectionPage,
    WorkspaceGraphProjectionReader,
};
use cabinet_usecases::global_graph::{
    GetGlobalKnowledgeGraphInput, GetGlobalKnowledgeGraphUsecase,
};

#[test]
fn global_graph_deduplicates_shared_identity_and_honors_hard_limits() {
    let reader = FakeReader {
        records: vec![
            record("doc-a", "doc-b", "edge-shared"),
            record("doc-b", "doc-c", "edge-shared"),
        ],
    };
    let output = GetGlobalKnowledgeGraphUsecase::new()
        .execute(
            GetGlobalKnowledgeGraphInput::new("workspace-1", None, 10, 3, 2),
            &reader,
        )
        .unwrap();
    assert_eq!(output.nodes().len(), 3);
    assert_eq!(output.edges().len(), 1);
    assert_eq!(output.candidate_count(), 4);
}

#[test]
fn global_graph_propagates_cursor_and_degraded_status() {
    let reader = FakeReader {
        records: vec![record("doc-a", "doc-b", "edge-1")],
    };
    let output = GetGlobalKnowledgeGraphUsecase::new()
        .execute(
            GetGlobalKnowledgeGraphInput::new("workspace-1", Some("doc-0"), 1, 10, 10),
            &reader,
        )
        .unwrap();
    assert_eq!(output.next_cursor(), Some("next-center"));
    assert_eq!(output.status(), GraphProjectionStatus::Degraded);
}
struct FakeReader {
    records: Vec<GraphProjectionRecord>,
}
impl WorkspaceGraphProjectionReader for FakeReader {
    fn list_workspace_projections(
        &self,
        _: &WorkspaceId,
        _: Option<&str>,
        _: usize,
    ) -> Result<WorkspaceGraphProjectionPage, GraphProjectionError> {
        Ok(WorkspaceGraphProjectionPage::new(
            self.records.clone(),
            Some("next-center".into()),
        ))
    }
}
fn record(center: &str, neighbor: &str, edge: &str) -> GraphProjectionRecord {
    let center_id = DocumentId::new(center).unwrap();
    let graph = KnowledgeGraph::new_with_center(
        center_id.clone(),
        vec![
            GraphNode::new_document(center_id),
            GraphNode::new_document(DocumentId::new(neighbor).unwrap()),
        ],
        vec![
            GraphEdge::new(
                edge,
                center.to_string(),
                neighbor.to_string(),
                GraphEdgeKind::DocumentLink,
            )
            .unwrap(),
        ],
        GraphProjectionStatus::Degraded,
    )
    .unwrap();
    GraphProjectionRecord::new_with_revision(graph, "v1").unwrap()
}
