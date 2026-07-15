use cabinet_domain::collaboration::{
    DocumentOperation, EditSessionId, OperationSequence, Presence,
};
use cabinet_domain::realtime::{DocumentRoomId, RealtimeError, RoomOwnerKey};
use cabinet_domain::user::UserId;

pub trait DocumentRoomOwnerPolicy {
    fn owner_key(&self, room_id: &DocumentRoomId) -> Result<RoomOwnerKey, RoomOwnerPolicyError>;
}

pub trait RealtimeTransport {
    fn join_document_room(
        &mut self,
        request: JoinDocumentRoomRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError>;

    fn broadcast_operation(
        &mut self,
        request: OperationBroadcastRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError>;

    fn broadcast_presence(
        &mut self,
        request: PresenceBroadcastRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError>;

    fn request_replay(
        &mut self,
        request: ReplayLocalChangesRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinDocumentRoomRequest {
    room_id: DocumentRoomId,
    session_id: EditSessionId,
    actor_user_id: UserId,
}

impl JoinDocumentRoomRequest {
    pub fn new(
        room_id: DocumentRoomId,
        session_id: EditSessionId,
        actor_user_id: UserId,
    ) -> Result<Self, RealtimeTransportError> {
        Ok(Self {
            room_id,
            session_id,
            actor_user_id,
        })
    }

    pub fn room_id(&self) -> &DocumentRoomId {
        &self.room_id
    }

    pub fn session_id(&self) -> &EditSessionId {
        &self.session_id
    }

    pub fn actor_user_id(&self) -> &UserId {
        &self.actor_user_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationBroadcastRequest {
    room_id: DocumentRoomId,
    session_id: EditSessionId,
    operation: DocumentOperation,
}

impl OperationBroadcastRequest {
    pub fn new(
        room_id: DocumentRoomId,
        session_id: EditSessionId,
        operation: DocumentOperation,
    ) -> Result<Self, RealtimeTransportError> {
        if operation.document_id() != room_id.document_id() {
            return Err(RealtimeTransportError::InvalidInput);
        }
        Ok(Self {
            room_id,
            session_id,
            operation,
        })
    }

    pub fn room_id(&self) -> &DocumentRoomId {
        &self.room_id
    }

    pub fn session_id(&self) -> &EditSessionId {
        &self.session_id
    }

    pub fn operation(&self) -> &DocumentOperation {
        &self.operation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceBroadcastRequest {
    room_id: DocumentRoomId,
    session_id: EditSessionId,
    presence: Presence,
}

impl PresenceBroadcastRequest {
    pub fn new(
        room_id: DocumentRoomId,
        session_id: EditSessionId,
        presence: Presence,
    ) -> Result<Self, RealtimeTransportError> {
        if presence.document_id() != room_id.document_id() {
            return Err(RealtimeTransportError::InvalidInput);
        }
        Ok(Self {
            room_id,
            session_id,
            presence,
        })
    }

    pub fn room_id(&self) -> &DocumentRoomId {
        &self.room_id
    }

    pub fn session_id(&self) -> &EditSessionId {
        &self.session_id
    }

    pub fn presence(&self) -> &Presence {
        &self.presence
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayLocalChangesRequest {
    room_id: DocumentRoomId,
    session_id: EditSessionId,
    last_acknowledged_sequence: Option<OperationSequence>,
}

impl ReplayLocalChangesRequest {
    pub fn new(
        room_id: DocumentRoomId,
        session_id: EditSessionId,
        last_acknowledged_sequence: Option<OperationSequence>,
    ) -> Result<Self, RealtimeTransportError> {
        Ok(Self {
            room_id,
            session_id,
            last_acknowledged_sequence,
        })
    }

    pub fn room_id(&self) -> &DocumentRoomId {
        &self.room_id
    }

    pub fn session_id(&self) -> &EditSessionId {
        &self.session_id
    }

    pub const fn last_acknowledged_sequence(&self) -> Option<OperationSequence> {
        self.last_acknowledged_sequence
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimeAcknowledgement {
    room_id: DocumentRoomId,
}

impl RealtimeAcknowledgement {
    pub const fn accepted(room_id: DocumentRoomId) -> Self {
        Self { room_id }
    }

    pub fn room_id(&self) -> &DocumentRoomId {
        &self.room_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomOwnerPolicyError {
    InvalidRoom,
    Unavailable,
}

impl RoomOwnerPolicyError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidRoom => "room_owner_policy.invalid_room",
            Self::Unavailable => "room_owner_policy.unavailable",
        }
    }
}

impl From<RealtimeError> for RoomOwnerPolicyError {
    fn from(_: RealtimeError) -> Self {
        Self::InvalidRoom
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimeTransportError {
    InvalidInput,
    ConnectionUnavailable,
    RoomNotJoined,
    PayloadRejected,
}

impl RealtimeTransportError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "realtime_transport.invalid_input",
            Self::ConnectionUnavailable => "realtime_transport.connection_unavailable",
            Self::RoomNotJoined => "realtime_transport.room_not_joined",
            Self::PayloadRejected => "realtime_transport.payload_rejected",
        }
    }
}
