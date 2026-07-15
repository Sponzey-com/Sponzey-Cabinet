use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;
use cabinet_domain::workflow::{
    PublishWorkflow, PublishWorkflowErrorCode, PublishWorkflowEvent, PublishWorkflowGuard,
    PublishWorkflowSideEffectRequest, PublishWorkflowState, ReviewDecision, ReviewDecisionKind,
    ReviewRequest,
};

#[test]
fn publish_workflow_allows_review_approval_and_publish_path() {
    let guard = PublishWorkflowGuard::allow_all();

    let requested = PublishWorkflow::transition(
        PublishWorkflowState::Editing,
        PublishWorkflowEvent::RequestReview,
        guard,
    )
    .expect("editing documents can request review");
    assert_eq!(requested.next_state, PublishWorkflowState::ReviewRequested);
    assert_eq!(requested.product_log_event_name, "review.requested");
    assert_eq!(
        requested.side_effect_requests,
        vec![PublishWorkflowSideEffectRequest::CreateReviewRequest]
    );

    let approved =
        PublishWorkflow::transition(requested.next_state, PublishWorkflowEvent::Approve, guard)
            .expect("reviewer can approve a review request");
    assert_eq!(approved.next_state, PublishWorkflowState::Approved);
    assert_eq!(approved.product_log_event_name, "review.approved");
    assert_eq!(
        approved.side_effect_requests,
        vec![PublishWorkflowSideEffectRequest::RecordReviewDecision]
    );

    let published =
        PublishWorkflow::transition(approved.next_state, PublishWorkflowEvent::Publish, guard)
            .expect("publisher can publish an approved document");
    assert_eq!(published.next_state, PublishWorkflowState::Published);
    assert_eq!(published.product_log_event_name, "document.published");
    assert_eq!(
        published.side_effect_requests,
        vec![
            PublishWorkflowSideEffectRequest::CreateVersionEntry,
            PublishWorkflowSideEffectRequest::RecordAuditEvent,
        ]
    );
}

#[test]
fn publish_workflow_supports_changes_requested_and_rejected_paths() {
    let guard = PublishWorkflowGuard::allow_all();

    let changes_requested = PublishWorkflow::transition(
        PublishWorkflowState::ReviewRequested,
        PublishWorkflowEvent::RequestChanges,
        guard,
    )
    .expect("reviewer can request changes");
    assert_eq!(
        changes_requested.next_state,
        PublishWorkflowState::ChangesRequested
    );
    assert_eq!(
        changes_requested.product_log_event_name,
        "review.changes_requested"
    );
    assert_eq!(
        changes_requested.side_effect_requests,
        vec![PublishWorkflowSideEffectRequest::RecordReviewDecision]
    );

    let editing_again = PublishWorkflow::transition(
        changes_requested.next_state,
        PublishWorkflowEvent::ResumeEditing,
        guard,
    )
    .expect("document can return to editing after changes are requested");
    assert_eq!(editing_again.next_state, PublishWorkflowState::Editing);
    assert_eq!(
        editing_again.product_log_event_name,
        "document.workflow.editing_resumed"
    );
    assert!(editing_again.side_effect_requests.is_empty());

    let rejected = PublishWorkflow::transition(
        PublishWorkflowState::ReviewRequested,
        PublishWorkflowEvent::Reject,
        guard,
    )
    .expect("reviewer can reject a review request");
    assert_eq!(rejected.next_state, PublishWorkflowState::Rejected);
    assert_eq!(rejected.product_log_event_name, "review.rejected");
    assert_eq!(
        rejected.side_effect_requests,
        vec![PublishWorkflowSideEffectRequest::RecordReviewDecision]
    );
}

#[test]
fn publish_workflow_rejects_publish_before_approval() {
    let failure = PublishWorkflow::transition(
        PublishWorkflowState::ReviewRequested,
        PublishWorkflowEvent::Publish,
        PublishWorkflowGuard::allow_all(),
    )
    .expect_err("publish before approval must be rejected");

    assert_eq!(
        failure.previous_state,
        PublishWorkflowState::ReviewRequested
    );
    assert_eq!(failure.event, PublishWorkflowEvent::Publish);
    assert_eq!(
        failure.error_code,
        PublishWorkflowErrorCode::InvalidWorkflowTransition
    );
    assert_eq!(
        failure.product_log_event_name,
        "document.workflow.invalid_transition"
    );
}

#[test]
fn publish_workflow_requires_review_and_publish_guards() {
    let review_failure = PublishWorkflow::transition(
        PublishWorkflowState::ReviewRequested,
        PublishWorkflowEvent::Approve,
        PublishWorkflowGuard::publisher_only(),
    )
    .expect_err("approve requires reviewer permission");
    assert_eq!(
        review_failure.error_code,
        PublishWorkflowErrorCode::ReviewPermissionRequired
    );
    assert_eq!(
        review_failure.product_log_event_name,
        "document.workflow.invalid_transition"
    );

    let publish_failure = PublishWorkflow::transition(
        PublishWorkflowState::Approved,
        PublishWorkflowEvent::Publish,
        PublishWorkflowGuard::reviewer_only(),
    )
    .expect_err("publish requires publish permission");
    assert_eq!(
        publish_failure.error_code,
        PublishWorkflowErrorCode::PublishPermissionRequired
    );
    assert_eq!(
        publish_failure.product_log_event_name,
        "document.workflow.invalid_transition"
    );
}

#[test]
fn published_to_editing_requests_new_version_and_audit_side_effects() {
    let transition = PublishWorkflow::transition(
        PublishWorkflowState::Published,
        PublishWorkflowEvent::EditPublishedDocument,
        PublishWorkflowGuard::allow_all(),
    )
    .expect("editing a published document must create traceable side effects");

    assert_eq!(transition.next_state, PublishWorkflowState::Editing);
    assert_eq!(
        transition.product_log_event_name,
        "document.workflow.edit_published"
    );
    assert_eq!(
        transition.side_effect_requests,
        vec![
            PublishWorkflowSideEffectRequest::CreateVersionEntry,
            PublishWorkflowSideEffectRequest::RecordAuditEvent,
        ]
    );
}

#[test]
fn review_request_and_decision_models_keep_document_and_actor_identity() {
    let document_id = DocumentId::new("doc-1").expect("valid document id");
    let requester_id = UserId::new("user-requester").expect("valid requester id");
    let reviewer_id = UserId::new("user-reviewer").expect("valid reviewer id");

    let review_request = ReviewRequest::new(document_id.clone(), requester_id.clone());
    assert_eq!(review_request.document_id(), &document_id);
    assert_eq!(review_request.requested_by(), &requester_id);

    let approval = ReviewDecision::approved(document_id.clone(), reviewer_id.clone());
    assert_eq!(approval.document_id(), &document_id);
    assert_eq!(approval.reviewer_id(), &reviewer_id);
    assert_eq!(approval.kind(), ReviewDecisionKind::Approved);

    let rejection = ReviewDecision::rejected(document_id.clone(), reviewer_id.clone());
    assert_eq!(rejection.kind(), ReviewDecisionKind::Rejected);

    let changes_requested = ReviewDecision::changes_requested(document_id, reviewer_id);
    assert_eq!(
        changes_requested.kind(),
        ReviewDecisionKind::ChangesRequested
    );
}
