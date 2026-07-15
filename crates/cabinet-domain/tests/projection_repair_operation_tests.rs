use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_repair::{
    ProjectionRepairEvent, ProjectionRepairOperation, ProjectionRepairOperationId,
    ProjectionRepairSideEffect, ProjectionRepairState, ProjectionRepairTransitionError,
};
use cabinet_domain::workspace::WorkspaceId;

#[test]
fn repair_operation_follows_queued_running_publishing_succeeded_flow() {
    let queued = operation();
    let running = queued.transition(ProjectionRepairEvent::Start).unwrap();
    let publishing = running
        .operation()
        .transition(ProjectionRepairEvent::PublishStarted)
        .unwrap();
    let succeeded = publishing
        .operation()
        .transition(ProjectionRepairEvent::Succeeded)
        .unwrap();

    assert_eq!(queued.state(), ProjectionRepairState::Queued);
    assert_eq!(running.operation().state(), ProjectionRepairState::Running);
    assert_eq!(running.operation().attempt(), 1);
    assert_eq!(
        running.side_effect(),
        Some(ProjectionRepairSideEffect::RunProjectionRepair)
    );
    assert_eq!(
        running.product_log_event(),
        Some("projection.reindex.started")
    );
    assert_eq!(
        publishing.operation().state(),
        ProjectionRepairState::Publishing
    );
    assert_eq!(
        succeeded.operation().state(),
        ProjectionRepairState::Succeeded
    );
    assert_eq!(
        succeeded.product_log_event(),
        Some("projection.reindex.completed")
    );
}

#[test]
fn repair_operation_supports_cancel_before_publish_and_rejects_late_cancel() {
    let running = operation()
        .transition(ProjectionRepairEvent::Start)
        .unwrap()
        .into_operation();
    let cancelling = running
        .transition(ProjectionRepairEvent::CancelRequested)
        .unwrap();
    let cancelled = cancelling
        .operation()
        .transition(ProjectionRepairEvent::Cancelled)
        .unwrap();

    assert_eq!(
        cancelling.operation().state(),
        ProjectionRepairState::CancelPending
    );
    assert_eq!(
        cancelling.side_effect(),
        Some(ProjectionRepairSideEffect::RequestCancellation)
    );
    assert_eq!(
        cancelled.operation().state(),
        ProjectionRepairState::Cancelled
    );
    assert_eq!(
        cancelled.product_log_event(),
        Some("projection.reindex.cancelled")
    );

    let publishing = operation()
        .transition(ProjectionRepairEvent::Start)
        .unwrap()
        .into_operation()
        .transition(ProjectionRepairEvent::PublishStarted)
        .unwrap()
        .into_operation();
    assert_eq!(
        publishing.transition(ProjectionRepairEvent::CancelRequested),
        Err(ProjectionRepairTransitionError::CancellationTooLate)
    );
}

#[test]
fn only_retryable_failure_can_return_to_queued_and_stale_events_are_rejected() {
    let running = operation()
        .transition(ProjectionRepairEvent::Start)
        .unwrap()
        .into_operation();
    let retryable = running
        .transition(ProjectionRepairEvent::FailedRetryable)
        .unwrap()
        .into_operation();
    let retried = retryable.transition(ProjectionRepairEvent::Retry).unwrap();
    assert_eq!(retried.operation().state(), ProjectionRepairState::Queued);
    assert_eq!(retried.operation().attempt(), 1);

    let fatal = operation()
        .transition(ProjectionRepairEvent::Start)
        .unwrap()
        .into_operation()
        .transition(ProjectionRepairEvent::FailedFatal)
        .unwrap()
        .into_operation();
    assert_eq!(
        fatal.transition(ProjectionRepairEvent::Retry),
        Err(ProjectionRepairTransitionError::TerminalState)
    );
    assert_eq!(
        retried
            .operation()
            .transition(ProjectionRepairEvent::Succeeded),
        Err(ProjectionRepairTransitionError::InvalidTransition)
    );
}

#[test]
fn operation_identity_rejects_blank_and_control_characters() {
    assert_eq!(
        ProjectionRepairOperationId::new("  "),
        Err(ProjectionRepairTransitionError::InvalidOperationId)
    );
    assert_eq!(
        ProjectionRepairOperationId::new("repair\n1"),
        Err(ProjectionRepairTransitionError::InvalidOperationId)
    );
}

fn operation() -> ProjectionRepairOperation {
    ProjectionRepairOperation::queued(
        ProjectionRepairOperationId::new("repair-1").unwrap(),
        WorkspaceId::new("workspace-1").unwrap(),
        DocumentId::new("doc-1").unwrap(),
    )
}
