use cabinet_domain::comment::{
    Comment, CommentBody, CommentBodyPolicy, CommentError, CommentId, CommentThread,
    CommentThreadEvent, CommentThreadId, CommentThreadState, InlineAnchorResolutionStatus,
    InlineCommentAnchor, InlineCommentRange, resolve_inline_comment_anchor,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{AccessResource, Permission, PermissionDecisionResult};
use cabinet_domain::user::UserId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::comment_repository::{
    CommentPermissionCheckError, CommentPermissionChecker, CommentRepository,
    CommentRepositoryError, InlineAnchorDocumentLookup, InlineAnchorDocumentLookupError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddCommentInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: String,
    thread_id: String,
    comment_id: String,
    body: String,
}

impl AddCommentInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        document_id: &str,
        thread_id: &str,
        comment_id: &str,
        body: &str,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            thread_id: thread_id.to_string(),
            comment_id: comment_id.to_string(),
            body: body.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddInlineCommentInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: String,
    version_id: String,
    start_offset: usize,
    end_offset: usize,
    thread_id: String,
    comment_id: String,
    body: String,
}

impl AddInlineCommentInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        document_id: &str,
        version_id: &str,
        start_offset: usize,
        end_offset: usize,
        thread_id: &str,
        comment_id: &str,
        body: &str,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            version_id: version_id.to_string(),
            start_offset,
            end_offset,
            thread_id: thread_id.to_string(),
            comment_id: comment_id.to_string(),
            body: body.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveCommentInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: String,
    thread_id: String,
}

impl ResolveCommentInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        document_id: &str,
        thread_id: &str,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            thread_id: thread_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReopenCommentInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: String,
    thread_id: String,
}

impl ReopenCommentInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        document_id: &str,
        thread_id: &str,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            thread_id: thread_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListDocumentCommentsInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: String,
}

impl ListDocumentCommentsInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, document_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentThreadOutput {
    thread: CommentThread,
}

impl CommentThreadOutput {
    pub fn thread(&self) -> &CommentThread {
        &self.thread
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListDocumentCommentsOutput {
    threads: Vec<CommentThread>,
}

impl ListDocumentCommentsOutput {
    pub fn threads(&self) -> &[CommentThread] {
        &self.threads
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentProductEvent {
    CommentAdded {
        masked_actor_id: String,
        document_id: String,
        thread_id: String,
        comment_count: usize,
    },
    CommentResolved {
        masked_actor_id: String,
        document_id: String,
        thread_id: String,
        comment_count: usize,
    },
    CommentReopened {
        masked_actor_id: String,
        document_id: String,
        thread_id: String,
        comment_count: usize,
    },
    InlineCommentAdded {
        masked_actor_id: String,
        document_id: String,
        thread_id: String,
        version_id_hash: String,
        anchor_status: &'static str,
        comment_count: usize,
    },
    InlineCommentAnchorStale {
        masked_actor_id: String,
        document_id: String,
        version_id_hash: String,
        anchor_status: &'static str,
    },
    UsecaseFailed {
        error_code: &'static str,
    },
}

impl CommentProductEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::CommentAdded { .. } => "comment.added",
            Self::CommentResolved { .. } => "comment.resolved",
            Self::CommentReopened { .. } => "comment.reopened",
            Self::InlineCommentAdded { .. } => "inline_comment.added",
            Self::InlineCommentAnchorStale { .. } => "inline_comment.anchor_stale",
            Self::UsecaseFailed { .. } => "comment.usecase.failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentFieldDebugEvent {
    thread_state: &'static str,
    transition_event: &'static str,
    permission_summary: &'static str,
    has_anchor: bool,
    anchor_status: Option<&'static str>,
    range_length: Option<usize>,
    version_relation: Option<&'static str>,
    version_id_hash: Option<String>,
}

impl CommentFieldDebugEvent {
    pub const fn thread_state(&self) -> &'static str {
        self.thread_state
    }

    pub const fn transition_event(&self) -> &'static str {
        self.transition_event
    }

    pub const fn permission_summary(&self) -> &'static str {
        self.permission_summary
    }

    pub const fn has_anchor(&self) -> bool {
        self.has_anchor
    }

    pub const fn anchor_status(&self) -> &'static str {
        match self.anchor_status {
            Some(value) => value,
            None => "none",
        }
    }

    pub const fn range_length(&self) -> Option<usize> {
        self.range_length
    }

    pub const fn version_relation(&self) -> Option<&'static str> {
        self.version_relation
    }

    pub fn version_id_hash(&self) -> Option<&str> {
        self.version_id_hash.as_deref()
    }
}

pub trait CommentUsecaseLogger {
    fn write_product(&mut self, event: CommentProductEvent);
    fn write_field_debug(&mut self, event: CommentFieldDebugEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddCommentUsecase {
    body_policy: CommentBodyPolicy,
}

impl AddCommentUsecase {
    pub const fn new(body_policy: CommentBodyPolicy) -> Self {
        Self { body_policy }
    }

    pub fn with_body_limit(max_body_bytes: usize) -> Result<Self, CommentUsecaseError> {
        CommentBodyPolicy::new(max_body_bytes)
            .map(Self::new)
            .map_err(|_| CommentUsecaseError::InvalidInput)
    }

    pub fn execute(
        &self,
        input: AddCommentInput,
        permission_checker: &impl CommentPermissionChecker,
        repository: &mut impl CommentRepository,
        logger: &mut impl CommentUsecaseLogger,
    ) -> Result<CommentThreadOutput, CommentUsecaseError> {
        let command = AddCommentCommand::from_input(input, self.body_policy)
            .map_err(|error| log_comment_error(logger, error))?;
        ensure_document_permission(
            permission_checker,
            logger,
            &command.actor_user_id,
            &command.workspace_id,
            &command.document_id,
            Permission::Write,
        )?;

        let existing = repository
            .get_thread(&command.workspace_id, &command.thread_id)
            .map_err(CommentUsecaseError::from_repository_error)
            .map_err(|error| log_comment_error(logger, error))?;

        let thread = match existing {
            Some(thread) if thread.document_id() == &command.document_id => {
                thread
                    .add_comment(command.comment.clone())
                    .map_err(CommentUsecaseError::from_comment_error)
                    .map_err(|error| log_comment_error(logger, error))?;
                repository
                    .append_comment(&command.workspace_id, &command.thread_id, command.comment)
                    .map_err(CommentUsecaseError::from_repository_error)
                    .map_err(|error| log_comment_error(logger, error))?
                    .ok_or_else(|| log_comment_error(logger, CommentUsecaseError::ThreadNotFound))?
            }
            Some(_) => {
                return Err(log_comment_error(
                    logger,
                    CommentUsecaseError::ThreadNotFound,
                ));
            }
            None => {
                let thread = CommentThread::new(
                    command.thread_id.clone(),
                    command.workspace_id.clone(),
                    command.document_id.clone(),
                    command.comment,
                );
                repository
                    .save_thread(&command.workspace_id, thread.clone())
                    .map_err(CommentUsecaseError::from_repository_error)
                    .map_err(|error| log_comment_error(logger, error))?;
                thread
            }
        };

        write_comment_field_debug(
            logger,
            thread.state(),
            CommentThreadEvent::AddComment,
            "allowed",
        );
        logger.write_product(CommentProductEvent::CommentAdded {
            masked_actor_id: mask_user_id(&command.actor_user_id),
            document_id: command.document_id.as_str().to_string(),
            thread_id: command.thread_id.as_str().to_string(),
            comment_count: thread.comments().len(),
        });
        Ok(CommentThreadOutput { thread })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddInlineCommentUsecase {
    body_policy: CommentBodyPolicy,
}

impl AddInlineCommentUsecase {
    pub const fn new(body_policy: CommentBodyPolicy) -> Self {
        Self { body_policy }
    }

    pub fn with_body_limit(max_body_bytes: usize) -> Result<Self, CommentUsecaseError> {
        CommentBodyPolicy::new(max_body_bytes)
            .map(Self::new)
            .map_err(|_| CommentUsecaseError::InvalidInput)
    }

    pub fn execute(
        &self,
        input: AddInlineCommentInput,
        permission_checker: &impl CommentPermissionChecker,
        document_lookup: &impl InlineAnchorDocumentLookup,
        repository: &mut impl CommentRepository,
        logger: &mut impl CommentUsecaseLogger,
    ) -> Result<CommentThreadOutput, CommentUsecaseError> {
        let command = AddInlineCommentCommand::from_input(input, self.body_policy)
            .map_err(|error| log_comment_error(logger, error))?;
        ensure_document_permission(
            permission_checker,
            logger,
            &command.actor_user_id,
            &command.workspace_id,
            &command.document_id,
            Permission::Write,
        )?;

        let version_state = document_lookup
            .get_anchor_document_state(
                &command.workspace_id,
                &command.document_id,
                command.anchor.version_id(),
            )
            .map_err(CommentUsecaseError::from_anchor_lookup_error)
            .map_err(|error| log_comment_error(logger, error))?;
        let resolution = match version_state {
            Some(state) => resolve_inline_comment_anchor(
                &command.anchor,
                true,
                Some(state.current_version_id()),
                state.anchored_document_len(),
            ),
            None => resolve_inline_comment_anchor(&command.anchor, false, None, 0),
        };

        write_anchor_field_debug(
            logger,
            &command.anchor,
            resolution.status(),
            resolution.range_length(),
            resolution.version_relation().as_str(),
        );

        match resolution.status() {
            InlineAnchorResolutionStatus::Valid => {}
            InlineAnchorResolutionStatus::Stale => {
                log_anchor_resolution_failure(logger, &command, resolution.status());
                return Err(CommentUsecaseError::StaleAnchor);
            }
            InlineAnchorResolutionStatus::InvalidRange => {
                log_anchor_resolution_failure(logger, &command, resolution.status());
                return Err(CommentUsecaseError::InvalidAnchorRange);
            }
            InlineAnchorResolutionStatus::DocumentVersionMissing => {
                log_anchor_resolution_failure(logger, &command, resolution.status());
                return Err(CommentUsecaseError::DocumentVersionMissing);
            }
        }

        let thread = CommentThread::new_inline(
            command.thread_id.clone(),
            command.workspace_id.clone(),
            command.document_id.clone(),
            command.anchor.clone(),
            command.comment,
        );
        repository
            .save_thread(&command.workspace_id, thread.clone())
            .map_err(CommentUsecaseError::from_repository_error)
            .map_err(|error| log_comment_error(logger, error))?;

        logger.write_product(CommentProductEvent::InlineCommentAdded {
            masked_actor_id: mask_user_id(&command.actor_user_id),
            document_id: command.document_id.as_str().to_string(),
            thread_id: command.thread_id.as_str().to_string(),
            version_id_hash: version_id_hash(command.anchor.version_id()),
            anchor_status: resolution.status().as_str(),
            comment_count: thread.comments().len(),
        });
        Ok(CommentThreadOutput { thread })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolveCommentUsecase;

impl ResolveCommentUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ResolveCommentInput,
        permission_checker: &impl CommentPermissionChecker,
        repository: &mut impl CommentRepository,
        logger: &mut impl CommentUsecaseLogger,
    ) -> Result<CommentThreadOutput, CommentUsecaseError> {
        transition_thread(
            input.actor_user_id,
            input.workspace_id,
            input.document_id,
            input.thread_id,
            CommentThreadEvent::Resolve,
            permission_checker,
            repository,
            logger,
        )
    }
}

impl Default for ResolveCommentUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReopenCommentUsecase;

impl ReopenCommentUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ReopenCommentInput,
        permission_checker: &impl CommentPermissionChecker,
        repository: &mut impl CommentRepository,
        logger: &mut impl CommentUsecaseLogger,
    ) -> Result<CommentThreadOutput, CommentUsecaseError> {
        transition_thread(
            input.actor_user_id,
            input.workspace_id,
            input.document_id,
            input.thread_id,
            CommentThreadEvent::Reopen,
            permission_checker,
            repository,
            logger,
        )
    }
}

impl Default for ReopenCommentUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListDocumentCommentsUsecase;

impl ListDocumentCommentsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ListDocumentCommentsInput,
        permission_checker: &impl CommentPermissionChecker,
        repository: &impl CommentRepository,
        logger: &mut impl CommentUsecaseLogger,
    ) -> Result<ListDocumentCommentsOutput, CommentUsecaseError> {
        let actor_user_id = parse_user_id(&input.actor_user_id)
            .map_err(|error| log_comment_error(logger, error))?;
        let workspace_id = parse_workspace_id(&input.workspace_id)
            .map_err(|error| log_comment_error(logger, error))?;
        let document_id = parse_document_id(&input.document_id)
            .map_err(|error| log_comment_error(logger, error))?;
        ensure_document_permission(
            permission_checker,
            logger,
            &actor_user_id,
            &workspace_id,
            &document_id,
            Permission::Read,
        )?;
        let threads = repository
            .list_document_threads(&workspace_id, &document_id)
            .map_err(CommentUsecaseError::from_repository_error)
            .map_err(|error| log_comment_error(logger, error))?;

        logger.write_field_debug(CommentFieldDebugEvent {
            thread_state: "mixed",
            transition_event: "list",
            permission_summary: "allowed",
            has_anchor: false,
            anchor_status: None,
            range_length: None,
            version_relation: None,
            version_id_hash: None,
        });
        Ok(ListDocumentCommentsOutput { threads })
    }
}

impl Default for ListDocumentCommentsUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentUsecaseError {
    InvalidInput,
    Unauthorized,
    ThreadNotFound,
    InvalidTransition,
    BodyTooLarge,
    InvalidAnchorRange,
    StaleAnchor,
    DocumentVersionMissing,
    StorageUnavailable,
    Conflict,
}

impl CommentUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "COMMENT_INVALID_INPUT",
            Self::Unauthorized => "COMMENT_UNAUTHORIZED",
            Self::ThreadNotFound => "COMMENT_THREAD_NOT_FOUND",
            Self::InvalidTransition => "COMMENT_INVALID_TRANSITION",
            Self::BodyTooLarge => "COMMENT_BODY_TOO_LARGE",
            Self::InvalidAnchorRange => "COMMENT_INVALID_ANCHOR_RANGE",
            Self::StaleAnchor => "COMMENT_STALE_ANCHOR",
            Self::DocumentVersionMissing => "COMMENT_DOCUMENT_VERSION_MISSING",
            Self::StorageUnavailable => "COMMENT_STORAGE_UNAVAILABLE",
            Self::Conflict => "COMMENT_CONFLICT",
        }
    }

    const fn from_comment_error(error: CommentError) -> Self {
        match error {
            CommentError::BodyTooLarge { .. } => Self::BodyTooLarge,
            CommentError::InvalidThreadTransition { .. } => Self::InvalidTransition,
            CommentError::InvalidInlineAnchorRange => Self::InvalidAnchorRange,
            CommentError::EmptyCommentId
            | CommentError::InvalidCommentId
            | CommentError::EmptyCommentThreadId
            | CommentError::InvalidCommentThreadId
            | CommentError::EmptyBody
            | CommentError::InvalidBodyPolicy => Self::InvalidInput,
        }
    }

    const fn from_repository_error(error: CommentRepositoryError) -> Self {
        match error {
            CommentRepositoryError::StorageUnavailable | CommentRepositoryError::CorruptedState => {
                Self::StorageUnavailable
            }
            CommentRepositoryError::Conflict => Self::Conflict,
        }
    }

    const fn from_permission_error(error: CommentPermissionCheckError) -> Self {
        match error {
            CommentPermissionCheckError::StorageUnavailable => Self::StorageUnavailable,
        }
    }

    const fn from_anchor_lookup_error(error: InlineAnchorDocumentLookupError) -> Self {
        match error {
            InlineAnchorDocumentLookupError::StorageUnavailable
            | InlineAnchorDocumentLookupError::CorruptedVersion => Self::StorageUnavailable,
        }
    }
}

struct AddCommentCommand {
    actor_user_id: UserId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    thread_id: CommentThreadId,
    comment: Comment,
}

impl AddCommentCommand {
    fn from_input(
        input: AddCommentInput,
        body_policy: CommentBodyPolicy,
    ) -> Result<Self, CommentUsecaseError> {
        let actor_user_id = parse_user_id(&input.actor_user_id)?;
        let workspace_id = parse_workspace_id(&input.workspace_id)?;
        let document_id = parse_document_id(&input.document_id)?;
        let thread_id = CommentThreadId::new(&input.thread_id)
            .map_err(CommentUsecaseError::from_comment_error)?;
        let comment_id =
            CommentId::new(&input.comment_id).map_err(CommentUsecaseError::from_comment_error)?;
        let body = CommentBody::new(&input.body, body_policy)
            .map_err(CommentUsecaseError::from_comment_error)?;
        let comment = Comment::new(comment_id, actor_user_id.clone(), body);

        Ok(Self {
            actor_user_id,
            workspace_id,
            document_id,
            thread_id,
            comment,
        })
    }
}

struct AddInlineCommentCommand {
    actor_user_id: UserId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    thread_id: CommentThreadId,
    anchor: InlineCommentAnchor,
    comment: Comment,
}

impl AddInlineCommentCommand {
    fn from_input(
        input: AddInlineCommentInput,
        body_policy: CommentBodyPolicy,
    ) -> Result<Self, CommentUsecaseError> {
        let actor_user_id = parse_user_id(&input.actor_user_id)?;
        let workspace_id = parse_workspace_id(&input.workspace_id)?;
        let document_id = parse_document_id(&input.document_id)?;
        let version_id = parse_version_id(&input.version_id)?;
        let range = InlineCommentRange::new(input.start_offset, input.end_offset)
            .map_err(CommentUsecaseError::from_comment_error)?;
        let anchor = InlineCommentAnchor::new(document_id.clone(), version_id, range);
        let thread_id = CommentThreadId::new(&input.thread_id)
            .map_err(CommentUsecaseError::from_comment_error)?;
        let comment_id =
            CommentId::new(&input.comment_id).map_err(CommentUsecaseError::from_comment_error)?;
        let body = CommentBody::new(&input.body, body_policy)
            .map_err(CommentUsecaseError::from_comment_error)?;
        let comment = Comment::new(comment_id, actor_user_id.clone(), body);

        Ok(Self {
            actor_user_id,
            workspace_id,
            document_id,
            thread_id,
            anchor,
            comment,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn transition_thread(
    actor_user_id: String,
    workspace_id: String,
    document_id: String,
    thread_id: String,
    event: CommentThreadEvent,
    permission_checker: &impl CommentPermissionChecker,
    repository: &mut impl CommentRepository,
    logger: &mut impl CommentUsecaseLogger,
) -> Result<CommentThreadOutput, CommentUsecaseError> {
    let actor_user_id =
        parse_user_id(&actor_user_id).map_err(|error| log_comment_error(logger, error))?;
    let workspace_id =
        parse_workspace_id(&workspace_id).map_err(|error| log_comment_error(logger, error))?;
    let document_id =
        parse_document_id(&document_id).map_err(|error| log_comment_error(logger, error))?;
    let thread_id = CommentThreadId::new(&thread_id)
        .map_err(CommentUsecaseError::from_comment_error)
        .map_err(|error| log_comment_error(logger, error))?;
    ensure_document_permission(
        permission_checker,
        logger,
        &actor_user_id,
        &workspace_id,
        &document_id,
        Permission::Write,
    )?;

    let thread = repository
        .get_thread(&workspace_id, &thread_id)
        .map_err(CommentUsecaseError::from_repository_error)
        .map_err(|error| log_comment_error(logger, error))?
        .ok_or_else(|| log_comment_error(logger, CommentUsecaseError::ThreadNotFound))?;
    if thread.document_id() != &document_id {
        return Err(log_comment_error(
            logger,
            CommentUsecaseError::ThreadNotFound,
        ));
    }

    let next_thread = thread
        .transition(event)
        .map_err(CommentUsecaseError::from_comment_error)
        .map_err(|error| log_comment_error(logger, error))?;
    let saved = repository
        .update_thread_state(&workspace_id, &thread_id, next_thread.state())
        .map_err(CommentUsecaseError::from_repository_error)
        .map_err(|error| log_comment_error(logger, error))?
        .ok_or_else(|| log_comment_error(logger, CommentUsecaseError::ThreadNotFound))?;

    write_comment_field_debug(logger, saved.state(), event, "allowed");
    match event {
        CommentThreadEvent::Resolve => logger.write_product(CommentProductEvent::CommentResolved {
            masked_actor_id: mask_user_id(&actor_user_id),
            document_id: document_id.as_str().to_string(),
            thread_id: thread_id.as_str().to_string(),
            comment_count: saved.comments().len(),
        }),
        CommentThreadEvent::Reopen => logger.write_product(CommentProductEvent::CommentReopened {
            masked_actor_id: mask_user_id(&actor_user_id),
            document_id: document_id.as_str().to_string(),
            thread_id: thread_id.as_str().to_string(),
            comment_count: saved.comments().len(),
        }),
        CommentThreadEvent::AddComment => {}
    }

    Ok(CommentThreadOutput { thread: saved })
}

fn ensure_document_permission(
    permission_checker: &impl CommentPermissionChecker,
    logger: &mut impl CommentUsecaseLogger,
    actor_user_id: &UserId,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
    permission: Permission,
) -> Result<(), CommentUsecaseError> {
    let resource = AccessResource::document(workspace_id.clone(), None, document_id.clone());
    let decision = permission_checker
        .check_permission(actor_user_id, &resource, permission)
        .map_err(CommentUsecaseError::from_permission_error)
        .map_err(|error| log_comment_error(logger, error))?;
    let summary = if decision.result() == PermissionDecisionResult::Allowed {
        "allowed"
    } else {
        "denied"
    };
    logger.write_field_debug(CommentFieldDebugEvent {
        thread_state: "permission_checked",
        transition_event: permission.as_str(),
        permission_summary: summary,
        has_anchor: false,
        anchor_status: None,
        range_length: None,
        version_relation: None,
        version_id_hash: None,
    });
    if decision.result() != PermissionDecisionResult::Allowed {
        return Err(log_comment_error(logger, CommentUsecaseError::Unauthorized));
    }
    Ok(())
}

fn write_comment_field_debug(
    logger: &mut impl CommentUsecaseLogger,
    state: CommentThreadState,
    event: CommentThreadEvent,
    permission_summary: &'static str,
) {
    logger.write_field_debug(CommentFieldDebugEvent {
        thread_state: state.as_str(),
        transition_event: event.as_str(),
        permission_summary,
        has_anchor: false,
        anchor_status: None,
        range_length: None,
        version_relation: None,
        version_id_hash: None,
    });
}

fn write_anchor_field_debug(
    logger: &mut impl CommentUsecaseLogger,
    anchor: &InlineCommentAnchor,
    status: InlineAnchorResolutionStatus,
    range_length: usize,
    version_relation: &'static str,
) {
    logger.write_field_debug(CommentFieldDebugEvent {
        thread_state: "anchor_resolved",
        transition_event: "add_inline_comment",
        permission_summary: "allowed",
        has_anchor: true,
        anchor_status: Some(status.as_str()),
        range_length: Some(range_length),
        version_relation: Some(version_relation),
        version_id_hash: Some(version_id_hash(anchor.version_id())),
    });
}

fn parse_user_id(value: &str) -> Result<UserId, CommentUsecaseError> {
    UserId::new(value).map_err(|_| CommentUsecaseError::InvalidInput)
}

fn parse_workspace_id(value: &str) -> Result<WorkspaceId, CommentUsecaseError> {
    WorkspaceId::new(value).map_err(|_| CommentUsecaseError::InvalidInput)
}

fn parse_document_id(value: &str) -> Result<DocumentId, CommentUsecaseError> {
    DocumentId::new(value).map_err(|_| CommentUsecaseError::InvalidInput)
}

fn parse_version_id(value: &str) -> Result<VersionId, CommentUsecaseError> {
    VersionId::new(value).map_err(|_| CommentUsecaseError::InvalidInput)
}

fn log_anchor_resolution_failure(
    logger: &mut impl CommentUsecaseLogger,
    command: &AddInlineCommentCommand,
    status: InlineAnchorResolutionStatus,
) {
    logger.write_product(CommentProductEvent::InlineCommentAnchorStale {
        masked_actor_id: mask_user_id(&command.actor_user_id),
        document_id: command.document_id.as_str().to_string(),
        version_id_hash: version_id_hash(command.anchor.version_id()),
        anchor_status: status.as_str(),
    });
}

fn log_comment_error(
    logger: &mut impl CommentUsecaseLogger,
    error: CommentUsecaseError,
) -> CommentUsecaseError {
    logger.write_product(CommentProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
    error
}

fn mask_user_id(user_id: &UserId) -> String {
    mask_raw_id(user_id.as_str())
}

fn mask_raw_id(value: &str) -> String {
    let suffix_start = value.len().saturating_sub(4);
    format!("masked:{}", &value[suffix_start..])
}

fn version_id_hash(version_id: &VersionId) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in version_id.as_str().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("vhash:{hash:016x}")
}
