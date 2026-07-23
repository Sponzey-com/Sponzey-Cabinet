use cabinet_adapters::composite_graph_projection::CompositeGraphProjectionStore;
use cabinet_adapters::durable_canvas_graph_projection::DurableCanvasGraphRelationProjectionStore;
use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_domain::canvas::{CanvasId, CanvasRevision};
use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_graph_projection::{
    CanvasGraphRelationProjectionBatch, CanvasGraphRelationProjectionRecord,
    CanvasGraphRelationProjectionWriter,
};
use cabinet_ports::graph_projection::{
    GraphProjectionError, GraphProjectionRecord, GraphProjectionStore,
    WorkspaceGraphProjectionReader,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn composite_preserves_base_revision_and_merges_canvas_relations_for_local_and_global_reads() {
    let temp = Temp::new("merge");
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let center = DocumentId::new("doc-a").unwrap();
    seed_base(&temp.path, &workspace);
    seed_canvas(&temp.path, &workspace, relation_record(false));

    let composite = CompositeGraphProjectionStore::new(
        DurableLocalGraphProjectionStore::new(temp.path.clone()),
        DurableCanvasGraphRelationProjectionStore::new(temp.path.clone()),
        10,
    )
    .unwrap();
    let local = composite
        .get_projection(&workspace, &center)
        .unwrap()
        .unwrap();
    assert_eq!(local.freshness_revision(), "doc-v1");
    assert_eq!(local.graph().nodes().len(), 3);
    assert_eq!(local.graph().edges().len(), 2);
    assert_eq!(
        local
            .graph()
            .edges()
            .iter()
            .filter(|edge| edge.kind() == GraphEdgeKind::CanvasRelation)
            .count(),
        1
    );

    let global = composite
        .list_workspace_projections(&workspace, None, 10)
        .unwrap();
    assert_eq!(global.records().len(), 1);
    assert_eq!(global.records()[0], local);
    assert_eq!(global.next_cursor(), None);
}

#[test]
fn composite_rejects_node_identity_conflict_instead_of_returning_invalid_graph() {
    let temp = Temp::new("conflict");
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let center = DocumentId::new("doc-a").unwrap();
    seed_base(&temp.path, &workspace);
    seed_canvas(&temp.path, &workspace, relation_record(true));
    let composite = CompositeGraphProjectionStore::new(
        DurableLocalGraphProjectionStore::new(temp.path.clone()),
        DurableCanvasGraphRelationProjectionStore::new(temp.path.clone()),
        10,
    )
    .unwrap();

    assert_eq!(
        composite.get_projection(&workspace, &center),
        Err(GraphProjectionError::CorruptedProjection)
    );
}

fn seed_base(root: &std::path::Path, workspace: &WorkspaceId) {
    let center = DocumentId::new("doc-a").unwrap();
    let source = GraphNode::new_document(center.clone());
    let target = GraphNode::new_document(DocumentId::new("doc-b").unwrap());
    let edge = GraphEdge::new(
        "document-edge",
        source.id().into(),
        target.id().into(),
        GraphEdgeKind::DocumentLink,
    )
    .unwrap();
    let graph = KnowledgeGraph::new_with_center(
        center,
        vec![source, target],
        vec![edge],
        GraphProjectionStatus::Clean,
    )
    .unwrap();
    DurableLocalGraphProjectionStore::new(root.to_path_buf())
        .replace_projection(
            workspace,
            GraphProjectionRecord::new_with_revision(graph, "doc-v1").unwrap(),
        )
        .unwrap();
}

fn seed_canvas(
    root: &std::path::Path,
    workspace: &WorkspaceId,
    relation: CanvasGraphRelationProjectionRecord,
) {
    DurableCanvasGraphRelationProjectionStore::new(root.to_path_buf())
        .replace_canvas_relations(
            workspace,
            CanvasGraphRelationProjectionBatch::new(
                CanvasId::new("canvas-1").unwrap(),
                CanvasRevision::new(1).unwrap(),
                vec![relation],
            )
            .unwrap(),
        )
        .unwrap();
}

fn relation_record(conflict: bool) -> CanvasGraphRelationProjectionRecord {
    let center = DocumentId::new("doc-a").unwrap();
    let source = GraphNode::new_document(center.clone());
    let target = if conflict {
        GraphNode::new_attachment("doc-b").unwrap()
    } else {
        GraphNode::new_document(DocumentId::new("doc-c").unwrap())
    };
    let edge = GraphEdge::new(
        "canvas:canvas-1:edge-1",
        source.id().into(),
        target.id().into(),
        GraphEdgeKind::CanvasRelation,
    )
    .unwrap();
    CanvasGraphRelationProjectionRecord::new(center, vec![source, target], vec![edge]).unwrap()
}

struct Temp {
    path: PathBuf,
}

impl Temp {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cabinet-composite-graph-{label}-{}-{nonce}",
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
