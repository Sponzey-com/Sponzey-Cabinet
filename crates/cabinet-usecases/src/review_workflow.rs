use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{Permission, PermissionDecisionResult};
use cabinet_domain::user::UserId;
use cabinet_domain::workflow::{
    PublishWorkflow, PublishWorkflowErrorCode, PublishWorkflowEvent, PublishWorkflowGuard,
    PublishWorkflowState, ReviewDecisionKind, ReviewRequest,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::review_workflow::{
    ReviewRequestRecord, ReviewRequestStatus, ReviewWorkflowPermissionCheckError,
    ReviewWorkflowPermissionChecker, ReviewWorkflowRepository, ReviewWorkflowRepositoryError,
    ReviewWorkflowSideEffectError, ReviewWorkflowSideEffectRecord,
    ReviewWorkflowSideEffectRecorder,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewWorkflowPolicy {
    review_permission: Permission,
    publish_permission: Permission,
    list_permission: Permission,
}

impl ReviewWorkflowPolicy {
    pub const fn new(
        review_permission: Permission,
        publish_permission: Permission,
        list_permission: Permission,
    ) -> Self {
        Self {
            review_permission,
            publish_permission,
            list_permission,
        }
    }

    pub const fn review_permission(self) -> Permission {
        self.review_permission
    }

    pub const fn publish_permission(self) -> Permission {
        self.publish_permission
    }

    pub const fn list_permission(self) -> Permission {
        self.list_permission
    }
}

impl Default for ReviewWorkflowPolicy {
    fn default() -> Self {
        Self::new(Permission::Review, Permission::Publish, Permission::Read)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestDocumentReviewInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: String,
    review_request_id: String,
}

impl RequestDocumentReviewInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        document_id: &str,
        review_request_id: &str,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            review_request_id: review_request_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApproveDocumentInput {
    actor_user_id: String,
    workspace_id: String,
    review_request_id: String,
}

impl ApproveDocumentInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, review_request_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            review_request_id: review_request_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectDocumentInput {
    actor_user_id: String,
    workspace_id: String,
    review_request_id: String,
}

impl RejectDocumentInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, review_request_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            review_request_id: review_request_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishDocumentInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: String,
}

impl PublishDocumentInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, document_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListReviewRequestsInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: Option<String>,
}

impl ListReviewRequestsInput {
    pub fn for_workspace(actor_user_id: &str, workspace_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: None,
        }
    }

    pub fn for_document(actor_user_id: &str, workspace_id: &str, document_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: Some(document_id.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewWorkflowOutput {
    document_id: String,
    review_request_id: Option<String>,
    previous_state: PublishWorkflowState,
    next_state: PublishWorkflowState,
    product_log_event_name: &'static str,
}

impl ReviewWorkflowOutput {
    pub fn document_id(&self) -> &str {
        &self.document_id
    }

    pub fn review_request_id(&self) -> Option<&str> {
        self.review_request_id.as_deref()
    }

    pub const fn previous_state(&self) -> PublishWorkflowState {
        self.previous_state
    }

    pub fn previous_state_name(&self) -> &'static str {
        workflow_state_name(self.previous_state)
    }

    pub const fn next_state(&self) -> PublishWorkflowState {
        self.next_state
    }

    pub fn next_state_name(&self) -> &'static str {
        workflow_state_name(self.next_state)
    }

    pub const fn product_log_event_name(&self) -> &'static str {
        self.product_log_event_name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewRequestSummary {
    review_request_id: String,
    document_id: String,
    requested_by: String,
    status: ReviewRequestStatus,
}

impl ReviewRequestSummary {
    pub fn review_request_id(&self) -> &str {
        &self.review_request_id
    }

    pub fn document_id(&self) -> &str {
        &self.document_id
    }

    pub fn requested_by(&self) -> &str {
        &self.requested_by
    }

    pub const fn status(&self) -> ReviewRequestStatus {
        self.status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListReviewRequestsOutput {
    requests: Vec<ReviewRequestSummary>,
}

impl ListReviewRequestsOutput {
    pub fn requests(&self) -> &[ReviewRequestSummary] {
        &self.requests
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewWorkflowProductEvent {
    TransitionCompleted {
        event_name: &'static str,
        masked_actor_id: String,
        document_id: String,
        review_request_id: Option<String>,
        from_state: &'static str,
        to_state: &'static str,
    },
    WorkflowTransitionFailed {
        masked_actor_id: String,
        document_id: Option<String>,
        review_request_id: Option<String>,
        from_state: Option<&'static str>,
        error_code: &'static str,
    },
}

impl ReviewWorkflowProductEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::TransitionCompleted { event_name, .. } => event_name,
            Self::WorkflowTransitionFailed { .. } => "workflow.transition.failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewWorkflowFieldDebugEvent {
    from_state: Option<&'static str>,
    to_state: Option<&'static str>,
    transition_event: &'static str,
    guard_result: &'static str,
    permission_decision: &'static str,
}

impl ReviewWorkflowFieldDebugEvent {
    pub const fn from_state(&self) -> Option<&'static str> {
        self.from_state
    }

    pub const fn to_state(&self) -> Option<&'static str> {
        self.to_state
    }

    pub const fn transition_event(&self) -> &'static str {
        self.transition_event
    }

    pub const fn guard_result(&self) -> &'static str {
        self.guard_result
    }

    pub const fn permission_decision(&self) -> &'static str {
        self.permission_decision
    }
}

pub trait ReviewWorkflowUsecaseLogger {
    fn write_product(&mut self, event: ReviewWorkflowProductEvent);
    fn write_field_debug(&mut self, event: ReviewWorkflowFieldDebugEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestDocumentReviewUsecase {
    policy: ReviewWorkflowPolicy,
}

impl RequestDocumentReviewUsecase {
    pub const fn new(policy: ReviewWorkflowPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: RequestDocumentReviewInput,
        repository: &mut impl ReviewWorkflowRepository,
        side_effect_recorder: &mut impl ReviewWorkflowSideEffectRecorder,
        logger: &mut impl ReviewWorkflowUsecaseLogger,
    ) -> Result<ReviewWorkflowOutput, ReviewWorkflowUsecaseError> {
        let command = RequestReviewCommand::from_input(input)
            .map_err(|error| log_error(logger, error, None, None, None, None))?;
        let state = load_state_or_editing(repository, &command.workspace_id, &command.document_id)
            .map_err(|error| {
                log_error(
                    logger,
                    error,
                    Some(&command.actor_user_id),
                    Some(&command.document_id),
                    Some(&command.review_request_id),
                    None,
                )
            })?;
        let transition = PublishWorkflow::transition(
            state,
            PublishWorkflowEvent::RequestReview,
            PublishWorkflowGuard::allow_all(),
        )
        .map_err(ReviewWorkflowUsecaseError::from_transition_error)
        .map_err(|error| {
            log_error(
                logger,
                error,
                Some(&command.actor_user_id),
                Some(&command.document_id),
                Some(&command.review_request_id),
                Some(state),
            )
        })?;

        let request =
            ReviewRequest::new(command.document_id.clone(), command.actor_user_id.clone());
        repository
            .save_review_request(&command.workspace_id, &command.review_request_id, request)
            .map_err(ReviewWorkflowUsecaseError::from_repository_error)
            .map_err(|error| {
                log_error(
                    logger,
                    error,
                    Some(&command.actor_user_id),
                    Some(&command.document_id),
                    Some(&command.review_request_id),
                    Some(state),
                )
            })?;
        persist_transition(
            repository,
            side_effect_recorder,
            logger,
            &command.workspace_id,
            &command.document_id,
            Some(command.review_request_id.as_str()),
            &command.actor_user_id,
            transition,
        )
    }
}

impl Default for RequestDocumentReviewUsecase {
    fn default() -> Self {
        Self::new(ReviewWorkflowPolicy::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApproveDocumentUsecase {
    policy: ReviewWorkflowPolicy,
}

impl ApproveDocumentUsecase {
    pub const fn new(policy: ReviewWorkflowPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: ApproveDocumentInput,
        permission_checker: &impl ReviewWorkflowPermissionChecker,
        repository: &mut impl ReviewWorkflowRepository,
        side_effect_recorder: &mut impl ReviewWorkflowSideEffectRecorder,
        logger: &mut impl ReviewWorkflowUsecaseLogger,
    ) -> Result<ReviewWorkflowOutput, ReviewWorkflowUsecaseError> {
        decide_review_request(
            DecisionInput {
                actor_user_id: input.actor_user_id,
                workspace_id: input.workspace_id,
                review_request_id: input.review_request_id,
                event: PublishWorkflowEvent::Approve,
                status: ReviewRequestStatus::Approved,
                decision_kind: ReviewDecisionKind::Approved,
            },
            self.policy,
            permission_checker,
            repository,
            side_effect_recorder,
            logger,
        )
    }
}

impl Default for ApproveDocumentUsecase {
    fn default() -> Self {
        Self::new(ReviewWorkflowPolicy::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RejectDocumentUsecase {
    policy: ReviewWorkflowPolicy,
}

impl RejectDocumentUsecase {
    pub const fn new(policy: ReviewWorkflowPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: RejectDocumentInput,
        permission_checker: &impl ReviewWorkflowPermissionChecker,
        repository: &mut impl ReviewWorkflowRepository,
        side_effect_recorder: &mut impl ReviewWorkflowSideEffectRecorder,
        logger: &mut impl ReviewWorkflowUsecaseLogger,
    ) -> Result<ReviewWorkflowOutput, ReviewWorkflowUsecaseError> {
        decide_review_request(
            DecisionInput {
                actor_user_id: input.actor_user_id,
                workspace_id: input.workspace_id,
                review_request_id: input.review_request_id,
                event: PublishWorkflowEvent::Reject,
                status: ReviewRequestStatus::Rejected,
                decision_kind: ReviewDecisionKind::Rejected,
            },
            self.policy,
            permission_checker,
            repository,
            side_effect_recorder,
            logger,
        )
    }
}

impl Default for RejectDocumentUsecase {
    fn default() -> Self {
        Self::new(ReviewWorkflowPolicy::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishDocumentUsecase {
    policy: ReviewWorkflowPolicy,
}

impl PublishDocumentUsecase {
    pub const fn new(policy: ReviewWorkflowPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: PublishDocumentInput,
        permission_checker: &impl ReviewWorkflowPermissionChecker,
        repository: &mut impl ReviewWorkflowRepository,
        side_effect_recorder: &mut impl ReviewWorkflowSideEffectRecorder,
        logger: &mut impl ReviewWorkflowUsecaseLogger,
    ) -> Result<ReviewWorkflowOutput, ReviewWorkflowUsecaseError> {
        let command = PublishCommand::from_input(input)
            .map_err(|error| log_error(logger, error, None, None, None, None))?;
        let publish_allowed = ensure_permission(
            self.policy.publish_permission(),
            permission_checker,
            logger,
            &command.actor_user_id,
            &command.workspace_id,
            &command.document_id,
        )?;
        if !publish_allowed {
            return Err(log_error(
                logger,
                ReviewWorkflowUsecaseError::PublishPermissionRequired,
                Some(&command.actor_user_id),
                Some(&command.document_id),
                None,
                None,
            ));
        }
        let state = load_state_or_editing(repository, &command.workspace_id, &command.document_id)
            .map_err(|error| {
                log_error(
                    logger,
                    error,
                    Some(&command.actor_user_id),
                    Some(&command.document_id),
                    None,
                    None,
                )
            })?;
        let transition = PublishWorkflow::transition(
            state,
            PublishWorkflowEvent::Publish,
            PublishWorkflowGuard::publisher_only(),
        )
        .map_err(ReviewWorkflowUsecaseError::from_transition_error)
        .map_err(|error| {
            log_error(
                logger,
                error,
                Some(&command.actor_user_id),
                Some(&command.document_id),
                None,
                Some(state),
            )
        })?;
        persist_transition(
            repository,
            side_effect_recorder,
            logger,
            &command.workspace_id,
            &command.document_id,
            None,
            &command.actor_user_id,
            transition,
        )
    }
}

impl Default for PublishDocumentUsecase {
    fn default() -> Self {
        Self::new(ReviewWorkflowPolicy::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListReviewRequestsUsecase {
    policy: ReviewWorkflowPolicy,
}

impl ListReviewRequestsUsecase {
    pub const fn new(policy: ReviewWorkflowPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: ListReviewRequestsInput,
        permission_checker: &impl ReviewWorkflowPermissionChecker,
        repository: &impl ReviewWorkflowRepository,
        logger: &mut impl ReviewWorkflowUsecaseLogger,
    ) -> Result<ListReviewRequestsOutput, ReviewWorkflowUsecaseError> {
        let actor_user_id = parse_user_id(&input.actor_user_id)
            .map_err(|error| log_error(logger, error, None, None, None, None))?;
        let workspace_id = parse_workspace_id(&input.workspace_id)
            .map_err(|error| log_error(logger, error, Some(&actor_user_id), None, None, None))?;
        let document_id = input
            .document_id
            .as_deref()
            .map(parse_document_id)
            .transpose()
            .map_err(|error| log_error(logger, error, Some(&actor_user_id), None, None, None))?;

        if let Some(document_id) = &document_id {
            let allowed = ensure_permission(
                self.policy.list_permission(),
                permission_checker,
                logger,
                &actor_user_id,
                &workspace_id,
                document_id,
            )?;
            if !allowed {
                return Err(log_error(
                    logger,
                    ReviewWorkflowUsecaseError::Unauthorized,
                    Some(&actor_user_id),
                    Some(document_id),
                    None,
                    None,
                ));
            }
        }

        let records = repository
            .list_review_requests(&workspace_id, document_id.as_ref())
            .map_err(ReviewWorkflowUsecaseError::from_repository_error)
            .map_err(|error| {
                log_error(
                    logger,
                    error,
                    Some(&actor_user_id),
                    document_id.as_ref(),
                    None,
                    None,
                )
            })?;
        let requests = records
            .iter()
            .map(ReviewRequestSummary::from_record)
            .collect();
        Ok(ListReviewRequestsOutput { requests })
    }
}

impl Default for ListReviewRequestsUsecase {
    fn default() -> Self {
        Self::new(ReviewWorkflowPolicy::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewWorkflowUsecaseError {
    InvalidInput,
    Unauthorized,
    ReviewPermissionRequired,
    PublishPermissionRequired,
    ReviewRequestNotFound,
    InvalidWorkflowTransition,
    StorageUnavailable,
    Conflict,
    SideEffectUnavailable,
}

impl ReviewWorkflowUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "REVIEW_WORKFLOW_INVALID_INPUT",
            Self::Unauthorized => "REVIEW_WORKFLOW_UNAUTHORIZED",
            Self::ReviewPermissionRequired => "REVIEW_PERMISSION_REQUIRED",
            Self::PublishPermissionRequired => "PUBLISH_PERMISSION_REQUIRED",
            Self::ReviewRequestNotFound => "REVIEW_REQUEST_NOT_FOUND",
            Self::InvalidWorkflowTransition => "INVALID_WORKFLOW_TRANSITION",
            Self::StorageUnavailable => "REVIEW_WORKFLOW_STORAGE_UNAVAILABLE",
            Self::Conflict => "REVIEW_WORKFLOW_CONFLICT",
            Self::SideEffectUnavailable => "REVIEW_WORKFLOW_SIDE_EFFECT_UNAVAILABLE",
        }
    }

    const fn from_repository_error(error: ReviewWorkflowRepositoryError) -> Self {
        match error {
            ReviewWorkflowRepositoryError::InvalidReviewRequestId => Self::InvalidInput,
            ReviewWorkflowRepositoryError::StorageUnavailable
            | ReviewWorkflowRepositoryError::CorruptedState => Self::StorageUnavailable,
            ReviewWorkflowRepositoryError::Conflict => Self::Conflict,
        }
    }

    const fn from_permission_error(error: ReviewWorkflowPermissionCheckError) -> Self {
        match error {
            ReviewWorkflowPermissionCheckError::StorageUnavailable => Self::StorageUnavailable,
        }
    }

    const fn from_side_effect_error(error: ReviewWorkflowSideEffectError) -> Self {
        match error {
            ReviewWorkflowSideEffectError::StorageUnavailable => Self::SideEffectUnavailable,
        }
    }

    const fn from_transition_error(
        error: cabinet_domain::workflow::PublishWorkflowTransitionError,
    ) -> Self {
        match error.error_code {
            PublishWorkflowErrorCode::InvalidWorkflowTransition => Self::InvalidWorkflowTransition,
            PublishWorkflowErrorCode::ReviewPermissionRequired => Self::ReviewPermissionRequired,
            PublishWorkflowErrorCode::PublishPermissionRequired => Self::PublishPermissionRequired,
        }
    }
}

struct RequestReviewCommand {
    actor_user_id: UserId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    review_request_id: String,
}

impl RequestReviewCommand {
    fn from_input(input: RequestDocumentReviewInput) -> Result<Self, ReviewWorkflowUsecaseError> {
        Ok(Self {
            actor_user_id: parse_user_id(&input.actor_user_id)?,
            workspace_id: parse_workspace_id(&input.workspace_id)?,
            document_id: parse_document_id(&input.document_id)?,
            review_request_id: parse_review_request_id(&input.review_request_id)?,
        })
    }
}

struct ReviewDecisionCommand {
    actor_user_id: UserId,
    workspace_id: WorkspaceId,
    review_request_id: String,
}

struct DecisionInput {
    actor_user_id: String,
    workspace_id: String,
    review_request_id: String,
    event: PublishWorkflowEvent,
    status: ReviewRequestStatus,
    decision_kind: ReviewDecisionKind,
}

impl ReviewDecisionCommand {
    fn from_input(input: &DecisionInput) -> Result<Self, ReviewWorkflowUsecaseError> {
        Ok(Self {
            actor_user_id: parse_user_id(&input.actor_user_id)?,
            workspace_id: parse_workspace_id(&input.workspace_id)?,
            review_request_id: parse_review_request_id(&input.review_request_id)?,
        })
    }
}

struct PublishCommand {
    actor_user_id: UserId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
}

impl PublishCommand {
    fn from_input(input: PublishDocumentInput) -> Result<Self, ReviewWorkflowUsecaseError> {
        Ok(Self {
            actor_user_id: parse_user_id(&input.actor_user_id)?,
            workspace_id: parse_workspace_id(&input.workspace_id)?,
            document_id: parse_document_id(&input.document_id)?,
        })
    }
}

fn decide_review_request(
    input: DecisionInput,
    policy: ReviewWorkflowPolicy,
    permission_checker: &impl ReviewWorkflowPermissionChecker,
    repository: &mut impl ReviewWorkflowRepository,
    side_effect_recorder: &mut impl ReviewWorkflowSideEffectRecorder,
    logger: &mut impl ReviewWorkflowUsecaseLogger,
) -> Result<ReviewWorkflowOutput, ReviewWorkflowUsecaseError> {
    let command = ReviewDecisionCommand::from_input(&input)
        .map_err(|error| log_error(logger, error, None, None, None, None))?;
    let record = repository
        .get_review_request(&command.workspace_id, &command.review_request_id)
        .map_err(ReviewWorkflowUsecaseError::from_repository_error)
        .map_err(|error| {
            log_error(
                logger,
                error,
                Some(&command.actor_user_id),
                None,
                Some(&command.review_request_id),
                None,
            )
        })?
        .ok_or_else(|| {
            log_error(
                logger,
                ReviewWorkflowUsecaseError::ReviewRequestNotFound,
                Some(&command.actor_user_id),
                None,
                Some(&command.review_request_id),
                None,
            )
        })?;
    let document_id = record.request().document_id().clone();
    let reviewer_allowed = ensure_permission(
        policy.review_permission(),
        permission_checker,
        logger,
        &command.actor_user_id,
        &command.workspace_id,
        &document_id,
    )?;
    if !reviewer_allowed {
        return Err(log_error(
            logger,
            ReviewWorkflowUsecaseError::ReviewPermissionRequired,
            Some(&command.actor_user_id),
            Some(&document_id),
            Some(&command.review_request_id),
            None,
        ));
    }

    let state = load_state_or_editing(repository, &command.workspace_id, &document_id).map_err(
        |error| {
            log_error(
                logger,
                error,
                Some(&command.actor_user_id),
                Some(&document_id),
                Some(&command.review_request_id),
                None,
            )
        },
    )?;
    let transition =
        PublishWorkflow::transition(state, input.event, PublishWorkflowGuard::reviewer_only())
            .map_err(ReviewWorkflowUsecaseError::from_transition_error)
            .map_err(|error| {
                log_error(
                    logger,
                    error,
                    Some(&command.actor_user_id),
                    Some(&document_id),
                    Some(&command.review_request_id),
                    Some(state),
                )
            })?;

    let _decision_kind = input.decision_kind;
    repository
        .update_review_request_status(
            &command.workspace_id,
            &command.review_request_id,
            input.status,
        )
        .map_err(ReviewWorkflowUsecaseError::from_repository_error)
        .map_err(|error| {
            log_error(
                logger,
                error,
                Some(&command.actor_user_id),
                Some(&document_id),
                Some(&command.review_request_id),
                Some(state),
            )
        })?
        .ok_or_else(|| {
            log_error(
                logger,
                ReviewWorkflowUsecaseError::ReviewRequestNotFound,
                Some(&command.actor_user_id),
                Some(&document_id),
                Some(&command.review_request_id),
                Some(state),
            )
        })?;

    persist_transition(
        repository,
        side_effect_recorder,
        logger,
        &command.workspace_id,
        &document_id,
        Some(command.review_request_id.as_str()),
        &command.actor_user_id,
        transition,
    )
}

#[allow(clippy::too_many_arguments)]
fn persist_transition(
    repository: &mut impl ReviewWorkflowRepository,
    side_effect_recorder: &mut impl ReviewWorkflowSideEffectRecorder,
    logger: &mut impl ReviewWorkflowUsecaseLogger,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
    review_request_id: Option<&str>,
    actor_user_id: &UserId,
    transition: cabinet_domain::workflow::PublishWorkflowTransition,
) -> Result<ReviewWorkflowOutput, ReviewWorkflowUsecaseError> {
    for side_effect in &transition.side_effect_requests {
        side_effect_recorder
            .record_review_workflow_side_effect(ReviewWorkflowSideEffectRecord::new(
                workspace_id.clone(),
                document_id.clone(),
                review_request_id.map(str::to_string),
                actor_user_id.clone(),
                transition.previous_state,
                transition.next_state,
                transition.event,
                *side_effect,
                transition.product_log_event_name,
            ))
            .map_err(ReviewWorkflowUsecaseError::from_side_effect_error)
            .map_err(|error| {
                log_error(
                    logger,
                    error,
                    Some(actor_user_id),
                    Some(document_id),
                    review_request_id,
                    Some(transition.previous_state),
                )
            })?;
    }

    repository
        .save_workflow_state(workspace_id, document_id, transition.next_state)
        .map_err(ReviewWorkflowUsecaseError::from_repository_error)
        .map_err(|error| {
            log_error(
                logger,
                error,
                Some(actor_user_id),
                Some(document_id),
                review_request_id,
                Some(transition.previous_state),
            )
        })?;

    logger.write_field_debug(ReviewWorkflowFieldDebugEvent {
        from_state: Some(workflow_state_name(transition.previous_state)),
        to_state: Some(workflow_state_name(transition.next_state)),
        transition_event: workflow_event_name(transition.event),
        guard_result: "allowed",
        permission_decision: "allowed",
    });
    logger.write_product(ReviewWorkflowProductEvent::TransitionCompleted {
        event_name: transition.product_log_event_name,
        masked_actor_id: mask_user_id(actor_user_id),
        document_id: document_id.as_str().to_string(),
        review_request_id: review_request_id.map(str::to_string),
        from_state: workflow_state_name(transition.previous_state),
        to_state: workflow_state_name(transition.next_state),
    });

    Ok(ReviewWorkflowOutput {
        document_id: document_id.as_str().to_string(),
        review_request_id: review_request_id.map(str::to_string),
        previous_state: transition.previous_state,
        next_state: transition.next_state,
        product_log_event_name: transition.product_log_event_name,
    })
}

fn load_state_or_editing(
    repository: &impl ReviewWorkflowRepository,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
) -> Result<PublishWorkflowState, ReviewWorkflowUsecaseError> {
    repository
        .get_workflow_state(workspace_id, document_id)
        .map_err(ReviewWorkflowUsecaseError::from_repository_error)
        .map(|state| state.unwrap_or(PublishWorkflowState::Editing))
}

fn ensure_permission(
    permission: Permission,
    permission_checker: &impl ReviewWorkflowPermissionChecker,
    logger: &mut impl ReviewWorkflowUsecaseLogger,
    actor_user_id: &UserId,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
) -> Result<bool, ReviewWorkflowUsecaseError> {
    let decision = permission_checker
        .check_document_permission(actor_user_id, workspace_id, document_id, permission)
        .map_err(ReviewWorkflowUsecaseError::from_permission_error)
        .map_err(|error| {
            log_error(
                logger,
                error,
                Some(actor_user_id),
                Some(document_id),
                None,
                None,
            )
        })?;
    let allowed = decision.result() == PermissionDecisionResult::Allowed;
    logger.write_field_debug(ReviewWorkflowFieldDebugEvent {
        from_state: None,
        to_state: None,
        transition_event: permission.as_str(),
        guard_result: if allowed { "allowed" } else { "denied" },
        permission_decision: if allowed { "allowed" } else { "denied" },
    });
    Ok(allowed)
}

impl ReviewRequestSummary {
    fn from_record(record: &ReviewRequestRecord) -> Self {
        Self {
            review_request_id: record.review_request_id().to_string(),
            document_id: record.request().document_id().as_str().to_string(),
            requested_by: record.request().requested_by().as_str().to_string(),
            status: record.status(),
        }
    }
}

fn parse_user_id(value: &str) -> Result<UserId, ReviewWorkflowUsecaseError> {
    UserId::new(value).map_err(|_| ReviewWorkflowUsecaseError::InvalidInput)
}

fn parse_workspace_id(value: &str) -> Result<WorkspaceId, ReviewWorkflowUsecaseError> {
    WorkspaceId::new(value).map_err(|_| ReviewWorkflowUsecaseError::InvalidInput)
}

fn parse_document_id(value: &str) -> Result<DocumentId, ReviewWorkflowUsecaseError> {
    DocumentId::new(value).map_err(|_| ReviewWorkflowUsecaseError::InvalidInput)
}

fn parse_review_request_id(value: &str) -> Result<String, ReviewWorkflowUsecaseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(ReviewWorkflowUsecaseError::InvalidInput);
    }
    Ok(trimmed.to_string())
}

fn log_error(
    logger: &mut impl ReviewWorkflowUsecaseLogger,
    error: ReviewWorkflowUsecaseError,
    actor_user_id: Option<&UserId>,
    document_id: Option<&DocumentId>,
    review_request_id: Option<&str>,
    from_state: Option<PublishWorkflowState>,
) -> ReviewWorkflowUsecaseError {
    logger.write_product(ReviewWorkflowProductEvent::WorkflowTransitionFailed {
        masked_actor_id: actor_user_id
            .map(mask_user_id)
            .unwrap_or_else(|| "masked:unknown".to_string()),
        document_id: document_id.map(|id| id.as_str().to_string()),
        review_request_id: review_request_id.map(str::to_string),
        from_state: from_state.map(workflow_state_name),
        error_code: error.code(),
    });
    error
}

fn workflow_state_name(state: PublishWorkflowState) -> &'static str {
    match state {
        PublishWorkflowState::Editing => "editing",
        PublishWorkflowState::ReviewRequested => "review_requested",
        PublishWorkflowState::ChangesRequested => "changes_requested",
        PublishWorkflowState::Approved => "approved",
        PublishWorkflowState::Published => "published",
        PublishWorkflowState::Rejected => "rejected",
    }
}

fn workflow_event_name(event: PublishWorkflowEvent) -> &'static str {
    match event {
        PublishWorkflowEvent::RequestReview => "request_review",
        PublishWorkflowEvent::Approve => "approve",
        PublishWorkflowEvent::Reject => "reject",
        PublishWorkflowEvent::RequestChanges => "request_changes",
        PublishWorkflowEvent::ResumeEditing => "resume_editing",
        PublishWorkflowEvent::Publish => "publish",
        PublishWorkflowEvent::EditPublishedDocument => "edit_published_document",
    }
}

fn mask_user_id(user_id: &UserId) -> String {
    let value = user_id.as_str();
    let suffix_start = value.len().saturating_sub(4);
    format!("masked:{}", &value[suffix_start..])
}
