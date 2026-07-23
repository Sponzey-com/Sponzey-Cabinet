use std::cell::RefCell;

use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_platform::local_desktop_runtime::LocalDesktopUsecaseInput;
use cabinet_platform::workspace_home_command::{
    WorkspaceHomeCommandLoadState, execute_workspace_home_command,
};
use cabinet_ports::workspace_home::{
    WorkspaceHomeBackupStatus, WorkspaceHomeChangeProjection, WorkspaceHomeDocumentProjection,
    WorkspaceHomeHealthStatus, WorkspaceHomeProjection, WorkspaceHomeProjectionError,
    WorkspaceHomeProjectionLimits, WorkspaceHomeProjectionPort, WorkspaceHomeSummaryProjection,
    WorkspaceHomeTagProjection, WorkspaceHomeUnfinishedProjection,
};

struct FakeProjectionPort {
    result: Result<WorkspaceHomeProjection, WorkspaceHomeProjectionError>,
    call: RefCell<Option<(String, [u16; 5])>>,
}

impl FakeProjectionPort {
    fn returning(projection: WorkspaceHomeProjection) -> Self {
        Self {
            result: Ok(projection),
            call: RefCell::new(None),
        }
    }

    fn failing() -> Self {
        Self {
            result: Err(WorkspaceHomeProjectionError::StorageUnavailable),
            call: RefCell::new(None),
        }
    }
}

impl WorkspaceHomeProjectionPort for FakeProjectionPort {
    fn load_workspace_home(
        &self,
        workspace_id: &WorkspaceId,
        limits: WorkspaceHomeProjectionLimits,
    ) -> Result<WorkspaceHomeProjection, WorkspaceHomeProjectionError> {
        self.call.replace(Some((
            workspace_id.as_str().to_string(),
            [
                limits.recent_documents(),
                limits.favorites(),
                limits.tags(),
                limits.recent_changes(),
                limits.unfinished_items(),
            ],
        )));
        self.result.clone()
    }
}

#[test]
fn executor_maps_ready_projection_to_ui_safe_owned_dto() {
    let port = FakeProjectionPort::returning(populated_projection());

    let result = execute_workspace_home_command(home_input(), &port).expect("home executes");

    assert_eq!(result.workspace_id, "workspace-1");
    assert_eq!(result.state, WorkspaceHomeCommandLoadState::Ready);
    assert_eq!(result.recent_documents[0].document_id, "doc-recent");
    assert_eq!(result.recent_documents[0].title, "Recent");
    assert_eq!(result.favorites[0].path, "notes/favorite.md");
    assert_eq!(result.tags[0].document_count, 2);
    assert_eq!(result.recent_changes[0].summary, "Updated links");
    assert_eq!(result.unfinished_items[0].label, "Review draft");
    assert_eq!(result.backup_status, "Fresh");
    assert_eq!(result.health_status, "Healthy");
    assert_eq!(result.document_count, 10_000);
    assert_eq!(result.asset_count, 2_500);
    assert_eq!(result.canvas_count, 24);
    assert_eq!(result.product_log_event_name, None);
    assert_eq!(
        port.call.borrow().as_ref(),
        Some(&("workspace-1".to_string(), [12, 8, 10, 14, 6]))
    );
    assert!(!format!("{result:?}").contains("raw document body"));
    assert!(!format!("{result:?}").contains("/Users/private/app-data"));
}

#[test]
fn executor_preserves_empty_and_degraded_usecase_states() {
    let empty = execute_workspace_home_command(
        home_input(),
        &FakeProjectionPort::returning(WorkspaceHomeProjection::empty(
            WorkspaceHomeBackupStatus::NeverCreated,
            WorkspaceHomeHealthStatus::Healthy,
        )),
    )
    .expect("empty executes");
    let degraded = execute_workspace_home_command(
        home_input(),
        &FakeProjectionPort::returning(WorkspaceHomeProjection::empty(
            WorkspaceHomeBackupStatus::Failed,
            WorkspaceHomeHealthStatus::ReadOnlyRecovery,
        )),
    )
    .expect("degraded executes");

    assert_eq!(empty.state, WorkspaceHomeCommandLoadState::Empty);
    assert_eq!(empty.backup_status, "NeverCreated");
    assert_eq!(degraded.state, WorkspaceHomeCommandLoadState::Degraded);
    assert_eq!(degraded.health_status, "ReadOnlyRecovery");
}

#[test]
fn executor_maps_invalid_input_and_projection_failure_to_stable_safe_errors() {
    let invalid = execute_workspace_home_command(
        LocalDesktopUsecaseInput::WorkspaceHome {
            workspace_id: "".to_string(),
            recent_documents: 12,
            favorites: 8,
            tags: 10,
            recent_changes: 14,
            unfinished_items: 6,
        },
        &FakeProjectionPort::returning(populated_projection()),
    )
    .expect_err("invalid input fails");
    let unavailable = execute_workspace_home_command(home_input(), &FakeProjectionPort::failing())
        .expect_err("projection failure surfaces");

    assert_eq!(invalid.error_code, "WORKSPACE_HOME_INVALID_INPUT");
    assert!(!invalid.retryable);
    assert_eq!(invalid.product_log_event_name, None);
    assert_eq!(
        unavailable.error_code,
        "WORKSPACE_HOME_PROJECTION_UNAVAILABLE"
    );
    assert!(unavailable.retryable);
    assert_eq!(
        unavailable.product_log_event_name,
        Some("workspace.home.failed")
    );
    assert!(!format!("{unavailable:?}").contains("StorageUnavailable"));
}

#[test]
fn executor_rejects_non_home_command_without_calling_projection_port() {
    let port = FakeProjectionPort::returning(populated_projection());

    let error = execute_workspace_home_command(LocalDesktopUsecaseInput::BootstrapWorkspace, &port)
        .expect_err("wrong executor input fails");

    assert_eq!(error.error_code, "WORKSPACE_HOME_COMMAND_UNSUPPORTED");
    assert!(!error.retryable);
    assert_eq!(*port.call.borrow(), None);
}

fn home_input() -> LocalDesktopUsecaseInput {
    LocalDesktopUsecaseInput::WorkspaceHome {
        workspace_id: "workspace-1".to_string(),
        recent_documents: 12,
        favorites: 8,
        tags: 10,
        recent_changes: 14,
        unfinished_items: 6,
    }
}

fn populated_projection() -> WorkspaceHomeProjection {
    WorkspaceHomeProjection::new(
        vec![document("doc-recent", "Recent", "notes/recent.md")],
        vec![document("doc-favorite", "Favorite", "notes/favorite.md")],
        vec![WorkspaceHomeTagProjection::new("rust", 2).expect("tag")],
        vec![
            WorkspaceHomeChangeProjection::new(
                DocumentId::new("doc-recent").expect("id"),
                "Updated links",
            )
            .expect("change"),
        ],
        vec![
            WorkspaceHomeUnfinishedProjection::new(
                DocumentId::new("doc-recent").expect("id"),
                "Review draft",
            )
            .expect("unfinished"),
        ],
        WorkspaceHomeBackupStatus::Fresh,
        WorkspaceHomeHealthStatus::Healthy,
    )
    .with_summary(WorkspaceHomeSummaryProjection::new(10_000, 2_500, 24))
}

fn document(id: &str, title: &str, path: &str) -> WorkspaceHomeDocumentProjection {
    WorkspaceHomeDocumentProjection::new(
        DocumentId::new(id).expect("id"),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
    )
}
