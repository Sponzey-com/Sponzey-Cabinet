use std::collections::HashMap;

use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::permission::{
    AccessResource, Permission, PermissionDecision, PermissionDecisionReason, PolicySource,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::graph_projection::{
    GraphProjectionError, GraphProjectionRecord, GraphProjectionStore,
};
use cabinet_ports::permission_aware_query::{PermissionAwareQueryError, PermissionDecisionPort};
use cabinet_usecases::graph::{
    PermissionAwareGraphError, PermissionAwareGraphInput, PermissionAwareGraphUsecase,
};

#[derive(Default)]
struct FakeGraphProjectionStore {
    records: HashMap<(String, String), GraphProjectionRecord>,
}

impl FakeGraphProjectionStore {
    fn insert(&mut self, workspace_id: &WorkspaceId, record: GraphProjectionRecord) {
        self.records.insert(
            (
                workspace_id.as_str().to_string(),
                record.graph().center_document_id().as_str().to_string(),
            ),
            record,
        );
    }
}

impl GraphProjectionStore for FakeGraphProjectionStore {
    fn replace_projection(
        &mut self,
        workspace_id: &WorkspaceId,
        record: GraphProjectionRecord,
    ) -> Result<(), GraphProjectionError> {
        self.insert(workspace_id, record);
        Ok(())
    }

    fn get_projection(
        &self,
        workspace_id: &WorkspaceId,
        center_document_id: &DocumentId,
    ) -> Result<Option<GraphProjectionRecord>, GraphProjectionError> {
        Ok(self
            .records
            .get(&(
                workspace_id.as_str().to_string(),
                center_document_id.as_str().to_string(),
            ))
            .cloned())
    }
}

#[derive(Default)]
struct FakePermissionDecision {
    denied_document_ids: Vec<String>,
}

impl PermissionDecisionPort for FakePermissionDecision {
    fn check_permission(
        &self,
        _actor_user_id: &UserId,
        resource: &AccessResource,
        permission: Permission,
    ) -> Result<PermissionDecision, PermissionAwareQueryError> {
        assert_eq!(permission, Permission::Read);
        let denied = resource
            .document_id()
            .map(|document_id| {
                self.denied_document_ids
                    .contains(&document_id.as_str().to_string())
            })
            .unwrap_or(false);
        Ok(if denied {
            PermissionDecision::denied(
                PolicySource::Document,
                PermissionDecisionReason::HiddenByPolicy,
            )
        } else {
            PermissionDecision::allowed(
                PolicySource::Document,
                PermissionDecisionReason::RoleAllowsPermission,
            )
        })
    }
}

#[test]
fn permission_aware_graph_filters_denied_document_nodes_and_edges() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace");
    let center_document_id = document_id("center-doc");
    let mut projection_store = FakeGraphProjectionStore::default();
    projection_store.insert(
        &workspace_id,
        GraphProjectionRecord::new(graph_with_two_neighbors(center_document_id.clone()))
            .expect("record"),
    );
    let permission = FakePermissionDecision {
        denied_document_ids: vec!["hidden-doc".to_string()],
    };
    let usecase = PermissionAwareGraphUsecase::new();

    let output = usecase
        .execute(
            PermissionAwareGraphInput::new("workspace-1", "viewer-1", "center-doc"),
            &projection_store,
            &permission,
        )
        .expect("graph output");

    assert!(
        output
            .graph()
            .nodes()
            .iter()
            .any(|node| node.id() == "visible-doc")
    );
    assert!(
        !output
            .graph()
            .nodes()
            .iter()
            .any(|node| node.id() == "hidden-doc")
    );
    assert!(
        !output
            .graph()
            .edges()
            .iter()
            .any(|edge| edge.target_id() == "hidden-doc")
    );
    assert_eq!(output.stats().candidate_count(), 3);
    assert_eq!(output.stats().filtered_count(), 1);
}

#[test]
fn permission_aware_graph_returns_stable_not_found_error() {
    let projection_store = FakeGraphProjectionStore::default();
    let permission = FakePermissionDecision::default();
    let usecase = PermissionAwareGraphUsecase::new();

    let error = usecase
        .execute(
            PermissionAwareGraphInput::new("workspace-1", "viewer-1", "missing-doc"),
            &projection_store,
            &permission,
        )
        .expect_err("missing graph");

    assert_eq!(error, PermissionAwareGraphError::ProjectionNotFound);
    assert_eq!(error.code(), "graph.projection_not_found");
}

fn graph_with_two_neighbors(center_document_id: DocumentId) -> KnowledgeGraph {
    let center = GraphNode::new_document(center_document_id.clone());
    let visible = GraphNode::new_document(document_id("visible-doc"));
    let hidden = GraphNode::new_document(document_id("hidden-doc"));
    let edges = vec![
        GraphEdge::new(
            "edge-visible",
            center.id().to_string(),
            visible.id().to_string(),
            GraphEdgeKind::DocumentLink,
        )
        .expect("visible edge"),
        GraphEdge::new(
            "edge-hidden",
            center.id().to_string(),
            hidden.id().to_string(),
            GraphEdgeKind::DocumentLink,
        )
        .expect("hidden edge"),
    ];
    KnowledgeGraph::new_with_center(
        center_document_id,
        vec![center, visible, hidden],
        edges,
        GraphProjectionStatus::Clean,
    )
    .expect("graph")
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}
