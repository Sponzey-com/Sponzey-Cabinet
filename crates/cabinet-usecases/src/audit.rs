use cabinet_domain::audit::{
    AuditAction, AuditActor, AuditError, AuditEvent, AuditEventId, AuditMetadata, AuditTarget,
    AuditTargetId,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{Permission, PermissionDecisionResult};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::audit_log::{
    AuditClock, AuditCursor, AuditListQuery, AuditListScope, AuditLogStore, AuditLogStoreError,
    AuditPageRequest, AuditPermissionCheckError, AuditPermissionChecker,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditRetentionPolicy {
    retention_days: u32,
}

impl AuditRetentionPolicy {
    pub const fn new(retention_days: u32) -> Result<Self, AuditUsecaseError> {
        if retention_days == 0 {
            return Err(AuditUsecaseError::InvalidInput);
        }
        Ok(Self { retention_days })
    }

    pub const fn retention_days(self) -> u32 {
        self.retention_days
    }
}

impl Default for AuditRetentionPolicy {
    fn default() -> Self {
        Self::new(365).expect("default audit retention must be non-zero")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordAuditEventInput {
    actor_user_id: String,
    workspace_id: String,
    event_id: String,
    action: AuditAction,
    target: AuditTargetInput,
    metadata: Vec<(String, String)>,
}

impl RecordAuditEventInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        event_id: &str,
        action: AuditAction,
        target: AuditTargetInput,
        metadata: Vec<(&str, &str)>,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            event_id: event_id.to_string(),
            action,
            target,
            metadata: metadata
                .into_iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditTargetInput {
    Workspace,
    Document {
        document_id: String,
    },
    ReviewRequest {
        document_id: String,
        review_request_id: String,
    },
    DocumentLock {
        document_id: String,
        lock_id: String,
    },
    BackupJob {
        job_id: String,
    },
}

impl AuditTargetInput {
    pub const fn workspace() -> Self {
        Self::Workspace
    }

    pub fn document(document_id: &str) -> Self {
        Self::Document {
            document_id: document_id.to_string(),
        }
    }

    pub fn review_request(document_id: &str, review_request_id: &str) -> Self {
        Self::ReviewRequest {
            document_id: document_id.to_string(),
            review_request_id: review_request_id.to_string(),
        }
    }

    pub fn document_lock(document_id: &str, lock_id: &str) -> Self {
        Self::DocumentLock {
            document_id: document_id.to_string(),
            lock_id: lock_id.to_string(),
        }
    }

    pub fn backup_job(job_id: &str) -> Self {
        Self::BackupJob {
            job_id: job_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordAuditEventOutput {
    status: RecordAuditEventStatus,
    event_id: String,
    retention_days: u32,
}

impl RecordAuditEventOutput {
    pub const fn status(&self) -> RecordAuditEventStatus {
        self.status
    }

    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub const fn retention_days(&self) -> u32 {
        self.retention_days
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordAuditEventStatus {
    Recorded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAuditEventsInput {
    actor_user_id: String,
    workspace_id: String,
    scope: ListAuditEventsScopeInput,
    limit: usize,
    cursor: Option<String>,
}

impl ListAuditEventsInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        scope: ListAuditEventsScopeInput,
        limit: usize,
        cursor: Option<&str>,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            scope,
            limit,
            cursor: cursor.map(str::to_string),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListAuditEventsScopeInput {
    Workspace,
    Actor {
        actor_user_id: String,
    },
    Target {
        target_type: String,
        target_id: String,
    },
}

impl ListAuditEventsScopeInput {
    pub const fn workspace() -> Self {
        Self::Workspace
    }

    pub fn actor(actor_user_id: &str) -> Self {
        Self::Actor {
            actor_user_id: actor_user_id.to_string(),
        }
    }

    pub fn target(target_type: &str, target_id: &str) -> Self {
        Self::Target {
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAuditEventsOutput {
    events: Vec<AuditEventSummary>,
    next_cursor: Option<String>,
    retention_days: u32,
}

impl ListAuditEventsOutput {
    pub fn events(&self) -> &[AuditEventSummary] {
        &self.events
    }

    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }

    pub const fn retention_days(&self) -> u32 {
        self.retention_days
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEventSummary {
    event_id: String,
    workspace_id: String,
    actor_type: &'static str,
    actor_id: String,
    action: &'static str,
    target_type: &'static str,
    target_id: String,
    document_id: Option<String>,
    occurred_at_millis: u64,
    metadata: Vec<(String, String)>,
}

impl AuditEventSummary {
    fn from_event(event: &AuditEvent) -> Self {
        Self {
            event_id: event.event_id().as_str().to_string(),
            workspace_id: event.workspace_id().as_str().to_string(),
            actor_type: event.actor().actor_type(),
            actor_id: event.actor().actor_id().to_string(),
            action: event.action().as_str(),
            target_type: event.target().target_type(),
            target_id: event.target().target_id().to_string(),
            document_id: event
                .target()
                .document_id()
                .map(|document_id| document_id.as_str().to_string()),
            occurred_at_millis: event.occurred_at().as_millis(),
            metadata: event
                .metadata()
                .entries()
                .iter()
                .map(|entry| (entry.key().to_string(), entry.value().to_string()))
                .collect(),
        }
    }

    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub const fn actor_type(&self) -> &'static str {
        self.actor_type
    }

    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    pub const fn action(&self) -> &'static str {
        self.action
    }

    pub const fn target_type(&self) -> &'static str {
        self.target_type
    }

    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub fn document_id(&self) -> Option<&str> {
        self.document_id.as_deref()
    }

    pub const fn occurred_at_millis(&self) -> u64 {
        self.occurred_at_millis
    }

    pub fn metadata(&self) -> &[(String, String)] {
        &self.metadata
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditUsecaseError {
    InvalidInput,
    InvalidMetadata,
    Unauthorized,
    InvalidCursor,
    StoreUnavailable,
    Conflict,
}

impl AuditUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "AUDIT_INVALID_INPUT",
            Self::InvalidMetadata => "AUDIT_INVALID_METADATA",
            Self::Unauthorized => "AUDIT_UNAUTHORIZED",
            Self::InvalidCursor => "AUDIT_INVALID_CURSOR",
            Self::StoreUnavailable => "AUDIT_STORE_UNAVAILABLE",
            Self::Conflict => "AUDIT_CONFLICT",
        }
    }

    const fn from_store_error(error: AuditLogStoreError) -> Self {
        match error {
            AuditLogStoreError::InvalidLimit | AuditLogStoreError::InvalidScope => {
                Self::InvalidInput
            }
            AuditLogStoreError::InvalidCursor => Self::InvalidCursor,
            AuditLogStoreError::StorageUnavailable | AuditLogStoreError::CorruptedState => {
                Self::StoreUnavailable
            }
            AuditLogStoreError::Conflict => Self::Conflict,
        }
    }

    const fn from_permission_error(error: AuditPermissionCheckError) -> Self {
        match error {
            AuditPermissionCheckError::StorageUnavailable => Self::StoreUnavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditProductEvent {
    StoreFailed {
        masked_actor_id: String,
        action: &'static str,
        target_type: &'static str,
        error_code: &'static str,
    },
    QueryDenied {
        masked_actor_id: String,
        scope: &'static str,
        error_code: &'static str,
    },
}

impl AuditProductEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::StoreFailed { .. } => "audit.store.failed",
            Self::QueryDenied { .. } => "audit.query.denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditFieldDebugEvent {
    query_scope: &'static str,
    cursor_present: bool,
    result_count: usize,
}

impl AuditFieldDebugEvent {
    pub const fn query_scope(&self) -> &'static str {
        self.query_scope
    }

    pub const fn cursor_present(&self) -> bool {
        self.cursor_present
    }

    pub const fn result_count(&self) -> usize {
        self.result_count
    }
}

pub trait AuditUsecaseLogger {
    fn write_product(&mut self, event: AuditProductEvent);
    fn write_field_debug(&mut self, event: AuditFieldDebugEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordAuditEventUsecase {
    policy: AuditRetentionPolicy,
}

impl RecordAuditEventUsecase {
    pub const fn new(policy: AuditRetentionPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: RecordAuditEventInput,
        store: &mut impl AuditLogStore,
        clock: &impl AuditClock,
        logger: &mut impl AuditUsecaseLogger,
    ) -> Result<RecordAuditEventOutput, AuditUsecaseError> {
        let ids = ParsedRecordAuditEventInput::new(input)?;
        let event = AuditEvent::new(
            ids.event_id,
            ids.workspace_id,
            AuditActor::user(ids.actor_user_id),
            ids.action,
            ids.target,
            ids.metadata,
            clock.now(),
        );
        let event_id = event.event_id().as_str().to_string();
        if let Err(error) = store.append_audit_event(event.clone()) {
            logger.write_product(AuditProductEvent::StoreFailed {
                masked_actor_id: mask_id(event.actor().actor_id()),
                action: event.action().as_str(),
                target_type: event.target().target_type(),
                error_code: error.code(),
            });
            return Err(AuditUsecaseError::from_store_error(error));
        }

        Ok(RecordAuditEventOutput {
            status: RecordAuditEventStatus::Recorded,
            event_id,
            retention_days: self.policy.retention_days(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListAuditEventsUsecase {
    policy: AuditRetentionPolicy,
}

impl ListAuditEventsUsecase {
    pub const fn new(policy: AuditRetentionPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: ListAuditEventsInput,
        permission_checker: &impl AuditPermissionChecker,
        store: &impl AuditLogStore,
        logger: &mut impl AuditUsecaseLogger,
    ) -> Result<ListAuditEventsOutput, AuditUsecaseError> {
        let parsed = ParsedListAuditEventsInput::new(input)?;
        let decision = permission_checker
            .check_workspace_permission(
                &parsed.actor_user_id,
                &parsed.workspace_id,
                Permission::Manage,
            )
            .map_err(AuditUsecaseError::from_permission_error)?;

        if decision.result() != PermissionDecisionResult::Allowed {
            let error = AuditUsecaseError::Unauthorized;
            logger.write_product(AuditProductEvent::QueryDenied {
                masked_actor_id: mask_id(parsed.actor_user_id.as_str()),
                scope: parsed.scope.name(),
                error_code: error.code(),
            });
            return Err(error);
        }

        let cursor_present = parsed.page.cursor().is_some();
        let scope_name = parsed.scope.name();
        let page = store
            .list_audit_events(AuditListQuery::new(
                parsed.workspace_id,
                parsed.scope,
                parsed.page,
            ))
            .map_err(AuditUsecaseError::from_store_error)?;

        logger.write_field_debug(AuditFieldDebugEvent {
            query_scope: scope_name,
            cursor_present,
            result_count: page.events().len(),
        });

        Ok(ListAuditEventsOutput {
            events: page
                .events()
                .iter()
                .map(AuditEventSummary::from_event)
                .collect(),
            next_cursor: page.next_cursor().map(|cursor| cursor.as_str().to_string()),
            retention_days: self.policy.retention_days(),
        })
    }
}

struct ParsedRecordAuditEventInput {
    actor_user_id: UserId,
    workspace_id: WorkspaceId,
    event_id: AuditEventId,
    action: AuditAction,
    target: AuditTarget,
    metadata: AuditMetadata,
}

impl ParsedRecordAuditEventInput {
    fn new(input: RecordAuditEventInput) -> Result<Self, AuditUsecaseError> {
        let workspace_id =
            WorkspaceId::new(&input.workspace_id).map_err(|_| AuditUsecaseError::InvalidInput)?;
        Ok(Self {
            actor_user_id: UserId::new(&input.actor_user_id)
                .map_err(|_| AuditUsecaseError::InvalidInput)?,
            workspace_id: workspace_id.clone(),
            event_id: AuditEventId::new(&input.event_id)
                .map_err(|_| AuditUsecaseError::InvalidInput)?,
            action: input.action,
            target: parse_target_input(input.target, &workspace_id)?,
            metadata: AuditMetadata::from_pairs(&input.metadata).map_err(map_audit_error)?,
        })
    }
}

struct ParsedListAuditEventsInput {
    actor_user_id: UserId,
    workspace_id: WorkspaceId,
    scope: AuditListScope,
    page: AuditPageRequest,
}

impl ParsedListAuditEventsInput {
    fn new(input: ListAuditEventsInput) -> Result<Self, AuditUsecaseError> {
        let actor_user_id =
            UserId::new(&input.actor_user_id).map_err(|_| AuditUsecaseError::InvalidInput)?;
        let workspace_id =
            WorkspaceId::new(&input.workspace_id).map_err(|_| AuditUsecaseError::InvalidInput)?;
        let cursor = input
            .cursor
            .as_deref()
            .map(AuditCursor::new)
            .transpose()
            .map_err(AuditUsecaseError::from_store_error)?;
        let page = AuditPageRequest::new(input.limit, cursor)
            .map_err(AuditUsecaseError::from_store_error)?;
        let scope = match input.scope {
            ListAuditEventsScopeInput::Workspace => AuditListScope::Workspace,
            ListAuditEventsScopeInput::Actor { actor_user_id } => AuditListScope::actor(
                UserId::new(&actor_user_id).map_err(|_| AuditUsecaseError::InvalidInput)?,
            ),
            ListAuditEventsScopeInput::Target {
                target_type,
                target_id,
            } => AuditListScope::target(&target_type, &target_id)
                .map_err(AuditUsecaseError::from_store_error)?,
        };

        Ok(Self {
            actor_user_id,
            workspace_id,
            scope,
            page,
        })
    }
}

fn parse_target_input(
    input: AuditTargetInput,
    workspace_id: &WorkspaceId,
) -> Result<AuditTarget, AuditUsecaseError> {
    match input {
        AuditTargetInput::Workspace => Ok(AuditTarget::workspace(workspace_id.clone())),
        AuditTargetInput::Document { document_id } => Ok(AuditTarget::document(
            DocumentId::new(&document_id).map_err(|_| AuditUsecaseError::InvalidInput)?,
        )),
        AuditTargetInput::ReviewRequest {
            document_id,
            review_request_id,
        } => Ok(AuditTarget::review_request(
            DocumentId::new(&document_id).map_err(|_| AuditUsecaseError::InvalidInput)?,
            AuditTargetId::new(&review_request_id).map_err(|_| AuditUsecaseError::InvalidInput)?,
        )),
        AuditTargetInput::DocumentLock {
            document_id,
            lock_id,
        } => Ok(AuditTarget::document_lock(
            DocumentId::new(&document_id).map_err(|_| AuditUsecaseError::InvalidInput)?,
            AuditTargetId::new(&lock_id).map_err(|_| AuditUsecaseError::InvalidInput)?,
        )),
        AuditTargetInput::BackupJob { job_id } => Ok(AuditTarget::backup_job(
            AuditTargetId::new(&job_id).map_err(|_| AuditUsecaseError::InvalidInput)?,
        )),
    }
}

fn map_audit_error(error: AuditError) -> AuditUsecaseError {
    match error {
        AuditError::EmptyMetadataKey
        | AuditError::InvalidMetadataKey
        | AuditError::EmptyMetadataValue
        | AuditError::InvalidMetadataValue
        | AuditError::SensitiveMetadataKey
        | AuditError::SensitiveMetadataValue => AuditUsecaseError::InvalidMetadata,
        AuditError::EmptyEventId
        | AuditError::InvalidEventId
        | AuditError::EmptyTargetId
        | AuditError::InvalidTargetId => AuditUsecaseError::InvalidInput,
    }
}

fn mask_id(value: &str) -> String {
    match value.len() {
        0 => "masked:empty".to_string(),
        1..=4 => "masked:short".to_string(),
        len => format!("masked:{}", &value[len - 4..]),
    }
}
