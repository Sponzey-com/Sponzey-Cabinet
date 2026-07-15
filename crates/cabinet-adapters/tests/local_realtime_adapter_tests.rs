use cabinet_adapters::local_realtime::{LocalDocumentRoomOwnerPolicy, LocalRealtimeTransport};
use cabinet_domain::collaboration::{
    BaseRevision, DocumentOperation, EditSessionId, OperationId, OperationSequence, Presence,
    TextRange,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::realtime::DocumentRoomId;
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::realtime::{
    DocumentRoomOwnerPolicy, JoinDocumentRoomRequest, OperationBroadcastRequest,
    PresenceBroadcastRequest, RealtimeTransport, RealtimeTransportError, ReplayLocalChangesRequest,
};

#[test]
fn local_room_owner_policy_uses_explicit_namespace_without_global_state() {
    let policy = LocalDocumentRoomOwnerPolicy::new("local-self-host").expect("policy");
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
    assert_eq!(
        policy.owner_key(&room).expect("owner").as_str(),
        "local-self-host:workspace-1:doc-1",
    );
}

#[test]
fn local_realtime_transport_records_join_operation_presence_and_replay_separately() {
    let room = room_id("workspace-1", "doc-1");
    let session_id = session_id("session-1");
    let actor_user_id = user_id("user-1");
    let mut transport = LocalRealtimeTransport::new();

    transport
        .join_document_room(
            JoinDocumentRoomRequest::new(room.clone(), session_id.clone(), actor_user_id)
                .expect("join request"),
        )
        .expect("join");
    transport
        .broadcast_operation(
            OperationBroadcastRequest::new(
                room.clone(),
                session_id.clone(),
                operation(&room, "op-1"),
            )
            .expect("operation request"),
        )
        .expect("operation");
    transport
        .broadcast_presence(
            PresenceBroadcastRequest::new(room.clone(), session_id.clone(), presence(&room))
                .expect("presence request"),
        )
        .expect("presence");
    transport
        .request_replay(
            ReplayLocalChangesRequest::new(
                room.clone(),
                session_id,
                Some(OperationSequence::new(1).expect("sequence")),
            )
            .expect("replay request"),
        )
        .expect("replay");

    assert_eq!(transport.joined_room_count(), 1);
    assert_eq!(transport.recorded_operations(&room).len(), 1);
    assert_eq!(transport.recorded_presences(&room).len(), 1);
    assert_eq!(transport.recorded_replay_requests(&room).len(), 1);
}

#[test]
fn local_realtime_transport_rejects_unjoined_room_broadcasts_with_stable_error() {
    let room = room_id("workspace-1", "doc-1");
    let session_id = session_id("session-1");
    let mut transport = LocalRealtimeTransport::new();

    assert_eq!(
        transport
            .broadcast_operation(
                OperationBroadcastRequest::new(
                    room.clone(),
                    session_id.clone(),
                    operation(&room, "op-1"),
                )
                .expect("operation request"),
            )
            .expect_err("unjoined room"),
        RealtimeTransportError::RoomNotJoined,
    );
    assert_eq!(
        transport
            .broadcast_presence(
                PresenceBroadcastRequest::new(room.clone(), session_id.clone(), presence(&room))
                    .expect("presence request"),
            )
            .expect_err("unjoined room"),
        RealtimeTransportError::RoomNotJoined,
    );
    assert_eq!(
        transport
            .request_replay(
                ReplayLocalChangesRequest::new(room, session_id, None).expect("replay request"),
            )
            .expect_err("unjoined room"),
        RealtimeTransportError::RoomNotJoined,
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

fn presence(room: &DocumentRoomId) -> Presence {
    Presence::new(
        room.document_id().clone(),
        user_id("user-1"),
        TextRange::new(3, 3).expect("cursor"),
    )
    .expect("presence")
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

fn session_id(value: &str) -> EditSessionId {
    EditSessionId::new(value).expect("session id")
}
