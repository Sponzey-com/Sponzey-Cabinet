use cabinet_domain::collaboration::{
    BaseRevision, CollaborationError, DocumentOperation, EditSession, EditSessionId,
    EditSessionState, OperationId, OperationSequence, Presence, TextRange,
    detect_collaboration_conflict,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{AccessResource, Permission, PermissionDecisionResult};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::collaboration::{
    CollaborationEventLog, CollaborationEventLogError, CollaborationSessionStore,
    CollaborationSessionStoreError,
};
use cabinet_ports::permission_aware_query::{PermissionAwareQueryError, PermissionDecisionPort};

pub struct StartEditSessionUsecase;

impl StartEditSessionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: StartEditSessionInput,
        session_store: &mut impl CollaborationSessionStore,
        permission_decision: &impl PermissionDecisionPort,
    ) -> Result<StartEditSessionOutput, CollaborationUsecaseError> {
        let workspace_id = workspace_id(&input.workspace_id)?;
        let document_id = document_id(&input.document_id)?;
        let actor_user_id = user_id(&input.actor_user_id)?;
        let session_id = EditSessionId::new(&input.session_id)
            .map_err(CollaborationUsecaseError::from_domain)?;
        ensure_document_permission(
            permission_decision,
            &workspace_id,
            &document_id,
            &actor_user_id,
            Permission::Write,
        )?;
        let session = EditSession::new(
            session_id.clone(),
            document_id,
            actor_user_id,
            EditSessionState::SessionStarted,
        )
        .map_err(CollaborationUsecaseError::from_domain)?;
        session_store
            .save_session(&workspace_id, session)
            .map_err(CollaborationUsecaseError::from_session_store)?;
        Ok(StartEditSessionOutput {
            session_id,
            state: EditSessionState::SessionStarted,
        })
    }
}

impl Default for StartEditSessionUsecase {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ApplyCollaborativeEditUsecase;

impl ApplyCollaborativeEditUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ApplyCollaborativeEditInput,
        event_log: &mut impl CollaborationEventLog,
        permission_decision: &impl PermissionDecisionPort,
    ) -> Result<ApplyCollaborativeEditOutput, CollaborationUsecaseError> {
        let workspace_id = workspace_id(&input.workspace_id)?;
        let document_id = document_id(&input.document_id)?;
        let actor_user_id = user_id(&input.actor_user_id)?;
        ensure_document_permission(
            permission_decision,
            &workspace_id,
            &document_id,
            &actor_user_id,
            Permission::Write,
        )?;
        let operation = DocumentOperation::replace_text(
            OperationId::new(&input.operation_id)
                .map_err(CollaborationUsecaseError::from_domain)?,
            document_id.clone(),
            actor_user_id,
            BaseRevision::new(input.base_revision)
                .map_err(CollaborationUsecaseError::from_domain)?,
            TextRange::new(input.start_offset, input.end_offset)
                .map_err(CollaborationUsecaseError::from_domain)?,
            &input.inserted_text,
        )
        .map_err(CollaborationUsecaseError::from_domain)?;
        let current_revision = BaseRevision::new(input.current_revision)
            .map_err(CollaborationUsecaseError::from_domain)?;
        if detect_collaboration_conflict(&operation, current_revision).is_some() {
            return Ok(ApplyCollaborativeEditOutput {
                status: ApplyCollaborativeEditStatus::ConflictDetected,
                sequence: None,
            });
        }
        let sequence = event_log
            .append_operation(&workspace_id, operation)
            .map_err(CollaborationUsecaseError::from_event_log)?;
        Ok(ApplyCollaborativeEditOutput {
            status: ApplyCollaborativeEditStatus::Accepted,
            sequence: Some(sequence),
        })
    }
}

impl Default for ApplyCollaborativeEditUsecase {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UpdatePresenceUsecase;

impl UpdatePresenceUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: UpdatePresenceInput,
        session_store: &mut impl CollaborationSessionStore,
        permission_decision: &impl PermissionDecisionPort,
    ) -> Result<UpdatePresenceOutput, CollaborationUsecaseError> {
        let workspace_id = workspace_id(&input.workspace_id)?;
        let document_id = document_id(&input.document_id)?;
        let actor_user_id = user_id(&input.actor_user_id)?;
        ensure_document_permission(
            permission_decision,
            &workspace_id,
            &document_id,
            &actor_user_id,
            Permission::Read,
        )?;
        let presence = Presence::new(
            document_id.clone(),
            actor_user_id,
            TextRange::new(input.cursor_start, input.cursor_end)
                .map_err(CollaborationUsecaseError::from_domain)?,
        )
        .map_err(CollaborationUsecaseError::from_domain)?;
        session_store
            .save_presence(&workspace_id, presence)
            .map_err(CollaborationUsecaseError::from_session_store)?;
        let presence_count = session_store
            .list_presence(&workspace_id, &document_id)
            .map_err(CollaborationUsecaseError::from_session_store)?
            .len();
        Ok(UpdatePresenceOutput { presence_count })
    }
}

impl Default for UpdatePresenceUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartEditSessionInput {
    workspace_id: String,
    document_id: String,
    actor_user_id: String,
    session_id: String,
}

impl StartEditSessionInput {
    pub fn new(
        workspace_id: &str,
        document_id: &str,
        actor_user_id: &str,
        session_id: &str,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            actor_user_id: actor_user_id.to_string(),
            session_id: session_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyCollaborativeEditInput {
    workspace_id: String,
    document_id: String,
    actor_user_id: String,
    operation_id: String,
    base_revision: u64,
    current_revision: u64,
    start_offset: usize,
    end_offset: usize,
    inserted_text: String,
}

impl ApplyCollaborativeEditInput {
    #[allow(clippy::too_many_arguments)]
    pub fn replace_text(
        workspace_id: &str,
        document_id: &str,
        actor_user_id: &str,
        operation_id: &str,
        base_revision: u64,
        current_revision: u64,
        start_offset: usize,
        end_offset: usize,
        inserted_text: &str,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            actor_user_id: actor_user_id.to_string(),
            operation_id: operation_id.to_string(),
            base_revision,
            current_revision,
            start_offset,
            end_offset,
            inserted_text: inserted_text.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePresenceInput {
    workspace_id: String,
    document_id: String,
    actor_user_id: String,
    cursor_start: usize,
    cursor_end: usize,
}

impl UpdatePresenceInput {
    pub fn new(
        workspace_id: &str,
        document_id: &str,
        actor_user_id: &str,
        cursor_start: usize,
        cursor_end: usize,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            actor_user_id: actor_user_id.to_string(),
            cursor_start,
            cursor_end,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartEditSessionOutput {
    session_id: EditSessionId,
    state: EditSessionState,
}

impl StartEditSessionOutput {
    pub fn session_id(&self) -> &EditSessionId {
        &self.session_id
    }

    pub const fn state(&self) -> EditSessionState {
        self.state
    }

    pub const fn product_log_event(&self) -> &'static str {
        "collaboration.session.started"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyCollaborativeEditStatus {
    Accepted,
    ConflictDetected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyCollaborativeEditOutput {
    status: ApplyCollaborativeEditStatus,
    sequence: Option<OperationSequence>,
}

impl ApplyCollaborativeEditOutput {
    pub const fn status(&self) -> ApplyCollaborativeEditStatus {
        self.status
    }

    pub const fn sequence(&self) -> Option<OperationSequence> {
        self.sequence
    }

    pub const fn product_log_event(&self) -> &'static str {
        match self.status {
            ApplyCollaborativeEditStatus::Accepted => "collaboration.operation.accepted",
            ApplyCollaborativeEditStatus::ConflictDetected => "collaboration.conflict.detected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePresenceOutput {
    presence_count: usize,
}

impl UpdatePresenceOutput {
    pub const fn presence_count(&self) -> usize {
        self.presence_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollaborationUsecaseError {
    InvalidInput,
    PermissionDenied,
    PermissionUnavailable,
    SessionStoreUnavailable,
    EventLogUnavailable,
}

impl CollaborationUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "collaboration.invalid_input",
            Self::PermissionDenied => "collaboration.permission_denied",
            Self::PermissionUnavailable => "collaboration.permission_unavailable",
            Self::SessionStoreUnavailable => "collaboration.session_store_unavailable",
            Self::EventLogUnavailable => "collaboration.event_log_unavailable",
        }
    }

    const fn from_domain(_error: CollaborationError) -> Self {
        Self::InvalidInput
    }

    const fn from_session_store(error: CollaborationSessionStoreError) -> Self {
        match error {
            CollaborationSessionStoreError::InvalidInput => Self::InvalidInput,
            CollaborationSessionStoreError::NotFound
            | CollaborationSessionStoreError::Conflict
            | CollaborationSessionStoreError::StorageUnavailable => Self::SessionStoreUnavailable,
        }
    }

    const fn from_event_log(error: CollaborationEventLogError) -> Self {
        match error {
            CollaborationEventLogError::InvalidInput => Self::InvalidInput,
            CollaborationEventLogError::Conflict
            | CollaborationEventLogError::StorageUnavailable => Self::EventLogUnavailable,
        }
    }

    const fn from_permission_error(error: PermissionAwareQueryError) -> Self {
        match error {
            PermissionAwareQueryError::InvalidInput => Self::InvalidInput,
            PermissionAwareQueryError::NotFound
            | PermissionAwareQueryError::IndexStale
            | PermissionAwareQueryError::StorageUnavailable
            | PermissionAwareQueryError::CorruptedProjection => Self::PermissionUnavailable,
        }
    }
}

fn ensure_document_permission(
    permission_decision: &impl PermissionDecisionPort,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
    actor_user_id: &UserId,
    permission: Permission,
) -> Result<(), CollaborationUsecaseError> {
    let resource = AccessResource::document(workspace_id.clone(), None, document_id.clone());
    let decision = permission_decision
        .check_permission(actor_user_id, &resource, permission)
        .map_err(CollaborationUsecaseError::from_permission_error)?;
    if decision.result() != PermissionDecisionResult::Allowed {
        return Err(CollaborationUsecaseError::PermissionDenied);
    }
    Ok(())
}

fn workspace_id(value: &str) -> Result<WorkspaceId, CollaborationUsecaseError> {
    WorkspaceId::new(value).map_err(|_| CollaborationUsecaseError::InvalidInput)
}

fn document_id(value: &str) -> Result<DocumentId, CollaborationUsecaseError> {
    DocumentId::new(value).map_err(|_| CollaborationUsecaseError::InvalidInput)
}

fn user_id(value: &str) -> Result<UserId, CollaborationUsecaseError> {
    UserId::new(value).map_err(|_| CollaborationUsecaseError::InvalidInput)
}
