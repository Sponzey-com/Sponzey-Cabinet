use std::cell::{Cell, RefCell};

use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_navigator::{
    DocumentNavigatorItem, DocumentNavigatorPage, DocumentNavigatorProjectionError,
    DocumentNavigatorProjectionPort, DocumentNavigatorProjectionQuery, NavigatorViewKind,
};
use cabinet_usecases::document_navigator::{
    GetDocumentNavigatorError, GetDocumentNavigatorInput, GetDocumentNavigatorUsecase,
    NavigatorLoadEvent, NavigatorLoadState, transition_navigator_load,
};

struct FakeNavigatorProjectionPort {
    result: Result<DocumentNavigatorPage, DocumentNavigatorProjectionError>,
    call_count: Cell<usize>,
    last_query: RefCell<Option<DocumentNavigatorProjectionQuery>>,
}

impl FakeNavigatorProjectionPort {
    fn returning(page: DocumentNavigatorPage) -> Self {
        Self {
            result: Ok(page),
            call_count: Cell::new(0),
            last_query: RefCell::new(None),
        }
    }

    fn failing(error: DocumentNavigatorProjectionError) -> Self {
        Self {
            result: Err(error),
            call_count: Cell::new(0),
            last_query: RefCell::new(None),
        }
    }
}

impl DocumentNavigatorProjectionPort for FakeNavigatorProjectionPort {
    fn load_navigator_page(
        &self,
        _workspace_id: &WorkspaceId,
        query: &DocumentNavigatorProjectionQuery,
    ) -> Result<DocumentNavigatorPage, DocumentNavigatorProjectionError> {
        self.call_count.set(self.call_count.get() + 1);
        self.last_query.replace(Some(query.clone()));
        self.result.clone()
    }
}

#[test]
fn navigator_returns_ready_bounded_page_and_normalized_filter() {
    let port = FakeNavigatorProjectionPort::returning(DocumentNavigatorPage::new(
        vec![item("doc-1", "Architecture", "notes/architecture.md")],
        Some(1),
        false,
    ));
    let usecase = GetDocumentNavigatorUsecase::new();

    let output = usecase
        .execute(
            GetDocumentNavigatorInput::new(
                "workspace-1",
                NavigatorViewKind::Collection,
                Some(" work "),
                Some("  ARCH  "),
                50,
                None,
            ),
            &port,
        )
        .expect("navigator page");

    assert_eq!(output.workspace_id(), "workspace-1");
    assert_eq!(output.view(), NavigatorViewKind::Collection);
    assert_eq!(output.state(), NavigatorLoadState::Ready);
    assert_eq!(output.items()[0].document_id(), "doc-1");
    assert_eq!(output.next_cursor(), Some("1"));
    assert_eq!(output.product_log_event_name(), None);
    let query = port.last_query.borrow();
    let query = query.as_ref().expect("query captured");
    assert_eq!(query.view_key(), Some("work"));
    assert_eq!(query.filter(), Some("arch"));
    assert_eq!(query.limit(), 50);
    assert_eq!(query.offset(), 0);
}

#[test]
fn navigator_classifies_empty_filtered_and_degraded_results() {
    let usecase = GetDocumentNavigatorUsecase::new();
    let empty = usecase
        .execute(
            GetDocumentNavigatorInput::new(
                "workspace-1",
                NavigatorViewKind::Tree,
                None,
                Some("missing"),
                20,
                None,
            ),
            &FakeNavigatorProjectionPort::returning(DocumentNavigatorPage::empty(false)),
        )
        .expect("filtered empty");
    let degraded = usecase
        .execute(
            GetDocumentNavigatorInput::new(
                "workspace-1",
                NavigatorViewKind::Recent,
                None,
                None,
                20,
                None,
            ),
            &FakeNavigatorProjectionPort::returning(DocumentNavigatorPage::empty(true)),
        )
        .expect("degraded navigator");

    assert_eq!(empty.state(), NavigatorLoadState::EmptyResult);
    assert_eq!(degraded.state(), NavigatorLoadState::Degraded);
    assert!(empty.items().is_empty());
}

#[test]
fn navigator_rejects_invalid_input_before_projection_read() {
    let port = FakeNavigatorProjectionPort::returning(DocumentNavigatorPage::empty(false));
    let usecase = GetDocumentNavigatorUsecase::new();
    let invalid = [
        GetDocumentNavigatorInput::new("", NavigatorViewKind::Tree, None, None, 20, None),
        GetDocumentNavigatorInput::new(
            "workspace-1",
            NavigatorViewKind::Tag,
            Some(" "),
            None,
            20,
            None,
        ),
        GetDocumentNavigatorInput::new(
            "workspace-1",
            NavigatorViewKind::Favorite,
            None,
            None,
            0,
            None,
        ),
        GetDocumentNavigatorInput::new(
            "workspace-1",
            NavigatorViewKind::Recent,
            None,
            None,
            101,
            None,
        ),
        GetDocumentNavigatorInput::new(
            "workspace-1",
            NavigatorViewKind::Recent,
            None,
            None,
            20,
            Some("not-a-cursor"),
        ),
    ];

    for input in invalid {
        assert_eq!(
            usecase.execute(input, &port),
            Err(GetDocumentNavigatorError::InvalidInput)
        );
    }
    assert_eq!(port.call_count.get(), 0);
}

#[test]
fn navigator_maps_projection_failure_to_stable_sanitized_error() {
    let port =
        FakeNavigatorProjectionPort::failing(DocumentNavigatorProjectionError::CorruptedProjection);
    let error = GetDocumentNavigatorUsecase::new()
        .execute(
            GetDocumentNavigatorInput::new(
                "workspace-1",
                NavigatorViewKind::Tree,
                None,
                None,
                20,
                None,
            ),
            &port,
        )
        .expect_err("corrupt projection must fail");

    assert_eq!(error, GetDocumentNavigatorError::ProjectionUnavailable);
    assert_eq!(error.code(), "document_navigator.projection_unavailable");
    assert!(!format!("{error:?}").contains("/Users/"));
}

#[test]
fn navigator_state_machine_handles_filter_ready_empty_degraded_failed_and_invalid() {
    let filtering = transition_navigator_load(
        NavigatorLoadState::Loading,
        NavigatorLoadEvent::FilterChanged,
    );
    let ready = transition_navigator_load(
        filtering.state,
        NavigatorLoadEvent::ProjectionLoaded {
            item_count: 1,
            degraded: false,
        },
    );
    let empty = transition_navigator_load(
        NavigatorLoadState::Filtering,
        NavigatorLoadEvent::ProjectionLoaded {
            item_count: 0,
            degraded: false,
        },
    );
    let degraded = transition_navigator_load(
        NavigatorLoadState::Loading,
        NavigatorLoadEvent::ProjectionLoaded {
            item_count: 0,
            degraded: true,
        },
    );
    let failed = transition_navigator_load(
        NavigatorLoadState::Loading,
        NavigatorLoadEvent::ProjectionFailed,
    );
    let invalid =
        transition_navigator_load(NavigatorLoadState::Ready, NavigatorLoadEvent::FilterChanged);

    assert_eq!(filtering.state, NavigatorLoadState::Filtering);
    assert_eq!(ready.state, NavigatorLoadState::Ready);
    assert_eq!(empty.state, NavigatorLoadState::EmptyResult);
    assert_eq!(degraded.state, NavigatorLoadState::Degraded);
    assert_eq!(failed.state, NavigatorLoadState::Failed);
    assert_eq!(
        failed.error_code,
        Some("document_navigator.projection_unavailable")
    );
    assert_eq!(invalid.state, NavigatorLoadState::Failed);
    assert_eq!(
        invalid.error_code,
        Some("document_navigator.invalid_transition")
    );
}

fn item(id: &str, title: &str, path: &str) -> DocumentNavigatorItem {
    DocumentNavigatorItem::new(
        DocumentId::new(id).expect("id"),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
        vec!["work".to_string()],
        vec!["rust".to_string()],
        true,
        1,
    )
    .expect("navigator item")
}
