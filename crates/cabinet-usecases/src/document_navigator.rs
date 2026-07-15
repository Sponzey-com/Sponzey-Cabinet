use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_navigator::{
    DocumentNavigatorItem, DocumentNavigatorProjectionPort, DocumentNavigatorProjectionQuery,
    NavigatorViewKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigatorLoadState {
    Loading,
    Ready,
    Filtering,
    EmptyResult,
    Degraded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigatorLoadEvent {
    FilterChanged,
    ProjectionLoaded { item_count: usize, degraded: bool },
    ProjectionFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigatorLoadTransition {
    pub state: NavigatorLoadState,
    pub error_code: Option<&'static str>,
}

pub fn transition_navigator_load(
    state: NavigatorLoadState,
    event: NavigatorLoadEvent,
) -> NavigatorLoadTransition {
    match (state, event) {
        (NavigatorLoadState::Loading, NavigatorLoadEvent::FilterChanged) => {
            transition(NavigatorLoadState::Filtering, None)
        }
        (
            NavigatorLoadState::Loading | NavigatorLoadState::Filtering,
            NavigatorLoadEvent::ProjectionLoaded { degraded: true, .. },
        ) => transition(NavigatorLoadState::Degraded, None),
        (
            NavigatorLoadState::Loading | NavigatorLoadState::Filtering,
            NavigatorLoadEvent::ProjectionLoaded {
                item_count: 0,
                degraded: false,
            },
        ) => transition(NavigatorLoadState::EmptyResult, None),
        (
            NavigatorLoadState::Loading | NavigatorLoadState::Filtering,
            NavigatorLoadEvent::ProjectionLoaded {
                degraded: false, ..
            },
        ) => transition(NavigatorLoadState::Ready, None),
        (
            NavigatorLoadState::Loading | NavigatorLoadState::Filtering,
            NavigatorLoadEvent::ProjectionFailed,
        ) => transition(
            NavigatorLoadState::Failed,
            Some("document_navigator.projection_unavailable"),
        ),
        _ => transition(
            NavigatorLoadState::Failed,
            Some("document_navigator.invalid_transition"),
        ),
    }
}

const fn transition(
    state: NavigatorLoadState,
    error_code: Option<&'static str>,
) -> NavigatorLoadTransition {
    NavigatorLoadTransition { state, error_code }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetDocumentNavigatorInput {
    workspace_id: String,
    view: NavigatorViewKind,
    view_key: Option<String>,
    filter: Option<String>,
    limit: u16,
    cursor: Option<String>,
}

impl GetDocumentNavigatorInput {
    pub fn new(
        workspace_id: &str,
        view: NavigatorViewKind,
        view_key: Option<&str>,
        filter: Option<&str>,
        limit: u16,
        cursor: Option<&str>,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            view,
            view_key: view_key.map(str::to_string),
            filter: filter.map(str::to_string),
            limit,
            cursor: cursor.map(str::to_string),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetDocumentNavigatorOutput {
    workspace_id: WorkspaceId,
    view: NavigatorViewKind,
    state: NavigatorLoadState,
    items: Vec<DocumentNavigatorItem>,
    next_cursor: Option<String>,
}

impl GetDocumentNavigatorOutput {
    pub fn workspace_id(&self) -> &str {
        self.workspace_id.as_str()
    }

    pub const fn view(&self) -> NavigatorViewKind {
        self.view
    }

    pub const fn state(&self) -> NavigatorLoadState {
        self.state
    }

    pub fn items(&self) -> &[DocumentNavigatorItem] {
        &self.items
    }

    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }

    pub const fn product_log_event_name(&self) -> Option<&'static str> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetDocumentNavigatorError {
    InvalidInput,
    ProjectionUnavailable,
}

impl GetDocumentNavigatorError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document_navigator.invalid_input",
            Self::ProjectionUnavailable => "document_navigator.projection_unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GetDocumentNavigatorUsecase;

impl GetDocumentNavigatorUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetDocumentNavigatorInput,
        projection_port: &impl DocumentNavigatorProjectionPort,
    ) -> Result<GetDocumentNavigatorOutput, GetDocumentNavigatorError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| GetDocumentNavigatorError::InvalidInput)?;
        let offset = input
            .cursor
            .as_deref()
            .map(str::parse::<u32>)
            .transpose()
            .map_err(|_| GetDocumentNavigatorError::InvalidInput)?
            .unwrap_or(0);
        let query = DocumentNavigatorProjectionQuery::new(
            input.view,
            input.view_key.as_deref(),
            input.filter.as_deref(),
            offset,
            input.limit,
        )
        .map_err(|_| GetDocumentNavigatorError::InvalidInput)?;
        let initial_state = if query.filter().is_some() {
            transition_navigator_load(
                NavigatorLoadState::Loading,
                NavigatorLoadEvent::FilterChanged,
            )
            .state
        } else {
            NavigatorLoadState::Loading
        };
        let page = projection_port
            .load_navigator_page(&workspace_id, &query)
            .map_err(|_| GetDocumentNavigatorError::ProjectionUnavailable)?;
        let loaded = transition_navigator_load(
            initial_state,
            NavigatorLoadEvent::ProjectionLoaded {
                item_count: page.items().len(),
                degraded: page.degraded(),
            },
        );

        Ok(GetDocumentNavigatorOutput {
            workspace_id,
            view: input.view,
            state: loaded.state,
            items: page.items().to_vec(),
            next_cursor: page.next_offset().map(|value| value.to_string()),
        })
    }
}
