use cabinet_domain::collaboration::{
    BaseRevision, CollaborationError, DocumentOperation, EditSessionEvent, EditSessionState,
    OperationId, Presence, TextRange, detect_collaboration_conflict, transition_edit_session_state,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;

#[test]
fn document_operation_rejects_invalid_range_and_empty_operation_id() {
    let document_id = document_id("doc-1");
    let actor = user_id("user-1");
    let revision = BaseRevision::new(3).expect("revision");
    let invalid_range = TextRange::new(8, 3).expect_err("range start must not exceed end");

    assert_eq!(invalid_range, CollaborationError::InvalidTextRange);
    assert_eq!(
        OperationId::new(" ").expect_err("empty operation id"),
        CollaborationError::EmptyOperationId,
    );
    assert_eq!(
        DocumentOperation::replace_text(
            OperationId::new("op-1").expect("operation id"),
            document_id,
            actor,
            revision,
            TextRange::new(0, 0).expect("range"),
            "",
        )
        .expect_err("empty replacement must be rejected"),
        CollaborationError::EmptyOperationPatch,
    );
}

#[test]
fn edit_session_state_machine_rejects_invalid_transition_and_tracks_conflict() {
    assert_eq!(
        transition_edit_session_state(EditSessionState::Idle, EditSessionEvent::StartSession)
            .expect("start"),
        EditSessionState::SessionStarted,
    );
    assert_eq!(
        transition_edit_session_state(
            EditSessionState::SessionStarted,
            EditSessionEvent::BeginEdit
        )
        .expect("edit"),
        EditSessionState::Editing,
    );
    assert_eq!(
        transition_edit_session_state(EditSessionState::Syncing, EditSessionEvent::DetectConflict)
            .expect("conflict"),
        EditSessionState::ConflictDetected,
    );
    assert_eq!(
        transition_edit_session_state(EditSessionState::Idle, EditSessionEvent::SyncSucceeded)
            .expect_err("invalid transition"),
        CollaborationError::InvalidStateTransition,
    );
}

#[test]
fn stale_base_revision_is_reported_as_collaboration_conflict() {
    let operation = DocumentOperation::replace_text(
        OperationId::new("op-1").expect("operation id"),
        document_id("doc-1"),
        user_id("user-1"),
        BaseRevision::new(2).expect("base revision"),
        TextRange::new(0, 4).expect("range"),
        "next",
    )
    .expect("operation");

    let conflict =
        detect_collaboration_conflict(&operation, BaseRevision::new(3).expect("current revision"))
            .expect("stale revision conflict");

    assert_eq!(conflict.reason_code(), "collaboration.stale_base_revision");
    assert_eq!(conflict.operation_id(), operation.operation_id());
}

#[test]
fn presence_is_validated_without_document_body_or_selection_text() {
    let presence = Presence::new(
        document_id("doc-1"),
        user_id("user-1"),
        TextRange::new(2, 2).expect("cursor"),
    )
    .expect("presence");

    assert_eq!(presence.document_id().as_str(), "doc-1");
    assert_eq!(presence.actor_user_id().as_str(), "user-1");
    assert_eq!(presence.cursor().start(), 2);
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}

fn user_id(value: &str) -> UserId {
    UserId::new(value).expect("user id")
}
