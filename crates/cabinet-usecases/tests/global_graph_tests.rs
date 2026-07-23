use cabinet_domain::{
    document::DocumentId,
    graph::{GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph},
    version::VersionId,
    workspace::WorkspaceId,
};
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::graph_projection::{
    GraphProjectionError, GraphProjectionRecord, WorkspaceGraphProjectionPage,
    WorkspaceGraphProjectionReader,
};
use cabinet_usecases::global_graph::{
    GetGlobalKnowledgeGraphError, GetGlobalKnowledgeGraphInput, GetGlobalKnowledgeGraphUsecase,
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
            GetGlobalKnowledgeGraphInput::new("workspace-1", None, true, true, 10, 3, 2),
            &reader,
            &FakePointer::matching(),
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
            GetGlobalKnowledgeGraphInput::new("workspace-1", Some("doc-0"), true, true, 1, 10, 10),
            &reader,
            &FakePointer::matching(),
        )
        .unwrap();
    assert_eq!(output.next_cursor(), Some("next-center"));
    assert_eq!(output.status(), GraphProjectionStatus::Degraded);
}

#[test]
fn global_graph_rejects_a_page_that_cannot_fit_without_silent_cursor_skips() {
    let reader = FakeReader {
        records: vec![record("doc-a", "doc-b", "edge-1")],
    };
    assert_eq!(
        GetGlobalKnowledgeGraphUsecase::new().execute(
            GetGlobalKnowledgeGraphInput::new("workspace-1", None, true, true, 1, 1, 10),
            &reader,
            &FakePointer::matching(),
        ),
        Err(GetGlobalKnowledgeGraphError::InvalidInput)
    );
}

#[test]
fn global_graph_rejects_a_malformed_center_cursor() {
    let reader = FakeReader { records: vec![] };
    assert_eq!(
        GetGlobalKnowledgeGraphUsecase::new().execute(
            GetGlobalKnowledgeGraphInput::new("workspace-1", Some("\n"), true, true, 1, 10, 10,),
            &reader,
            &FakePointer::matching(),
        ),
        Err(GetGlobalKnowledgeGraphError::InvalidInput)
    );
}

#[test]
fn global_graph_filters_optional_node_kinds_without_dangling_edges() {
    let center = DocumentId::new("doc-a").unwrap();
    let document = GraphNode::new_document(center.clone());
    let unresolved = GraphNode::new_unresolved("Missing Note").unwrap();
    let attachment = GraphNode::new_attachment(&"a".repeat(64)).unwrap();
    let graph = KnowledgeGraph::new_with_center(
        center,
        vec![document.clone(), unresolved.clone(), attachment.clone()],
        vec![
            GraphEdge::new(
                "missing-edge",
                document.id().into(),
                unresolved.id().into(),
                GraphEdgeKind::DocumentLink,
            )
            .unwrap(),
            GraphEdge::new(
                "asset-edge",
                document.id().into(),
                attachment.id().into(),
                GraphEdgeKind::AttachmentReference,
            )
            .unwrap(),
        ],
        GraphProjectionStatus::Clean,
    )
    .unwrap();
    let reader = FakeReader {
        records: vec![GraphProjectionRecord::new_with_revision(graph, "v1").unwrap()],
    };
    let output = GetGlobalKnowledgeGraphUsecase::new()
        .execute(
            GetGlobalKnowledgeGraphInput::new("workspace-1", None, false, false, 1, 10, 10),
            &reader,
            &FakePointer::matching(),
        )
        .unwrap();

    assert_eq!(output.candidate_count(), 3);
    assert_eq!(output.nodes(), &[document]);
    assert!(output.edges().is_empty());
    assert_eq!(output.status(), GraphProjectionStatus::Clean);
}

#[test]
fn global_graph_marks_missing_or_mismatched_current_pointer_as_degraded() {
    let reader = FakeReader {
        records: vec![clean_record("doc-a")],
    };
    for pointer in [FakePointer::missing(), FakePointer::mismatched()] {
        let output = GetGlobalKnowledgeGraphUsecase::new()
            .execute(
                GetGlobalKnowledgeGraphInput::new("workspace-1", None, true, true, 1, 10, 10),
                &reader,
                &pointer,
            )
            .unwrap();
        assert_eq!(output.status(), GraphProjectionStatus::Degraded);
    }
}

#[test]
fn global_graph_maps_current_pointer_storage_and_corruption_failures() {
    let reader = FakeReader {
        records: vec![clean_record("doc-a")],
    };
    for (pointer, expected) in [
        (
            FakePointer::failing(CurrentDocumentVersionPointerError::StorageUnavailable),
            GetGlobalKnowledgeGraphError::ProjectionUnavailable,
        ),
        (
            FakePointer::failing(CurrentDocumentVersionPointerError::CorruptedPointer),
            GetGlobalKnowledgeGraphError::CorruptedProjection,
        ),
    ] {
        assert_eq!(
            GetGlobalKnowledgeGraphUsecase::new().execute(
                GetGlobalKnowledgeGraphInput::new("workspace-1", None, true, true, 1, 10, 10,),
                &reader,
                &pointer,
            ),
            Err(expected)
        );
    }
}
struct FakeReader {
    records: Vec<GraphProjectionRecord>,
}

struct FakePointer {
    result: Result<Option<VersionId>, CurrentDocumentVersionPointerError>,
}
impl FakePointer {
    fn matching() -> Self {
        Self {
            result: Ok(Some(VersionId::new("v1").unwrap())),
        }
    }
    fn missing() -> Self {
        Self { result: Ok(None) }
    }
    fn mismatched() -> Self {
        Self {
            result: Ok(Some(VersionId::new("v2").unwrap())),
        }
    }
    const fn failing(error: CurrentDocumentVersionPointerError) -> Self {
        Self { result: Err(error) }
    }
}
impl CurrentDocumentVersionPointerPort for FakePointer {
    fn load_current_version(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<Option<VersionId>, CurrentDocumentVersionPointerError> {
        self.result.clone()
    }

    fn compare_and_set_current_version(
        &mut self,
        _: &WorkspaceId,
        _: &DocumentId,
        _: Option<&VersionId>,
        _: VersionId,
    ) -> Result<(), CurrentDocumentVersionPointerError> {
        unreachable!("read-only query fake")
    }
}

fn clean_record(center: &str) -> GraphProjectionRecord {
    let center = DocumentId::new(center).unwrap();
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
