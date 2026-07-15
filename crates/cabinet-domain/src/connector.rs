#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorId(String);

impl ConnectorId {
    pub fn new(value: &str) -> Result<Self, ConnectorError> {
        Ok(Self(normalize_reference(value, ConnectorError::InvalidId)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorKind {
    Slack,
    Teams,
    Jira,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorScope {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorAction {
    Read,
    Write,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorScopeSet {
    scopes: Vec<ConnectorScope>,
}

impl ConnectorScopeSet {
    pub fn new(scopes: Vec<ConnectorScope>) -> Result<Self, ConnectorError> {
        if scopes.is_empty() {
            return Err(ConnectorError::EmptyScope);
        }

        let mut unique = Vec::new();
        for scope in scopes {
            if !unique.contains(&scope) {
                unique.push(scope);
            }
        }
        Ok(Self { scopes: unique })
    }

    pub fn read_only() -> Self {
        Self {
            scopes: vec![ConnectorScope::Read],
        }
    }

    pub fn read_write() -> Self {
        Self {
            scopes: vec![ConnectorScope::Read, ConnectorScope::Write],
        }
    }

    pub fn scopes(&self) -> &[ConnectorScope] {
        &self.scopes
    }

    pub fn contains_all(&self, requested: &ConnectorScopeSet) -> bool {
        requested
            .scopes()
            .iter()
            .copied()
            .all(|scope| self.scopes.contains(&scope))
    }

    pub fn allows(&self, action: ConnectorAction) -> bool {
        match action {
            ConnectorAction::Read => self.scopes.contains(&ConnectorScope::Read),
            ConnectorAction::Write => self.scopes.contains(&ConnectorScope::Write),
        }
    }

    pub fn require_action(&self, action: ConnectorAction) -> Result<(), ConnectorError> {
        if self.allows(action) {
            return Ok(());
        }
        Err(ConnectorError::ScopeDenied)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorDefinition {
    id: ConnectorId,
    kind: ConnectorKind,
    display_name: String,
    scopes: ConnectorScopeSet,
}

impl ConnectorDefinition {
    pub fn new(
        id: ConnectorId,
        kind: ConnectorKind,
        display_name: &str,
        scopes: ConnectorScopeSet,
    ) -> Result<Self, ConnectorError> {
        let display_name = normalize_reference(display_name, ConnectorError::InvalidName)?;
        Ok(Self {
            id,
            kind,
            display_name,
            scopes,
        })
    }

    pub fn id(&self) -> &ConnectorId {
        &self.id
    }

    pub const fn kind(&self) -> ConnectorKind {
        self.kind
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn scopes(&self) -> &ConnectorScopeSet {
        &self.scopes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorCredentialReference(String);

impl ConnectorCredentialReference {
    pub fn new(value: &str) -> Result<Self, ConnectorError> {
        let reference = normalize_reference(value, ConnectorError::InvalidCredentialReference)?;
        if !reference.starts_with("connector-credential:")
            || reference.len() <= "connector-credential:".len()
            || contains_sensitive_connector_fixture(&reference)
        {
            return Err(ConnectorError::InvalidCredentialReference);
        }
        Ok(Self(reference))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorExternalObjectReference(String);

impl ConnectorExternalObjectReference {
    pub fn new(value: &str) -> Result<Self, ConnectorError> {
        let reference = normalize_reference(value, ConnectorError::InvalidExternalObjectReference)?;
        if !reference.starts_with("external-object:")
            || reference.len() <= "external-object:".len()
            || contains_sensitive_connector_fixture(&reference)
        {
            return Err(ConnectorError::InvalidExternalObjectReference);
        }
        Ok(Self(reference))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorActivityId(String);

impl ConnectorActivityId {
    pub fn new(value: &str) -> Result<Self, ConnectorError> {
        Ok(Self(normalize_reference(value, ConnectorError::InvalidId)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorActivityKind {
    SyncCompleted,
    SyncFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorActivity {
    id: ConnectorActivityId,
    installation_id: ConnectorInstallationId,
    connector_id: ConnectorId,
    workspace_id_hash: String,
    kind: ConnectorActivityKind,
    object_count: u32,
    error_code: Option<String>,
}

impl ConnectorActivity {
    pub fn new(
        id: ConnectorActivityId,
        installation_id: ConnectorInstallationId,
        connector_id: ConnectorId,
        workspace_id_hash: &str,
        kind: ConnectorActivityKind,
        object_count: u32,
        error_code: Option<&str>,
    ) -> Result<Self, ConnectorError> {
        let error_code = match error_code {
            Some(value) => {
                let normalized = normalize_reference(value, ConnectorError::InvalidActivity)?;
                if contains_sensitive_connector_fixture(&normalized) {
                    return Err(ConnectorError::InvalidActivity);
                }
                Some(normalized)
            }
            None => None,
        };

        Ok(Self {
            id,
            installation_id,
            connector_id,
            workspace_id_hash: normalize_reference(workspace_id_hash, ConnectorError::InvalidId)?,
            kind,
            object_count,
            error_code,
        })
    }

    pub fn id(&self) -> &ConnectorActivityId {
        &self.id
    }

    pub fn installation_id(&self) -> &ConnectorInstallationId {
        &self.installation_id
    }

    pub fn connector_id(&self) -> &ConnectorId {
        &self.connector_id
    }

    pub fn workspace_id_hash(&self) -> &str {
        &self.workspace_id_hash
    }

    pub const fn kind(&self) -> ConnectorActivityKind {
        self.kind
    }

    pub const fn object_count(&self) -> u32 {
        self.object_count
    }

    pub fn error_code(&self) -> Option<&str> {
        self.error_code.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorInstallationId(String);

impl ConnectorInstallationId {
    pub fn new(value: &str) -> Result<Self, ConnectorError> {
        Ok(Self(normalize_reference(value, ConnectorError::InvalidId)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorInstallation {
    id: ConnectorInstallationId,
    workspace_id_hash: String,
    connector_id: ConnectorId,
    credential_reference: ConnectorCredentialReference,
    scopes: ConnectorScopeSet,
    state: ConnectorInstallationState,
}

impl ConnectorInstallation {
    pub fn new(
        id: ConnectorInstallationId,
        workspace_id_hash: &str,
        connector_id: ConnectorId,
        credential_reference: ConnectorCredentialReference,
        scopes: ConnectorScopeSet,
        state: ConnectorInstallationState,
    ) -> Result<Self, ConnectorError> {
        Ok(Self {
            id,
            workspace_id_hash: normalize_reference(workspace_id_hash, ConnectorError::InvalidId)?,
            connector_id,
            credential_reference,
            scopes,
            state,
        })
    }

    pub fn id(&self) -> &ConnectorInstallationId {
        &self.id
    }

    pub fn workspace_id_hash(&self) -> &str {
        &self.workspace_id_hash
    }

    pub fn connector_id(&self) -> &ConnectorId {
        &self.connector_id
    }

    pub fn credential_reference(&self) -> &ConnectorCredentialReference {
        &self.credential_reference
    }

    pub fn scopes(&self) -> &ConnectorScopeSet {
        &self.scopes
    }

    pub const fn state(&self) -> ConnectorInstallationState {
        self.state
    }

    pub fn with_state(&self, state: ConnectorInstallationState) -> Self {
        Self {
            id: self.id.clone(),
            workspace_id_hash: self.workspace_id_hash.clone(),
            connector_id: self.connector_id.clone(),
            credential_reference: self.credential_reference.clone(),
            scopes: self.scopes.clone(),
            state,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorInstallationState {
    NotInstalled,
    AuthorizationRequested,
    Installed,
    AuthorizationFailed,
    SyncQueued,
    Syncing,
    Synced,
    RetryScheduled,
    Failed,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorInstallationEvent {
    RequestAuthorization,
    Authorize,
    FailAuthorization,
    QueueSync,
    StartSync,
    CompleteSync,
    ScheduleRetry,
    Retry,
    FailSync,
    Disable,
}

pub fn transition_connector_installation(
    state: ConnectorInstallationState,
    event: ConnectorInstallationEvent,
) -> Result<ConnectorInstallationState, ConnectorError> {
    use ConnectorInstallationEvent as Event;
    use ConnectorInstallationState as State;

    match (state, event) {
        (State::NotInstalled, Event::RequestAuthorization) => Ok(State::AuthorizationRequested),
        (State::AuthorizationRequested, Event::Authorize) => Ok(State::Installed),
        (State::AuthorizationRequested, Event::FailAuthorization) => Ok(State::AuthorizationFailed),
        (State::AuthorizationFailed, Event::RequestAuthorization) => {
            Ok(State::AuthorizationRequested)
        }
        (State::Installed, Event::QueueSync) | (State::Synced, Event::QueueSync) => {
            Ok(State::SyncQueued)
        }
        (State::SyncQueued, Event::StartSync) | (State::RetryScheduled, Event::Retry) => {
            Ok(State::Syncing)
        }
        (State::Syncing, Event::CompleteSync) => Ok(State::Synced),
        (State::Syncing, Event::ScheduleRetry) => Ok(State::RetryScheduled),
        (State::Syncing, Event::FailSync) => Ok(State::Failed),
        (State::Installed, Event::Disable)
        | (State::AuthorizationRequested, Event::Disable)
        | (State::AuthorizationFailed, Event::Disable)
        | (State::SyncQueued, Event::Disable)
        | (State::Syncing, Event::Disable)
        | (State::Synced, Event::Disable)
        | (State::RetryScheduled, Event::Disable)
        | (State::Failed, Event::Disable) => Ok(State::Disabled),
        _ => Err(ConnectorError::InvalidTransition),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorError {
    InvalidId,
    InvalidName,
    EmptyScope,
    ScopeDenied,
    InvalidCredentialReference,
    InvalidExternalObjectReference,
    InvalidActivity,
    InvalidTransition,
}

impl ConnectorError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidId => "connector.invalid_id",
            Self::InvalidName => "connector.invalid_name",
            Self::EmptyScope => "connector.empty_scope",
            Self::ScopeDenied => "connector.scope_denied",
            Self::InvalidCredentialReference => "connector.invalid_credential_reference",
            Self::InvalidExternalObjectReference => "connector.invalid_external_object_reference",
            Self::InvalidActivity => "connector.invalid_activity",
            Self::InvalidTransition => "connector.invalid_transition",
        }
    }
}

fn normalize_reference(value: &str, error: ConnectorError) -> Result<String, ConnectorError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(error);
    }
    Ok(trimmed.to_string())
}

fn contains_sensitive_connector_fixture(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    lowered.contains("connector_access_token_fixture")
        || lowered.contains("connector_refresh_token_fixture")
        || lowered.contains("connector_client_secret_fixture")
        || lowered.contains("connector_payload")
        || lowered.contains("oauth_token")
        || lowered.contains("client_secret")
}
