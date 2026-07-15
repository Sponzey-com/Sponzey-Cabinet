use crate::document::DocumentId;
use crate::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentRoomId {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
}

impl DocumentRoomId {
    pub fn new(workspace_id: WorkspaceId, document_id: DocumentId) -> Result<Self, RealtimeError> {
        Ok(Self {
            workspace_id,
            document_id,
        })
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomOwnerKey(String);

impl RoomOwnerKey {
    pub fn for_room(namespace: &str, room_id: &DocumentRoomId) -> Result<Self, RealtimeError> {
        let namespace = namespace.trim();
        if namespace.is_empty() {
            return Err(RealtimeError::EmptyOwnerNamespace);
        }
        if namespace.chars().any(char::is_control) {
            return Err(RealtimeError::InvalidOwnerNamespace);
        }

        Ok(Self(format!(
            "{}:{}:{}",
            namespace,
            room_id.workspace_id().as_str(),
            room_id.document_id().as_str(),
        )))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimeConnectionState {
    Disconnected,
    Connecting,
    Connected,
    JoiningDocument,
    Editing,
    Syncing,
    Synced,
    ConflictDetected,
    Reconnecting,
    Offline,
    ReplayingLocalChanges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimeConnectionEvent {
    ConnectRequested,
    ConnectionEstablished,
    ConnectionFailed,
    JoinDocumentRequested,
    JoinSucceeded,
    JoinFailed,
    SyncRequested,
    SyncSucceeded,
    ConflictDetected,
    ConnectionInterrupted,
    ReconnectSucceeded,
    ReconnectFailed,
    ReplayLocalChangesRequested,
    ReplaySucceeded,
    ReplayFailed,
}

pub fn transition_realtime_connection_state(
    state: RealtimeConnectionState,
    event: RealtimeConnectionEvent,
) -> Result<RealtimeConnectionState, RealtimeError> {
    match (state, event) {
        (RealtimeConnectionState::Disconnected, RealtimeConnectionEvent::ConnectRequested) => {
            Ok(RealtimeConnectionState::Connecting)
        }
        (RealtimeConnectionState::Connecting, RealtimeConnectionEvent::ConnectionEstablished) => {
            Ok(RealtimeConnectionState::Connected)
        }
        (RealtimeConnectionState::Connecting, RealtimeConnectionEvent::ConnectionFailed) => {
            Ok(RealtimeConnectionState::Offline)
        }
        (RealtimeConnectionState::Connected, RealtimeConnectionEvent::JoinDocumentRequested) => {
            Ok(RealtimeConnectionState::JoiningDocument)
        }
        (RealtimeConnectionState::JoiningDocument, RealtimeConnectionEvent::JoinSucceeded) => {
            Ok(RealtimeConnectionState::Editing)
        }
        (RealtimeConnectionState::JoiningDocument, RealtimeConnectionEvent::JoinFailed) => {
            Ok(RealtimeConnectionState::Connected)
        }
        (RealtimeConnectionState::Editing, RealtimeConnectionEvent::SyncRequested)
        | (RealtimeConnectionState::Synced, RealtimeConnectionEvent::SyncRequested) => {
            Ok(RealtimeConnectionState::Syncing)
        }
        (RealtimeConnectionState::Syncing, RealtimeConnectionEvent::SyncSucceeded) => {
            Ok(RealtimeConnectionState::Synced)
        }
        (RealtimeConnectionState::Syncing, RealtimeConnectionEvent::ConflictDetected) => {
            Ok(RealtimeConnectionState::ConflictDetected)
        }
        (RealtimeConnectionState::Connected, RealtimeConnectionEvent::ConnectionInterrupted)
        | (
            RealtimeConnectionState::JoiningDocument,
            RealtimeConnectionEvent::ConnectionInterrupted,
        )
        | (RealtimeConnectionState::Editing, RealtimeConnectionEvent::ConnectionInterrupted)
        | (RealtimeConnectionState::Syncing, RealtimeConnectionEvent::ConnectionInterrupted)
        | (RealtimeConnectionState::Synced, RealtimeConnectionEvent::ConnectionInterrupted)
        | (
            RealtimeConnectionState::ConflictDetected,
            RealtimeConnectionEvent::ConnectionInterrupted,
        ) => Ok(RealtimeConnectionState::Reconnecting),
        (RealtimeConnectionState::Reconnecting, RealtimeConnectionEvent::ReconnectSucceeded) => {
            Ok(RealtimeConnectionState::Connected)
        }
        (RealtimeConnectionState::Reconnecting, RealtimeConnectionEvent::ReconnectFailed) => {
            Ok(RealtimeConnectionState::Offline)
        }
        (
            RealtimeConnectionState::Offline,
            RealtimeConnectionEvent::ReplayLocalChangesRequested,
        ) => Ok(RealtimeConnectionState::ReplayingLocalChanges),
        (
            RealtimeConnectionState::ReplayingLocalChanges,
            RealtimeConnectionEvent::ReplaySucceeded,
        ) => Ok(RealtimeConnectionState::Synced),
        (RealtimeConnectionState::ReplayingLocalChanges, RealtimeConnectionEvent::ReplayFailed) => {
            Ok(RealtimeConnectionState::Offline)
        }
        _ => Err(RealtimeError::InvalidStateTransition),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimeError {
    EmptyOwnerNamespace,
    InvalidOwnerNamespace,
    InvalidStateTransition,
}

impl RealtimeError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyOwnerNamespace => "realtime.empty_owner_namespace",
            Self::InvalidOwnerNamespace => "realtime.invalid_owner_namespace",
            Self::InvalidStateTransition => "realtime.invalid_state_transition",
        }
    }
}
