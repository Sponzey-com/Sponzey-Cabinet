use std::collections::BTreeMap;

use cabinet_server::adapter::UsecaseInputDto;
use cabinet_server::collaboration_realtime::{
    CollaborationRealtimeCommandError, CollaborationRealtimeServerCommand,
    accepted_realtime_response, command_from_input, rejected_realtime_response,
};

#[test]
fn command_mapper_maps_join_operation_and_replay_without_framework_request() {
    let join = command_from_input(&input(
        "collaboration.join_document_room",
        Some("{\"sessionId\":\"session-1\",\"actorUserId\":\"user-1\"}"),
    ))
    .expect("join command");
    let operation = command_from_input(&input(
        "collaboration.broadcast_operation",
        Some(
            "{\"sessionId\":\"session-1\",\"actorUserId\":\"user-1\",\"operationId\":\"op-1\",\"baseRevision\":3,\"currentRevision\":3,\"startOffset\":4,\"endOffset\":7,\"insertedText\":\"next\"}",
        ),
    ))
    .expect("operation command");
    let replay = command_from_input(&input(
        "collaboration.request_replay",
        Some("{\"sessionId\":\"session-1\",\"lastAcknowledgedSequence\":2}"),
    ))
    .expect("replay command");

    assert_eq!(
        join,
        CollaborationRealtimeServerCommand::JoinDocumentRoom {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            session_id: "session-1".to_string(),
            actor_user_id: "user-1".to_string(),
        },
    );
    assert_eq!(
        operation,
        CollaborationRealtimeServerCommand::BroadcastOperation {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            session_id: "session-1".to_string(),
            actor_user_id: "user-1".to_string(),
            operation_id: "op-1".to_string(),
            base_revision: 3,
            current_revision: 3,
            start_offset: 4,
            end_offset: 7,
            inserted_text: "next".to_string(),
        },
    );
    assert_eq!(
        replay,
        CollaborationRealtimeServerCommand::RequestReplay {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            session_id: "session-1".to_string(),
            last_acknowledged_sequence: Some(2),
        },
    );
}

#[test]
fn command_mapper_strips_sensitive_presence_body_fields() {
    let command = command_from_input(&input(
        "collaboration.broadcast_presence",
        Some(
            "{\"sessionId\":\"session-1\",\"actorUserId\":\"user-1\",\"cursorStart\":5,\"cursorEnd\":8,\"selectedText\":\"secret selection\",\"documentBody\":\"secret body\",\"token\":\"secret token\"}",
        ),
    ))
    .expect("presence command");

    assert_eq!(
        command,
        CollaborationRealtimeServerCommand::BroadcastPresence {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            session_id: "session-1".to_string(),
            actor_user_id: "user-1".to_string(),
            cursor_start: 5,
            cursor_end: 8,
        },
    );
}

#[test]
fn command_mapper_returns_stable_error_for_missing_body_field_and_unknown_route() {
    let missing = command_from_input(&input(
        "collaboration.join_document_room",
        Some("{\"sessionId\":\"session-1\"}"),
    ))
    .expect_err("missing actor");
    let unsupported = command_from_input(&UsecaseInputDto::new("health.check", None))
        .expect_err("unsupported route");

    assert_eq!(missing, CollaborationRealtimeCommandError::MissingField);
    assert_eq!(missing.code(), "collaboration_realtime.missing_field");
    assert_eq!(
        unsupported,
        CollaborationRealtimeCommandError::UnsupportedRoute,
    );
    assert_eq!(
        unsupported.code(),
        "collaboration_realtime.unsupported_route",
    );
}

#[test]
fn acknowledgement_response_mapper_excludes_raw_document_and_operation_text() {
    let accepted = accepted_realtime_response("workspace-1", "doc-1");
    let rejected =
        rejected_realtime_response("workspace-1", "doc-1", "realtime_transport.room_not_joined");

    assert_eq!(accepted.status_code(), 202);
    assert_eq!(
        accepted.body(),
        "{\"status\":\"accepted\",\"workspaceId\":\"workspace-1\",\"documentId\":\"doc-1\"}",
    );
    assert_eq!(rejected.status_code(), 409);
    assert_eq!(
        rejected.body(),
        "{\"status\":\"rejected\",\"workspaceId\":\"workspace-1\",\"documentId\":\"doc-1\",\"errorCode\":\"realtime_transport.room_not_joined\"}",
    );
    assert!(!accepted.body().contains("secret"));
    assert!(!rejected.body().contains("secret"));
    assert!(!rejected.body().contains("insertedText"));
    assert!(!rejected.body().contains("documentBody"));
}

fn input(route_id: &str, body: Option<&str>) -> UsecaseInputDto {
    UsecaseInputDto::new_with_path_params(route_id, body, path_params())
}

fn path_params() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("workspaceId".to_string(), "workspace-1".to_string()),
        ("documentId".to_string(), "doc-1".to_string()),
    ])
}
