use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{
    Permission, PermissionDecision, PermissionDecisionReason, PolicySource,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workflow::{
    PublishWorkflowSideEffectRequest, PublishWorkflowState, ReviewRequest,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::review_workflow::{
    ReviewRequestRecord, ReviewRequestStatus, ReviewWorkflowPermissionCheckError,
    ReviewWorkflowPermissionChecker, ReviewWorkflowRepository, ReviewWorkflowRepositoryError,
    ReviewWorkflowSideEffectError, ReviewWorkflowSideEffectRecord,
    ReviewWorkflowSideEffectRecorder,
};
use cabinet_usecases::review_workflow::{
    ApproveDocumentInput, ApproveDocumentUsecase, ListReviewRequestsInput,
    ListReviewRequestsUsecase, PublishDocumentInput, PublishDocumentUsecase, RejectDocumentInput,
    RejectDocumentUsecase, RequestDocumentReviewInput, RequestDocumentReviewUsecase,
    ReviewWorkflowFieldDebugEvent, ReviewWorkflowPolicy, ReviewWorkflowProductEvent,
    ReviewWorkflowUsecaseError, ReviewWorkflowUsecaseLogger,
};

#[derive(Default)]
struct FakeReviewWorkflowRepository {
    states: HashMap<String, PublishWorkflowState>,
    requests: HashMap<String, ReviewRequestRecord>,
    save_state_count: Cell<usize>,
    save_request_count: Cell<usize>,
    update_request_count: Cell<usize>,
    list_count: Cell<usize>,
}

impl ReviewWorkflowRepository for FakeReviewWorkflowRepository {
    fn get_workflow_state(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<PublishWorkflowState>, ReviewWorkflowRepositoryError> {
        Ok(self
            .states
            .get(&document_key(workspace_id, document_id))
            .copied())
    }

    fn save_workflow_state(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        state: PublishWorkflowState,
    ) -> Result<(), ReviewWorkflowRepositoryError> {
        self.save_state_count.set(self.save_state_count.get() + 1);
        self.states
            .insert(document_key(workspace_id, document_id), state);
        Ok(())
    }

    fn save_review_request(
        &mut self,
        workspace_id: &WorkspaceId,
        review_request_id: &str,
        request: ReviewRequest,
    ) -> Result<ReviewRequestRecord, ReviewWorkflowRepositoryError> {
        self.save_request_count
            .set(self.save_request_count.get() + 1);
        let record = ReviewRequestRecord::new(
            workspace_id,
            review_request_id,
            request,
            ReviewRequestStatus::ReviewRequested,
        )?;
        self.requests
            .insert(review_key(workspace_id, review_request_id), record.clone());
        Ok(record)
    }

    fn get_review_request(
        &self,
        workspace_id: &WorkspaceId,
        review_request_id: &str,
    ) -> Result<Option<ReviewRequestRecord>, ReviewWorkflowRepositoryError> {
        Ok(self
            .requests
            .get(&review_key(workspace_id, review_request_id))
            .cloned())
    }

    fn update_review_request_status(
        &mut self,
        workspace_id: &WorkspaceId,
        review_request_id: &str,
        status: ReviewRequestStatus,
    ) -> Result<Option<ReviewRequestRecord>, ReviewWorkflowRepositoryError> {
        self.update_request_count
            .set(self.update_request_count.get() + 1);
        let key = review_key(workspace_id, review_request_id);
        let Some(record) = self.requests.get(&key) else {
            return Ok(None);
        };
        let updated = record.with_status(status);
        self.requests.insert(key, updated.clone());
        Ok(Some(updated))
    }

    fn list_review_requests(
        &self,
        workspace_id: &WorkspaceId,
        document_id: Option<&DocumentId>,
    ) -> Result<Vec<ReviewRequestRecord>, ReviewWorkflowRepositoryError> {
        self.list_count.set(self.list_count.get() + 1);
        Ok(self
            .requests
            .values()
            .filter(|record| {
                record.workspace_matches(workspace_id)
                    && document_id
                        .map(|document_id| record.request().document_id() == document_id)
                        .unwrap_or(true)
            })
            .cloned()
            .collect())
    }
}

#[derive(Default)]
struct FakePermissionChecker {
    decisions: Vec<(Permission, PermissionDecision)>,
    requested_permissions: Cell<usize>,
}

impl FakePermissionChecker {
    fn allow(&mut self, permission: Permission) {
        self.decisions.push((
            permission,
            PermissionDecision::allowed(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleAllowsPermission,
            ),
        ));
    }

    fn deny(&mut self, permission: Permission) {
        self.decisions.push((
            permission,
            PermissionDecision::denied(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleDoesNotAllowPermission,
            ),
        ));
    }
}

impl ReviewWorkflowPermissionChecker for FakePermissionChecker {
    fn check_document_permission(
        &self,
        _actor_user_id: &UserId,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        permission: Permission,
    ) -> Result<PermissionDecision, ReviewWorkflowPermissionCheckError> {
        self.requested_permissions
            .set(self.requested_permissions.get() + 1);
        Ok(self
            .decisions
            .iter()
            .rev()
            .find_map(|(candidate, decision)| (*candidate == permission).then_some(*decision))
            .unwrap_or(PermissionDecision::denied(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleDoesNotAllowPermission,
            )))
    }
}

#[derive(Default)]
struct FakeSideEffectRecorder {
    records: Vec<ReviewWorkflowSideEffectRecord>,
    fail: bool,
}

impl ReviewWorkflowSideEffectRecorder for FakeSideEffectRecorder {
    fn record_review_workflow_side_effect(
        &mut self,
        record: ReviewWorkflowSideEffectRecord,
    ) -> Result<(), ReviewWorkflowSideEffectError> {
        if self.fail {
            return Err(ReviewWorkflowSideEffectError::StorageUnavailable);
        }
        self.records.push(record);
        Ok(())
    }
}

#[derive(Default)]
struct FakeReviewWorkflowLogger {
    product_events: Vec<ReviewWorkflowProductEvent>,
    field_debug_events: Vec<ReviewWorkflowFieldDebugEvent>,
}

impl ReviewWorkflowUsecaseLogger for FakeReviewWorkflowLogger {
    fn write_product(&mut self, event: ReviewWorkflowProductEvent) {
        self.product_events.push(event);
    }

    fn write_field_debug(&mut self, event: ReviewWorkflowFieldDebugEvent) {
        self.field_debug_events.push(event);
    }
}

#[test]
fn approve_and_reject_require_reviewer_permission_without_state_change() {
    let mut repository = seeded_review_request(PublishWorkflowState::ReviewRequested);
    let mut checker = FakePermissionChecker::default();
    checker.deny(Permission::Review);
    let mut side_effects = FakeSideEffectRecorder::default();
    let mut logger = FakeReviewWorkflowLogger::default();

    let approve_error = ApproveDocumentUsecase::new(ReviewWorkflowPolicy::default())
        .execute(
            ApproveDocumentInput::new("actor-1", "workspace-1", "review-1"),
            &checker,
            &mut repository,
            &mut side_effects,
            &mut logger,
        )
        .expect_err("approve requires reviewer permission");
    assert_eq!(
        approve_error,
        ReviewWorkflowUsecaseError::ReviewPermissionRequired
    );

    let reject_error = RejectDocumentUsecase::new(ReviewWorkflowPolicy::default())
        .execute(
            RejectDocumentInput::new("actor-1", "workspace-1", "review-1"),
            &checker,
            &mut repository,
            &mut side_effects,
            &mut logger,
        )
        .expect_err("reject requires reviewer permission");
    assert_eq!(
        reject_error,
        ReviewWorkflowUsecaseError::ReviewPermissionRequired
    );

    assert_eq!(repository.save_state_count.get(), 0);
    assert!(side_effects.records.is_empty());
    assert!(logger.product_events.iter().all(|event| {
        matches!(
            event,
            ReviewWorkflowProductEvent::WorkflowTransitionFailed { .. }
        )
    }));
}

#[test]
fn publish_requires_publish_permission_without_side_effects() {
    let mut repository = FakeReviewWorkflowRepository::default();
    repository.states.insert(
        "workspace-1:doc-1".to_string(),
        PublishWorkflowState::Approved,
    );
    let mut checker = FakePermissionChecker::default();
    checker.deny(Permission::Publish);
    let mut side_effects = FakeSideEffectRecorder::default();
    let mut logger = FakeReviewWorkflowLogger::default();

    let error = PublishDocumentUsecase::new(ReviewWorkflowPolicy::default())
        .execute(
            PublishDocumentInput::new("actor-1", "workspace-1", "doc-1"),
            &checker,
            &mut repository,
            &mut side_effects,
            &mut logger,
        )
        .expect_err("publish requires publish permission");

    assert_eq!(error, ReviewWorkflowUsecaseError::PublishPermissionRequired);
    assert_eq!(repository.save_state_count.get(), 0);
    assert!(side_effects.records.is_empty());
    assert!(matches!(
        logger.product_events.last(),
        Some(ReviewWorkflowProductEvent::WorkflowTransitionFailed {
            error_code: "PUBLISH_PERMISSION_REQUIRED",
            ..
        })
    ));
}

#[test]
fn publish_before_approval_returns_stable_invalid_transition() {
    let mut repository = FakeReviewWorkflowRepository::default();
    repository.states.insert(
        "workspace-1:doc-1".to_string(),
        PublishWorkflowState::ReviewRequested,
    );
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Publish);
    let mut side_effects = FakeSideEffectRecorder::default();
    let mut logger = FakeReviewWorkflowLogger::default();

    let error = PublishDocumentUsecase::new(ReviewWorkflowPolicy::default())
        .execute(
            PublishDocumentInput::new("actor-1", "workspace-1", "doc-1"),
            &checker,
            &mut repository,
            &mut side_effects,
            &mut logger,
        )
        .expect_err("only approved documents can be published");

    assert_eq!(error, ReviewWorkflowUsecaseError::InvalidWorkflowTransition);
    assert_eq!(repository.save_state_count.get(), 0);
    assert!(side_effects.records.is_empty());
    assert!(matches!(
        logger.product_events.last(),
        Some(ReviewWorkflowProductEvent::WorkflowTransitionFailed {
            error_code: "INVALID_WORKFLOW_TRANSITION",
            ..
        })
    ));
}

#[test]
fn request_approve_publish_path_updates_state_records_side_effects_and_product_logs() {
    let mut repository = FakeReviewWorkflowRepository::default();
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Review);
    checker.allow(Permission::Publish);
    let mut side_effects = FakeSideEffectRecorder::default();
    let mut logger = FakeReviewWorkflowLogger::default();
    let policy = ReviewWorkflowPolicy::default();

    let requested = RequestDocumentReviewUsecase::new(policy)
        .execute(
            RequestDocumentReviewInput::new("author-1", "workspace-1", "doc-1", "review-1"),
            &mut repository,
            &mut side_effects,
            &mut logger,
        )
        .expect("request review");
    assert_eq!(
        requested.next_state(),
        PublishWorkflowState::ReviewRequested
    );
    assert_eq!(repository.save_request_count.get(), 1);

    let approved = ApproveDocumentUsecase::new(policy)
        .execute(
            ApproveDocumentInput::new("reviewer-1", "workspace-1", "review-1"),
            &checker,
            &mut repository,
            &mut side_effects,
            &mut logger,
        )
        .expect("approve");
    assert_eq!(approved.next_state(), PublishWorkflowState::Approved);

    let published = PublishDocumentUsecase::new(policy)
        .execute(
            PublishDocumentInput::new("publisher-1", "workspace-1", "doc-1"),
            &checker,
            &mut repository,
            &mut side_effects,
            &mut logger,
        )
        .expect("publish");
    assert_eq!(published.next_state(), PublishWorkflowState::Published);
    assert_eq!(
        repository
            .states
            .get("workspace-1:doc-1")
            .copied()
            .expect("state"),
        PublishWorkflowState::Published
    );
    assert_eq!(
        side_effects
            .records
            .iter()
            .map(|record| record.side_effect())
            .collect::<Vec<_>>(),
        vec![
            PublishWorkflowSideEffectRequest::CreateReviewRequest,
            PublishWorkflowSideEffectRequest::RecordReviewDecision,
            PublishWorkflowSideEffectRequest::CreateVersionEntry,
            PublishWorkflowSideEffectRequest::RecordAuditEvent,
        ]
    );
    assert_eq!(
        logger
            .product_events
            .iter()
            .map(ReviewWorkflowProductEvent::event_name)
            .collect::<Vec<_>>(),
        vec!["review.requested", "review.approved", "document.published",]
    );
}

#[test]
fn list_review_requests_returns_summaries_after_read_permission() {
    let repository = seeded_review_request(PublishWorkflowState::ReviewRequested);
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Read);
    let mut logger = FakeReviewWorkflowLogger::default();

    let output = ListReviewRequestsUsecase::new(ReviewWorkflowPolicy::default())
        .execute(
            ListReviewRequestsInput::for_document("actor-1", "workspace-1", "doc-1"),
            &checker,
            &repository,
            &mut logger,
        )
        .expect("list review requests");

    assert_eq!(output.requests().len(), 1);
    assert_eq!(output.requests()[0].review_request_id(), "review-1");
    assert_eq!(output.requests()[0].document_id(), "doc-1");
    assert_eq!(
        output.requests()[0].status(),
        ReviewRequestStatus::ReviewRequested
    );
    assert_eq!(repository.list_count.get(), 1);
}

#[test]
fn side_effect_failure_stops_publish_and_logs_failure_without_success_log() {
    let mut repository = FakeReviewWorkflowRepository::default();
    repository.states.insert(
        "workspace-1:doc-1".to_string(),
        PublishWorkflowState::Approved,
    );
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Publish);
    let mut side_effects = FakeSideEffectRecorder {
        records: Vec::new(),
        fail: true,
    };
    let mut logger = FakeReviewWorkflowLogger::default();

    let error = PublishDocumentUsecase::new(ReviewWorkflowPolicy::default())
        .execute(
            PublishDocumentInput::new("publisher-1", "workspace-1", "doc-1"),
            &checker,
            &mut repository,
            &mut side_effects,
            &mut logger,
        )
        .expect_err("side effect failure should fail publish");

    assert_eq!(error, ReviewWorkflowUsecaseError::SideEffectUnavailable);
    assert!(
        logger
            .product_events
            .iter()
            .all(|event| { event.event_name() == "workflow.transition.failed" })
    );
}

fn seeded_review_request(state: PublishWorkflowState) -> FakeReviewWorkflowRepository {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let requester = UserId::new("author-1").expect("requester id");
    let request = ReviewRequest::new(document_id.clone(), requester);
    let record = ReviewRequestRecord::new(
        &workspace_id,
        "review-1",
        request,
        ReviewRequestStatus::ReviewRequested,
    )
    .expect("review request record");
    let mut repository = FakeReviewWorkflowRepository::default();
    repository
        .states
        .insert(document_key(&workspace_id, &document_id), state);
    repository
        .requests
        .insert(review_key(&workspace_id, "review-1"), record);
    repository
}

fn document_key(workspace_id: &WorkspaceId, document_id: &DocumentId) -> String {
    format!("{}:{}", workspace_id.as_str(), document_id.as_str())
}

fn review_key(workspace_id: &WorkspaceId, review_request_id: &str) -> String {
    format!("{}:{}", workspace_id.as_str(), review_request_id)
}
