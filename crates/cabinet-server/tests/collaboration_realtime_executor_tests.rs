use std::collections::HashMap;

use cabinet_domain::collaboration::{
    DocumentOperation, EditSession, EditSessionId, OperationSequence, Presence,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{
    AccessResource, Permission, PermissionDecision, PermissionDecisionReason, PolicySource,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::collaboration::{
    CollaborationEventLog, CollaborationEventLogError, CollaborationOperationEvent,
    CollaborationSessionStore, CollaborationSessionStoreError,
};
use cabinet_ports::permission_aware_query::{PermissionAwareQueryError, PermissionDecisionPort};
use cabinet_ports::realtime::{
    JoinDocumentRoomRequest, OperationBroadcastRequest, PresenceBroadcastRequest,
    RealtimeAcknowledgement, RealtimeTransport, RealtimeTransportError, ReplayLocalChangesRequest,
};
use cabinet_server::collaboration_realtime::{
    CollaborationRealtimeServerCommand, execute_realtime_command,
};

#[test]
fn executor_starts_session_before_transport_join() {
    let mut session_store = FakeCollaborationSessionStore::default();
    let mut event_log = FakeCollaborationEventLog::default();
    let permission = FakePermissionDecision { allow: true };
    let mut transport = FakeRealtimeTransport::default();

    let response = execute_realtime_command(
        CollaborationRealtimeServerCommand::JoinDocumentRoom {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            session_id: "session-1".to_string(),
            actor_user_id: "user-1".to_string(),
        },
        &mut session_store,
        &mut event_log,
        &permission,
        &mut transport,
    );

    assert_eq!(response.status_code(), 202);
    assert_eq!(session_store.sessions.len(), 1);
    assert_eq!(transport.joined_rooms.len(), 1);
}

#[test]
fn executor_rejects_permission_denied_join_without_transport_call() {
    let mut session_store = FakeCollaborationSessionStore::default();
    let mut event_log = FakeCollaborationEventLog::default();
    let permission = FakePermissionDecision { allow: false };
    let mut transport = FakeRealtimeTransport::default();

    let response = execute_realtime_command(
        CollaborationRealtimeServerCommand::JoinDocumentRoom {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            session_id: "session-1".to_string(),
            actor_user_id: "user-1".to_string(),
        },
        &mut session_store,
        &mut event_log,
        &permission,
        &mut transport,
    );

    assert_eq!(response.status_code(), 409);
    assert!(response.body().contains("collaboration.permission_denied"));
    assert_eq!(session_store.sessions.len(), 0);
    assert_eq!(transport.joined_rooms.len(), 0);
}

#[test]
fn executor_appends_operation_before_transport_broadcast() {
    let mut session_store = FakeCollaborationSessionStore::default();
    let mut event_log = FakeCollaborationEventLog::default();
    let permission = FakePermissionDecision { allow: true };
    let mut transport = FakeRealtimeTransport::default();

    let response = execute_realtime_command(
        operation_command(1, 1),
        &mut session_store,
        &mut event_log,
        &permission,
        &mut transport,
    );

    assert_eq!(response.status_code(), 202);
    assert_eq!(event_log.events.len(), 1);
    assert_eq!(transport.operations.len(), 1);
}

#[test]
fn executor_rejects_conflict_without_transport_broadcast() {
    let mut session_store = FakeCollaborationSessionStore::default();
    let mut event_log = FakeCollaborationEventLog::default();
    let permission = FakePermissionDecision { allow: true };
    let mut transport = FakeRealtimeTransport::default();

    let response = execute_realtime_command(
        operation_command(1, 2),
        &mut session_store,
        &mut event_log,
        &permission,
        &mut transport,
    );

    assert_eq!(response.status_code(), 409);
    assert!(response.body().contains("collaboration.conflict.detected"));
    assert_eq!(event_log.events.len(), 0);
    assert_eq!(transport.operations.len(), 0);
}

#[test]
fn executor_updates_presence_without_event_log_and_requests_replay_through_transport() {
    let mut session_store = FakeCollaborationSessionStore::default();
    let mut event_log = FakeCollaborationEventLog::default();
    let permission = FakePermissionDecision { allow: true };
    let mut transport = FakeRealtimeTransport::default();

    let presence_response = execute_realtime_command(
        CollaborationRealtimeServerCommand::BroadcastPresence {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            session_id: "session-1".to_string(),
            actor_user_id: "user-1".to_string(),
            cursor_start: 4,
            cursor_end: 4,
        },
        &mut session_store,
        &mut event_log,
        &permission,
        &mut transport,
    );
    let replay_response = execute_realtime_command(
        CollaborationRealtimeServerCommand::RequestReplay {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            session_id: "session-1".to_string(),
            last_acknowledged_sequence: Some(1),
        },
        &mut session_store,
        &mut event_log,
        &permission,
        &mut transport,
    );

    assert_eq!(presence_response.status_code(), 202);
    assert_eq!(replay_response.status_code(), 202);
    assert_eq!(session_store.presences.len(), 1);
    assert_eq!(event_log.events.len(), 0);
    assert_eq!(transport.presences.len(), 1);
    assert_eq!(transport.replay_requests.len(), 1);
    assert!(!presence_response.body().contains("selectedText"));
    assert!(!presence_response.body().contains("documentBody"));
    assert!(!presence_response.body().contains("token"));
}

fn operation_command(
    base_revision: u64,
    current_revision: u64,
) -> CollaborationRealtimeServerCommand {
    CollaborationRealtimeServerCommand::BroadcastOperation {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        session_id: "session-1".to_string(),
        actor_user_id: "user-1".to_string(),
        operation_id: "op-1".to_string(),
        base_revision,
        current_revision,
        start_offset: 0,
        end_offset: 1,
        inserted_text: "x".to_string(),
    }
}

#[derive(Default)]
struct FakeCollaborationSessionStore {
    sessions: HashMap<(String, String), EditSession>,
    presences: Vec<Presence>,
}

impl CollaborationSessionStore for FakeCollaborationSessionStore {
    fn save_session(
        &mut self,
        workspace_id: &WorkspaceId,
        session: EditSession,
    ) -> Result<(), CollaborationSessionStoreError> {
        self.sessions.insert(
            (
                workspace_id.as_str().to_string(),
                session.session_id().as_str().to_string(),
            ),
            session,
        );
        Ok(())
    }

    fn get_session(
        &self,
        workspace_id: &WorkspaceId,
        session_id: &EditSessionId,
    ) -> Result<Option<EditSession>, CollaborationSessionStoreError> {
        Ok(self
            .sessions
            .get(&(
                workspace_id.as_str().to_string(),
                session_id.as_str().to_string(),
            ))
            .cloned())
    }

    fn save_presence(
        &mut self,
        _workspace_id: &WorkspaceId,
        presence: Presence,
    ) -> Result<(), CollaborationSessionStoreError> {
        self.presences.push(presence);
        Ok(())
    }

    fn list_presence(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<Vec<Presence>, CollaborationSessionStoreError> {
        Ok(self.presences.clone())
    }
}

#[derive(Default)]
struct FakeCollaborationEventLog {
    events: Vec<CollaborationOperationEvent>,
}

impl CollaborationEventLog for FakeCollaborationEventLog {
    fn append_operation(
        &mut self,
        _workspace_id: &WorkspaceId,
        operation: DocumentOperation,
    ) -> Result<OperationSequence, CollaborationEventLogError> {
        let sequence = OperationSequence::new((self.events.len() + 1) as u64).expect("sequence");
        self.events
            .push(CollaborationOperationEvent::new(sequence, operation).expect("event"));
        Ok(sequence)
    }

    fn list_operations(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<Vec<CollaborationOperationEvent>, CollaborationEventLogError> {
        Ok(self.events.clone())
    }
}

struct FakePermissionDecision {
    allow: bool,
}

impl PermissionDecisionPort for FakePermissionDecision {
    fn check_permission(
        &self,
        _actor_user_id: &UserId,
        _resource: &AccessResource,
        _permission: Permission,
    ) -> Result<PermissionDecision, PermissionAwareQueryError> {
        Ok(if self.allow {
            PermissionDecision::allowed(
                PolicySource::Document,
                PermissionDecisionReason::RoleAllowsPermission,
            )
        } else {
            PermissionDecision::denied(
                PolicySource::Document,
                PermissionDecisionReason::RoleDoesNotAllowPermission,
            )
        })
    }
}

#[derive(Default)]
struct FakeRealtimeTransport {
    joined_rooms: Vec<JoinDocumentRoomRequest>,
    operations: Vec<OperationBroadcastRequest>,
    presences: Vec<PresenceBroadcastRequest>,
    replay_requests: Vec<ReplayLocalChangesRequest>,
}

impl RealtimeTransport for FakeRealtimeTransport {
    fn join_document_room(
        &mut self,
        request: JoinDocumentRoomRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError> {
        self.joined_rooms.push(request.clone());
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
