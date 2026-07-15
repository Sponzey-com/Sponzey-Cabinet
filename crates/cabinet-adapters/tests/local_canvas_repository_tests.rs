use cabinet_adapters::local_canvas_repository::LocalCanvasRepository;
use cabinet_domain::canvas::{
    Canvas, CanvasId, CanvasLifecycleState, CanvasNode, CanvasNodeId, CanvasNodeTarget,
    CanvasPosition,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository};

#[test]
fn local_canvas_repository_keeps_workspace_canvases_separate() {
    let canvas_id = CanvasId::new("canvas-1").expect("canvas id");
    let first_workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let second_workspace = WorkspaceId::new("workspace-2").expect("workspace");
    let mut repository = LocalCanvasRepository::new();

    repository
        .create_canvas(
            &first_workspace,
            CanvasRecord::new(canvas_with_node(
                canvas_id.clone(),
                "first-node",
                CanvasLifecycleState::Saved,
            ))
            .expect("record"),
        )
        .expect("save first");
    repository
        .create_canvas(
            &second_workspace,
            CanvasRecord::new(canvas_with_node(
                canvas_id.clone(),
                "second-node",
                CanvasLifecycleState::Updated,
            ))
            .expect("record"),
        )
        .expect("save second");

    let first = repository
        .get_canvas(&first_workspace, &canvas_id)
        .expect("first")
        .expect("first canvas");
    let second = repository
        .get_canvas(&second_workspace, &canvas_id)
        .expect("second")
        .expect("second canvas");

    assert_eq!(first.canvas().state(), CanvasLifecycleState::Saved);
    assert!(
        first
            .canvas()
            .nodes()
            .iter()
            .any(|node| node.id().as_str() == "first-node")
    );
    assert_eq!(second.canvas().state(), CanvasLifecycleState::Updated);
    assert!(
        second
            .canvas()
            .nodes()
            .iter()
            .any(|node| node.id().as_str() == "second-node")
    );
}

#[test]
fn local_canvas_repository_replaces_existing_canvas_record() {
    let canvas_id = CanvasId::new("canvas-1").expect("canvas id");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace");
    let mut repository = LocalCanvasRepository::new();

    repository
        .create_canvas(
            &workspace_id,
            CanvasRecord::new(canvas_with_node(
                canvas_id.clone(),
                "first-node",
                CanvasLifecycleState::Saved,
            ))
            .expect("record"),
        )
        .expect("save first");
    let current = repository
        .get_canvas(&workspace_id, &canvas_id)
        .expect("get current")
        .expect("current");
    let expected_revision = current.revision();
    let replacement = current
        .next(canvas_with_node(
            canvas_id.clone(),
            "replacement-node",
            CanvasLifecycleState::Updated,
        ))
        .expect("next record");
    repository
        .replace_canvas(&workspace_id, expected_revision, replacement)
        .expect("save replacement");

    let stored = repository
        .get_canvas(&workspace_id, &canvas_id)
        .expect("get")
        .expect("stored");

    assert_eq!(stored.canvas().state(), CanvasLifecycleState::Updated);
    assert_eq!(stored.canvas().nodes().len(), 1);
    assert_eq!(stored.canvas().nodes()[0].id().as_str(), "replacement-node");
}

fn canvas_with_node(canvas_id: CanvasId, node_id: &str, state: CanvasLifecycleState) -> Canvas {
    Canvas::new(
        canvas_id,
        vec![
            CanvasNode::new(
                CanvasNodeId::new(node_id).expect("node id"),
                CanvasNodeTarget::Document(DocumentId::new("doc-1").expect("document")),
                CanvasPosition::new(0, 0),
            )
            .expect("node"),
        ],
        vec![],
        state,
    )
    .expect("canvas")
}
