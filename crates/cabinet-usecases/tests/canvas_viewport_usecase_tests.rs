use cabinet_domain::canvas::{
    CanvasId, CanvasLifecycleState, CanvasRevision, CanvasTitle, CanvasViewport,
};
use cabinet_ports::canvas_viewport_query::{
    CanvasViewportPage, CanvasViewportQuery, CanvasViewportQueryError, CanvasViewportQueryPort,
};
use cabinet_usecases::canvas_viewport::{
    GetCanvasViewportError, GetCanvasViewportInput, GetCanvasViewportUsecase,
};

#[test]
fn viewport_usecase_validates_bounds_and_returns_bounded_page() {
    let port = FakePort {
        result: Ok(Some(page())),
    };
    let output = GetCanvasViewportUsecase::new()
        .execute(input(250, 500), &port)
        .expect("viewport");
    assert_eq!(output.revision.value(), 7);
    assert_eq!(output.total_node_count, 2_000);

    assert_eq!(
        GetCanvasViewportUsecase::new()
            .execute(input(251, 500), &port)
            .expect_err("node cap"),
        GetCanvasViewportError::InvalidInput,
    );
}

#[test]
fn viewport_usecase_maps_stale_and_corrupt_projection_explicitly() {
    for (source, expected) in [
        (
            CanvasViewportQueryError::StaleProjection,
            GetCanvasViewportError::StaleProjection,
        ),
        (
            CanvasViewportQueryError::CorruptedProjection,
            GetCanvasViewportError::RecoveryRequired,
        ),
    ] {
        let port = FakePort {
            result: Err(source),
        };
        assert_eq!(
            GetCanvasViewportUsecase::new().execute(input(250, 500), &port),
            Err(expected),
        );
    }
}

fn input(node_limit: usize, edge_limit: usize) -> GetCanvasViewportInput {
    GetCanvasViewportInput::new(
        "workspace-1",
        "canvas-1",
        None,
        None,
        None,
        1_200,
        720,
        120,
        node_limit,
        edge_limit,
    )
}

fn page() -> CanvasViewportPage {
    CanvasViewportPage {
        canvas_id: CanvasId::new("canvas-1").expect("canvas"),
        title: CanvasTitle::new("Map").expect("title"),
        revision: CanvasRevision::new(7).expect("revision"),
        lifecycle: CanvasLifecycleState::Updated,
        viewport: CanvasViewport::default(),
        nodes: vec![],
        edges: vec![],
        total_node_count: 2_000,
        total_edge_count: 4_000,
        matching_node_count: 0,
        matching_edge_count: 0,
        truncated: true,
    }
}

struct FakePort {
    result: Result<Option<CanvasViewportPage>, CanvasViewportQueryError>,
}
impl CanvasViewportQueryPort for FakePort {
    fn query_viewport(
        &self,
        _: &cabinet_domain::workspace::WorkspaceId,
        _: &CanvasId,
        _: CanvasViewportQuery,
    ) -> Result<Option<CanvasViewportPage>, CanvasViewportQueryError> {
        self.result.clone()
    }
}
