use cabinet_domain::collaboration::{
    BaseRevision, DocumentOperation, EditSessionId, OperationId, OperationSequence, Presence,
    TextRange,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::realtime::{DocumentRoomId, RoomOwnerKey};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::realtime::{
    DocumentRoomOwnerPolicy, JoinDocumentRoomRequest, OperationBroadcastRequest,
    PresenceBroadcastRequest, RealtimeAcknowledgement, RealtimeTransport, RealtimeTransportError,
    ReplayLocalChangesRequest, RoomOwnerPolicyError,
};

struct NamespaceOwnerPolicy {
    namespace: String,
}

impl DocumentRoomOwnerPolicy for NamespaceOwnerPolicy {
    fn owner_key(&self, room_id: &DocumentRoomId) -> Result<RoomOwnerKey, RoomOwnerPolicyError> {
        RoomOwnerKey::for_room(&self.namespace, room_id)
            .map_err(|_| RoomOwnerPolicyError::InvalidRoom)
    }
}

#[derive(Default)]
struct FakeRealtimeTransport {
    joined_rooms: Vec<DocumentRoomId>,
    operations: Vec<OperationBroadcastRequest>,
    presences: Vec<PresenceBroadcastRequest>,
    replay_requests: Vec<ReplayLocalChangesRequest>,
}

impl RealtimeTransport for FakeRealtimeTransport {
    fn join_document_room(
        &mut self,
        request: JoinDocumentRoomRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError> {
        self.joined_rooms.push(request.room_id().clone());
        Ok(RealtimeAcknowledgement::accepted(request.room_id().clone()))
    }

    fn broadcast_operation(
        &mut self,
        request: OperationBroadcastRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError> {
        self.operations.push(request.clone());
        Ok(RealtimeAcknowledgement::accepted(request.room_id().clone()))
    }

    fn broadcast_presence(
        &mut self,
        request: PresenceBroadcastRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError> {
        self.presences.push(request.clone());
        Ok(RealtimeAcknowledgement::accepted(request.room_id().clone()))
    }

    fn request_replay(
        &mut self,
        request: ReplayLocalChangesRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError> {
        self.replay_requests.push(request.clone());
        Ok(RealtimeAcknowledgement::accepted(request.room_id().clone()))
    }
}

#[test]
fn room_owner_policy_maps_same_room_to_same_owner_without_global_singleton() {
    let policy = NamespaceOwnerPolicy {
        namespace: "local-self-host".to_string(),
    };
    let room = room_id("workspace-1", "doc-1");
    let same_room = room_id("workspace-1", "doc-1");
    let other_room = room_id("workspace-1", "doc-2");

    assert_eq!(
        policy.owner_key(&room).expect("owner"),
        policy.owner_key(&same_room).expect("owner"),
    );
    assert_ne!(
        policy.owner_key(&room).expect("owner"),
        policy.owner_key(&other_room).expect("owner"),
    );
}

#[test]
fn realtime_transport_contract_keeps_join_operation_presence_and_replay_separate() {
    let room = room_id("workspace-1", "doc-1");
    let session_id = EditSessionId::new("session-1").expect("session");
    let actor_user_id = user_id("user-1");
    let operation = operation(&room, "op-1");
    let presence = Presence::new(
        room.document_id().clone(),
        actor_user_id.clone(),
        TextRange::new(3, 3).expect("cursor"),
    )
    .expect("presence");
    let mut transport = FakeRealtimeTransport::default();

    let join_ack = transport
        .join_document_room(
            JoinDocumentRoomRequest::new(room.clone(), session_id.clone(), actor_user_id)
                .expect("join request"),
        )
        .expect("join");
    let operation_ack = transport
        .broadcast_operation(
            OperationBroadcastRequest::new(room.clone(), session_id.clone(), operation)
                .expect("operation request"),
        )
        .expect("operation");
    let presence_ack = transport
        .broadcast_presence(
            PresenceBroadcastRequest::new(room.clone(), session_id.clone(), presence)
                .expect("presence request"),
        )
        .expect("presence");
    let replay_ack = transport
        .request_replay(
            ReplayLocalChangesRequest::new(
                room.clone(),
                session_id,
                Some(OperationSequence::new(1).expect("sequence")),
            )
            .expect("replay request"),
        )
        .expect("replay");

    assert_eq!(join_ack.room_id(), &room);
    assert_eq!(operation_ack.room_id(), &room);
    assert_eq!(presence_ack.room_id(), &room);
    assert_eq!(replay_ack.room_id(), &room);
    assert_eq!(transport.joined_rooms.len(), 1);
    assert_eq!(transport.operations.len(), 1);
    assert_eq!(transport.presences.len(), 1);
    assert_eq!(transport.replay_requests.len(), 1);
}

#[test]
fn realtime_transport_error_codes_are_stable() {
    assert_eq!(
        RealtimeTransportError::ConnectionUnavailable.code(),
        "realtime_transport.connection_unavailable",
    );
    assert_eq!(
        RealtimeTransportError::RoomNotJoined.code(),
        "realtime_transport.room_not_joined",
    );
    assert_eq!(
        RoomOwnerPolicyError::InvalidRoom.code(),
        "room_owner_policy.invalid_room",
    );
}

fn operation(room: &DocumentRoomId, operation_id: &str) -> DocumentOperation {
    DocumentOperation::replace_text(
        OperationId::new(operation_id).expect("operation id"),
        room.document_id().clone(),
        user_id("user-1"),
        BaseRevision::new(1).expect("revision"),
        TextRange::new(0, 1).expect("range"),
        "x",
    )
    .expect("operation")
}

fn room_id(workspace: &str, document: &str) -> DocumentRoomId {
    DocumentRoomId::new(workspace_id(workspace), document_id(document)).expect("room id")
}

fn workspace_id(value: &str) -> WorkspaceId {
    WorkspaceId::new(value).expect("workspace id")
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}

fn user_id(value: &str) -> UserId {
    UserId::new(value).expect("user id")
}
