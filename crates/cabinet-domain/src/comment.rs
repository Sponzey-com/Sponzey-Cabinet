use crate::document::DocumentId;
use crate::user::UserId;
use crate::version::VersionId;
use crate::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    id: CommentId,
    author_user_id: UserId,
    body: CommentBody,
}

impl Comment {
    pub fn new(id: CommentId, author_user_id: UserId, body: CommentBody) -> Self {
        Self {
            id,
            author_user_id,
            body,
        }
    }

    pub fn id(&self) -> &CommentId {
        &self.id
    }

    pub fn author_user_id(&self) -> &UserId {
        &self.author_user_id
    }

    pub fn body(&self) -> &CommentBody {
        &self.body
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentThread {
    id: CommentThreadId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    inline_anchor: Option<InlineCommentAnchor>,
    state: CommentThreadState,
    comments: Vec<Comment>,
}

impl CommentThread {
    pub fn new(
        id: CommentThreadId,
        workspace_id: WorkspaceId,
        document_id: DocumentId,
        first_comment: Comment,
    ) -> Self {
        Self {
            id,
            workspace_id,
            document_id,
            inline_anchor: None,
            state: CommentThreadState::Open,
            comments: vec![first_comment],
        }
    }

    pub fn new_inline(
        id: CommentThreadId,
        workspace_id: WorkspaceId,
        document_id: DocumentId,
        inline_anchor: InlineCommentAnchor,
        first_comment: Comment,
    ) -> Self {
        Self {
            id,
            workspace_id,
            document_id,
            inline_anchor: Some(inline_anchor),
            state: CommentThreadState::Open,
            comments: vec![first_comment],
        }
    }

    pub fn id(&self) -> &CommentThreadId {
        &self.id
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub const fn state(&self) -> CommentThreadState {
        self.state
    }

    pub fn inline_anchor(&self) -> Option<&InlineCommentAnchor> {
        self.inline_anchor.as_ref()
    }

    pub fn comments(&self) -> &[Comment] {
        &self.comments
    }

    pub fn add_comment(&self, comment: Comment) -> Result<Self, CommentError> {
        transition_comment_thread(self.state, CommentThreadEvent::AddComment)?;
        let mut next = self.clone();
        next.comments.push(comment);
        Ok(next)
    }

    pub fn transition(&self, event: CommentThreadEvent) -> Result<Self, CommentError> {
        let transition = transition_comment_thread(self.state, event)?;
        Ok(self.with_state(transition.next_state))
    }

    pub fn with_state(&self, state: CommentThreadState) -> Self {
        let mut next = self.clone();
        next.state = state;
        next
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentId {
    value: String,
}

impl CommentId {
    pub fn new(value: &str) -> Result<Self, CommentError> {
        let value = validate_identity(
            value,
            CommentError::EmptyCommentId,
            CommentError::InvalidCommentId,
        )?;
        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentThreadId {
    value: String,
}

impl CommentThreadId {
    pub fn new(value: &str) -> Result<Self, CommentError> {
        let value = validate_identity(
            value,
            CommentError::EmptyCommentThreadId,
            CommentError::InvalidCommentThreadId,
        )?;
        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentBody {
    value: String,
}

impl CommentBody {
    pub fn new(value: &str, policy: CommentBodyPolicy) -> Result<Self, CommentError> {
        let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
        let trimmed = normalized.trim();
        if trimmed.is_empty() {
            return Err(CommentError::EmptyBody);
        }
        if trimmed.len() > policy.max_bytes {
            return Err(CommentError::BodyTooLarge {
                max_bytes: policy.max_bytes,
            });
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommentBodyPolicy {
    max_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineCommentAnchor {
    document_id: DocumentId,
    version_id: VersionId,
    range: InlineCommentRange,
}

impl InlineCommentAnchor {
    pub fn new(document_id: DocumentId, version_id: VersionId, range: InlineCommentRange) -> Self {
        Self {
            document_id,
            version_id,
            range,
        }
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn version_id(&self) -> &VersionId {
        &self.version_id
    }

    pub const fn range(&self) -> InlineCommentRange {
        self.range
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineCommentRange {
    start_offset: usize,
    end_offset: usize,
}

impl InlineCommentRange {
    pub fn new(start_offset: usize, end_offset: usize) -> Result<Self, CommentError> {
        if start_offset >= end_offset {
            return Err(CommentError::InvalidInlineAnchorRange);
        }
        Ok(Self {
            start_offset,
            end_offset,
        })
    }

    pub const fn start_offset(self) -> usize {
        self.start_offset
    }

    pub const fn end_offset(self) -> usize {
        self.end_offset
    }

    pub const fn len(self) -> usize {
        self.end_offset - self.start_offset
    }

    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    const fn fits_document_len(self, document_len: usize) -> bool {
        self.end_offset <= document_len
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineAnchorResolution {
    status: InlineAnchorResolutionStatus,
    range_length: usize,
    version_relation: InlineCommentVersionRelation,
}

impl InlineAnchorResolution {
    pub const fn status(self) -> InlineAnchorResolutionStatus {
        self.status
    }

    pub const fn range_length(self) -> usize {
        self.range_length
    }

    pub const fn version_relation(self) -> InlineCommentVersionRelation {
        self.version_relation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineAnchorResolutionStatus {
    Valid,
    Stale,
    InvalidRange,
    DocumentVersionMissing,
}

impl InlineAnchorResolutionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Stale => "stale",
            Self::InvalidRange => "invalid_range",
            Self::DocumentVersionMissing => "document_version_missing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineCommentVersionRelation {
    Current,
    Stale,
    Missing,
}

impl InlineCommentVersionRelation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Stale => "stale",
            Self::Missing => "missing",
        }
    }
}

pub fn resolve_inline_comment_anchor(
    anchor: &InlineCommentAnchor,
    anchored_version_exists: bool,
    current_version_id: Option<&VersionId>,
    anchored_document_len: usize,
) -> InlineAnchorResolution {
    if !anchored_version_exists {
        return InlineAnchorResolution {
            status: InlineAnchorResolutionStatus::DocumentVersionMissing,
            range_length: anchor.range.len(),
            version_relation: InlineCommentVersionRelation::Missing,
        };
    }

    let version_relation = if current_version_id == Some(anchor.version_id()) {
        InlineCommentVersionRelation::Current
    } else {
        InlineCommentVersionRelation::Stale
    };

    if !anchor.range.fits_document_len(anchored_document_len) {
        return InlineAnchorResolution {
            status: InlineAnchorResolutionStatus::InvalidRange,
            range_length: anchor.range.len(),
            version_relation,
        };
    }

    let status = match version_relation {
        InlineCommentVersionRelation::Current => InlineAnchorResolutionStatus::Valid,
        InlineCommentVersionRelation::Stale | InlineCommentVersionRelation::Missing => {
            InlineAnchorResolutionStatus::Stale
        }
    };

    InlineAnchorResolution {
        status,
        range_length: anchor.range.len(),
        version_relation,
    }
}

impl CommentBodyPolicy {
    pub fn new(max_bytes: usize) -> Result<Self, CommentError> {
        if max_bytes == 0 {
            return Err(CommentError::InvalidBodyPolicy);
        }
        Ok(Self { max_bytes })
    }

    pub const fn max_bytes(self) -> usize {
        self.max_bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentThreadState {
    Open,
    Resolved,
    Reopened,
}

impl CommentThreadState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Resolved => "resolved",
            Self::Reopened => "reopened",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentThreadEvent {
    AddComment,
    Resolve,
    Reopen,
}

impl CommentThreadEvent {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AddComment => "add_comment",
            Self::Resolve => "resolve",
            Self::Reopen => "reopen",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommentThreadTransition {
    pub previous_state: CommentThreadState,
    pub event: CommentThreadEvent,
    pub next_state: CommentThreadState,
}

pub fn transition_comment_thread(
    state: CommentThreadState,
    event: CommentThreadEvent,
) -> Result<CommentThreadTransition, CommentError> {
    let next_state = match (state, event) {
        (CommentThreadState::Open, CommentThreadEvent::AddComment) => CommentThreadState::Open,
        (CommentThreadState::Reopened, CommentThreadEvent::AddComment) => {
            CommentThreadState::Reopened
        }
        (CommentThreadState::Open | CommentThreadState::Reopened, CommentThreadEvent::Resolve) => {
            CommentThreadState::Resolved
        }
        (CommentThreadState::Resolved, CommentThreadEvent::Reopen) => CommentThreadState::Reopened,
        _ => {
            return Err(CommentError::InvalidThreadTransition { state, event });
        }
    };

    Ok(CommentThreadTransition {
        previous_state: state,
        event,
        next_state,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentError {
    EmptyCommentId,
    InvalidCommentId,
    EmptyCommentThreadId,
    InvalidCommentThreadId,
    EmptyBody,
    BodyTooLarge {
        max_bytes: usize,
    },
    InvalidBodyPolicy,
    InvalidInlineAnchorRange,
    InvalidThreadTransition {
        state: CommentThreadState,
        event: CommentThreadEvent,
    },
}

impl CommentError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EmptyCommentId => "comment.empty_comment_id",
            Self::InvalidCommentId => "comment.invalid_comment_id",
            Self::EmptyCommentThreadId => "comment.empty_thread_id",
            Self::InvalidCommentThreadId => "comment.invalid_thread_id",
            Self::EmptyBody => "comment.empty_body",
            Self::BodyTooLarge { .. } => "comment.body_too_large",
            Self::InvalidBodyPolicy => "comment.invalid_body_policy",
            Self::InvalidInlineAnchorRange => "comment.invalid_inline_anchor_range",
            Self::InvalidThreadTransition { .. } => "comment_thread.invalid_transition",
        }
    }
}

fn validate_identity(
    value: &str,
    empty_error: CommentError,
    invalid_error: CommentError,
) -> Result<String, CommentError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(empty_error);
    }
    if trimmed.chars().any(char::is_control) {
        return Err(invalid_error);
    }
    Ok(trimmed.to_string())
}
