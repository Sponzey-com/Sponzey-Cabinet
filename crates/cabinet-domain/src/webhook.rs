#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    DocumentUpdated,
    AttachmentChanged,
    CommentCreated,
    ReviewStateChanged,
    GraphChanged,
    CanvasChanged,
    AiAnswerCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventMetadataSummary(String);

impl EventMetadataSummary {
    pub fn new(value: &str) -> Result<Self, WebhookError> {
        let trimmed = value.trim();
        if !trimmed.starts_with("metadata:")
            || trimmed.len() <= "metadata:".len()
            || trimmed.chars().any(char::is_control)
        {
            return Err(WebhookError::InvalidMetadata);
        }
        if contains_sensitive_fixture(trimmed) {
            return Err(WebhookError::SensitiveMetadata);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventEnvelope {
    event_id: String,
    workspace_id_hash: String,
    actor_id_hash: String,
    resource_id: String,
    event_type: EventType,
    metadata_summary: EventMetadataSummary,
}

impl EventEnvelope {
    pub fn new(
        event_id: &str,
        workspace_id_hash: &str,
        actor_id_hash: &str,
        resource_id: &str,
        event_type: EventType,
        metadata_summary: EventMetadataSummary,
    ) -> Result<Self, WebhookError> {
        Ok(Self {
            event_id: normalize_id(event_id)?,
            workspace_id_hash: normalize_id(workspace_id_hash)?,
            actor_id_hash: normalize_id(actor_id_hash)?,
            resource_id: normalize_id(resource_id)?,
            event_type,
            metadata_summary,
        })
    }

    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn workspace_id_hash(&self) -> &str {
        &self.workspace_id_hash
    }

    pub fn actor_id_hash(&self) -> &str {
        &self.actor_id_hash
    }

    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    pub const fn event_type(&self) -> EventType {
        self.event_type
    }

    pub fn metadata_summary(&self) -> &EventMetadataSummary {
        &self.metadata_summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookDestinationReference(String);

impl WebhookDestinationReference {
    pub fn new(value: &str) -> Result<Self, WebhookError> {
        let trimmed = value.trim();
        if !trimmed.starts_with("destination:")
            || trimmed.len() <= "destination:".len()
            || trimmed.chars().any(char::is_control)
            || trimmed.contains("://")
        {
            return Err(WebhookError::InvalidDestinationReference);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookSecretReference(String);

impl WebhookSecretReference {
    pub fn new(value: &str) -> Result<Self, WebhookError> {
        let trimmed = value.trim();
        if !trimmed.starts_with("secret-ref:")
            || trimmed.len() <= "secret-ref:".len()
            || trimmed.chars().any(char::is_control)
            || contains_sensitive_fixture(trimmed)
        {
            return Err(WebhookError::InvalidSecretReference);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookDestination {
    reference: WebhookDestinationReference,
    secret_reference: WebhookSecretReference,
}

impl WebhookDestination {
    pub fn new(
        reference: WebhookDestinationReference,
        secret_reference: WebhookSecretReference,
    ) -> Self {
        Self {
            reference,
            secret_reference,
        }
    }

    pub fn reference(&self) -> &WebhookDestinationReference {
        &self.reference
    }

    pub fn secret_reference(&self) -> &WebhookSecretReference {
        &self.secret_reference
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSubscriptionId(String);

impl EventSubscriptionId {
    pub fn new(value: &str) -> Result<Self, WebhookError> {
        Ok(Self(normalize_id(value)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSubscriptionState {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSubscription {
    id: EventSubscriptionId,
    workspace_id_hash: String,
    destination: WebhookDestination,
    event_types: Vec<EventType>,
    state: EventSubscriptionState,
}

impl EventSubscription {
    pub fn new(
        id: EventSubscriptionId,
        workspace_id_hash: &str,
        destination: WebhookDestination,
        event_types: Vec<EventType>,
    ) -> Result<Self, WebhookError> {
        if event_types.is_empty() {
            return Err(WebhookError::EmptyEventTypes);
        }
        Ok(Self {
            id,
            workspace_id_hash: normalize_id(workspace_id_hash)?,
            destination,
            event_types,
            state: EventSubscriptionState::Enabled,
        })
    }

    pub fn id(&self) -> &EventSubscriptionId {
        &self.id
    }

    pub fn workspace_id_hash(&self) -> &str {
        &self.workspace_id_hash
    }

    pub fn destination(&self) -> &WebhookDestination {
        &self.destination
    }

    pub fn event_types(&self) -> &[EventType] {
        &self.event_types
    }

    pub const fn state(&self) -> EventSubscriptionState {
        self.state
    }

    pub fn disable(&self) -> Self {
        Self {
            id: self.id.clone(),
            workspace_id_hash: self.workspace_id_hash.clone(),
            destination: self.destination.clone(),
            event_types: self.event_types.clone(),
            state: EventSubscriptionState::Disabled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookSignature(String);

impl WebhookSignature {
    pub fn new(value: &str) -> Result<Self, WebhookError> {
        let trimmed = value.trim();
        if !trimmed.starts_with("signature:")
            || trimmed.len() <= "signature:".len()
            || trimmed.chars().any(char::is_control)
            || contains_sensitive_fixture(trimmed)
        {
            return Err(WebhookError::InvalidSignatureReference);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDeliveryJobId(String);

impl EventDeliveryJobId {
    pub fn new(value: &str) -> Result<Self, WebhookError> {
        Ok(Self(normalize_id(value)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDeliveryJob {
    id: EventDeliveryJobId,
    event: EventEnvelope,
    max_attempts: u32,
    state: EventDeliveryJobState,
}

impl EventDeliveryJob {
    pub fn new(
        id: EventDeliveryJobId,
        event: EventEnvelope,
        max_attempts: u32,
    ) -> Result<Self, WebhookError> {
        if max_attempts == 0 {
            return Err(WebhookError::InvalidRetryPolicy);
        }
        Ok(Self {
            id,
            event,
            max_attempts,
            state: EventDeliveryJobState::Queued,
        })
    }

    pub fn id(&self) -> &EventDeliveryJobId {
        &self.id
    }

    pub fn event(&self) -> &EventEnvelope {
        &self.event
    }

    pub const fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    pub const fn state(&self) -> EventDeliveryJobState {
        self.state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDeliveryJobState {
    Queued,
    Signing,
    Sending,
    Delivered,
    RetryScheduled,
    DeadLettered,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDeliveryJobEvent {
    Sign,
    Send,
    Deliver,
    Retry,
    DeadLetter,
    Fail,
}

pub fn transition_event_delivery_job(
    state: EventDeliveryJobState,
    event: EventDeliveryJobEvent,
) -> Result<EventDeliveryJobState, WebhookError> {
    use EventDeliveryJobEvent as Event;
    use EventDeliveryJobState as State;

    match (state, event) {
        (State::Queued, Event::Sign) => Ok(State::Signing),
        (State::Signing, Event::Send) => Ok(State::Sending),
        (State::RetryScheduled, Event::Send) => Ok(State::Sending),
        (State::Sending, Event::Deliver) => Ok(State::Delivered),
        (State::Sending, Event::Retry) => Ok(State::RetryScheduled),
        (State::Sending, Event::DeadLetter) => Ok(State::DeadLettered),
        (State::Signing, Event::Fail) | (State::Sending, Event::Fail) => Ok(State::Failed),
        _ => Err(WebhookError::InvalidDeliveryTransition),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadLetterEntry {
    id: String,
    delivery_job_id: EventDeliveryJobId,
    workspace_id_hash: Option<String>,
    reason_code: String,
}

impl DeadLetterEntry {
    pub fn new(
        id: &str,
        delivery_job_id: EventDeliveryJobId,
        reason_code: &str,
    ) -> Result<Self, WebhookError> {
        let id = normalize_id(id)?;
        let reason_code = reason_code.trim();
        if reason_code.is_empty() || reason_code.chars().any(char::is_control) {
            return Err(WebhookError::InvalidMetadata);
        }
        if contains_sensitive_fixture(reason_code) {
            return Err(WebhookError::SensitiveMetadata);
        }
        Ok(Self {
            id,
            delivery_job_id,
            workspace_id_hash: None,
            reason_code: reason_code.to_string(),
        })
    }

    pub fn new_for_workspace(
        id: &str,
        delivery_job_id: EventDeliveryJobId,
        workspace_id_hash: &str,
        reason_code: &str,
    ) -> Result<Self, WebhookError> {
        let mut entry = Self::new(id, delivery_job_id, reason_code)?;
        entry.workspace_id_hash = Some(normalize_id(workspace_id_hash)?);
        Ok(entry)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn delivery_job_id(&self) -> &EventDeliveryJobId {
        &self.delivery_job_id
    }

    pub fn workspace_id_hash(&self) -> Option<&str> {
        self.workspace_id_hash.as_deref()
    }

    pub fn reason_code(&self) -> &str {
        &self.reason_code
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookError {
    InvalidId,
    InvalidMetadata,
    SensitiveMetadata,
    InvalidDestinationReference,
    InvalidSecretReference,
    InvalidSignatureReference,
    EmptyEventTypes,
    InvalidRetryPolicy,
    InvalidDeliveryTransition,
}

impl WebhookError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidId => "webhook.invalid_id",
            Self::InvalidMetadata => "webhook.invalid_metadata",
            Self::SensitiveMetadata => "webhook.sensitive_metadata",
            Self::InvalidDestinationReference => "webhook.invalid_destination_reference",
            Self::InvalidSecretReference => "webhook.invalid_secret_reference",
            Self::InvalidSignatureReference => "webhook.invalid_signature_reference",
            Self::EmptyEventTypes => "webhook.empty_event_types",
            Self::InvalidRetryPolicy => "webhook.invalid_retry_policy",
            Self::InvalidDeliveryTransition => "webhook.invalid_delivery_transition",
        }
    }
}

fn normalize_id(value: &str) -> Result<String, WebhookError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(WebhookError::InvalidId);
    }
    Ok(trimmed.to_string())
}

fn contains_sensitive_fixture(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    lowered.contains("document_body_fixture")
        || lowered.contains("comment_body_fixture")
        || lowered.contains("attachment_body_fixture")
        || lowered.contains("connector_payload")
        || lowered.contains("webhook_secret_fixture")
        || lowered.contains("webhook_payload_body_fixture")
}
