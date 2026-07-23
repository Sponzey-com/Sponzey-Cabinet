use cabinet_domain::attachment_operation::{
    AttachmentOperationError, AttachmentOperationEvent as E, AttachmentOperationSideEffect as Fx,
    AttachmentOperationState as S, transition_attachment_operation,
};

#[test]
fn successful_attachment_operation_reaches_completed_through_every_durable_stage() {
    let cases = [
        (
            S::Selected,
            E::ValidationRequested,
            S::Validating,
            Fx::ValidateSource,
        ),
        (
            S::Validating,
            E::ValidationSucceeded,
            S::Staging,
            Fx::StageSource,
        ),
        (S::Staging, E::StagingCompleted, S::Hashing, Fx::HashContent),
        (
            S::Hashing,
            E::HashCompleted,
            S::PublishingObject,
            Fx::PublishObject,
        ),
        (
            S::PublishingObject,
            E::ObjectPublished,
            S::PersistingMetadata,
            Fx::PersistMetadata,
        ),
        (
            S::PersistingMetadata,
            E::MetadataPersisted,
            S::PreparingRevision,
            Fx::LoadCurrentRevision,
        ),
        (
            S::PreparingRevision,
            E::RevisionPrepared,
            S::Associating,
            Fx::CommitAssociation,
        ),
        (
            S::Associating,
            E::PrimaryCommitSucceeded,
            S::Projecting,
            Fx::ProjectAssociation,
        ),
        (
            S::Projecting,
            E::ProjectionCompleted,
            S::Verifying,
            Fx::VerifyReadback,
        ),
        (
            S::Verifying,
            E::ReadbackMatched,
            S::Completed,
            Fx::PersistTerminalResult,
        ),
    ];

    for (from, event, to, side_effect) in cases {
        let result = transition_attachment_operation(from, event).expect("valid transition");
        assert_eq!(result.previous_state, from);
        assert_eq!(result.event, event);
        assert_eq!(result.next_state, to);
        assert_eq!(result.side_effect, side_effect);
        assert_eq!(result.retryable, false);
    }
}

#[test]
fn cancellation_before_and_after_primary_commit_never_claims_the_same_outcome() {
    for state in [
        S::Selected,
        S::Validating,
        S::Staging,
        S::Hashing,
        S::PublishingObject,
        S::PersistingMetadata,
        S::PreparingRevision,
    ] {
        let result =
            transition_attachment_operation(state, E::CancelRequested).expect("pre-commit cancel");
        assert_eq!(result.next_state, S::Cancelled);
        assert_eq!(result.side_effect, Fx::CleanupStaging);
        assert_eq!(
            result.product_log_event,
            Some("document.attachment.cancelled")
        );
    }

    for state in [S::Associating, S::Projecting, S::Verifying] {
        let result =
            transition_attachment_operation(state, E::CancelRequested).expect("post-commit cancel");
        assert_eq!(result.next_state, S::RecoveryRequired);
        assert_eq!(result.side_effect, Fx::RepairAndVerify);
        assert_eq!(result.error_code, Some("ATTACHMENT_POST_COMMIT_RECOVERY"));
        assert_eq!(
            result.product_log_event,
            Some("document.attachment.recovery_required")
        );
    }
}

#[test]
fn terminal_replay_is_idempotent_and_other_terminal_events_are_rejected() {
    let completed = transition_attachment_operation(S::Completed, E::ReadbackMatched)
        .expect("completed replay");
    assert_eq!(completed.next_state, S::Completed);
    assert_eq!(completed.side_effect, Fx::None);
    assert_eq!(completed.product_log_event, None);

    let cancelled =
        transition_attachment_operation(S::Cancelled, E::CancelRequested).expect("cancel replay");
    assert_eq!(cancelled.next_state, S::Cancelled);
    assert_eq!(cancelled.side_effect, Fx::None);

    assert_eq!(
        transition_attachment_operation(S::Completed, E::CancelRequested),
        Err(AttachmentOperationError::InvalidTransition {
            state: S::Completed,
            event: E::CancelRequested
        }),
    );
}

#[test]
fn conflict_and_recovery_only_accept_their_explicit_resume_events() {
    let conflict =
        transition_attachment_operation(S::Conflict, E::RetryRequested).expect("conflict retry");
    assert_eq!(conflict.next_state, S::PreparingRevision);
    assert_eq!(conflict.side_effect, Fx::ReloadCurrentRevision);
    assert_eq!(conflict.retryable, true);

    let recovery =
        transition_attachment_operation(S::RecoveryRequired, E::RepairRequested).expect("repair");
    assert_eq!(recovery.next_state, S::Projecting);
    assert_eq!(recovery.side_effect, Fx::RepairProjection);
    assert_eq!(recovery.retryable, true);

    assert!(matches!(
        transition_attachment_operation(S::Conflict, E::ValidationRequested),
        Err(AttachmentOperationError::InvalidTransition { .. })
    ));
    assert!(matches!(
        transition_attachment_operation(S::RecoveryRequired, E::RetryRequested),
        Err(AttachmentOperationError::InvalidTransition { .. })
    ));
}

#[test]
fn every_failure_stage_has_a_stable_non_sensitive_result() {
    let cases = [
        (
            S::Validating,
            E::ValidationFailed,
            S::Failed,
            "ATTACHMENT_VALIDATION_FAILED",
        ),
        (
            S::Staging,
            E::StagingFailed,
            S::Failed,
            "ATTACHMENT_STAGING_FAILED",
        ),
        (
            S::Hashing,
            E::HashFailed,
            S::Failed,
            "ATTACHMENT_HASH_FAILED",
        ),
        (
            S::PublishingObject,
            E::ObjectPublishFailed,
            S::Failed,
            "ATTACHMENT_OBJECT_PUBLISH_FAILED",
        ),
        (
            S::PersistingMetadata,
            E::MetadataPersistFailed,
            S::RecoveryRequired,
            "ATTACHMENT_METADATA_RECONCILIATION_REQUIRED",
        ),
        (
            S::PreparingRevision,
            E::RevisionConflict,
            S::Conflict,
            "DOCUMENT_CURRENT_CONFLICT",
        ),
        (
            S::Associating,
            E::PrimaryCommitFailed,
            S::RecoveryRequired,
            "ATTACHMENT_ASSOCIATION_RECOVERY_REQUIRED",
        ),
        (
            S::Projecting,
            E::ProjectionFailed,
            S::RecoveryRequired,
            "ATTACHMENT_PROJECTION_RECOVERY_REQUIRED",
        ),
        (
            S::Verifying,
            E::ReadbackMismatch,
            S::RecoveryRequired,
            "ATTACHMENT_READBACK_MISMATCH",
        ),
    ];

    for (state, event, expected_state, expected_code) in cases {
        let result = transition_attachment_operation(state, event).expect("failure transition");
        assert_eq!(result.next_state, expected_state);
        assert_eq!(result.error_code, Some(expected_code));
        assert!(result.product_log_event.is_some());
        let serialized = format!("{result:?}");
        assert!(!serialized.contains('/'));
        assert!(!serialized.contains("filename"));
    }
}
