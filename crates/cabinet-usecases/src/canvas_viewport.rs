use cabinet_domain::canvas::CanvasId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_viewport_query::{
    CanvasViewportPage, CanvasViewportQuery, CanvasViewportQueryError, CanvasViewportQueryPort,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetCanvasViewportInput {
    workspace_id: String,
    canvas_id: String,
    center_x: Option<i32>,
    center_y: Option<i32>,
    zoom_percent: Option<u16>,
    surface_width: u32,
    surface_height: u32,
    overscan: u32,
    node_limit: usize,
    edge_limit: usize,
}

impl GetCanvasViewportInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: &str,
        canvas_id: &str,
        center_x: Option<i32>,
        center_y: Option<i32>,
        zoom_percent: Option<u16>,
        surface_width: u32,
        surface_height: u32,
        overscan: u32,
        node_limit: usize,
        edge_limit: usize,
    ) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            canvas_id: canvas_id.into(),
            center_x,
            center_y,
            zoom_percent,
            surface_width,
            surface_height,
            overscan,
            node_limit,
            edge_limit,
        }
    }
}

pub struct GetCanvasViewportUsecase;
impl GetCanvasViewportUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute<P: CanvasViewportQueryPort>(
        &self,
        input: GetCanvasViewportInput,
        port: &P,
    ) -> Result<CanvasViewportPage, GetCanvasViewportError> {
        if input.surface_width == 0
            || input.surface_height == 0
            || input.node_limit == 0
            || input.node_limit > 250
            || input.edge_limit == 0
            || input.edge_limit > 500
            || input.center_x.is_some() != input.center_y.is_some()
            || input.center_x.is_some() != input.zoom_percent.is_some()
        {
            return Err(GetCanvasViewportError::InvalidInput);
        }
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| GetCanvasViewportError::InvalidInput)?;
        let canvas =
            CanvasId::new(&input.canvas_id).map_err(|_| GetCanvasViewportError::InvalidInput)?;
        port.query_viewport(
            &workspace,
            &canvas,
            CanvasViewportQuery {
                center_x: input.center_x,
                center_y: input.center_y,
                zoom_percent: input.zoom_percent,
                surface_width: input.surface_width,
                surface_height: input.surface_height,
                overscan: input.overscan,
                node_limit: input.node_limit,
                edge_limit: input.edge_limit,
            },
        )
        .map_err(map_port)?
        .ok_or(GetCanvasViewportError::NotFound)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetCanvasViewportError {
    InvalidInput,
    NotFound,
    StorageUnavailable,
    StaleProjection,
    RecoveryRequired,
}
impl GetCanvasViewportError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "CANVAS_INVALID_INPUT",
            Self::NotFound => "CANVAS_NOT_FOUND",
            Self::StorageUnavailable => "CANVAS_STORAGE_UNAVAILABLE",
            Self::StaleProjection => "CANVAS_PROJECTION_STALE",
            Self::RecoveryRequired => "CANVAS_RECOVERY_REQUIRED",
        }
    }
}

fn map_port(error: CanvasViewportQueryError) -> GetCanvasViewportError {
    match error {
        CanvasViewportQueryError::InvalidInput => GetCanvasViewportError::InvalidInput,
        CanvasViewportQueryError::StorageUnavailable => GetCanvasViewportError::StorageUnavailable,
        CanvasViewportQueryError::StaleProjection => GetCanvasViewportError::StaleProjection,
        CanvasViewportQueryError::CorruptedProjection
        | CanvasViewportQueryError::UnsupportedSchema => GetCanvasViewportError::RecoveryRequired,
    }
}
