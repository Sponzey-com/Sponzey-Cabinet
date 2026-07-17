use cabinet_domain::document_diff_operation::{
    DocumentDiffOperation, DocumentDiffOperationError, DocumentDiffOperationEvent,
    DocumentDiffOperationId, DocumentDiffOperationSideEffect, DocumentDiffOperationState,
};

#[test]
fn operation_id_rejects_empty_control_and_overlong_values() {
    assert_eq!(
        DocumentDiffOperationId::new(" ").unwrap_err(),
        DocumentDiffOperationError::InvalidOperationId
    );
    assert_eq!(
        DocumentDiffOperationId::new("diff\noperation").unwrap_err(),
        DocumentDiffOperationError::InvalidOperationId
    );
    assert_eq!(
        DocumentDiffOperationId::new(&"x".repeat(129)).unwrap_err(),
        DocumentDiffOperationError::InvalidOperationId
    );

    let id = DocumentDiffOperationId::new("  diff-operation-1  ").unwrap();
    assert_eq!(id.as_str(), "diff-operation-1");
}

#[test]
fn accepted_operation_runs_and_completes_with_explicit_side_effects() {
    let accepted = operation();
    assert_eq!(accepted.state(), DocumentDiffOperationState::Accepted);
    assert_eq!(
        accepted.state().product_log_event(),
        "document.diff.background.accepted"
    );

    let started = accepted
        .transition(DocumentDiffOperationEvent::Start)
        .unwrap();
    assert_eq!(
        started.operation().state(),
        DocumentDiffOperationState::Running
    );
    assert_eq!(
        started.side_effect(),
        Some(DocumentDiffOperationSideEffect::RunDiff)
    );
    assert_eq!(
        started.product_log_event(),
        "document.diff.background.running"
    );

    let completed = started
        .operation()
        .transition(DocumentDiffOperationEvent::Complete)
        .unwrap();
    assert_eq!(
        completed.operation().state(),
        DocumentDiffOperationState::Completed
    );
    assert_eq!(completed.side_effect(), None);
    assert_eq!(
        completed.product_log_event(),
        "document.diff.background.completed"
    );
}

#[test]
fn accepted_and_running_operations_have_explicit_terminal_outcomes() {
    let cases = [
        (
            DocumentDiffOperationState::Accepted,
            DocumentDiffOperationEvent::Cancel,
            DocumentDiffOperationState::Cancelled,
            None,
        ),
        (
            DocumentDiffOperationState::Accepted,
            DocumentDiffOperationEvent::Expire,
            DocumentDiffOperationState::Expired,
            None,
        ),
        (
            DocumentDiffOperationState::Accepted,
            DocumentDiffOperationEvent::Fail,
            DocumentDiffOperationState::Failed,
            None,
        ),
        (
            DocumentDiffOperationState::Running,
            DocumentDiffOperationEvent::Cancel,
            DocumentDiffOperationState::Cancelled,
            Some(DocumentDiffOperationSideEffect::RequestCancellation),
        ),
        (
            DocumentDiffOperationState::Running,
            DocumentDiffOperationEvent::Expire,
            DocumentDiffOperationState::Expired,
            None,
        ),
        (
            DocumentDiffOperationState::Running,
            DocumentDiffOperationEvent::Fail,
            DocumentDiffOperationState::Failed,
            None,
        ),
    ];

    for (initial, event, expected, side_effect) in cases {
        let current = operation_in(initial);
        let transition = current.transition(event).unwrap();
        assert_eq!(transition.operation().state(), expected);
        assert_eq!(transition.side_effect(), side_effect);
        assert_eq!(transition.product_log_event(), expected.product_log_event());
        assert!(expected.is_terminal());
    }
}

#[test]
fn invalid_and_terminal_transitions_are_distinct_and_stable() {
    let running = operation_in(DocumentDiffOperationState::Running);
    assert_eq!(
        running
            .transition(DocumentDiffOperationEvent::Start)
            .unwrap_err(),
        DocumentDiffOperationError::InvalidTransition {
            state: DocumentDiffOperationState::Running,
            event: DocumentDiffOperationEvent::Start,
        }
    );

    for state in [
        DocumentDiffOperationState::Completed,
        DocumentDiffOperationState::Cancelled,
        DocumentDiffOperationState::Expired,
        DocumentDiffOperationState::Failed,
    ] {
        let error = operation_in(state)
            .transition(DocumentDiffOperationEvent::Start)
            .unwrap_err();
        assert_eq!(error, DocumentDiffOperationError::TerminalState { state });
        assert_eq!(error.code(), "document_diff_operation.terminal_state");
    }
}

#[test]
fn transition_is_deterministic_for_the_same_snapshot_and_event() {
    let current = operation_in(DocumentDiffOperationState::Running);
    let first = current
        .transition(DocumentDiffOperationEvent::Complete)
        .unwrap();
    let second = current
        .transition(DocumentDiffOperationEvent::Complete)
        .unwrap();
    assert_eq!(first, second);
}

fn operation() -> DocumentDiffOperation {
    DocumentDiffOperation::accepted(DocumentDiffOperationId::new("diff-operation-1").unwrap())
}

fn operation_in(state: DocumentDiffOperationState) -> DocumentDiffOperation {
    DocumentDiffOperation::restore(
        DocumentDiffOperationId::new("diff-operation-1").unwrap(),
        state,
    )
}
