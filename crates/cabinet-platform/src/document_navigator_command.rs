use cabinet_ports::document_navigator::{DocumentNavigatorProjectionPort, NavigatorViewKind};
use cabinet_usecases::document_navigator::{
    GetDocumentNavigatorError, GetDocumentNavigatorInput, GetDocumentNavigatorUsecase,
    NavigatorLoadState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentNavigatorCommandView {
    Tree,
    Collection,
    Tag,
    Recent,
    Favorite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentNavigatorCommandRequest {
    pub workspace_id: String,
    pub view: DocumentNavigatorCommandView,
    pub view_key: Option<String>,
    pub filter: Option<String>,
    pub limit: u16,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentNavigatorCommandLoadState {
    Ready,
    EmptyResult,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentNavigatorCommandItem {
    pub document_id: String,
    pub title: String,
    pub path: String,
    pub collections: Vec<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentNavigatorCommandResult {
    pub workspace_id: String,
    pub view: DocumentNavigatorCommandView,
    pub state: DocumentNavigatorCommandLoadState,
    pub items: Vec<DocumentNavigatorCommandItem>,
    pub next_cursor: Option<String>,
    pub product_log_event_name: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentNavigatorCommandFailure {
    pub error_code: &'static str,
    pub retryable: bool,
    pub product_log_event_name: Option<&'static str>,
}

pub fn execute_document_navigator_command(
    request: DocumentNavigatorCommandRequest,
    projection_port: &impl DocumentNavigatorProjectionPort,
) -> Result<DocumentNavigatorCommandResult, DocumentNavigatorCommandFailure> {
    let view = map_view(request.view);
    let output = GetDocumentNavigatorUsecase::new()
        .execute(
            GetDocumentNavigatorInput::new(
                &request.workspace_id,
                view,
                request.view_key.as_deref(),
                request.filter.as_deref(),
                request.limit,
                request.cursor.as_deref(),
            ),
            projection_port,
        )
        .map_err(map_error)?;
    let state = match output.state() {
        NavigatorLoadState::Ready => DocumentNavigatorCommandLoadState::Ready,
        NavigatorLoadState::EmptyResult => DocumentNavigatorCommandLoadState::EmptyResult,
        NavigatorLoadState::Degraded => DocumentNavigatorCommandLoadState::Degraded,
        NavigatorLoadState::Loading
        | NavigatorLoadState::Filtering
        | NavigatorLoadState::Failed => {
            return Err(DocumentNavigatorCommandFailure {
                error_code: "DOCUMENT_NAVIGATOR_RESULT_MAPPING_FAILED",
                retryable: false,
                product_log_event_name: None,
            });
        }
    };

    Ok(DocumentNavigatorCommandResult {
        workspace_id: output.workspace_id().to_string(),
        view: request.view,
        state,
        items: output
            .items()
            .iter()
            .map(|item| DocumentNavigatorCommandItem {
                document_id: item.document_id().to_string(),
                title: item.title().to_string(),
                path: item.path().to_string(),
                collections: item.collections().to_vec(),
                tags: item.tags().to_vec(),
                favorite: item.favorite(),
            })
            .collect(),
        next_cursor: output.next_cursor().map(str::to_string),
        product_log_event_name: output.product_log_event_name(),
    })
}

const fn map_view(view: DocumentNavigatorCommandView) -> NavigatorViewKind {
    match view {
        DocumentNavigatorCommandView::Tree => NavigatorViewKind::Tree,
        DocumentNavigatorCommandView::Collection => NavigatorViewKind::Collection,
        DocumentNavigatorCommandView::Tag => NavigatorViewKind::Tag,
        DocumentNavigatorCommandView::Recent => NavigatorViewKind::Recent,
        DocumentNavigatorCommandView::Favorite => NavigatorViewKind::Favorite,
    }
}

const fn map_error(error: GetDocumentNavigatorError) -> DocumentNavigatorCommandFailure {
    match error {
        GetDocumentNavigatorError::InvalidInput => DocumentNavigatorCommandFailure {
            error_code: "DOCUMENT_NAVIGATOR_INVALID_INPUT",
            retryable: false,
            product_log_event_name: None,
        },
        GetDocumentNavigatorError::ProjectionUnavailable => DocumentNavigatorCommandFailure {
            error_code: "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE",
            retryable: true,
            product_log_event_name: None,
        },
    }
}
