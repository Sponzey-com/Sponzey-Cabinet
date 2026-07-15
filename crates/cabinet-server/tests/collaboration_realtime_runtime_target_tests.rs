use std::collections::HashMap;

use cabinet_core::server_config::ServerConfigInput;
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
use cabinet_server::adapter::{HttpMethod, ServerRequest, handle_request};
use cabinet_server::collaboration_realtime::CollaborationRealtimeRuntimeTarget;
use cabinet_server::composition::build_server_composition;

#[test]
fn realtime_runtime_target_executes_join_route_through_handle_request() {
    let composition = build_server_composition(default_config());
    let target = target_with_permission(true);

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/workspaces/workspace-1/documents/doc-1/collaboration/join",
            Some("{\"sessionId\":\"session-1\",\"actorUserId\":\"user-1\"}"),
        ),
    )
    .expect("join route");

    assert_eq!(response.status_code(), 202);
    assert_eq!(target.session_store().sessions.len(), 1);
    assert_eq!(target.transport().joined_rooms.len(), 1);
}

#[test]
fn realtime_runtime_target_executes_operation_route_through_handle_request() {
    let composition = build_server_composition(default_config());
    let target = target_with_permission(true);

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/workspaces/workspace-1/documents/doc-1/collaboration/operations",
            Some("{\"sessionId\":\"session-1\",\"actorUserId\":\"user-1\",\"operationId\":\"op-1\",\"baseRevision\":1,\"currentRevision\":1,\"startOffset\":0,\"endOffset\":1,\"insertedText\":\"x\"}"),
        ),
    )
    .expect("operation route");

    assert_eq!(response.status_code(), 202);
    assert_eq!(target.event_log().events.len(), 1);
    assert_eq!(target.transport().operations.len(), 1);
    assert!(!response.body().contains("insertedText"));
}

#[test]
fn realtime_runtime_target_returns_stable_rejected_response_for_invalid_body() {
    let composition = build_server_composition(default_config());
    let target = target_with_permission(true);

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/workspaces/workspace-1/documents/doc-1/collaboration/join",
            Some("{\"sessionId\":\"session-1\"}"),
        ),
    )
    .expect("join route");

    assert_eq!(response.status_code(), 409);
    assert!(
        response
            .body()
            .contains("collaboration_realtime.missing_field")
    );
    assert_eq!(target.session_store().sessions.len(), 0);
    assert_eq!(target.transport().joined_rooms.len(), 0);
}

fn target_with_permission(
    allow: bool,
) -> CollaborationRealtimeRuntimeTarget<
    FakeCollaborationSessionStore,
    FakeCollaborationEventLog,
    FakePermissionDecision,
    FakeRealtimeTransport,
> {
    CollaborationRealtimeRuntimeTarget::new(
        FakeCollaborationSessionStore::default(),
        FakeCollaborationEventLog::default(),
        FakePermissionDecision { allow },
        FakeRealtimeTransport::default(),
    )
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
        Ok(RealtimeAcknowledgement::accepted(request.room_id().clone()))
    }

    fn request_replay(
        &mut self,
        request: ReplayLocalChangesRequest,
    ) -> Result<RealtimeAcknowledgement, RealtimeTransportError> {
        Ok(RealtimeAcknowledgement::accepted(request.room_id().clone()))
    }
}

fn default_config() -> cabinet_core::server_config::ServerConfig {
    ServerConfigInput::local_dev_defaults()
        .validate()
        .expect("valid server config")
}
