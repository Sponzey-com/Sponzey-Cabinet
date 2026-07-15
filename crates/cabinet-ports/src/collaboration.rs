use cabinet_domain::collaboration::{
    DocumentOperation, EditSession, EditSessionId, OperationSequence, Presence,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;

pub trait CollaborationSessionStore {
    fn save_session(
        &mut self,
        workspace_id: &WorkspaceId,
        session: EditSession,
    ) -> Result<(), CollaborationSessionStoreError>;

    fn get_session(
        &self,
        workspace_id: &WorkspaceId,
        session_id: &EditSessionId,
    ) -> Result<Option<EditSession>, CollaborationSessionStoreError>;

    fn save_presence(
        &mut self,
        workspace_id: &WorkspaceId,
        presence: Presence,
    ) -> Result<(), CollaborationSessionStoreError>;

    fn list_presence(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Vec<Presence>, CollaborationSessionStoreError>;
}

pub trait CollaborationEventLog {
    fn append_operation(
        &mut self,
        workspace_id: &WorkspaceId,
        operation: DocumentOperation,
    ) -> Result<OperationSequence, CollaborationEventLogError>;

    fn list_operations(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Vec<CollaborationOperationEvent>, CollaborationEventLogError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollaborationOperationEvent {
    sequence: OperationSequence,
    operation: DocumentOperation,
}

impl CollaborationOperationEvent {
    pub fn new(
        sequence: OperationSequence,
        operation: DocumentOperation,
    ) -> Result<Self, CollaborationEventLogError> {
        Ok(Self {
            sequence,
            operation,
        })
    }

    pub const fn sequence(&self) -> OperationSequence {
        self.sequence
    }

    pub fn operation(&self) -> &DocumentOperation {
        &self.operation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollaborationSessionStoreError {
    InvalidInput,
    NotFound,
    Conflict,
    StorageUnavailable,
}

impl CollaborationSessionStoreError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "collaboration_session.invalid_input",
            Self::NotFound => "collaboration_session.not_found",
            Self::Conflict => "collaboration_session.conflict",
            Self::StorageUnavailable => "collaboration_session.storage_unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollaborationEventLogError {
    InvalidInput,
    Conflict,
    StorageUnavailable,
}

impl CollaborationEventLogError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "collaboration_event_log.invalid_input",
            Self::Conflict => "collaboration_event_log.conflict",
            Self::StorageUnavailable => "collaboration_event_log.storage_unavailable",
        }
    }
}
