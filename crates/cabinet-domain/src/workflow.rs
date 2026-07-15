use crate::document::DocumentId;
use crate::user::UserId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewRequest {
    document_id: DocumentId,
    requested_by: UserId,
}

impl ReviewRequest {
    pub fn new(document_id: DocumentId, requested_by: UserId) -> Self {
        Self {
            document_id,
            requested_by,
        }
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn requested_by(&self) -> &UserId {
        &self.requested_by
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewDecision {
    document_id: DocumentId,
    reviewer_id: UserId,
    kind: ReviewDecisionKind,
}

impl ReviewDecision {
    pub fn approved(document_id: DocumentId, reviewer_id: UserId) -> Self {
        Self::new(document_id, reviewer_id, ReviewDecisionKind::Approved)
    }

    pub fn rejected(document_id: DocumentId, reviewer_id: UserId) -> Self {
        Self::new(document_id, reviewer_id, ReviewDecisionKind::Rejected)
    }

    pub fn changes_requested(document_id: DocumentId, reviewer_id: UserId) -> Self {
        Self::new(
            document_id,
            reviewer_id,
            ReviewDecisionKind::ChangesRequested,
        )
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn reviewer_id(&self) -> &UserId {
        &self.reviewer_id
    }

    pub const fn kind(&self) -> ReviewDecisionKind {
        self.kind
    }

    fn new(document_id: DocumentId, reviewer_id: UserId, kind: ReviewDecisionKind) -> Self {
        Self {
            document_id,
            reviewer_id,
            kind,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewDecisionKind {
    Approved,
    Rejected,
    ChangesRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishWorkflowState {
    Editing,
    ReviewRequested,
    ChangesRequested,
    Approved,
    Published,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishWorkflowEvent {
    RequestReview,
    Approve,
    Reject,
    RequestChanges,
    ResumeEditing,
    Publish,
    EditPublishedDocument,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishWorkflowGuard {
    reviewer_allowed: bool,
    publisher_allowed: bool,
}

impl PublishWorkflowGuard {
    pub const fn new(reviewer_allowed: bool, publisher_allowed: bool) -> Self {
        Self {
            reviewer_allowed,
            publisher_allowed,
        }
    }

    pub const fn allow_all() -> Self {
        Self::new(true, true)
    }

    pub const fn reviewer_only() -> Self {
        Self::new(true, false)
    }

    pub const fn publisher_only() -> Self {
        Self::new(false, true)
    }

    pub const fn none() -> Self {
        Self::new(false, false)
    }

    pub const fn reviewer_allowed(self) -> bool {
        self.reviewer_allowed
    }

    pub const fn publisher_allowed(self) -> bool {
        self.publisher_allowed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishWorkflowSideEffectRequest {
    CreateReviewRequest,
    RecordReviewDecision,
    CreateVersionEntry,
    RecordAuditEvent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishWorkflowTransition {
    pub previous_state: PublishWorkflowState,
    pub event: PublishWorkflowEvent,
    pub next_state: PublishWorkflowState,
    pub side_effect_requests: Vec<PublishWorkflowSideEffectRequest>,
    pub product_log_event_name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishWorkflowTransitionError {
    pub previous_state: PublishWorkflowState,
    pub event: PublishWorkflowEvent,
    pub error_code: PublishWorkflowErrorCode,
    pub product_log_event_name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishWorkflowErrorCode {
    InvalidWorkflowTransition,
    ReviewPermissionRequired,
    PublishPermissionRequired,
}

impl PublishWorkflowErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidWorkflowTransition => "INVALID_WORKFLOW_TRANSITION",
            Self::ReviewPermissionRequired => "REVIEW_PERMISSION_REQUIRED",
            Self::PublishPermissionRequired => "PUBLISH_PERMISSION_REQUIRED",
        }
    }
}

pub struct PublishWorkflow;

impl PublishWorkflow {
    pub fn transition(
        state: PublishWorkflowState,
        event: PublishWorkflowEvent,
        guard: PublishWorkflowGuard,
    ) -> Result<PublishWorkflowTransition, PublishWorkflowTransitionError> {
        match (state, event) {
            (PublishWorkflowState::Editing, PublishWorkflowEvent::RequestReview) => Ok(success(
                state,
                event,
                PublishWorkflowState::ReviewRequested,
                vec![PublishWorkflowSideEffectRequest::CreateReviewRequest],
                "review.requested",
            )),
            (PublishWorkflowState::ReviewRequested, PublishWorkflowEvent::Approve) => {
                require_reviewer(state, event, guard)?;
                Ok(success(
                    state,
                    event,
                    PublishWorkflowState::Approved,
                    vec![PublishWorkflowSideEffectRequest::RecordReviewDecision],
                    "review.approved",
                ))
            }
            (PublishWorkflowState::ReviewRequested, PublishWorkflowEvent::Reject) => {
                require_reviewer(state, event, guard)?;
                Ok(success(
                    state,
                    event,
                    PublishWorkflowState::Rejected,
                    vec![PublishWorkflowSideEffectRequest::RecordReviewDecision],
                    "review.rejected",
                ))
            }
            (PublishWorkflowState::ReviewRequested, PublishWorkflowEvent::RequestChanges) => {
                require_reviewer(state, event, guard)?;
                Ok(success(
                    state,
                    event,
                    PublishWorkflowState::ChangesRequested,
                    vec![PublishWorkflowSideEffectRequest::RecordReviewDecision],
                    "review.changes_requested",
                ))
            }
            (PublishWorkflowState::ChangesRequested, PublishWorkflowEvent::ResumeEditing) => {
                Ok(success(
                    state,
                    event,
                    PublishWorkflowState::Editing,
                    Vec::new(),
                    "document.workflow.editing_resumed",
                ))
            }
            (PublishWorkflowState::Approved, PublishWorkflowEvent::Publish) => {
                require_publisher(state, event, guard)?;
                Ok(success(
                    state,
                    event,
                    PublishWorkflowState::Published,
                    vec![
                        PublishWorkflowSideEffectRequest::CreateVersionEntry,
                        PublishWorkflowSideEffectRequest::RecordAuditEvent,
                    ],
                    "document.published",
                ))
            }
            (PublishWorkflowState::Published, PublishWorkflowEvent::EditPublishedDocument) => {
                require_publisher(state, event, guard)?;
                Ok(success(
                    state,
                    event,
                    PublishWorkflowState::Editing,
                    vec![
                        PublishWorkflowSideEffectRequest::CreateVersionEntry,
                        PublishWorkflowSideEffectRequest::RecordAuditEvent,
                    ],
                    "document.workflow.edit_published",
                ))
            }
            _ => Err(failure(
                state,
                event,
                PublishWorkflowErrorCode::InvalidWorkflowTransition,
            )),
        }
    }
}

fn require_reviewer(
    state: PublishWorkflowState,
    event: PublishWorkflowEvent,
    guard: PublishWorkflowGuard,
) -> Result<(), PublishWorkflowTransitionError> {
    if guard.reviewer_allowed() {
        Ok(())
    } else {
        Err(failure(
            state,
            event,
            PublishWorkflowErrorCode::ReviewPermissionRequired,
        ))
    }
}

fn require_publisher(
    state: PublishWorkflowState,
    event: PublishWorkflowEvent,
    guard: PublishWorkflowGuard,
) -> Result<(), PublishWorkflowTransitionError> {
    if guard.publisher_allowed() {
        Ok(())
    } else {
        Err(failure(
            state,
            event,
            PublishWorkflowErrorCode::PublishPermissionRequired,
        ))
    }
}

fn success(
    previous_state: PublishWorkflowState,
    event: PublishWorkflowEvent,
    next_state: PublishWorkflowState,
    side_effect_requests: Vec<PublishWorkflowSideEffectRequest>,
    product_log_event_name: &'static str,
) -> PublishWorkflowTransition {
    PublishWorkflowTransition {
        previous_state,
        event,
        next_state,
        side_effect_requests,
        product_log_event_name,
    }
}

fn failure(
    previous_state: PublishWorkflowState,
    event: PublishWorkflowEvent,
    error_code: PublishWorkflowErrorCode,
) -> PublishWorkflowTransitionError {
    PublishWorkflowTransitionError {
        previous_state,
        event,
        error_code,
        product_log_event_name: "document.workflow.invalid_transition",
    }
}
