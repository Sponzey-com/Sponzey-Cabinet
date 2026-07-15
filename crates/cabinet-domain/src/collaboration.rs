use crate::document::DocumentId;
use crate::user::UserId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationId(String);

impl OperationId {
    pub fn new(value: &str) -> Result<Self, CollaborationError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(CollaborationError::EmptyOperationId);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(CollaborationError::InvalidOperationId);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditSessionId(String);

impl EditSessionId {
    pub fn new(value: &str) -> Result<Self, CollaborationError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(CollaborationError::EmptySessionId);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(CollaborationError::InvalidSessionId);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BaseRevision(u64);

impl BaseRevision {
    pub fn new(value: u64) -> Result<Self, CollaborationError> {
        if value == 0 {
            return Err(CollaborationError::InvalidBaseRevision);
        }
        Ok(Self(value))
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperationSequence(u64);

impl OperationSequence {
    pub fn new(value: u64) -> Result<Self, CollaborationError> {
        if value == 0 {
            return Err(CollaborationError::InvalidOperationSequence);
        }
        Ok(Self(value))
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    start: usize,
    end: usize,
}

impl TextRange {
    pub fn new(start: usize, end: usize) -> Result<Self, CollaborationError> {
        if start > end {
            return Err(CollaborationError::InvalidTextRange);
        }
        Ok(Self { start, end })
    }

    pub const fn start(self) -> usize {
        self.start
    }

    pub const fn end(self) -> usize {
        self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentOperation {
    operation_id: OperationId,
    document_id: DocumentId,
    actor_user_id: UserId,
    base_revision: BaseRevision,
    range: TextRange,
    inserted_text: String,
}

impl DocumentOperation {
    pub fn replace_text(
        operation_id: OperationId,
        document_id: DocumentId,
        actor_user_id: UserId,
        base_revision: BaseRevision,
        range: TextRange,
        inserted_text: &str,
    ) -> Result<Self, CollaborationError> {
        if inserted_text.is_empty() {
            return Err(CollaborationError::EmptyOperationPatch);
        }
        Ok(Self {
            operation_id,
            document_id,
            actor_user_id,
            base_revision,
            range,
            inserted_text: inserted_text.to_string(),
        })
    }

    pub fn operation_id(&self) -> &OperationId {
        &self.operation_id
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn actor_user_id(&self) -> &UserId {
        &self.actor_user_id
    }

    pub const fn base_revision(&self) -> BaseRevision {
        self.base_revision
    }

    pub const fn range(&self) -> TextRange {
        self.range
    }

    pub fn inserted_text(&self) -> &str {
        &self.inserted_text
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presence {
    document_id: DocumentId,
    actor_user_id: UserId,
    cursor: TextRange,
}

impl Presence {
    pub fn new(
        document_id: DocumentId,
        actor_user_id: UserId,
        cursor: TextRange,
    ) -> Result<Self, CollaborationError> {
        Ok(Self {
            document_id,
            actor_user_id,
            cursor,
        })
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn actor_user_id(&self) -> &UserId {
        &self.actor_user_id
    }

    pub const fn cursor(&self) -> TextRange {
        self.cursor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditSession {
    session_id: EditSessionId,
    document_id: DocumentId,
    actor_user_id: UserId,
    state: EditSessionState,
}

impl EditSession {
    pub fn new(
        session_id: EditSessionId,
        document_id: DocumentId,
        actor_user_id: UserId,
        state: EditSessionState,
    ) -> Result<Self, CollaborationError> {
        Ok(Self {
            session_id,
            document_id,
            actor_user_id,
            state,
        })
    }

    pub fn session_id(&self) -> &EditSessionId {
        &self.session_id
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn actor_user_id(&self) -> &UserId {
        &self.actor_user_id
    }

    pub const fn state(&self) -> EditSessionState {
        self.state
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollaborationConflict {
    operation_id: OperationId,
    expected_revision: BaseRevision,
    actual_revision: BaseRevision,
}

impl CollaborationConflict {
    pub fn operation_id(&self) -> &OperationId {
        &self.operation_id
    }

    pub const fn expected_revision(&self) -> BaseRevision {
        self.expected_revision
    }

    pub const fn actual_revision(&self) -> BaseRevision {
        self.actual_revision
    }

    pub const fn reason_code(&self) -> &'static str {
        "collaboration.stale_base_revision"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditSessionState {
    Idle,
    SessionStarted,
    Editing,
    Syncing,
    Synced,
    ConflictDetected,
    Resolving,
    Failed,
    SessionEnded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditSessionEvent {
    StartSession,
    BeginEdit,
    RequestSync,
    SyncSucceeded,
    DetectConflict,
    BeginResolve,
    ResolveSucceeded,
    ResolveFailed,
    EndSession,
}

pub fn transition_edit_session_state(
    state: EditSessionState,
    event: EditSessionEvent,
) -> Result<EditSessionState, CollaborationError> {
    match (state, event) {
        (EditSessionState::Idle, EditSessionEvent::StartSession) => {
            Ok(EditSessionState::SessionStarted)
        }
        (EditSessionState::SessionStarted, EditSessionEvent::BeginEdit) => {
            Ok(EditSessionState::Editing)
        }
        (EditSessionState::Editing, EditSessionEvent::RequestSync) => Ok(EditSessionState::Syncing),
        (EditSessionState::Syncing, EditSessionEvent::SyncSucceeded) => {
            Ok(EditSessionState::Synced)
        }
        (EditSessionState::Syncing, EditSessionEvent::DetectConflict) => {
            Ok(EditSessionState::ConflictDetected)
        }
        (EditSessionState::ConflictDetected, EditSessionEvent::BeginResolve) => {
            Ok(EditSessionState::Resolving)
        }
        (EditSessionState::Resolving, EditSessionEvent::ResolveSucceeded) => {
            Ok(EditSessionState::Synced)
        }
        (EditSessionState::Resolving, EditSessionEvent::ResolveFailed) => {
            Ok(EditSessionState::Failed)
        }
        (EditSessionState::SessionStarted, EditSessionEvent::EndSession)
        | (EditSessionState::Editing, EditSessionEvent::EndSession)
        | (EditSessionState::Synced, EditSessionEvent::EndSession)
        | (EditSessionState::Failed, EditSessionEvent::EndSession) => {
            Ok(EditSessionState::SessionEnded)
        }
        _ => Err(CollaborationError::InvalidStateTransition),
    }
}

pub fn detect_collaboration_conflict(
    operation: &DocumentOperation,
    current_revision: BaseRevision,
) -> Option<CollaborationConflict> {
    if operation.base_revision() == current_revision {
        return None;
    }
    Some(CollaborationConflict {
        operation_id: operation.operation_id().clone(),
        expected_revision: operation.base_revision(),
        actual_revision: current_revision,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollaborationError {
    EmptyOperationId,
    InvalidOperationId,
    EmptySessionId,
    InvalidSessionId,
    InvalidBaseRevision,
    InvalidOperationSequence,
    InvalidTextRange,
    EmptyOperationPatch,
    InvalidStateTransition,
}

impl CollaborationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyOperationId => "collaboration.empty_operation_id",
            Self::InvalidOperationId => "collaboration.invalid_operation_id",
            Self::EmptySessionId => "collaboration.empty_session_id",
            Self::InvalidSessionId => "collaboration.invalid_session_id",
            Self::InvalidBaseRevision => "collaboration.invalid_base_revision",
            Self::InvalidOperationSequence => "collaboration.invalid_operation_sequence",
            Self::InvalidTextRange => "collaboration.invalid_text_range",
            Self::EmptyOperationPatch => "collaboration.empty_operation_patch",
            Self::InvalidStateTransition => "collaboration.invalid_state_transition",
        }
    }
}
