use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_work::{
    ProjectionChangeKind, ProjectionKind, ProjectionWork, ProjectionWorkEvent,
    ProjectionWorkIdentity, ProjectionWorkState, ProjectionWorkTransitionError,
};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;

#[test]
fn projection_identity_key_is_stable_and_kind_specific() {
    let graph = identity(ProjectionKind::Graph);
    let same = identity(ProjectionKind::Graph);
    let links = identity(ProjectionKind::Links);

    assert_eq!(graph.idempotency_key(), same.idempotency_key());
    assert_ne!(graph.idempotency_key(), links.idempotency_key());
    assert_eq!(graph.workspace_id().as_str(), "workspace-1");
    assert_eq!(graph.document_id().as_str(), "doc-1");
    assert_eq!(graph.version_id().as_str(), "version-1");
}

#[test]
fn projection_identity_distinguishes_change_kind_for_the_same_version_and_projection() {
    let updated = ProjectionWorkIdentity::for_change(
        WorkspaceId::new("workspace-1").expect("workspace"),
        DocumentId::new("doc-1").expect("document"),
        VersionId::new("version-1").expect("version"),
        ProjectionKind::Graph,
        ProjectionChangeKind::Updated,
    );
    let renamed = ProjectionWorkIdentity::for_change(
        WorkspaceId::new("workspace-1").expect("workspace"),
        DocumentId::new("doc-1").expect("document"),
        VersionId::new("version-1").expect("version"),
        ProjectionKind::Graph,
        ProjectionChangeKind::Renamed,
    );

    assert_ne!(updated.idempotency_key(), renamed.idempotency_key());
    assert_eq!(renamed.change_kind(), ProjectionChangeKind::Renamed);
    assert_eq!(
        ProjectionChangeKind::AssetDetached.as_str(),
        "asset_detached"
    );
}

#[test]
fn projection_work_follows_pending_indexing_retry_and_ready_transitions() {
    let pending = ProjectionWork::pending(identity(ProjectionKind::Search));
    let indexing = pending
        .transition(ProjectionWorkEvent::Start)
        .expect("pending starts");
    let retry = indexing
        .transition(ProjectionWorkEvent::RetryScheduled)
        .expect("indexing schedules retry");
    let indexing_again = retry
        .transition(ProjectionWorkEvent::Start)
        .expect("retry starts");
    let ready = indexing_again
        .transition(ProjectionWorkEvent::Succeeded)
        .expect("indexing succeeds");

    assert_eq!(indexing.state(), ProjectionWorkState::Indexing);
    assert_eq!(indexing.attempt(), 1);
    assert_eq!(retry.state(), ProjectionWorkState::RetryScheduled);
    assert_eq!(indexing_again.attempt(), 2);
    assert_eq!(ready.state(), ProjectionWorkState::Ready);
}

#[test]
fn projection_work_rejects_invalid_and_terminal_transitions() {
    let pending = ProjectionWork::pending(identity(ProjectionKind::Graph));
    assert_eq!(
        pending.transition(ProjectionWorkEvent::Succeeded),
        Err(ProjectionWorkTransitionError::InvalidTransition)
    );

    let failed = pending
        .transition(ProjectionWorkEvent::Start)
        .expect("start")
        .transition(ProjectionWorkEvent::Failed)
        .expect("fail");
    assert_eq!(failed.state(), ProjectionWorkState::Failed);
    assert_eq!(
        failed.transition(ProjectionWorkEvent::Start),
        Err(ProjectionWorkTransitionError::TerminalState)
    );
}

#[test]
fn interrupted_indexing_becomes_retry_scheduled_before_restart() {
    let indexing = ProjectionWork::pending(identity(ProjectionKind::Graph))
        .transition(ProjectionWorkEvent::Start)
        .expect("start");
    let recovered = indexing
        .transition(ProjectionWorkEvent::Interrupted)
        .expect("recover");
    assert_eq!(recovered.state(), ProjectionWorkState::RetryScheduled);
    assert_eq!(
        recovered
            .transition(ProjectionWorkEvent::Start)
            .expect("restart")
            .attempt(),
        2
    );
}

#[test]
fn terminal_projection_work_accepts_only_explicit_reindex_repair() {
    for terminal in [ProjectionWorkState::Ready, ProjectionWorkState::Failed] {
        let work = ProjectionWork::restore(identity(ProjectionKind::Graph), terminal, 2).unwrap();
        let reset = work
            .transition(ProjectionWorkEvent::ReindexRequested)
            .expect("terminal repair");
        assert_eq!(reset.state(), ProjectionWorkState::Pending);
        assert_eq!(reset.attempt(), 0);
        assert_eq!(
            work.transition(ProjectionWorkEvent::Start),
            Err(ProjectionWorkTransitionError::TerminalState)
        );
    }
}

fn identity(kind: ProjectionKind) -> ProjectionWorkIdentity {
    ProjectionWorkIdentity::new(
        WorkspaceId::new("workspace-1").expect("workspace"),
        DocumentId::new("doc-1").expect("document"),
        VersionId::new("version-1").expect("version"),
        kind,
    )
}
