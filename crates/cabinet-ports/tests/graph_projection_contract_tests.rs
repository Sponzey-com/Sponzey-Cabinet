use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::graph_projection::{GraphProjectionRecord, GraphProjectionStore};

#[derive(Default)]
struct FakeGraphProjectionStore {
    records: Vec<(String, GraphProjectionRecord)>,
}

impl GraphProjectionStore for FakeGraphProjectionStore {
    fn replace_projection(
        &mut self,
        workspace_id: &WorkspaceId,
        record: GraphProjectionRecord,
    ) -> Result<(), cabinet_ports::graph_projection::GraphProjectionError> {
        self.records.retain(|(workspace, existing)| {
            workspace != workspace_id.as_str()
                || existing.graph().center_document_id() != record.graph().center_document_id()
        });
        self.records
            .push((workspace_id.as_str().to_string(), record));
        Ok(())
    }

    fn get_projection(
        &self,
        workspace_id: &WorkspaceId,
        center_document_id: &DocumentId,
    ) -> Result<Option<GraphProjectionRecord>, cabinet_ports::graph_projection::GraphProjectionError>
    {
        Ok(self
            .records
            .iter()
            .find(|(workspace, record)| {
                workspace == workspace_id.as_str()
                    && record.graph().center_document_id() == center_document_id
            })
            .map(|(_, record)| record.clone()))
    }
}

#[test]
fn graph_projection_store_contract_preserves_workspace_and_status() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace");
    let center_document_id = document_id("center-doc");
    let graph = graph_with_status(center_document_id.clone(), GraphProjectionStatus::Degraded);
    let record = GraphProjectionRecord::new(graph).expect("record");
    let mut store = FakeGraphProjectionStore::default();

    store
        .replace_projection(&workspace_id, record)
        .expect("replace projection");

    let stored = store
        .get_projection(&workspace_id, &center_document_id)
        .expect("get projection")
        .expect("stored projection");

    assert_eq!(stored.graph().status(), GraphProjectionStatus::Degraded);
    assert_eq!(stored.graph().center_document_id(), &center_document_id);
}

fn graph_with_status(
    center_document_id: DocumentId,
    status: GraphProjectionStatus,
) -> KnowledgeGraph {
    let center = GraphNode::new_document(center_document_id.clone());
    let neighbor = GraphNode::new_document(document_id("neighbor-doc"));
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
