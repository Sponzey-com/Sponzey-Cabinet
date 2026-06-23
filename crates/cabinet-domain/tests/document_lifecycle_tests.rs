use cabinet_domain::document::{
    DocumentError, DocumentLifecycleEvent, DocumentLifecycleState, transition_document_lifecycle,
};

#[test]
fn document_lifecycle_allows_expected_creation_edit_archive_delete_and_restore_flow() {
    let draft = transition_document_lifecycle(
        DocumentLifecycleState::Draft,
        DocumentLifecycleEvent::Create,
    )
    .expect("create should keep draft state");
    assert_eq!(draft.next_state, DocumentLifecycleState::Draft);

    let saved = transition_document_lifecycle(draft.next_state, DocumentLifecycleEvent::Save)
        .expect("save should transition");
    assert_eq!(saved.next_state, DocumentLifecycleState::Saved);

    let editing =
        transition_document_lifecycle(saved.next_state, DocumentLifecycleEvent::StartEdit)
            .expect("start edit should transition");
    assert_eq!(editing.next_state, DocumentLifecycleState::Editing);

    let saved_again =
        transition_document_lifecycle(editing.next_state, DocumentLifecycleEvent::Save)
            .expect("save from editing should transition");
    assert_eq!(saved_again.next_state, DocumentLifecycleState::Saved);

    let archived =
        transition_document_lifecycle(saved_again.next_state, DocumentLifecycleEvent::Archive)
            .expect("archive should transition");
    assert_eq!(archived.next_state, DocumentLifecycleState::Archived);

    let restored =
        transition_document_lifecycle(archived.next_state, DocumentLifecycleEvent::Restore)
            .expect("restore should transition");
    assert_eq!(restored.next_state, DocumentLifecycleState::Restored);

    let deleted =
        transition_document_lifecycle(restored.next_state, DocumentLifecycleEvent::Delete)
            .expect("delete should transition");
    assert_eq!(deleted.next_state, DocumentLifecycleState::Deleted);
}

#[test]
fn document_lifecycle_rejects_invalid_transition() {
    let error = transition_document_lifecycle(
        DocumentLifecycleState::Draft,
        DocumentLifecycleEvent::Restore,
    )
    .expect_err("restore from draft must fail");

    assert_eq!(
        error,
        DocumentError::InvalidLifecycleTransition {
            state: DocumentLifecycleState::Draft,
            event: DocumentLifecycleEvent::Restore,
        }
    );
}
