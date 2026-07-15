use cabinet_domain::canvas::{
    CanvasEdge, CanvasId, CanvasLifecycleState, CanvasNode, CanvasRevision, CanvasTitle,
    CanvasViewport,
};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasViewportQuery {
    pub center_x: Option<i32>,
    pub center_y: Option<i32>,
    pub zoom_percent: Option<u16>,
    pub surface_width: u32,
    pub surface_height: u32,
    pub overscan: u32,
    pub node_limit: usize,
    pub edge_limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasViewportPage {
    pub canvas_id: CanvasId,
    pub title: CanvasTitle,
    pub revision: CanvasRevision,
    pub lifecycle: CanvasLifecycleState,
    pub viewport: CanvasViewport,
    pub nodes: Vec<CanvasNode>,
    pub edges: Vec<CanvasEdge>,
    pub total_node_count: usize,
    pub total_edge_count: usize,
    pub matching_node_count: usize,
    pub matching_edge_count: usize,
    pub truncated: bool,
}

pub trait CanvasViewportQueryPort {
    fn query_viewport(
        &self,
        workspace_id: &WorkspaceId,
        canvas_id: &CanvasId,
        query: CanvasViewportQuery,
    ) -> Result<Option<CanvasViewportPage>, CanvasViewportQueryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasViewportQueryError {
    InvalidInput,
    StorageUnavailable,
    StaleProjection,
    CorruptedProjection,
    UnsupportedSchema,
}
