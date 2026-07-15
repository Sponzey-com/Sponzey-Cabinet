use cabinet_domain::document::DocumentId;
use cabinet_domain::realtime::{
    DocumentRoomId, RealtimeConnectionEvent, RealtimeConnectionState, RealtimeError, RoomOwnerKey,
    transition_realtime_connection_state,
};
use cabinet_domain::workspace::WorkspaceId;

#[test]
fn document_room_owner_key_is_stable_for_same_workspace_and_document() {
    let room = room_id("workspace-1", "doc-1");
    let same_room = room_id("workspace-1", "doc-1");
    let other_room = room_id("workspace-1", "doc-2");

    let owner_key = RoomOwnerKey::for_room("local-self-host", &room).expect("owner key");
    let same_owner_key = RoomOwnerKey::for_room("local-self-host", &same_room).expect("owner key");
    let other_owner_key =
        RoomOwnerKey::for_room("local-self-host", &other_room).expect("owner key");

    assert_eq!(owner_key, same_owner_key);
    assert_ne!(owner_key, other_owner_key);
    assert_eq!(owner_key.as_str(), "local-self-host:workspace-1:doc-1");
}

#[test]
fn realtime_connection_transition_supports_join_sync_conflict_and_replay_flow() {
    assert_eq!(
        transition_realtime_connection_state(
            RealtimeConnectionState::Disconnected,
            RealtimeConnectionEvent::ConnectRequested,
        )
        .expect("transition"),
        RealtimeConnectionState::Connecting,
    );
    assert_eq!(
        transition_realtime_connection_state(
            RealtimeConnectionState::Connecting,
            RealtimeConnectionEvent::ConnectionEstablished,
        )
        .expect("transition"),
        RealtimeConnectionState::Connected,
    );
    assert_eq!(
        transition_realtime_connection_state(
            RealtimeConnectionState::Connected,
            RealtimeConnectionEvent::JoinDocumentRequested,
        )
        .expect("transition"),
        RealtimeConnectionState::JoiningDocument,
    );
    assert_eq!(
        transition_realtime_connection_state(
            RealtimeConnectionState::JoiningDocument,
            RealtimeConnectionEvent::JoinSucceeded,
        )
        .expect("transition"),
        RealtimeConnectionState::Editing,
    );
    assert_eq!(
        transition_realtime_connection_state(
            RealtimeConnectionState::Editing,
            RealtimeConnectionEvent::SyncRequested,
        )
        .expect("transition"),
        RealtimeConnectionState::Syncing,
    );
    assert_eq!(
        transition_realtime_connection_state(
            RealtimeConnectionState::Syncing,
            RealtimeConnectionEvent::ConflictDetected,
        )
        .expect("transition"),
        RealtimeConnectionState::ConflictDetected,
    );
    assert_eq!(
        transition_realtime_connection_state(
            RealtimeConnectionState::Connected,
            RealtimeConnectionEvent::ConnectionInterrupted,
        )
        .expect("transition"),
        RealtimeConnectionState::Reconnecting,
    );
    assert_eq!(
        transition_realtime_connection_state(
            RealtimeConnectionState::Reconnecting,
            RealtimeConnectionEvent::ReconnectFailed,
        )
        .expect("transition"),
        RealtimeConnectionState::Offline,
    );
    assert_eq!(
        transition_realtime_connection_state(
            RealtimeConnectionState::Offline,
            RealtimeConnectionEvent::ReplayLocalChangesRequested,
        )
        .expect("transition"),
        RealtimeConnectionState::ReplayingLocalChanges,
    );
    assert_eq!(
        transition_realtime_connection_state(
            RealtimeConnectionState::ReplayingLocalChanges,
            RealtimeConnectionEvent::ReplaySucceeded,
        )
        .expect("transition"),
        RealtimeConnectionState::Synced,
    );
}

#[test]
fn realtime_connection_transition_rejects_invalid_sync_from_disconnected() {
    let error = transition_realtime_connection_state(
        RealtimeConnectionState::Disconnected,
        RealtimeConnectionEvent::SyncRequested,
    )
    .expect_err("invalid transition");

    assert_eq!(error, RealtimeError::InvalidStateTransition);
    assert_eq!(error.code(), "realtime.invalid_state_transition");
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
