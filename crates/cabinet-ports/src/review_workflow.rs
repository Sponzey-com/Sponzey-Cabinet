use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{Permission, PermissionDecision};
use cabinet_domain::user::UserId;
use cabinet_domain::workflow::{
    PublishWorkflowEvent, PublishWorkflowSideEffectRequest, PublishWorkflowState, ReviewRequest,
};
use cabinet_domain::workspace::WorkspaceId;

pub trait ReviewWorkflowRepository {
    fn get_workflow_state(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<PublishWorkflowState>, ReviewWorkflowRepositoryError>;

    fn save_workflow_state(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        state: PublishWorkflowState,
    ) -> Result<(), ReviewWorkflowRepositoryError>;

    fn save_review_request(
        &mut self,
        workspace_id: &WorkspaceId,
        review_request_id: &str,
        request: ReviewRequest,
    ) -> Result<ReviewRequestRecord, ReviewWorkflowRepositoryError>;

    fn get_review_request(
        &self,
        workspace_id: &WorkspaceId,
        review_request_id: &str,
    ) -> Result<Option<ReviewRequestRecord>, ReviewWorkflowRepositoryError>;

    fn update_review_request_status(
        &mut self,
        workspace_id: &WorkspaceId,
        review_request_id: &str,
        status: ReviewRequestStatus,
    ) -> Result<Option<ReviewRequestRecord>, ReviewWorkflowRepositoryError>;

    fn list_review_requests(
        &self,
        workspace_id: &WorkspaceId,
        document_id: Option<&DocumentId>,
    ) -> Result<Vec<ReviewRequestRecord>, ReviewWorkflowRepositoryError>;
}

pub trait ReviewWorkflowPermissionChecker {
    fn check_document_permission(
        &self,
        actor_user_id: &UserId,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        permission: Permission,
    ) -> Result<PermissionDecision, ReviewWorkflowPermissionCheckError>;
}

pub trait ReviewWorkflowSideEffectRecorder {
    fn record_review_workflow_side_effect(
        &mut self,
        record: ReviewWorkflowSideEffectRecord,
    ) -> Result<(), ReviewWorkflowSideEffectError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewRequestRecord {
    workspace_id: WorkspaceId,
    review_request_id: String,
    request: ReviewRequest,
    status: ReviewRequestStatus,
}

impl ReviewRequestRecord {
    pub fn new(
        workspace_id: &WorkspaceId,
        review_request_id: &str,
        request: ReviewRequest,
        status: ReviewRequestStatus,
    ) -> Result<Self, ReviewWorkflowRepositoryError> {
        let review_request_id = validate_review_request_id(review_request_id)?;
        Ok(Self {
            workspace_id: workspace_id.clone(),
            review_request_id,
            request,
            status,
        })
    }

    pub fn with_status(&self, status: ReviewRequestStatus) -> Self {
        Self {
            workspace_id: self.workspace_id.clone(),
            review_request_id: self.review_request_id.clone(),
            request: self.request.clone(),
            status,
        }
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn workspace_matches(&self, workspace_id: &WorkspaceId) -> bool {
        &self.workspace_id == workspace_id
    }

    pub fn review_request_id(&self) -> &str {
        &self.review_request_id
    }

    pub fn request(&self) -> &ReviewRequest {
        &self.request
    }

    pub const fn status(&self) -> ReviewRequestStatus {
        self.status
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewRequestStatus {
    ReviewRequested,
    Approved,
    Rejected,
    ChangesRequested,
    Published,
}

impl ReviewRequestStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReviewRequested => "review_requested",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::ChangesRequested => "changes_requested",
            Self::Published => "published",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewWorkflowSideEffectRecord {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    review_request_id: Option<String>,
    actor_user_id: UserId,
    from_state: PublishWorkflowState,
    to_state: PublishWorkflowState,
    event: PublishWorkflowEvent,
    side_effect: PublishWorkflowSideEffectRequest,
    product_log_event_name: &'static str,
}

impl ReviewWorkflowSideEffectRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: WorkspaceId,
        document_id: DocumentId,
        review_request_id: Option<String>,
        actor_user_id: UserId,
        from_state: PublishWorkflowState,
        to_state: PublishWorkflowState,
        event: PublishWorkflowEvent,
        side_effect: PublishWorkflowSideEffectRequest,
        product_log_event_name: &'static str,
    ) -> Self {
        Self {
            workspace_id,
            document_id,
            review_request_id,
            actor_user_id,
            from_state,
            to_state,
            event,
            side_effect,
            product_log_event_name,
        }
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn review_request_id(&self) -> Option<&str> {
        self.review_request_id.as_deref()
    }

    pub fn actor_user_id(&self) -> &UserId {
        &self.actor_user_id
    }

    pub const fn from_state(&self) -> PublishWorkflowState {
        self.from_state
    }

    pub const fn to_state(&self) -> PublishWorkflowState {
        self.to_state
    }

    pub const fn event(&self) -> PublishWorkflowEvent {
        self.event
    }

    pub const fn side_effect(&self) -> PublishWorkflowSideEffectRequest {
        self.side_effect
    }

    pub const fn product_log_event_name(&self) -> &'static str {
        self.product_log_event_name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewWorkflowRepositoryError {
    InvalidReviewRequestId,
    StorageUnavailable,
    Conflict,
    CorruptedState,
}

impl ReviewWorkflowRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidReviewRequestId => "review_workflow.invalid_review_request_id",
            Self::StorageUnavailable => "review_workflow.storage_unavailable",
            Self::Conflict => "review_workflow.conflict",
            Self::CorruptedState => "review_workflow.corrupted_state",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewWorkflowPermissionCheckError {
    StorageUnavailable,
}

impl ReviewWorkflowPermissionCheckError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "review_workflow_permission.storage_unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewWorkflowSideEffectError {
    StorageUnavailable,
}

impl ReviewWorkflowSideEffectError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "review_workflow_side_effect.storage_unavailable",
        }
    }
}

fn validate_review_request_id(value: &str) -> Result<String, ReviewWorkflowRepositoryError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(ReviewWorkflowRepositoryError::InvalidReviewRequestId);
    }
    Ok(trimmed.to_string())
}
