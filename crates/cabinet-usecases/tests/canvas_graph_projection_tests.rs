use cabinet_domain::asset::AssetId;
use cabinet_domain::canvas::{
    Canvas, CanvasEdge, CanvasEdgeId, CanvasExternalLink, CanvasId, CanvasLifecycleState,
    CanvasNode, CanvasNodeId, CanvasNodeTarget, CanvasPosition, CanvasRevision, CanvasTextCard,
    CanvasTitle, CanvasViewport,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{GraphEdgeKind, GraphNodeKind};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_graph_projection::{
    CanvasGraphRelationProjectionBatch, CanvasGraphRelationProjectionError,
    CanvasGraphRelationProjectionWriter,
};
use cabinet_ports::canvas_repository::CanvasRecord;
use cabinet_usecases::canvas_graph_projection::{
    CanvasGraphProjectionPolicy, ProjectCanvasGraphRelationsInput,
    ProjectCanvasGraphRelationsUsecase,
};

#[test]
fn projects_document_relations_with_stable_center_records_and_target_kinds() {
    let record = record(
        CanvasLifecycleState::Saved,
        vec![
            node("a", CanvasNodeTarget::Document(document("doc-a"))),
            node("b", CanvasNodeTarget::Document(document("doc-b"))),
            node(
                "asset",
                CanvasNodeTarget::Attachment(AssetId::from_sha256_hex(&"a".repeat(64)).unwrap()),
            ),
            node(
                "external",
                CanvasNodeTarget::ExternalLink(
                    CanvasExternalLink::new("https://example.com/private?q=secret").unwrap(),
                ),
            ),
        ],
        vec![
            edge("docs", "a", "b"),
            edge("asset", "a", "asset"),
            edge("web", "b", "external"),
        ],
    );
    let mut writer = Writer::default();

    let output =
        ProjectCanvasGraphRelationsUsecase::new(CanvasGraphProjectionPolicy::new(100).unwrap())
            .execute(
                ProjectCanvasGraphRelationsInput::new("workspace-1", record),
                &mut writer,
            )
            .unwrap();

    assert_eq!(output.projected_record_count(), 2);
    assert_eq!(output.projected_edge_count(), 4);
    let batch = writer.batch.unwrap();
    assert_eq!(batch.canvas_id().as_str(), "canvas-1");
    assert_eq!(batch.canvas_revision().value(), 7);
    assert_eq!(batch.records().len(), 2);
    let doc_a = batch
        .records()
        .iter()
        .find(|record| record.center_document_id().as_str() == "doc-a")
        .unwrap();
    assert_eq!(doc_a.edges().len(), 2);
    assert!(doc_a.edges().iter().all(|edge| {
        edge.kind() == GraphEdgeKind::CanvasRelation && edge.id().starts_with("canvas:canvas-1:")
    }));
    assert!(
        doc_a
            .nodes()
            .iter()
            .any(|node| node.kind() == GraphNodeKind::Attachment)
    );
    let doc_b = batch
        .records()
        .iter()
        .find(|record| record.center_document_id().as_str() == "doc-b")
        .unwrap();
    assert!(
        doc_b
            .nodes()
            .iter()
            .any(|node| node.kind() == GraphNodeKind::ExternalLink)
    );
    let doc_edge_a = doc_a
        .edges()
        .iter()
        .find(|edge| edge.id().ends_with(":docs"))
        .unwrap();
    let doc_edge_b = doc_b
        .edges()
        .iter()
        .find(|edge| edge.id().ends_with(":docs"))
        .unwrap();
    assert_eq!(doc_edge_a, doc_edge_b);
}

#[test]
fn skips_text_only_relations_and_archived_canvas_replaces_with_empty_batch() {
    let active = record(
        CanvasLifecycleState::Saved,
        vec![
            node("doc", CanvasNodeTarget::Document(document("doc-a"))),
            node(
                "text",
                CanvasNodeTarget::TextCard(CanvasTextCard::new("private text").unwrap()),
            ),
            node(
                "web",
                CanvasNodeTarget::ExternalLink(
                    CanvasExternalLink::new("https://example.com").unwrap(),
                ),
            ),
        ],
        vec![edge("text", "doc", "text"), edge("no-doc", "text", "web")],
    );
    let archived = record(CanvasLifecycleState::Archived, vec![], vec![]);
    let mut writer = Writer::default();
    let usecase =
        ProjectCanvasGraphRelationsUsecase::new(CanvasGraphProjectionPolicy::new(100).unwrap());

    let active_output = usecase
        .execute(
            ProjectCanvasGraphRelationsInput::new("workspace-1", active),
            &mut writer,
        )
        .unwrap();
    assert_eq!(active_output.skipped_edge_count(), 2);
    assert!(writer.batch.as_ref().unwrap().records().is_empty());

    usecase
        .execute(
            ProjectCanvasGraphRelationsInput::new("workspace-1", archived),
            &mut writer,
        )
        .unwrap();
    assert!(writer.batch.unwrap().records().is_empty());
}

#[test]
fn rejects_relation_count_over_policy_before_writing() {
    let record = record(
        CanvasLifecycleState::Saved,
        vec![
            node("a", CanvasNodeTarget::Document(document("doc-a"))),
            node("b", CanvasNodeTarget::Document(document("doc-b"))),
        ],
        vec![edge("one", "a", "b"), edge("two", "b", "a")],
    );
    let mut writer = Writer::default();

    let result =
        ProjectCanvasGraphRelationsUsecase::new(CanvasGraphProjectionPolicy::new(1).unwrap())
            .execute(
                ProjectCanvasGraphRelationsInput::new("workspace-1", record),
                &mut writer,
            );

    assert_eq!(
        result,
        Err(CanvasGraphRelationProjectionError::RelationLimitExceeded)
    );
    assert!(writer.batch.is_none());
}

#[test]
fn relation_limit_counts_only_edges_eligible_for_graph_projection() {
    let record = record(
        CanvasLifecycleState::Saved,
        vec![
            node(
                "text-a",
                CanvasNodeTarget::TextCard(CanvasTextCard::new("first private note").unwrap()),
            ),
            node(
                "text-b",
                CanvasNodeTarget::TextCard(CanvasTextCard::new("second private note").unwrap()),
            ),
        ],
        vec![
            edge("one", "text-a", "text-b"),
            edge("two", "text-b", "text-a"),
        ],
    );
    let mut writer = Writer::default();

    let output =
        ProjectCanvasGraphRelationsUsecase::new(CanvasGraphProjectionPolicy::new(1).unwrap())
            .execute(
                ProjectCanvasGraphRelationsInput::new("workspace-1", record),
                &mut writer,
            )
            .expect("text-only edges do not consume the topology relation budget");

    assert_eq!(output.projected_edge_count(), 0);
    assert_eq!(output.skipped_edge_count(), 2);
    assert!(writer.batch.unwrap().records().is_empty());
}

#[derive(Default)]
struct Writer {
    batch: Option<CanvasGraphRelationProjectionBatch>,
}

impl CanvasGraphRelationProjectionWriter for Writer {
    fn replace_canvas_relations(
        &mut self,
        _: &WorkspaceId,
        batch: CanvasGraphRelationProjectionBatch,
    ) -> Result<(), CanvasGraphRelationProjectionError> {
        self.batch = Some(batch);
        Ok(())
    }
}

fn record(
    state: CanvasLifecycleState,
    nodes: Vec<CanvasNode>,
    edges: Vec<CanvasEdge>,
) -> CanvasRecord {
    let canvas = Canvas::new(CanvasId::new("canvas-1").unwrap(), nodes, edges, state).unwrap();
    CanvasRecord::with_metadata(
        canvas,
        CanvasTitle::new("Product Map").unwrap(),
        CanvasRevision::new(7).unwrap(),
        CanvasViewport::default(),
    )
}

fn node(id: &str, target: CanvasNodeTarget) -> CanvasNode {
    CanvasNode::new(
        CanvasNodeId::new(id).unwrap(),
        target,
        CanvasPosition::new(0, 0),
    )
    .unwrap()
}

fn edge(id: &str, source: &str, target: &str) -> CanvasEdge {
    CanvasEdge::new(
        CanvasEdgeId::new(id).unwrap(),
        CanvasNodeId::new(source).unwrap(),
        CanvasNodeId::new(target).unwrap(),
    )
    .unwrap()
}

fn document(id: &str) -> DocumentId {
    DocumentId::new(id).unwrap()
}
