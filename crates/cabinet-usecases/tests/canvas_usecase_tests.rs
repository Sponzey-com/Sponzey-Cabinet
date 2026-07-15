use std::collections::HashMap;

use cabinet_domain::canvas::{CanvasId, CanvasLifecycleState, CanvasRevision};
use cabinet_domain::permission::{
    AccessResource, Permission, PermissionDecision, PermissionDecisionReason, PolicySource,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository, CanvasRepositoryError};
use cabinet_ports::permission_aware_query::{PermissionAwareQueryError, PermissionDecisionPort};
use cabinet_usecases::canvas::{
    AddCanvasNodeInput, AddCanvasNodeTargetInput, AddCanvasNodeUsecase, ConnectCanvasNodesInput,
    ConnectCanvasNodesUsecase, ConvertDocumentOutlineToCanvasInput,
    ConvertDocumentOutlineToCanvasUsecase, CreateCanvasInput, CreateCanvasUsecase,
    DocumentOutlineHeadingInput, EmbedCanvasInDocumentInput, EmbedCanvasInDocumentUsecase,
};

#[derive(Default)]
struct FakeCanvasRepository {
    records: HashMap<(String, String), CanvasRecord>,
}

impl CanvasRepository for FakeCanvasRepository {
    fn create_canvas(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        let key = (
            workspace_id.as_str().to_string(),
            record.canvas().id().as_str().to_string(),
        );
        if self.records.contains_key(&key) {
            return Err(CanvasRepositoryError::AlreadyExists);
        }
        self.records.insert(key, record);
        Ok(())
    }

    fn replace_canvas(
        &mut self,
        workspace_id: &WorkspaceId,
        expected_revision: CanvasRevision,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        let key = (
            workspace_id.as_str().to_string(),
            record.canvas().id().as_str().to_string(),
        );
        if self.records.get(&key).map(CanvasRecord::revision) != Some(expected_revision) {
            return Err(CanvasRepositoryError::VersionConflict);
        }
        self.records.insert(key, record);
        Ok(())
    }

    fn get_canvas(
        &self,
        workspace_id: &WorkspaceId,
        canvas_id: &CanvasId,
    ) -> Result<Option<CanvasRecord>, CanvasRepositoryError> {
        Ok(self
            .records
            .get(&(
                workspace_id.as_str().to_string(),
                canvas_id.as_str().to_string(),
            ))
            .cloned())
    }
}

struct FakePermissionDecision {
    allow: bool,
}

impl PermissionDecisionPort for FakePermissionDecision {
    fn check_permission(
        &self,
        _actor_user_id: &UserId,
        resource: &AccessResource,
        permission: Permission,
    ) -> Result<PermissionDecision, PermissionAwareQueryError> {
        assert!(matches!(
            resource,
            AccessResource::Workspace { .. } | AccessResource::Document { .. }
        ));
        assert_eq!(permission, Permission::Write);
        Ok(if self.allow {
            PermissionDecision::allowed(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleAllowsPermission,
            )
        } else {
            PermissionDecision::denied(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleDoesNotAllowPermission,
            )
        })
    }
}

#[test]
fn create_canvas_requires_write_permission_and_saves_draft_canvas() {
    let mut repository = FakeCanvasRepository::default();
    let usecase = CreateCanvasUsecase::new();

    let output = usecase
        .execute(
            CreateCanvasInput::new("user-1", "workspace-1", "canvas-1"),
            &mut repository,
            &FakePermissionDecision { allow: true },
        )
        .expect("create canvas");

    assert_eq!(output.canvas_id().as_str(), "canvas-1");
    assert_eq!(output.state(), CanvasLifecycleState::Draft);
    assert_eq!(output.product_log_event(), "canvas.created");
    assert_eq!(repository.records.len(), 1);
}

#[test]
fn create_canvas_rejects_permission_denied_without_repository_write() {
    let mut repository = FakeCanvasRepository::default();
    let usecase = CreateCanvasUsecase::new();

    let error = usecase
        .execute(
            CreateCanvasInput::new("user-1", "workspace-1", "canvas-1"),
            &mut repository,
            &FakePermissionDecision { allow: false },
        )
        .expect_err("permission denied");

    assert_eq!(error.code(), "CANVAS_PERMISSION_DENIED");
    assert_eq!(repository.records.len(), 0);
}

#[test]
fn add_canvas_node_updates_existing_canvas_without_raw_ui_state_output() {
    let mut repository = FakeCanvasRepository::default();
    let create = CreateCanvasUsecase::new();
    create
        .execute(
            CreateCanvasInput::new("user-1", "workspace-1", "canvas-1"),
            &mut repository,
            &FakePermissionDecision { allow: true },
        )
        .expect("create canvas");
    let add_node = AddCanvasNodeUsecase::new();

    let output = add_node
        .execute(
            AddCanvasNodeInput::new(
                "user-1",
                "workspace-1",
                "canvas-1",
                "node-1",
                AddCanvasNodeTargetInput::TextCard {
                    text: "Canvas note".to_string(),
                },
                12,
                24,
            ),
            &mut repository,
            &FakePermissionDecision { allow: true },
        )
        .expect("add node");

    assert_eq!(output.node_count(), 1);
    assert_eq!(output.state(), CanvasLifecycleState::Updated);
    assert_eq!(output.product_log_event(), "canvas.node.added");
    assert!(!output.product_log_event().contains("Canvas note"));
}

#[test]
fn connect_canvas_nodes_rejects_missing_node_edge_without_save() {
    let mut repository = FakeCanvasRepository::default();
    let create = CreateCanvasUsecase::new();
    create
        .execute(
            CreateCanvasInput::new("user-1", "workspace-1", "canvas-1"),
            &mut repository,
            &FakePermissionDecision { allow: true },
        )
        .expect("create canvas");
    let connect = ConnectCanvasNodesUsecase::new();

    let error = connect
        .execute(
            ConnectCanvasNodesInput::new(
                "user-1",
                "workspace-1",
                "canvas-1",
                "edge-1",
                "missing-source",
                "missing-target",
            ),
            &mut repository,
            &FakePermissionDecision { allow: true },
        )
        .expect_err("missing node");

    assert_eq!(error.code(), "CANVAS_INVALID_GRAPH");
    let stored = repository
        .get_canvas(
            &WorkspaceId::new("workspace-1").expect("workspace"),
            &CanvasId::new("canvas-1").expect("canvas"),
        )
        .expect("get canvas")
        .expect("stored");
    assert_eq!(stored.canvas().edges().len(), 0);
}

#[test]
fn embed_canvas_in_document_returns_stable_reference_without_raw_ui_state() {
    let mut repository = FakeCanvasRepository::default();
    let create = CreateCanvasUsecase::new();
    create
        .execute(
            CreateCanvasInput::new("user-1", "workspace-1", "canvas-1"),
            &mut repository,
            &FakePermissionDecision { allow: true },
        )
        .expect("create canvas");
    let embed = EmbedCanvasInDocumentUsecase::new();

    let output = embed
        .execute(
            EmbedCanvasInDocumentInput::new("user-1", "workspace-1", "doc-1", "canvas-1"),
            &repository,
            &FakePermissionDecision { allow: true },
        )
        .expect("embed canvas");

    assert_eq!(output.reference(), "canvas:canvas-1");
    assert_eq!(output.product_log_event(), "canvas.embedded");
    assert!(!output.reference().contains('{'));
    assert!(!output.reference().contains("nodes"));
    assert!(!output.product_log_event().contains("doc-1"));
}

#[test]
fn embed_canvas_in_document_rejects_missing_canvas() {
    let repository = FakeCanvasRepository::default();
    let embed = EmbedCanvasInDocumentUsecase::new();

    let error = embed
        .execute(
            EmbedCanvasInDocumentInput::new("user-1", "workspace-1", "doc-1", "missing-canvas"),
            &repository,
            &FakePermissionDecision { allow: true },
        )
        .expect_err("missing canvas");

    assert_eq!(error.code(), "CANVAS_NOT_FOUND");
}

#[test]
fn convert_document_outline_to_canvas_preserves_heading_order() {
    let usecase = ConvertDocumentOutlineToCanvasUsecase::new();

    let output = usecase
        .execute(ConvertDocumentOutlineToCanvasInput::new(vec![
            DocumentOutlineHeadingInput::new("h1", "Overview", 1),
            DocumentOutlineHeadingInput::new("h2", "Details", 2),
            DocumentOutlineHeadingInput::new("h3", "Decision", 1),
        ]))
        .expect("convert outline");

    assert_eq!(output.suggestions().len(), 3);
    assert_eq!(output.suggestions()[0].heading_id(), "h1");
    assert_eq!(output.suggestions()[1].heading_id(), "h2");
    assert_eq!(output.suggestions()[2].heading_id(), "h3");
    assert!(output.suggestions()[1].x() > output.suggestions()[0].x());
    assert!(output.suggestions()[2].y() > output.suggestions()[1].y());
}

#[test]
fn convert_document_outline_to_canvas_rejects_invalid_heading() {
    let usecase = ConvertDocumentOutlineToCanvasUsecase::new();

    let error = usecase
        .execute(ConvertDocumentOutlineToCanvasInput::new(vec![
            DocumentOutlineHeadingInput::new("h1", "Overview", 1),
            DocumentOutlineHeadingInput::new("", "Missing id", 2),
        ]))
        .expect_err("invalid heading");

    assert_eq!(error.code(), "CANVAS_INVALID_INPUT");
}
