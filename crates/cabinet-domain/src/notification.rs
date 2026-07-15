use crate::document::DocumentId;
use crate::user::UserId;
use crate::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeNotification {
    workspace_id: WorkspaceId,
    actor_user_id: UserId,
    target: ChangeNotificationTarget,
    event_type: ChangeNotificationEventType,
    occurred_at: ChangeNotificationTimestamp,
    correlation_id: ChangeNotificationCorrelationId,
}

impl ChangeNotification {
    pub fn new(
        workspace_id: WorkspaceId,
        actor_user_id: UserId,
        target: ChangeNotificationTarget,
        event_type: ChangeNotificationEventType,
        occurred_at: ChangeNotificationTimestamp,
        correlation_id: ChangeNotificationCorrelationId,
    ) -> Self {
        Self {
            workspace_id,
            actor_user_id,
            target,
            event_type,
            occurred_at,
            correlation_id,
        }
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn actor_user_id(&self) -> &UserId {
        &self.actor_user_id
    }

    pub fn target(&self) -> &ChangeNotificationTarget {
        &self.target
    }

    pub const fn event_type(&self) -> ChangeNotificationEventType {
        self.event_type
    }

    pub const fn occurred_at(&self) -> ChangeNotificationTimestamp {
        self.occurred_at
    }

    pub fn correlation_id(&self) -> &ChangeNotificationCorrelationId {
        &self.correlation_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeNotificationTarget {
    Document {
        document_id: DocumentId,
    },
    CommentThread {
        document_id: DocumentId,
        thread_id: ChangeNotificationTargetId,
    },
    ReviewRequest {
        document_id: DocumentId,
        review_request_id: ChangeNotificationTargetId,
    },
    DocumentLock {
        document_id: DocumentId,
        lock_id: ChangeNotificationTargetId,
    },
}

impl ChangeNotificationTarget {
    pub fn document(document_id: DocumentId) -> Self {
        Self::Document { document_id }
    }

    pub fn comment_thread(document_id: DocumentId, thread_id: ChangeNotificationTargetId) -> Self {
        Self::CommentThread {
            document_id,
            thread_id,
        }
    }

    pub fn review_request(
        document_id: DocumentId,
        review_request_id: ChangeNotificationTargetId,
    ) -> Self {
        Self::ReviewRequest {
            document_id,
            review_request_id,
        }
    }

    pub fn document_lock(document_id: DocumentId, lock_id: ChangeNotificationTargetId) -> Self {
        Self::DocumentLock {
            document_id,
            lock_id,
        }
    }

    pub fn document_id(&self) -> &DocumentId {
        match self {
            Self::Document { document_id }
            | Self::CommentThread { document_id, .. }
            | Self::ReviewRequest { document_id, .. }
            | Self::DocumentLock { document_id, .. } => document_id,
        }
    }

    pub fn target_id(&self) -> &str {
        match self {
            Self::Document { document_id } => document_id.as_str(),
            Self::CommentThread { thread_id, .. } => thread_id.as_str(),
            Self::ReviewRequest {
                review_request_id, ..
            } => review_request_id.as_str(),
            Self::DocumentLock { lock_id, .. } => lock_id.as_str(),
        }
    }

    pub const fn target_kind(&self) -> &'static str {
        match self {
            Self::Document { .. } => "document",
            Self::CommentThread { .. } => "comment_thread",
            Self::ReviewRequest { .. } => "review_request",
            Self::DocumentLock { .. } => "document_lock",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeNotificationEventType {
    DocumentChanged,
    CommentChanged,
    ReviewStateChanged,
    LockStateChanged,
}

impl ChangeNotificationEventType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DocumentChanged => "document.changed",
            Self::CommentChanged => "comment.changed",
            Self::ReviewStateChanged => "review.state_changed",
            Self::LockStateChanged => "lock.state_changed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChangeNotificationTargetId {
    value: String,
}

impl ChangeNotificationTargetId {
    pub fn new(value: &str) -> Result<Self, ChangeNotificationError> {
        let value = validate_id(
            value,
            ChangeNotificationError::EmptyTargetId,
            ChangeNotificationError::InvalidTargetId,
        )?;
        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChangeNotificationCorrelationId {
    value: String,
}

impl ChangeNotificationCorrelationId {
    pub fn new(value: &str) -> Result<Self, ChangeNotificationError> {
        let value = validate_id(
            value,
            ChangeNotificationError::EmptyCorrelationId,
            ChangeNotificationError::InvalidCorrelationId,
        )?;
        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChangeNotificationTimestamp {
    millis_since_epoch: u64,
}

impl ChangeNotificationTimestamp {
    pub const fn from_millis(millis_since_epoch: u64) -> Self {
        Self { millis_since_epoch }
    }

    pub const fn as_millis(self) -> u64 {
        self.millis_since_epoch
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeNotificationError {
    EmptyTargetId,
    InvalidTargetId,
    EmptyCorrelationId,
    InvalidCorrelationId,
}

fn validate_id(
    value: &str,
    empty_error: ChangeNotificationError,
    invalid_error: ChangeNotificationError,
) -> Result<String, ChangeNotificationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(empty_error);
    }
    if trimmed.chars().any(char::is_control) {
        return Err(invalid_error);
    }
    Ok(trimmed.to_string())
}
