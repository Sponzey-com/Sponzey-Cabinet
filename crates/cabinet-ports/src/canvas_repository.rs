use cabinet_domain::canvas::{Canvas, CanvasId, CanvasRevision, CanvasTitle, CanvasViewport};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasRecord {
    canvas: Canvas,
    title: CanvasTitle,
    revision: CanvasRevision,
    viewport: CanvasViewport,
}

impl CanvasRecord {
    pub fn new(canvas: Canvas) -> Result<Self, CanvasRepositoryError> {
        let title = CanvasTitle::new(canvas.id().as_str())
            .map_err(|_| CanvasRepositoryError::InvalidInput)?;
        Ok(Self {
            canvas,
            title,
            revision: CanvasRevision::new(1).map_err(|_| CanvasRepositoryError::InvalidInput)?,
            viewport: CanvasViewport::default(),
        })
    }

    pub fn with_metadata(
        canvas: Canvas,
        title: CanvasTitle,
        revision: CanvasRevision,
        viewport: CanvasViewport,
    ) -> Self {
        Self {
            canvas,
            title,
            revision,
            viewport,
        }
    }

    pub fn canvas(&self) -> &Canvas {
        &self.canvas
    }
    pub fn title(&self) -> &CanvasTitle {
        &self.title
    }
    pub const fn revision(&self) -> CanvasRevision {
        self.revision
    }
    pub const fn viewport(&self) -> CanvasViewport {
        self.viewport
    }

    pub fn next(&self, canvas: Canvas) -> Result<Self, CanvasRepositoryError> {
        self.revised(canvas, self.title.clone(), self.viewport)
    }

    pub fn revised(
        &self,
        canvas: Canvas,
        title: CanvasTitle,
        viewport: CanvasViewport,
    ) -> Result<Self, CanvasRepositoryError> {
        Ok(Self {
            canvas,
            title,
            revision: self
                .revision
                .next()
                .map_err(|_| CanvasRepositoryError::InvalidInput)?,
            viewport,
        })
    }
}

pub trait CanvasRepository {
    fn create_canvas(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError>;

    fn replace_canvas(
        &mut self,
        workspace_id: &WorkspaceId,
        expected_revision: CanvasRevision,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError>;

    fn get_canvas(
        &self,
        workspace_id: &WorkspaceId,
        canvas_id: &CanvasId,
    ) -> Result<Option<CanvasRecord>, CanvasRepositoryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasRepositoryError {
    InvalidInput,
    StorageUnavailable,
    CorruptedCanvas,
    AlreadyExists,
    VersionConflict,
    UnsupportedSchema,
}

impl CanvasRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "canvas_repository.invalid_input",
            Self::StorageUnavailable => "canvas_repository.storage_unavailable",
            Self::CorruptedCanvas => "canvas_repository.corrupted_canvas",
            Self::AlreadyExists => "canvas_repository.already_exists",
            Self::VersionConflict => "canvas_repository.version_conflict",
            Self::UnsupportedSchema => "canvas_repository.unsupported_schema",
        }
    }
}
