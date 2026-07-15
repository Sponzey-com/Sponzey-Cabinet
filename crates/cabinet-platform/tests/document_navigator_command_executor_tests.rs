use std::cell::RefCell;

use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_platform::document_navigator_command::{
    DocumentNavigatorCommandLoadState, DocumentNavigatorCommandRequest,
    DocumentNavigatorCommandView, execute_document_navigator_command,
};
use cabinet_ports::document_navigator::{
    DocumentNavigatorItem, DocumentNavigatorPage, DocumentNavigatorProjectionError,
    DocumentNavigatorProjectionPort, DocumentNavigatorProjectionQuery,
};

struct FakeProjectionPort {
    result: Result<DocumentNavigatorPage, DocumentNavigatorProjectionError>,
    call: RefCell<Option<(String, DocumentNavigatorProjectionQuery)>>,
}

impl FakeProjectionPort {
    fn returning(page: DocumentNavigatorPage) -> Self {
        Self {
            result: Ok(page),
            call: RefCell::new(None),
        }
    }

    fn failing() -> Self {
        Self {
            result: Err(DocumentNavigatorProjectionError::StorageUnavailable),
            call: RefCell::new(None),
        }
    }
}

impl DocumentNavigatorProjectionPort for FakeProjectionPort {
    fn load_navigator_page(
        &self,
        workspace_id: &WorkspaceId,
        query: &DocumentNavigatorProjectionQuery,
    ) -> Result<DocumentNavigatorPage, DocumentNavigatorProjectionError> {
        self.call
            .replace(Some((workspace_id.as_str().to_string(), query.clone())));
        self.result.clone()
    }
}

#[test]
fn navigator_executor_maps_ready_page_to_owned_safe_dto() {
    let port =
        FakeProjectionPort::returning(DocumentNavigatorPage::new(vec![item()], Some(20), false));

    let result = execute_document_navigator_command(
        DocumentNavigatorCommandRequest {
            workspace_id: "workspace-1".to_string(),
            view: DocumentNavigatorCommandView::Collection,
            view_key: Some("work".to_string()),
            filter: Some("arch".to_string()),
            limit: 20,
            cursor: None,
        },
        &port,
    )
    .expect("navigator command");

    assert_eq!(result.workspace_id, "workspace-1");
    assert_eq!(result.view, DocumentNavigatorCommandView::Collection);
    assert_eq!(result.state, DocumentNavigatorCommandLoadState::Ready);
    assert_eq!(result.items[0].document_id, "doc-1");
    assert_eq!(result.items[0].collections, vec!["work"]);
    assert_eq!(result.items[0].tags, vec!["rust"]);
    assert_eq!(result.next_cursor.as_deref(), Some("20"));
    assert_eq!(result.product_log_event_name, None);
    let call = port.call.borrow();
    let (workspace, query) = call.as_ref().expect("projection called");
    assert_eq!(workspace, "workspace-1");
    assert_eq!(query.view_key(), Some("work"));
    assert_eq!(query.filter(), Some("arch"));
    assert!(!format!("{result:?}").contains("raw document body"));
    assert!(!format!("{result:?}").contains("/Users/"));
}

#[test]
fn navigator_executor_preserves_empty_and_degraded_states() {
    let empty = execute_document_navigator_command(
        request(DocumentNavigatorCommandView::Tree),
        &FakeProjectionPort::returning(DocumentNavigatorPage::empty(false)),
    )
    .expect("empty");
    let degraded = execute_document_navigator_command(
        request(DocumentNavigatorCommandView::Recent),
        &FakeProjectionPort::returning(DocumentNavigatorPage::empty(true)),
    )
    .expect("degraded");

    assert_eq!(empty.state, DocumentNavigatorCommandLoadState::EmptyResult);
    assert_eq!(degraded.state, DocumentNavigatorCommandLoadState::Degraded);
}

#[test]
fn navigator_executor_maps_invalid_and_projection_failure_to_stable_errors() {
    let invalid = execute_document_navigator_command(
        DocumentNavigatorCommandRequest {
            workspace_id: "workspace-1".to_string(),
            view: DocumentNavigatorCommandView::Tag,
            view_key: None,
            filter: None,
            limit: 20,
            cursor: None,
        },
        &FakeProjectionPort::returning(DocumentNavigatorPage::empty(false)),
    )
    .expect_err("invalid request");
    let unavailable = execute_document_navigator_command(
        request(DocumentNavigatorCommandView::Favorite),
        &FakeProjectionPort::failing(),
    )
    .expect_err("unavailable projection");

    assert_eq!(invalid.error_code, "DOCUMENT_NAVIGATOR_INVALID_INPUT");
    assert!(!invalid.retryable);
    assert_eq!(
        unavailable.error_code,
        "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE"
    );
    assert!(unavailable.retryable);
    assert_eq!(unavailable.product_log_event_name, None);
}

#[test]
fn navigator_executor_maps_all_view_kinds_without_adapter_rules() {
    for view in [
        DocumentNavigatorCommandView::Tree,
        DocumentNavigatorCommandView::Collection,
        DocumentNavigatorCommandView::Tag,
        DocumentNavigatorCommandView::Recent,
        DocumentNavigatorCommandView::Favorite,
    ] {
        let mut request = request(view);
        if matches!(
            view,
            DocumentNavigatorCommandView::Collection | DocumentNavigatorCommandView::Tag
        ) {
            request.view_key = Some("key".to_string());
        }
        let result = execute_document_navigator_command(
            request,
            &FakeProjectionPort::returning(DocumentNavigatorPage::empty(false)),
        )
        .expect("view maps");
        assert_eq!(result.view, view);
    }
}

fn request(view: DocumentNavigatorCommandView) -> DocumentNavigatorCommandRequest {
    DocumentNavigatorCommandRequest {
        workspace_id: "workspace-1".to_string(),
        view,
        view_key: None,
        filter: None,
        limit: 20,
        cursor: None,
    }
}

fn item() -> DocumentNavigatorItem {
    DocumentNavigatorItem::new(
        DocumentId::new("doc-1").expect("id"),
        DocumentTitle::new("Architecture").expect("title"),
        DocumentPath::new("notes/architecture.md").expect("path"),
        vec!["work".to_string()],
        vec!["rust".to_string()],
        true,
        1,
    )
    .expect("item")
}
