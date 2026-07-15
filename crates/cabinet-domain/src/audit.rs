use crate::document::DocumentId;
use crate::user::UserId;
use crate::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    event_id: AuditEventId,
    workspace_id: WorkspaceId,
    actor: AuditActor,
    action: AuditAction,
    target: AuditTarget,
    metadata: AuditMetadata,
    occurred_at: AuditTimestamp,
}

impl AuditEvent {
    pub fn new(
        event_id: AuditEventId,
        workspace_id: WorkspaceId,
        actor: AuditActor,
        action: AuditAction,
        target: AuditTarget,
        metadata: AuditMetadata,
        occurred_at: AuditTimestamp,
    ) -> Self {
        Self {
            event_id,
            workspace_id,
            actor,
            action,
            target,
            metadata,
            occurred_at,
        }
    }

    pub fn event_id(&self) -> &AuditEventId {
        &self.event_id
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn actor(&self) -> &AuditActor {
        &self.actor
    }

    pub const fn action(&self) -> AuditAction {
        self.action
    }

    pub fn target(&self) -> &AuditTarget {
        &self.target
    }

    pub fn metadata(&self) -> &AuditMetadata {
        &self.metadata
    }

    pub const fn occurred_at(&self) -> AuditTimestamp {
        self.occurred_at
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditActor {
    User { user_id: UserId },
}

impl AuditActor {
    pub fn user(user_id: UserId) -> Self {
        Self::User { user_id }
    }

    pub fn actor_id(&self) -> &str {
        match self {
            Self::User { user_id } => user_id.as_str(),
        }
    }

    pub const fn actor_type(&self) -> &'static str {
        match self {
            Self::User { .. } => "user",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditTarget {
    Workspace {
        workspace_id: WorkspaceId,
    },
    Document {
        document_id: DocumentId,
    },
    CommentThread {
        document_id: DocumentId,
        thread_id: AuditTargetId,
    },
    ReviewRequest {
        document_id: DocumentId,
        review_request_id: AuditTargetId,
    },
    DocumentLock {
        document_id: DocumentId,
        lock_id: AuditTargetId,
    },
    BackupJob {
        job_id: AuditTargetId,
    },
}

impl AuditTarget {
    pub fn workspace(workspace_id: WorkspaceId) -> Self {
        Self::Workspace { workspace_id }
    }

    pub fn document(document_id: DocumentId) -> Self {
        Self::Document { document_id }
    }

    pub fn comment_thread(document_id: DocumentId, thread_id: AuditTargetId) -> Self {
        Self::CommentThread {
            document_id,
            thread_id,
        }
    }

    pub fn review_request(document_id: DocumentId, review_request_id: AuditTargetId) -> Self {
        Self::ReviewRequest {
            document_id,
            review_request_id,
        }
    }

    pub fn document_lock(document_id: DocumentId, lock_id: AuditTargetId) -> Self {
        Self::DocumentLock {
            document_id,
            lock_id,
        }
    }

    pub fn backup_job(job_id: AuditTargetId) -> Self {
        Self::BackupJob { job_id }
    }

    pub const fn target_type(&self) -> &'static str {
        match self {
            Self::Workspace { .. } => "workspace",
            Self::Document { .. } => "document",
            Self::CommentThread { .. } => "comment_thread",
            Self::ReviewRequest { .. } => "review_request",
            Self::DocumentLock { .. } => "document_lock",
            Self::BackupJob { .. } => "backup_job",
        }
    }

    pub fn target_id(&self) -> &str {
        match self {
            Self::Workspace { workspace_id } => workspace_id.as_str(),
            Self::Document { document_id } => document_id.as_str(),
            Self::CommentThread { thread_id, .. } => thread_id.as_str(),
            Self::ReviewRequest {
                review_request_id, ..
            } => review_request_id.as_str(),
            Self::DocumentLock { lock_id, .. } => lock_id.as_str(),
            Self::BackupJob { job_id } => job_id.as_str(),
        }
    }

    pub fn document_id(&self) -> Option<&DocumentId> {
        match self {
            Self::Document { document_id }
            | Self::CommentThread { document_id, .. }
            | Self::ReviewRequest { document_id, .. }
            | Self::DocumentLock { document_id, .. } => Some(document_id),
            Self::Workspace { .. } | Self::BackupJob { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuditAction {
    PermissionDenied,
    ReviewRequested,
    ReviewApproved,
    ReviewRejected,
    DocumentPublished,
    LockAcquired,
    LockReleased,
    LockExpired,
    BackupCreated,
    RestoreCompleted,
}

impl AuditAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PermissionDenied => "permission.denied",
            Self::ReviewRequested => "review.requested",
            Self::ReviewApproved => "review.approved",
            Self::ReviewRejected => "review.rejected",
            Self::DocumentPublished => "document.published",
            Self::LockAcquired => "lock.acquired",
            Self::LockReleased => "lock.released",
            Self::LockExpired => "lock.expired",
            Self::BackupCreated => "backup.created",
            Self::RestoreCompleted => "restore.completed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditMetadata {
    entries: Vec<AuditMetadataEntry>,
}

impl AuditMetadata {
    pub fn new<const N: usize>(entries: [(&str, &str); N]) -> Result<Self, AuditError> {
        let mut sanitized = Vec::with_capacity(N);
        for (key, value) in entries {
            sanitized.push(AuditMetadataEntry::new(key, value)?);
        }
        Ok(Self { entries: sanitized })
    }

    pub fn from_pairs(entries: &[(String, String)]) -> Result<Self, AuditError> {
        let mut sanitized = Vec::with_capacity(entries.len());
        for (key, value) in entries {
            sanitized.push(AuditMetadataEntry::new(key, value)?);
        }
        Ok(Self { entries: sanitized })
    }

    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn entries(&self) -> &[AuditMetadataEntry] {
        &self.entries
    }

    pub fn value(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditMetadataEntry {
    key: String,
    value: String,
}

impl AuditMetadataEntry {
    pub fn new(key: &str, value: &str) -> Result<Self, AuditError> {
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() {
            return Err(AuditError::EmptyMetadataKey);
        }
        if value.is_empty() {
            return Err(AuditError::EmptyMetadataValue);
        }
        if key.chars().any(char::is_control) {
            return Err(AuditError::InvalidMetadataKey);
        }
        if value.chars().any(char::is_control) {
            return Err(AuditError::InvalidMetadataValue);
        }
        if contains_sensitive_fragment(key) {
            return Err(AuditError::SensitiveMetadataKey);
        }
        if contains_sensitive_fragment(value) {
            return Err(AuditError::SensitiveMetadataValue);
        }
        Ok(Self {
            key: key.to_string(),
            value: value.to_string(),
        })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AuditEventId {
    value: String,
}

impl AuditEventId {
    pub fn new(value: &str) -> Result<Self, AuditError> {
        let value = validate_id(value, AuditError::EmptyEventId, AuditError::InvalidEventId)?;
        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AuditTargetId {
    value: String,
}

impl AuditTargetId {
    pub fn new(value: &str) -> Result<Self, AuditError> {
        let value = validate_id(
            value,
            AuditError::EmptyTargetId,
            AuditError::InvalidTargetId,
        )?;
        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuditTimestamp {
    millis_since_epoch: u64,
}

impl AuditTimestamp {
    pub const fn from_millis(millis_since_epoch: u64) -> Self {
        Self { millis_since_epoch }
    }

    pub const fn as_millis(self) -> u64 {
        self.millis_since_epoch
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditError {
    EmptyEventId,
    InvalidEventId,
    EmptyTargetId,
    InvalidTargetId,
    EmptyMetadataKey,
    InvalidMetadataKey,
    EmptyMetadataValue,
    InvalidMetadataValue,
    SensitiveMetadataKey,
    SensitiveMetadataValue,
}

fn validate_id(
    value: &str,
    empty_error: AuditError,
    invalid_error: AuditError,
) -> Result<String, AuditError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(empty_error);
    }
    if value.chars().any(char::is_control) {
        return Err(invalid_error);
    }
    Ok(value.to_string())
}

fn contains_sensitive_fragment(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "password",
        "token",
        "secret",
        "credential",
        "document_body",
        "comment_body",
        "comment body",
        "body:",
        "asset bytes",
        "attachment content",
        "file bytes",
    ]
    .iter()
    .any(|fragment| lower.contains(fragment))
}
