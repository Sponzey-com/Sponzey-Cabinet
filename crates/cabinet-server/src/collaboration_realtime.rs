use std::cell::{Ref, RefCell};

use crate::adapter::{ServerUsecaseTarget, UsecaseInputDto, UsecaseOutputDto};
use crate::errors::ServerBoundaryError;
use cabinet_domain::collaboration::{
    BaseRevision, DocumentOperation, EditSessionId, OperationId, OperationSequence, Presence,
    TextRange,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::collaboration::{CollaborationEventLog, CollaborationSessionStore};
use cabinet_ports::permission_aware_query::PermissionDecisionPort;
use cabinet_ports::realtime::{
    JoinDocumentRoomRequest, OperationBroadcastRequest, PresenceBroadcastRequest,
    RealtimeTransport, ReplayLocalChangesRequest,
};
use cabinet_usecases::collaboration::{
    ApplyCollaborativeEditInput, ApplyCollaborativeEditStatus, ApplyCollaborativeEditUsecase,
    StartEditSessionInput, StartEditSessionUsecase, UpdatePresenceInput, UpdatePresenceUsecase,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollaborationRealtimeServerCommand {
    JoinDocumentRoom {
        workspace_id: String,
        document_id: String,
        session_id: String,
        actor_user_id: String,
    },
    BroadcastOperation {
        workspace_id: String,
        document_id: String,
        session_id: String,
        actor_user_id: String,
        operation_id: String,
        base_revision: u64,
        current_revision: u64,
        start_offset: usize,
        end_offset: usize,
        inserted_text: String,
    },
    BroadcastPresence {
        workspace_id: String,
        document_id: String,
        session_id: String,
        actor_user_id: String,
        cursor_start: usize,
        cursor_end: usize,
    },
    RequestReplay {
        workspace_id: String,
        document_id: String,
        session_id: String,
        last_acknowledged_sequence: Option<u64>,
    },
}

pub fn command_from_input(
    input: &UsecaseInputDto,
) -> Result<CollaborationRealtimeServerCommand, CollaborationRealtimeCommandError> {
    if !matches!(
        input.route_id(),
        "collaboration.join_document_room"
            | "collaboration.broadcast_operation"
            | "collaboration.broadcast_presence"
            | "collaboration.request_replay"
    ) {
        return Err(CollaborationRealtimeCommandError::UnsupportedRoute);
    }

    let workspace_id = path_param(input, "workspaceId")?;
    let document_id = path_param(input, "documentId")?;
    let body = RealtimeBodyFields::parse(input.body());

    match input.route_id() {
        "collaboration.join_document_room" => {
            Ok(CollaborationRealtimeServerCommand::JoinDocumentRoom {
                workspace_id,
                document_id,
                session_id: body.string("sessionId")?,
                actor_user_id: body.string("actorUserId")?,
            })
        }
        "collaboration.broadcast_operation" => {
            Ok(CollaborationRealtimeServerCommand::BroadcastOperation {
                workspace_id,
                document_id,
                session_id: body.string("sessionId")?,
                actor_user_id: body.string("actorUserId")?,
                operation_id: body.string("operationId")?,
                base_revision: body.u64("baseRevision")?,
                current_revision: body.u64("currentRevision")?,
                start_offset: body.usize("startOffset")?,
                end_offset: body.usize("endOffset")?,
                inserted_text: body.string("insertedText")?,
            })
        }
        "collaboration.broadcast_presence" => {
            Ok(CollaborationRealtimeServerCommand::BroadcastPresence {
                workspace_id,
                document_id,
                session_id: body.string("sessionId")?,
                actor_user_id: body.string("actorUserId")?,
                cursor_start: body.usize("cursorStart")?,
                cursor_end: body.usize("cursorEnd")?,
            })
        }
        "collaboration.request_replay" => Ok(CollaborationRealtimeServerCommand::RequestReplay {
            workspace_id,
            document_id,
            session_id: body.string("sessionId")?,
            last_acknowledged_sequence: body.optional_u64("lastAcknowledgedSequence"),
        }),
        _ => Err(CollaborationRealtimeCommandError::UnsupportedRoute),
    }
}

pub fn accepted_realtime_response(workspace_id: &str, document_id: &str) -> UsecaseOutputDto {
    UsecaseOutputDto::new(
        202,
        &format!(
            "{{\"status\":\"accepted\",\"workspaceId\":\"{}\",\"documentId\":\"{}\"}}",
            workspace_id, document_id
        ),
    )
}

pub fn rejected_realtime_response(
    workspace_id: &str,
    document_id: &str,
    error_code: &str,
) -> UsecaseOutputDto {
    UsecaseOutputDto::new(
        409,
        &format!(
            "{{\"status\":\"rejected\",\"workspaceId\":\"{}\",\"documentId\":\"{}\",\"errorCode\":\"{}\"}}",
            workspace_id, document_id, error_code
        ),
    )
}

pub fn execute_realtime_command<
    S: CollaborationSessionStore,
    E: CollaborationEventLog,
    P: PermissionDecisionPort,
    T: RealtimeTransport,
>(
    command: CollaborationRealtimeServerCommand,
    session_store: &mut S,
    event_log: &mut E,
    permission_decision: &P,
    transport: &mut T,
) -> UsecaseOutputDto {
    match command {
        CollaborationRealtimeServerCommand::JoinDocumentRoom {
            workspace_id,
            document_id,
            session_id,
            actor_user_id,
        } => {
            let usecase = StartEditSessionUsecase::new();
            if let Err(error) = usecase.execute(
                StartEditSessionInput::new(
                    &workspace_id,
                    &document_id,
                    &actor_user_id,
                    &session_id,
                ),
                session_store,
                permission_decision,
            ) {
                return rejected_realtime_response(&workspace_id, &document_id, error.code());
            }
            match join_request(&workspace_id, &document_id, &session_id, &actor_user_id).and_then(
                |request| {
                    transport
                        .join_document_room(request)
                        .map_err(|error| error.code().to_string())
                },
            ) {
                Ok(_) => accepted_realtime_response(&workspace_id, &document_id),
                Err(error_code) => {
                    rejected_realtime_response(&workspace_id, &document_id, &error_code)
                }
            }
        }
        CollaborationRealtimeServerCommand::BroadcastOperation {
            workspace_id,
            document_id,
            session_id,
            actor_user_id,
            operation_id,
            base_revision,
            current_revision,
            start_offset,
            end_offset,
            inserted_text,
        } => {
            let usecase = ApplyCollaborativeEditUsecase::new();
            let output = match usecase.execute(
                ApplyCollaborativeEditInput::replace_text(
                    &workspace_id,
                    &document_id,
                    &actor_user_id,
                    &operation_id,
                    base_revision,
                    current_revision,
                    start_offset,
                    end_offset,
                    &inserted_text,
                ),
                event_log,
                permission_decision,
            ) {
                Ok(output) => output,
                Err(error) => {
                    return rejected_realtime_response(&workspace_id, &document_id, error.code());
                }
            };
            if output.status() == ApplyCollaborativeEditStatus::ConflictDetected {
                return rejected_realtime_response(
                    &workspace_id,
                    &document_id,
                    "collaboration.conflict.detected",
                );
            }
            match operation_request(
                &workspace_id,
                &document_id,
                &session_id,
                &actor_user_id,
                &operation_id,
                base_revision,
                start_offset,
                end_offset,
                &inserted_text,
            )
            .and_then(|request| {
                transport
                    .broadcast_operation(request)
                    .map_err(|error| error.code().to_string())
            }) {
                Ok(_) => accepted_realtime_response(&workspace_id, &document_id),
                Err(error_code) => {
                    rejected_realtime_response(&workspace_id, &document_id, &error_code)
                }
            }
        }
        CollaborationRealtimeServerCommand::BroadcastPresence {
            workspace_id,
            document_id,
            session_id,
            actor_user_id,
            cursor_start,
            cursor_end,
        } => {
            let usecase = UpdatePresenceUsecase::new();
            if let Err(error) = usecase.execute(
                UpdatePresenceInput::new(
                    &workspace_id,
                    &document_id,
                    &actor_user_id,
                    cursor_start,
                    cursor_end,
                ),
                session_store,
                permission_decision,
            ) {
                return rejected_realtime_response(&workspace_id, &document_id, error.code());
            }
            match presence_request(
                &workspace_id,
                &document_id,
                &session_id,
                &actor_user_id,
                cursor_start,
                cursor_end,
            )
            .and_then(|request| {
                transport
                    .broadcast_presence(request)
                    .map_err(|error| error.code().to_string())
            }) {
                Ok(_) => accepted_realtime_response(&workspace_id, &document_id),
                Err(error_code) => {
                    rejected_realtime_response(&workspace_id, &document_id, &error_code)
                }
            }
        }
        CollaborationRealtimeServerCommand::RequestReplay {
            workspace_id,
            document_id,
            session_id,
            last_acknowledged_sequence,
        } => match replay_request(
            &workspace_id,
            &document_id,
            &session_id,
            last_acknowledged_sequence,
        )
        .and_then(|request| {
            transport
                .request_replay(request)
                .map_err(|error| error.code().to_string())
        }) {
            Ok(_) => accepted_realtime_response(&workspace_id, &document_id),
            Err(error_code) => rejected_realtime_response(&workspace_id, &document_id, &error_code),
        },
    }
}

pub struct CollaborationRealtimeRuntimeTarget<
    S: CollaborationSessionStore,
    E: CollaborationEventLog,
    P: PermissionDecisionPort,
    T: RealtimeTransport,
> {
    session_store: RefCell<S>,
    event_log: RefCell<E>,
    permission_decision: P,
    transport: RefCell<T>,
}

pub struct SplitRealtimeServerTarget<P, R> {
    primary_target: P,
    realtime_target: R,
}

impl<P, R> SplitRealtimeServerTarget<P, R> {
    pub const fn new(primary_target: P, realtime_target: R) -> Self {
        Self {
            primary_target,
            realtime_target,
        }
    }
}

impl<P: ServerUsecaseTarget, R: ServerUsecaseTarget> ServerUsecaseTarget
    for SplitRealtimeServerTarget<P, R>
{
    fn handle(&self, input: UsecaseInputDto) -> Result<UsecaseOutputDto, ServerBoundaryError> {
        if is_collaboration_realtime_route(input.route_id()) {
            return self.realtime_target.handle(input);
        }
        self.primary_target.handle(input)
    }
}

pub fn is_collaboration_realtime_route(route_id: &str) -> bool {
    matches!(
        route_id,
        "collaboration.join_document_room"
            | "collaboration.broadcast_operation"
            | "collaboration.broadcast_presence"
            | "collaboration.request_replay"
    )
}

impl<
    S: CollaborationSessionStore,
    E: CollaborationEventLog,
    P: PermissionDecisionPort,
    T: RealtimeTransport,
> CollaborationRealtimeRuntimeTarget<S, E, P, T>
{
    pub fn new(session_store: S, event_log: E, permission_decision: P, transport: T) -> Self {
        Self {
            session_store: RefCell::new(session_store),
            event_log: RefCell::new(event_log),
            permission_decision,
            transport: RefCell::new(transport),
        }
    }

    pub fn session_store(&self) -> Ref<'_, S> {
        self.session_store.borrow()
    }

    pub fn event_log(&self) -> Ref<'_, E> {
        self.event_log.borrow()
    }

    pub fn transport(&self) -> Ref<'_, T> {
        self.transport.borrow()
    }
}

impl<
    S: CollaborationSessionStore,
    E: CollaborationEventLog,
    P: PermissionDecisionPort,
    T: RealtimeTransport,
> ServerUsecaseTarget for CollaborationRealtimeRuntimeTarget<S, E, P, T>
{
    fn handle(&self, input: UsecaseInputDto) -> Result<UsecaseOutputDto, ServerBoundaryError> {
        let command = match command_from_input(&input) {
            Ok(command) => command,
            Err(error) => return Ok(rejected_response_for_input(&input, error.code())),
        };
        Ok(execute_realtime_command(
            command,
            &mut *self.session_store.borrow_mut(),
            &mut *self.event_log.borrow_mut(),
            &self.permission_decision,
            &mut *self.transport.borrow_mut(),
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollaborationRealtimeCommandError {
    MissingField,
    UnsupportedRoute,
}

impl CollaborationRealtimeCommandError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::MissingField => "collaboration_realtime.missing_field",
            Self::UnsupportedRoute => "collaboration_realtime.unsupported_route",
        }
    }
}

struct RealtimeBodyFields {
    body: String,
}

impl RealtimeBodyFields {
    fn parse(body: Option<&str>) -> Self {
        Self {
            body: body.unwrap_or_default().to_string(),
        }
    }

    fn string(&self, key: &str) -> Result<String, CollaborationRealtimeCommandError> {
        let key_pattern = format!("\"{}\"", key);
        let Some(start) = self.body.find(&key_pattern) else {
            return Err(CollaborationRealtimeCommandError::MissingField);
        };
        let after_key = &self.body[start + key_pattern.len()..];
        let Some(colon) = after_key.find(':') else {
            return Err(CollaborationRealtimeCommandError::MissingField);
        };
        let after_colon = after_key[colon + 1..].trim_start();
        let Some(after_quote) = after_colon.strip_prefix('"') else {
            return Err(CollaborationRealtimeCommandError::MissingField);
        };
        let Some(end) = after_quote.find('"') else {
            return Err(CollaborationRealtimeCommandError::MissingField);
        };
        Ok(after_quote[..end].to_string())
    }

    fn u64(&self, key: &str) -> Result<u64, CollaborationRealtimeCommandError> {
        self.number_string(key)?
            .parse()
            .map_err(|_| CollaborationRealtimeCommandError::MissingField)
    }

    fn usize(&self, key: &str) -> Result<usize, CollaborationRealtimeCommandError> {
        self.number_string(key)?
            .parse()
            .map_err(|_| CollaborationRealtimeCommandError::MissingField)
    }

    fn optional_u64(&self, key: &str) -> Option<u64> {
        self.number_string(key)
            .ok()
            .and_then(|value| value.parse().ok())
    }

    fn number_string(&self, key: &str) -> Result<String, CollaborationRealtimeCommandError> {
        let key_pattern = format!("\"{}\"", key);
        let Some(start) = self.body.find(&key_pattern) else {
            return Err(CollaborationRealtimeCommandError::MissingField);
        };
        let after_key = &self.body[start + key_pattern.len()..];
        let Some(colon) = after_key.find(':') else {
            return Err(CollaborationRealtimeCommandError::MissingField);
        };
        let after_colon = after_key[colon + 1..].trim_start();
        let digits = after_colon
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect::<String>();
        if digits.is_empty() {
            return Err(CollaborationRealtimeCommandError::MissingField);
        }
        Ok(digits)
    }
}

fn path_param(
    input: &UsecaseInputDto,
    key: &str,
) -> Result<String, CollaborationRealtimeCommandError> {
    input
        .path_param(key)
        .map(str::to_string)
        .ok_or(CollaborationRealtimeCommandError::MissingField)
}

fn join_request(
    workspace_id: &str,
    document_id: &str,
    session_id: &str,
    actor_user_id: &str,
) -> Result<JoinDocumentRoomRequest, String> {
    JoinDocumentRoomRequest::new(
        room_id(workspace_id, document_id)?,
        EditSessionId::new(session_id).map_err(|_| "collaboration.invalid_input".to_string())?,
        UserId::new(actor_user_id).map_err(|_| "collaboration.invalid_input".to_string())?,
    )
    .map_err(|error| error.code().to_string())
}

#[allow(clippy::too_many_arguments)]
fn operation_request(
    workspace_id: &str,
    document_id: &str,
    session_id: &str,
    actor_user_id: &str,
    operation_id: &str,
    base_revision: u64,
    start_offset: usize,
    end_offset: usize,
    inserted_text: &str,
) -> Result<OperationBroadcastRequest, String> {
    let document_id_value =
        DocumentId::new(document_id).map_err(|_| "collaboration.invalid_input".to_string())?;
    let operation = DocumentOperation::replace_text(
        OperationId::new(operation_id).map_err(|_| "collaboration.invalid_input".to_string())?,
        document_id_value,
        UserId::new(actor_user_id).map_err(|_| "collaboration.invalid_input".to_string())?,
        BaseRevision::new(base_revision).map_err(|_| "collaboration.invalid_input".to_string())?,
        TextRange::new(start_offset, end_offset)
            .map_err(|_| "collaboration.invalid_input".to_string())?,
        inserted_text,
    )
    .map_err(|_| "collaboration.invalid_input".to_string())?;
    OperationBroadcastRequest::new(
        room_id(workspace_id, document_id)?,
        EditSessionId::new(session_id).map_err(|_| "collaboration.invalid_input".to_string())?,
        operation,
    )
    .map_err(|error| error.code().to_string())
}

fn presence_request(
    workspace_id: &str,
    document_id: &str,
    session_id: &str,
    actor_user_id: &str,
    cursor_start: usize,
    cursor_end: usize,
) -> Result<PresenceBroadcastRequest, String> {
    let presence = Presence::new(
        DocumentId::new(document_id).map_err(|_| "collaboration.invalid_input".to_string())?,
        UserId::new(actor_user_id).map_err(|_| "collaboration.invalid_input".to_string())?,
        TextRange::new(cursor_start, cursor_end)
            .map_err(|_| "collaboration.invalid_input".to_string())?,
    )
    .map_err(|_| "collaboration.invalid_input".to_string())?;
    PresenceBroadcastRequest::new(
        room_id(workspace_id, document_id)?,
        EditSessionId::new(session_id).map_err(|_| "collaboration.invalid_input".to_string())?,
        presence,
    )
    .map_err(|error| error.code().to_string())
}

fn replay_request(
    workspace_id: &str,
    document_id: &str,
    session_id: &str,
    last_acknowledged_sequence: Option<u64>,
) -> Result<ReplayLocalChangesRequest, String> {
    ReplayLocalChangesRequest::new(
        room_id(workspace_id, document_id)?,
        EditSessionId::new(session_id).map_err(|_| "collaboration.invalid_input".to_string())?,
        last_acknowledged_sequence
            .map(OperationSequence::new)
            .transpose()
            .map_err(|_| "collaboration.invalid_input".to_string())?,
    )
    .map_err(|error| error.code().to_string())
}

fn room_id(
    workspace_id: &str,
    document_id: &str,
) -> Result<cabinet_domain::realtime::DocumentRoomId, String> {
    cabinet_domain::realtime::DocumentRoomId::new(
        WorkspaceId::new(workspace_id).map_err(|_| "collaboration.invalid_input".to_string())?,
        DocumentId::new(document_id).map_err(|_| "collaboration.invalid_input".to_string())?,
    )
    .map_err(|_| "collaboration.invalid_input".to_string())
}

fn rejected_response_for_input(input: &UsecaseInputDto, error_code: &str) -> UsecaseOutputDto {
    rejected_realtime_response(
        input.path_param("workspaceId").unwrap_or("unknown"),
        input.path_param("documentId").unwrap_or("unknown"),
        error_code,
    )
}
