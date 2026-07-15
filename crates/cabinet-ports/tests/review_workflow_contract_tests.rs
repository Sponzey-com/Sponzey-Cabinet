use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;
use cabinet_domain::workflow::{
    PublishWorkflowEvent, PublishWorkflowSideEffectRequest, PublishWorkflowState, ReviewRequest,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::review_workflow::{
    ReviewRequestRecord, ReviewRequestStatus, ReviewWorkflowRepositoryError,
    ReviewWorkflowSideEffectRecord,
};

#[test]
fn review_request_record_keeps_domain_request_without_storage_schema() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let requested_by = UserId::new("user-1").expect("user id");
    let request = ReviewRequest::new(document_id.clone(), requested_by.clone());

    let record = ReviewRequestRecord::new(
        &workspace_id,
        "review-1",
        request,
        ReviewRequestStatus::ReviewRequested,
    )
    .expect("valid review request record");

    assert_eq!(record.workspace_id(), &workspace_id);
    assert_eq!(record.review_request_id(), "review-1");
    assert_eq!(record.request().document_id(), &document_id);
    assert_eq!(record.request().requested_by(), &requested_by);
    assert_eq!(record.status(), ReviewRequestStatus::ReviewRequested);

    let approved = record.with_status(ReviewRequestStatus::Approved);
    assert_eq!(approved.status(), ReviewRequestStatus::Approved);
    assert_eq!(approved.request().document_id(), &document_id);
}

#[test]
fn review_request_record_rejects_invalid_id() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let requested_by = UserId::new("user-1").expect("user id");
    let request = ReviewRequest::new(document_id, requested_by);

    let error = ReviewRequestRecord::new(
        &workspace_id,
        " \n ",
        request,
        ReviewRequestStatus::ReviewRequested,
    )
    .expect_err("control id must be rejected");

    assert_eq!(error, ReviewWorkflowRepositoryError::InvalidReviewRequestId);
}

#[test]
fn side_effect_record_exposes_transition_summary_without_payload_body() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let actor = UserId::new("publisher-1").expect("actor id");

    let record = ReviewWorkflowSideEffectRecord::new(
        workspace_id.clone(),
        document_id.clone(),
        Some("review-1".to_string()),
        actor.clone(),
        PublishWorkflowState::Approved,
        PublishWorkflowState::Published,
        PublishWorkflowEvent::Publish,
        PublishWorkflowSideEffectRequest::RecordAuditEvent,
        "document.published",
    );

    assert_eq!(record.workspace_id(), &workspace_id);
    assert_eq!(record.document_id(), &document_id);
    assert_eq!(record.review_request_id(), Some("review-1"));
    assert_eq!(record.actor_user_id(), &actor);
    assert_eq!(record.from_state(), PublishWorkflowState::Approved);
    assert_eq!(record.to_state(), PublishWorkflowState::Published);
    assert_eq!(record.event(), PublishWorkflowEvent::Publish);
    assert_eq!(
        record.side_effect(),
        PublishWorkflowSideEffectRequest::RecordAuditEvent
    );
    assert_eq!(record.product_log_event_name(), "document.published");
}
