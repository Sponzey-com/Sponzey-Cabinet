use cabinet_domain::canvas::CanvasLifecycleState;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_catalog::{
    CanvasCatalogEntry, CanvasCatalogError, CanvasCatalogPort, LastCanvasSelectionError,
    LastCanvasSelectionPort,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveInitialCanvasInput {
    workspace_id: String,
    limit: usize,
    include_archived: bool,
}

impl ResolveInitialCanvasInput {
    pub fn new(workspace_id: &str, limit: usize, include_archived: bool) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            limit,
            include_archived,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedCanvasSelectionSource {
    LastUsed,
    Fallback,
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveInitialCanvasOutput {
    entries: Vec<CanvasCatalogEntry>,
    selected_index: Option<usize>,
    selection_source: ResolvedCanvasSelectionSource,
}

impl ResolveInitialCanvasOutput {
    pub fn entries(&self) -> &[CanvasCatalogEntry] {
        &self.entries
    }

    pub fn selected_canvas_id(&self) -> Option<&str> {
        self.selected_index
            .map(|index| self.entries[index].canvas_id().as_str())
    }

    pub const fn selection_source(&self) -> ResolvedCanvasSelectionSource {
        self.selection_source
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveInitialCanvasError {
    InvalidInput,
    CatalogLimitExceeded,
    CatalogUnavailable,
    CorruptedCatalog,
    SelectionUnavailable,
    CorruptedSelection,
}

impl ResolveInitialCanvasError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "canvas_catalog.invalid_input",
            Self::CatalogLimitExceeded => "canvas_catalog.limit_exceeded",
            Self::CatalogUnavailable => "canvas_catalog.unavailable",
            Self::CorruptedCatalog => "canvas_catalog.corrupted",
            Self::SelectionUnavailable => "canvas_selection.unavailable",
            Self::CorruptedSelection => "canvas_selection.corrupted",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(self, Self::CatalogUnavailable | Self::SelectionUnavailable)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ResolveInitialCanvasUsecase;

impl ResolveInitialCanvasUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ResolveInitialCanvasInput,
        catalog: &impl CanvasCatalogPort,
        selection: &impl LastCanvasSelectionPort,
    ) -> Result<ResolveInitialCanvasOutput, ResolveInitialCanvasError> {
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ResolveInitialCanvasError::InvalidInput)?;
        if input.limit == 0 {
            return Err(ResolveInitialCanvasError::InvalidInput);
        }
        let entries = catalog
            .list_canvas_entries(&workspace, input.limit, input.include_archived)
            .map_err(map_catalog_error)?;
        if entries.len() > input.limit {
            return Err(ResolveInitialCanvasError::CatalogLimitExceeded);
        }
        let last = selection
            .load_last_canvas_id(&workspace)
            .map_err(map_selection_error)?;
        let last_index = last.as_ref().and_then(|last_id| {
            entries.iter().position(|entry| {
                entry.canvas_id() == last_id && entry.lifecycle() != CanvasLifecycleState::Archived
            })
        });
        let fallback_index = entries
            .iter()
            .position(|entry| entry.lifecycle() != CanvasLifecycleState::Archived);
        let selected_index = last_index.or(fallback_index);
        let selection_source = if last_index.is_some() {
            ResolvedCanvasSelectionSource::LastUsed
        } else if selected_index.is_some() {
            ResolvedCanvasSelectionSource::Fallback
        } else {
            ResolvedCanvasSelectionSource::Empty
        };
        Ok(ResolveInitialCanvasOutput {
            entries,
            selected_index,
            selection_source,
        })
    }
}

const fn map_catalog_error(error: CanvasCatalogError) -> ResolveInitialCanvasError {
    match error {
        CanvasCatalogError::InvalidLimit => ResolveInitialCanvasError::InvalidInput,
        CanvasCatalogError::LimitExceeded => ResolveInitialCanvasError::CatalogLimitExceeded,
        CanvasCatalogError::StorageUnavailable => ResolveInitialCanvasError::CatalogUnavailable,
        CanvasCatalogError::CorruptedCatalog => ResolveInitialCanvasError::CorruptedCatalog,
    }
}

const fn map_selection_error(error: LastCanvasSelectionError) -> ResolveInitialCanvasError {
    match error {
        LastCanvasSelectionError::StorageUnavailable => {
            ResolveInitialCanvasError::SelectionUnavailable
        }
        LastCanvasSelectionError::CorruptedSelection => {
            ResolveInitialCanvasError::CorruptedSelection
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectCanvasInput {
    workspace_id: String,
    canvas_id: String,
    catalog_limit: usize,
}

impl SelectCanvasInput {
    pub fn new(workspace_id: &str, canvas_id: &str, catalog_limit: usize) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            canvas_id: canvas_id.to_string(),
            catalog_limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectCanvasOutput {
    selected_canvas_id: String,
}

impl SelectCanvasOutput {
    pub fn selected_canvas_id(&self) -> &str {
        &self.selected_canvas_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectCanvasError {
    InvalidInput,
    CanvasNotFound,
    CanvasArchived,
    CatalogLimitExceeded,
    CatalogUnavailable,
    CorruptedCatalog,
    SelectionUnavailable,
    CorruptedSelection,
}

impl SelectCanvasError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "canvas_selection.invalid_input",
            Self::CanvasNotFound => "canvas_selection.not_found",
            Self::CanvasArchived => "canvas_selection.archived",
            Self::CatalogLimitExceeded => "canvas_selection.catalog_limit_exceeded",
            Self::CatalogUnavailable => "canvas_selection.catalog_unavailable",
            Self::CorruptedCatalog => "canvas_selection.catalog_corrupted",
            Self::SelectionUnavailable => "canvas_selection.unavailable",
            Self::CorruptedSelection => "canvas_selection.corrupted",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(self, Self::CatalogUnavailable | Self::SelectionUnavailable)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SelectCanvasUsecase;

impl SelectCanvasUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: SelectCanvasInput,
        catalog: &impl CanvasCatalogPort,
        selection: &mut impl LastCanvasSelectionPort,
    ) -> Result<SelectCanvasOutput, SelectCanvasError> {
        let workspace =
            WorkspaceId::new(&input.workspace_id).map_err(|_| SelectCanvasError::InvalidInput)?;
        let canvas_id = cabinet_domain::canvas::CanvasId::new(&input.canvas_id)
            .map_err(|_| SelectCanvasError::InvalidInput)?;
        if input.catalog_limit == 0 {
            return Err(SelectCanvasError::InvalidInput);
        }
        let entries = catalog
            .list_canvas_entries(&workspace, input.catalog_limit, true)
            .map_err(map_select_catalog_error)?;
        let entry = entries
            .iter()
            .find(|entry| entry.canvas_id() == &canvas_id)
            .ok_or(SelectCanvasError::CanvasNotFound)?;
        if entry.lifecycle() == CanvasLifecycleState::Archived {
            return Err(SelectCanvasError::CanvasArchived);
        }
        selection
            .save_last_canvas_id(&workspace, &canvas_id)
            .map_err(map_select_selection_error)?;
        Ok(SelectCanvasOutput {
            selected_canvas_id: canvas_id.as_str().to_string(),
        })
    }
}

const fn map_select_catalog_error(error: CanvasCatalogError) -> SelectCanvasError {
    match error {
        CanvasCatalogError::InvalidLimit => SelectCanvasError::InvalidInput,
        CanvasCatalogError::LimitExceeded => SelectCanvasError::CatalogLimitExceeded,
        CanvasCatalogError::StorageUnavailable => SelectCanvasError::CatalogUnavailable,
        CanvasCatalogError::CorruptedCatalog => SelectCanvasError::CorruptedCatalog,
    }
}

const fn map_select_selection_error(error: LastCanvasSelectionError) -> SelectCanvasError {
    match error {
        LastCanvasSelectionError::StorageUnavailable => SelectCanvasError::SelectionUnavailable,
        LastCanvasSelectionError::CorruptedSelection => SelectCanvasError::CorruptedSelection,
    }
}
