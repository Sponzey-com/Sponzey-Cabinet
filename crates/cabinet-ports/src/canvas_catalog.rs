use cabinet_domain::canvas::{CanvasId, CanvasLifecycleState, CanvasRevision, CanvasTitle};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasCatalogEntry {
    canvas_id: CanvasId,
    title: CanvasTitle,
    lifecycle: CanvasLifecycleState,
    revision: CanvasRevision,
}

impl CanvasCatalogEntry {
    pub fn new(
        canvas_id: CanvasId,
        title: CanvasTitle,
        lifecycle: CanvasLifecycleState,
        revision: CanvasRevision,
    ) -> Self {
        Self {
            canvas_id,
            title,
            lifecycle,
            revision,
        }
    }

    pub fn canvas_id(&self) -> &CanvasId {
        &self.canvas_id
    }

    pub fn title(&self) -> &CanvasTitle {
        &self.title
    }

    pub const fn lifecycle(&self) -> CanvasLifecycleState {
        self.lifecycle
    }

    pub const fn revision(&self) -> CanvasRevision {
        self.revision
    }
}

pub trait CanvasCatalogPort {
    fn list_canvas_entries(
        &self,
        workspace_id: &WorkspaceId,
        limit: usize,
        include_archived: bool,
    ) -> Result<Vec<CanvasCatalogEntry>, CanvasCatalogError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasCatalogError {
    InvalidLimit,
    LimitExceeded,
    StorageUnavailable,
    CorruptedCatalog,
}

pub trait LastCanvasSelectionPort {
    fn load_last_canvas_id(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Option<CanvasId>, LastCanvasSelectionError>;

    fn save_last_canvas_id(
        &mut self,
        workspace_id: &WorkspaceId,
        canvas_id: &CanvasId,
    ) -> Result<(), LastCanvasSelectionError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastCanvasSelectionError {
    StorageUnavailable,
    CorruptedSelection,
}
