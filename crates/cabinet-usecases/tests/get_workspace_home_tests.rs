use std::cell::Cell;

use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::workspace_home::{
    WorkspaceHomeBackupStatus, WorkspaceHomeChangeProjection, WorkspaceHomeDocumentProjection,
    WorkspaceHomeHealthStatus, WorkspaceHomeProjection, WorkspaceHomeProjectionError,
    WorkspaceHomeProjectionLimits, WorkspaceHomeProjectionPort, WorkspaceHomeTagProjection,
    WorkspaceHomeUnfinishedProjection,
};
use cabinet_usecases::workspace_home::{
    GetWorkspaceHomeError, GetWorkspaceHomeInput, GetWorkspaceHomeUsecase, WorkspaceHomeLoadEvent,
    WorkspaceHomeLoadState, transition_workspace_home_load,
};

struct FakeWorkspaceHomeProjectionPort {
    projection: Result<WorkspaceHomeProjection, WorkspaceHomeProjectionError>,
    call_count: Cell<usize>,
    last_limit: Cell<Option<u16>>,
}

impl FakeWorkspaceHomeProjectionPort {
    fn returning(projection: WorkspaceHomeProjection) -> Self {
        Self {
            projection: Ok(projection),
            call_count: Cell::new(0),
            last_limit: Cell::new(None),
        }
    }

    fn failing(error: WorkspaceHomeProjectionError) -> Self {
        Self {
            projection: Err(error),
            call_count: Cell::new(0),
            last_limit: Cell::new(None),
        }
    }
}

impl WorkspaceHomeProjectionPort for FakeWorkspaceHomeProjectionPort {
    fn load_workspace_home(
        &self,
        _workspace_id: &WorkspaceId,
        limits: WorkspaceHomeProjectionLimits,
    ) -> Result<WorkspaceHomeProjection, WorkspaceHomeProjectionError> {
        self.call_count.set(self.call_count.get() + 1);
        self.last_limit.set(Some(limits.recent_documents()));
        self.projection.clone()
    }
}

#[test]
fn get_workspace_home_returns_bounded_projection_without_success_product_log() {
    let port = FakeWorkspaceHomeProjectionPort::returning(populated_projection());
    let usecase = GetWorkspaceHomeUsecase::new();

    let output = usecase
        .execute(valid_input(12), &port)
        .expect("workspace home should load");

    assert_eq!(output.workspace_id(), "workspace-1");
    assert_eq!(output.state(), WorkspaceHomeLoadState::Ready);
    assert_eq!(output.recent_documents().len(), 1);
    assert_eq!(output.recent_documents()[0].document_id(), "doc-recent");
    assert_eq!(output.favorites()[0].document_id(), "doc-favorite");
    assert_eq!(output.tags()[0].label(), "rust");
    assert_eq!(output.recent_changes()[0].summary(), "Updated links");
    assert_eq!(output.unfinished_items()[0].label(), "Review draft");
    assert_eq!(output.backup_status(), WorkspaceHomeBackupStatus::Fresh);
    assert_eq!(output.health_status(), WorkspaceHomeHealthStatus::Healthy);
    assert_eq!(output.product_log_event_name(), None);
    assert_eq!(port.call_count.get(), 1);
    assert_eq!(port.last_limit.get(), Some(12));
    assert!(!format!("{output:?}").contains("raw document body"));
}

#[test]
fn get_workspace_home_classifies_empty_and_degraded_projection_states() {
    let usecase = GetWorkspaceHomeUsecase::new();
    let empty_port = FakeWorkspaceHomeProjectionPort::returning(WorkspaceHomeProjection::empty(
        WorkspaceHomeBackupStatus::NeverCreated,
        WorkspaceHomeHealthStatus::Healthy,
    ));
    let degraded_port = FakeWorkspaceHomeProjectionPort::returning(WorkspaceHomeProjection::empty(
        WorkspaceHomeBackupStatus::Failed,
        WorkspaceHomeHealthStatus::Degraded,
    ));

    let empty = usecase
        .execute(valid_input(10), &empty_port)
        .expect("empty home");
    let degraded = usecase
        .execute(valid_input(10), &degraded_port)
        .expect("degraded home");

    assert_eq!(empty.state(), WorkspaceHomeLoadState::Empty);
    assert_eq!(degraded.state(), WorkspaceHomeLoadState::Degraded);
    assert_eq!(empty.total_item_count(), 0);
    assert_eq!(degraded.product_log_event_name(), None);
}

#[test]
fn get_workspace_home_rejects_invalid_input_before_projection_read() {
    let port = FakeWorkspaceHomeProjectionPort::returning(populated_projection());
    let usecase = GetWorkspaceHomeUsecase::new();

    let invalid_workspace = usecase
        .execute(GetWorkspaceHomeInput::new("", 10, 10, 10, 10, 10), &port)
        .expect_err("empty workspace must fail");
    let zero_limit = usecase
        .execute(
            GetWorkspaceHomeInput::new("workspace-1", 0, 10, 10, 10, 10),
            &port,
        )
        .expect_err("zero limit must fail");
    let excessive_limit = usecase
        .execute(
            GetWorkspaceHomeInput::new("workspace-1", 101, 10, 10, 10, 10),
            &port,
        )
        .expect_err("excessive limit must fail");

    assert_eq!(invalid_workspace, GetWorkspaceHomeError::InvalidInput);
    assert_eq!(zero_limit, GetWorkspaceHomeError::InvalidInput);
    assert_eq!(excessive_limit, GetWorkspaceHomeError::InvalidInput);
    assert_eq!(port.call_count.get(), 0);
}

#[test]
fn get_workspace_home_maps_projection_failure_to_stable_error_and_log_candidate() {
    let port =
        FakeWorkspaceHomeProjectionPort::failing(WorkspaceHomeProjectionError::StorageUnavailable);
    let usecase = GetWorkspaceHomeUsecase::new();

    let error = usecase
        .execute(valid_input(10), &port)
        .expect_err("projection failure must surface");

    assert_eq!(error, GetWorkspaceHomeError::ProjectionUnavailable);
    assert_eq!(error.code(), "workspace_home.projection_unavailable");
    assert_eq!(
        error.product_log_event_name(),
        Some("workspace.home.failed")
    );
    assert_eq!(port.call_count.get(), 1);
}

#[test]
fn workspace_home_load_state_machine_handles_ready_empty_degraded_failure_and_invalid_events() {
    let loading = transition_workspace_home_load(
        WorkspaceHomeLoadState::Pending,
        WorkspaceHomeLoadEvent::LoadRequested,
    );
    let ready = transition_workspace_home_load(
        loading.state,
        WorkspaceHomeLoadEvent::ProjectionLoaded {
            total_item_count: 3,
            health: WorkspaceHomeHealthStatus::Healthy,
        },
    );
    let empty = transition_workspace_home_load(
        WorkspaceHomeLoadState::Loading,
        WorkspaceHomeLoadEvent::ProjectionLoaded {
            total_item_count: 0,
            health: WorkspaceHomeHealthStatus::Healthy,
        },
    );
    let degraded = transition_workspace_home_load(
        WorkspaceHomeLoadState::Loading,
        WorkspaceHomeLoadEvent::ProjectionLoaded {
            total_item_count: 0,
            health: WorkspaceHomeHealthStatus::Degraded,
        },
    );
    let failed = transition_workspace_home_load(
        WorkspaceHomeLoadState::Loading,
        WorkspaceHomeLoadEvent::ProjectionFailed,
    );
    let invalid = transition_workspace_home_load(
        WorkspaceHomeLoadState::Pending,
        WorkspaceHomeLoadEvent::ProjectionFailed,
    );

    assert_eq!(loading.state, WorkspaceHomeLoadState::Loading);
    assert_eq!(ready.state, WorkspaceHomeLoadState::Ready);
    assert_eq!(empty.state, WorkspaceHomeLoadState::Empty);
    assert_eq!(degraded.state, WorkspaceHomeLoadState::Degraded);
    assert_eq!(failed.state, WorkspaceHomeLoadState::Failed);
    assert_eq!(
        failed.error_code,
        Some("workspace_home.projection_unavailable")
    );
    assert_eq!(invalid.state, WorkspaceHomeLoadState::Failed);
    assert_eq!(
        invalid.error_code,
        Some("workspace_home.invalid_transition")
    );
}

fn valid_input(limit: u16) -> GetWorkspaceHomeInput {
    GetWorkspaceHomeInput::new("workspace-1", limit, limit, limit, limit, limit)
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
}

fn document(id: &str, title: &str, path: &str) -> WorkspaceHomeDocumentProjection {
    WorkspaceHomeDocumentProjection::new(
        DocumentId::new(id).expect("id"),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
    )
}
