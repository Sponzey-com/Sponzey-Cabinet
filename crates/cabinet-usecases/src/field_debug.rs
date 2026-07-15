use cabinet_domain::field_debug::{
    FieldDebugError, FieldDebugEvent, FieldDebugScope, FieldDebugSession, FieldDebugSessionId,
    FieldDebugSessionState, FieldDebugTimestamp, FieldDebugTtl, transition_field_debug_session,
};
use cabinet_domain::permission::{Permission, PermissionDecisionResult};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::field_debug::{
    FieldDebugClock, FieldDebugPermissionCheckError, FieldDebugPermissionChecker,
    FieldDebugSessionRepository, FieldDebugSessionRepositoryError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldDebugSessionPolicy {
    max_ttl_seconds: u32,
}

impl FieldDebugSessionPolicy {
    pub const fn new(max_ttl_seconds: u32) -> Result<Self, FieldDebugUsecaseError> {
        if max_ttl_seconds == 0 {
            return Err(FieldDebugUsecaseError::InvalidInput);
        }
        Ok(Self { max_ttl_seconds })
    }

    pub const fn max_ttl_seconds(self) -> u32 {
        self.max_ttl_seconds
    }
}

impl Default for FieldDebugSessionPolicy {
    fn default() -> Self {
        Self::new(900).expect("default field debug max ttl must be non-zero")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestFieldDebugSessionInput {
    actor_user_id: String,
    workspace_id: String,
    session_id: String,
    scope: Option<String>,
    ttl_seconds: Option<u32>,
}

impl RequestFieldDebugSessionInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        session_id: &str,
        scope: Option<&str>,
        ttl_seconds: Option<u32>,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            session_id: session_id.to_string(),
            scope: scope.map(str::to_string),
            ttl_seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApproveFieldDebugSessionInput {
    admin_user_id: String,
    workspace_id: String,
    session_id: String,
}

impl ApproveFieldDebugSessionInput {
    pub fn new(admin_user_id: &str, workspace_id: &str, session_id: &str) -> Self {
        Self {
            admin_user_id: admin_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            session_id: session_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpireFieldDebugSessionInput {
    admin_user_id: String,
    workspace_id: String,
    session_id: String,
}

impl ExpireFieldDebugSessionInput {
    pub fn new(admin_user_id: &str, workspace_id: &str, session_id: &str) -> Self {
        Self {
            admin_user_id: admin_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            session_id: session_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevokeFieldDebugSessionInput {
    admin_user_id: String,
    workspace_id: String,
    session_id: String,
}

impl RevokeFieldDebugSessionInput {
    pub fn new(admin_user_id: &str, workspace_id: &str, session_id: &str) -> Self {
        Self {
            admin_user_id: admin_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            session_id: session_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDebugDiagnosticInput {
    workspace_id: String,
    session_id: String,
    event_name: String,
    fields: Vec<(String, String)>,
}

impl FieldDebugDiagnosticInput {
    pub fn new(
        workspace_id: &str,
        session_id: &str,
        event_name: &str,
        fields: Vec<(&str, &str)>,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            session_id: session_id.to_string(),
            event_name: event_name.to_string(),
            fields: fields
                .into_iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDebugSessionOutput {
    status: FieldDebugSessionOutputStatus,
    session_id: String,
    scope: Option<String>,
    expires_at_millis: Option<u64>,
}

impl FieldDebugSessionOutput {
    fn from_session(session: &FieldDebugSession) -> Self {
        Self {
            status: FieldDebugSessionOutputStatus::from_state(session.state()),
            session_id: session.session_id().as_str().to_string(),
            scope: session.scope().map(|scope| scope.as_str().to_string()),
            expires_at_millis: session.expires_at().map(FieldDebugTimestamp::as_millis),
        }
    }

    pub const fn status(&self) -> FieldDebugSessionOutputStatus {
        self.status
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn scope(&self) -> Option<&str> {
        self.scope.as_deref()
    }

    pub const fn expires_at_millis(&self) -> Option<u64> {
        self.expires_at_millis
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldDebugSessionOutputStatus {
    Requested,
    Approved,
    Denied,
    Active,
    Expired,
    Revoked,
}

impl FieldDebugSessionOutputStatus {
    const fn from_state(state: FieldDebugSessionState) -> Self {
        match state {
            FieldDebugSessionState::Requested => Self::Requested,
            FieldDebugSessionState::Approved => Self::Approved,
            FieldDebugSessionState::Denied => Self::Denied,
            FieldDebugSessionState::Active => Self::Active,
            FieldDebugSessionState::Expired => Self::Expired,
            FieldDebugSessionState::Revoked => Self::Revoked,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldDebugUsecaseError {
    InvalidInput,
    MissingScope,
    MissingTtl,
    TtlExceedsPolicy,
    Unauthorized,
    SessionNotFound,
    InactiveSession,
    ExpiredSession,
    NotExpired,
    SensitiveField,
    StoreUnavailable,
    Conflict,
}

impl FieldDebugUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "FIELD_DEBUG_INVALID_INPUT",
            Self::MissingScope => "FIELD_DEBUG_MISSING_SCOPE",
            Self::MissingTtl => "FIELD_DEBUG_MISSING_TTL",
            Self::TtlExceedsPolicy => "FIELD_DEBUG_TTL_EXCEEDS_POLICY",
            Self::Unauthorized => "FIELD_DEBUG_UNAUTHORIZED",
            Self::SessionNotFound => "FIELD_DEBUG_SESSION_NOT_FOUND",
            Self::InactiveSession => "FIELD_DEBUG_INACTIVE_SESSION",
            Self::ExpiredSession => "FIELD_DEBUG_EXPIRED_SESSION",
            Self::NotExpired => "FIELD_DEBUG_NOT_EXPIRED",
            Self::SensitiveField => "FIELD_DEBUG_SENSITIVE_FIELD",
            Self::StoreUnavailable => "FIELD_DEBUG_STORE_UNAVAILABLE",
            Self::Conflict => "FIELD_DEBUG_CONFLICT",
        }
    }

    const fn from_repository_error(error: FieldDebugSessionRepositoryError) -> Self {
        match error {
            FieldDebugSessionRepositoryError::StorageUnavailable
            | FieldDebugSessionRepositoryError::CorruptedState => Self::StoreUnavailable,
            FieldDebugSessionRepositoryError::Conflict => Self::Conflict,
        }
    }

    const fn from_permission_error(error: FieldDebugPermissionCheckError) -> Self {
        match error {
            FieldDebugPermissionCheckError::StorageUnavailable => Self::StoreUnavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDebugProductEvent {
    event_name: &'static str,
    masked_admin_id: String,
    scope_id: String,
    ttl_bucket: &'static str,
    error_code: Option<&'static str>,
}

impl FieldDebugProductEvent {
    fn new(
        event_name: &'static str,
        admin_id: &str,
        scope_id: Option<&str>,
        ttl: Option<FieldDebugTtl>,
        error_code: Option<&'static str>,
    ) -> Self {
        Self {
            event_name,
            masked_admin_id: mask_id(admin_id),
            scope_id: scope_id.unwrap_or("missing").to_string(),
            ttl_bucket: ttl_bucket(ttl),
            error_code,
        }
    }

    pub const fn event_name(&self) -> &'static str {
        self.event_name
    }

    pub fn masked_admin_id(&self) -> &str {
        &self.masked_admin_id
    }

    pub fn scope_id(&self) -> &str {
        &self.scope_id
    }

    pub const fn ttl_bucket(&self) -> &'static str {
        self.ttl_bucket
    }

    pub const fn error_code(&self) -> Option<&'static str> {
        self.error_code
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDebugLogEvent {
    event_name: &'static str,
    scope: String,
    fields: Vec<(String, String)>,
}

impl FieldDebugLogEvent {
    fn new(scope: &FieldDebugScope, fields: Vec<(String, String)>) -> Self {
        Self {
            event_name: "field_debug.diagnostic",
            scope: scope.as_str().to_string(),
            fields,
        }
    }

    pub const fn event_name(&self) -> &'static str {
        self.event_name
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn fields(&self) -> Vec<(&str, &str)> {
        self.fields
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDebugDevelopmentEvent {
    event_name: &'static str,
}

impl FieldDebugDevelopmentEvent {
    pub const fn event_name(&self) -> &'static str {
        self.event_name
    }
}

pub trait FieldDebugUsecaseLogger {
    fn write_product(&mut self, event: FieldDebugProductEvent);
    fn write_field_debug(&mut self, event: FieldDebugLogEvent);
    fn write_development(&mut self, event: FieldDebugDevelopmentEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestFieldDebugSessionUsecase {
    policy: FieldDebugSessionPolicy,
}

impl RequestFieldDebugSessionUsecase {
    pub const fn new(policy: FieldDebugSessionPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: RequestFieldDebugSessionInput,
        repository: &mut impl FieldDebugSessionRepository,
        clock: &impl FieldDebugClock,
        logger: &mut impl FieldDebugUsecaseLogger,
    ) -> Result<FieldDebugSessionOutput, FieldDebugUsecaseError> {
        let parsed = ParsedRequestFieldDebugSessionInput::new(input, self.policy)?;
        let session = FieldDebugSession::requested(
            parsed.session_id,
            parsed.workspace_id,
            parsed.actor_user_id.clone(),
            parsed.scope,
            parsed.ttl,
            clock.now(),
        );
        repository
            .save_field_debug_session(session.clone())
            .map_err(FieldDebugUsecaseError::from_repository_error)?;
        logger.write_product(FieldDebugProductEvent::new(
            "field_debug.requested",
            parsed.actor_user_id.as_str(),
            session.scope().map(FieldDebugScope::as_str),
            session.ttl(),
            None,
        ));
        Ok(FieldDebugSessionOutput::from_session(&session))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApproveFieldDebugSessionUsecase {
    policy: FieldDebugSessionPolicy,
}

impl ApproveFieldDebugSessionUsecase {
    pub const fn new(policy: FieldDebugSessionPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: ApproveFieldDebugSessionInput,
        permission_checker: &impl FieldDebugPermissionChecker,
        repository: &mut impl FieldDebugSessionRepository,
        clock: &impl FieldDebugClock,
        logger: &mut impl FieldDebugUsecaseLogger,
    ) -> Result<FieldDebugSessionOutput, FieldDebugUsecaseError> {
        let ids = ParsedSessionActionInput::new(
            input.admin_user_id,
            input.workspace_id,
            input.session_id,
        )?;
        ensure_manage_permission(permission_checker, &ids.actor_user_id, &ids.workspace_id)?;
        let session = load_session(repository, &ids.workspace_id, &ids.session_id)?;
        validate_session_ttl(session.ttl(), self.policy)?;
        let approved = transition_field_debug_session(
            &session,
            FieldDebugEvent::Approve {
                admin_user_id: ids.actor_user_id.clone(),
                at: clock.now(),
            },
        )
        .map_err(map_domain_error)?
        .into_session();
        repository
            .save_field_debug_session(approved.clone())
            .map_err(FieldDebugUsecaseError::from_repository_error)?;
        logger.write_product(product_event_from_session(
            "field_debug.approved",
            &ids.actor_user_id,
            &approved,
            None,
        ));

        let active = transition_field_debug_session(
            &approved,
            FieldDebugEvent::Activate { at: clock.now() },
        )
        .map_err(map_domain_error)?
        .into_session();
        repository
            .save_field_debug_session(active.clone())
            .map_err(FieldDebugUsecaseError::from_repository_error)?;
        logger.write_product(product_event_from_session(
            "field_debug.active",
            &ids.actor_user_id,
            &active,
            None,
        ));
        Ok(FieldDebugSessionOutput::from_session(&active))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpireFieldDebugSessionUsecase {
    policy: FieldDebugSessionPolicy,
}

impl ExpireFieldDebugSessionUsecase {
    pub const fn new(policy: FieldDebugSessionPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: ExpireFieldDebugSessionInput,
        permission_checker: &impl FieldDebugPermissionChecker,
        repository: &mut impl FieldDebugSessionRepository,
        clock: &impl FieldDebugClock,
        logger: &mut impl FieldDebugUsecaseLogger,
    ) -> Result<FieldDebugSessionOutput, FieldDebugUsecaseError> {
        let ids = ParsedSessionActionInput::new(
            input.admin_user_id,
            input.workspace_id,
            input.session_id,
        )?;
        ensure_manage_permission(permission_checker, &ids.actor_user_id, &ids.workspace_id)?;
        let session = load_session(repository, &ids.workspace_id, &ids.session_id)?;
        validate_session_ttl(session.ttl(), self.policy)?;
        let expired =
            transition_field_debug_session(&session, FieldDebugEvent::Expire { at: clock.now() })
                .map_err(map_domain_error)?
                .into_session();
        repository
            .save_field_debug_session(expired.clone())
            .map_err(FieldDebugUsecaseError::from_repository_error)?;
        logger.write_product(product_event_from_session(
            "field_debug.expired",
            &ids.actor_user_id,
            &expired,
            None,
        ));
        Ok(FieldDebugSessionOutput::from_session(&expired))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevokeFieldDebugSessionUsecase {
    policy: FieldDebugSessionPolicy,
}

impl RevokeFieldDebugSessionUsecase {
    pub const fn new(policy: FieldDebugSessionPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: RevokeFieldDebugSessionInput,
        permission_checker: &impl FieldDebugPermissionChecker,
        repository: &mut impl FieldDebugSessionRepository,
        clock: &impl FieldDebugClock,
        logger: &mut impl FieldDebugUsecaseLogger,
    ) -> Result<FieldDebugSessionOutput, FieldDebugUsecaseError> {
        let ids = ParsedSessionActionInput::new(
            input.admin_user_id,
            input.workspace_id,
            input.session_id,
        )?;
        ensure_manage_permission(permission_checker, &ids.actor_user_id, &ids.workspace_id)?;
        let session = load_session(repository, &ids.workspace_id, &ids.session_id)?;
        validate_session_ttl(session.ttl(), self.policy)?;
        let revoked = transition_field_debug_session(
            &session,
            FieldDebugEvent::Revoke {
                admin_user_id: ids.actor_user_id.clone(),
                at: clock.now(),
            },
        )
        .map_err(map_domain_error)?
        .into_session();
        repository
            .save_field_debug_session(revoked.clone())
            .map_err(FieldDebugUsecaseError::from_repository_error)?;
        logger.write_product(product_event_from_session(
            "field_debug.revoked",
            &ids.actor_user_id,
            &revoked,
            None,
        ));
        Ok(FieldDebugSessionOutput::from_session(&revoked))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldDebugDiagnosticUsecase;

impl FieldDebugDiagnosticUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: FieldDebugDiagnosticInput,
        repository: &impl FieldDebugSessionRepository,
        clock: &impl FieldDebugClock,
        logger: &mut impl FieldDebugUsecaseLogger,
    ) -> Result<(), FieldDebugUsecaseError> {
        let parsed = ParsedDiagnosticInput::new(input)?;
        let session = load_session(repository, &parsed.workspace_id, &parsed.session_id)?;
        if session.state() != FieldDebugSessionState::Active {
            return Err(FieldDebugUsecaseError::InactiveSession);
        }
        if !session.is_active_at(clock.now()) {
            return Err(FieldDebugUsecaseError::ExpiredSession);
        }
        let scope = session
            .scope()
            .ok_or(FieldDebugUsecaseError::MissingScope)?;
        logger.write_field_debug(FieldDebugLogEvent::new(scope, parsed.fields));
        Ok(())
    }
}

struct ParsedRequestFieldDebugSessionInput {
    actor_user_id: UserId,
    workspace_id: WorkspaceId,
    session_id: FieldDebugSessionId,
    scope: Option<FieldDebugScope>,
    ttl: Option<FieldDebugTtl>,
}

impl ParsedRequestFieldDebugSessionInput {
    fn new(
        input: RequestFieldDebugSessionInput,
        policy: FieldDebugSessionPolicy,
    ) -> Result<Self, FieldDebugUsecaseError> {
        let ttl = input
            .ttl_seconds
            .map(|ttl| {
                if ttl > policy.max_ttl_seconds() {
                    return Err(FieldDebugUsecaseError::TtlExceedsPolicy);
                }
                FieldDebugTtl::seconds(ttl).map_err(map_domain_error)
            })
            .transpose()?;
        Ok(Self {
            actor_user_id: UserId::new(&input.actor_user_id)
                .map_err(|_| FieldDebugUsecaseError::InvalidInput)?,
            workspace_id: WorkspaceId::new(&input.workspace_id)
                .map_err(|_| FieldDebugUsecaseError::InvalidInput)?,
            session_id: FieldDebugSessionId::new(&input.session_id).map_err(map_domain_error)?,
            scope: input
                .scope
                .as_deref()
                .map(FieldDebugScope::new)
                .transpose()
                .map_err(map_domain_error)?,
            ttl,
        })
    }
}

struct ParsedSessionActionInput {
    actor_user_id: UserId,
    workspace_id: WorkspaceId,
    session_id: FieldDebugSessionId,
}

impl ParsedSessionActionInput {
    fn new(
        actor_user_id: String,
        workspace_id: String,
        session_id: String,
    ) -> Result<Self, FieldDebugUsecaseError> {
        Ok(Self {
            actor_user_id: UserId::new(&actor_user_id)
                .map_err(|_| FieldDebugUsecaseError::InvalidInput)?,
            workspace_id: WorkspaceId::new(&workspace_id)
                .map_err(|_| FieldDebugUsecaseError::InvalidInput)?,
            session_id: FieldDebugSessionId::new(&session_id).map_err(map_domain_error)?,
        })
    }
}

struct ParsedDiagnosticInput {
    workspace_id: WorkspaceId,
    session_id: FieldDebugSessionId,
    fields: Vec<(String, String)>,
}

impl ParsedDiagnosticInput {
    fn new(input: FieldDebugDiagnosticInput) -> Result<Self, FieldDebugUsecaseError> {
        validate_log_token(&input.event_name)?;
        let mut fields = Vec::with_capacity(input.fields.len());
        for (key, value) in input.fields {
            fields.push(validate_diagnostic_field(&key, &value)?);
        }
        Ok(Self {
            workspace_id: WorkspaceId::new(&input.workspace_id)
                .map_err(|_| FieldDebugUsecaseError::InvalidInput)?,
            session_id: FieldDebugSessionId::new(&input.session_id).map_err(map_domain_error)?,
            fields,
        })
    }
}

fn ensure_manage_permission(
    permission_checker: &impl FieldDebugPermissionChecker,
    actor_user_id: &UserId,
    workspace_id: &WorkspaceId,
) -> Result<(), FieldDebugUsecaseError> {
    let decision = permission_checker
        .check_workspace_permission(actor_user_id, workspace_id, Permission::Manage)
        .map_err(FieldDebugUsecaseError::from_permission_error)?;
    if decision.result() != PermissionDecisionResult::Allowed {
        return Err(FieldDebugUsecaseError::Unauthorized);
    }
    Ok(())
}

fn load_session(
    repository: &impl FieldDebugSessionRepository,
    workspace_id: &WorkspaceId,
    session_id: &FieldDebugSessionId,
) -> Result<FieldDebugSession, FieldDebugUsecaseError> {
    repository
        .get_field_debug_session(workspace_id, session_id)
        .map_err(FieldDebugUsecaseError::from_repository_error)?
        .ok_or(FieldDebugUsecaseError::SessionNotFound)
}

fn validate_session_ttl(
    ttl: Option<FieldDebugTtl>,
    policy: FieldDebugSessionPolicy,
) -> Result<(), FieldDebugUsecaseError> {
    let ttl = ttl.ok_or(FieldDebugUsecaseError::MissingTtl)?;
    if ttl.as_seconds() > policy.max_ttl_seconds() {
        return Err(FieldDebugUsecaseError::TtlExceedsPolicy);
    }
    Ok(())
}

fn product_event_from_session(
    event_name: &'static str,
    actor_user_id: &UserId,
    session: &FieldDebugSession,
    error_code: Option<&'static str>,
) -> FieldDebugProductEvent {
    FieldDebugProductEvent::new(
        event_name,
        actor_user_id.as_str(),
        session.scope().map(FieldDebugScope::as_str),
        session.ttl(),
        error_code,
    )
}

fn map_domain_error(error: FieldDebugError) -> FieldDebugUsecaseError {
    match error {
        FieldDebugError::EmptySessionId | FieldDebugError::EmptyScope => {
            FieldDebugUsecaseError::InvalidInput
        }
        FieldDebugError::SensitiveScope => FieldDebugUsecaseError::SensitiveField,
        FieldDebugError::MissingScope => FieldDebugUsecaseError::MissingScope,
        FieldDebugError::MissingTtl => FieldDebugUsecaseError::MissingTtl,
        FieldDebugError::InvalidTransition => FieldDebugUsecaseError::InactiveSession,
        FieldDebugError::NotExpired => FieldDebugUsecaseError::NotExpired,
    }
}

fn validate_diagnostic_field(
    key: &str,
    value: &str,
) -> Result<(String, String), FieldDebugUsecaseError> {
    let key = key.trim();
    let value = value.trim();
    if key.is_empty() || value.is_empty() || key.chars().any(char::is_control) {
        return Err(FieldDebugUsecaseError::InvalidInput);
    }
    if contains_sensitive_fragment(key) || contains_sensitive_fragment(value) {
        return Err(FieldDebugUsecaseError::SensitiveField);
    }
    Ok((key.to_string(), value.to_string()))
}

fn validate_log_token(value: &str) -> Result<(), FieldDebugUsecaseError> {
    let value = value.trim();
    if value.is_empty() || value.chars().any(char::is_control) || contains_sensitive_fragment(value)
    {
        return Err(FieldDebugUsecaseError::InvalidInput);
    }
    Ok(())
}

fn contains_sensitive_fragment(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "password",
        "token",
        "secret",
        "credential",
        "document_body",
        "comment_body",
        "document body",
        "comment body",
        "asset content",
        "asset bytes",
        "request body",
        "response body",
        "raw body",
        "content",
    ]
    .iter()
    .any(|fragment| lower.contains(fragment))
}

fn ttl_bucket(ttl: Option<FieldDebugTtl>) -> &'static str {
    match ttl.map(FieldDebugTtl::as_seconds) {
        None => "missing",
        Some(0..=300) => "0_300s",
        Some(301..=900) => "301_900s",
        Some(_) => "over_900s",
    }
}

fn mask_id(value: &str) -> String {
    match value.len() {
        0 => "masked:empty".to_string(),
        1..=4 => "masked:short".to_string(),
        len => format!("masked:{}", &value[len - 4..]),
    }
}
