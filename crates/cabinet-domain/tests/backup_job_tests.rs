use cabinet_domain::backup::{
    BackupJobEvent, BackupJobId, BackupJobOperation, BackupJobRetryPolicy, BackupJobSnapshot,
    BackupJobState, BackupJobStateMachine, BackupProgress,
};
use cabinet_domain::workspace::WorkspaceId;

#[test]
fn backup_job_transitions_through_retry_and_completion() {
    let policy = BackupJobRetryPolicy::new(2).expect("retry policy");
    let queued = job(BackupJobState::Queued, 0);

    let running = BackupJobStateMachine::transition(&queued, BackupJobEvent::Start, policy)
        .expect("queued starts");
    let failed =
        BackupJobStateMachine::transition(&running.job(), BackupJobEvent::FailRetryable, policy)
            .expect("retryable failure enters failed");
    let retrying = BackupJobStateMachine::transition(&failed.job(), BackupJobEvent::Retry, policy)
        .expect("failed can retry");
    let running_again =
        BackupJobStateMachine::transition(&retrying.job(), BackupJobEvent::Start, policy)
            .expect("retrying starts");
    let completed =
        BackupJobStateMachine::transition(&running_again.job(), BackupJobEvent::Complete, policy)
            .expect("running completes");

    assert_eq!(running.next_state(), BackupJobState::Running);
    assert_eq!(failed.next_state(), BackupJobState::Failed);
    assert_eq!(retrying.next_state(), BackupJobState::Retrying);
    assert_eq!(retrying.job().retry_count(), 1);
    assert_eq!(completed.next_state(), BackupJobState::Completed);
    assert_eq!(completed.product_log_event_name(), Some("backup.completed"));
}

#[test]
fn backup_job_abandons_when_retry_budget_is_exhausted() {
    let policy = BackupJobRetryPolicy::new(1).expect("retry policy");
    let running = job(BackupJobState::Running, 1);

    let abandoned =
        BackupJobStateMachine::transition(&running, BackupJobEvent::FailRetryable, policy)
            .expect("retry budget exhausted abandons job");

    assert_eq!(abandoned.next_state(), BackupJobState::Abandoned);
    assert_eq!(
        abandoned.job().error_code(),
        Some("BACKUP_JOB_RETRY_EXHAUSTED")
    );
    assert_eq!(abandoned.product_log_event_name(), Some("backup.failed"));
}

#[test]
fn backup_job_rejects_invalid_transition_from_terminal_state() {
    let policy = BackupJobRetryPolicy::new(1).expect("retry policy");
    let completed = job(BackupJobState::Completed, 0);

    let error = BackupJobStateMachine::transition(&completed, BackupJobEvent::Start, policy)
        .expect_err("terminal state cannot restart");

    assert_eq!(error.code(), "BACKUP_JOB_INVALID_TRANSITION");
    assert_eq!(error.previous_state(), BackupJobState::Completed);
    assert_eq!(error.event(), BackupJobEvent::Start);
}

fn job(state: BackupJobState, retry_count: u16) -> BackupJobSnapshot {
    BackupJobSnapshot::new(
        BackupJobId::new("backup-job-1").expect("job id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        BackupJobOperation::Backup,
        state,
        retry_count,
        BackupProgress::new(0, 1).expect("progress"),
        None,
    )
    .expect("job")
}
