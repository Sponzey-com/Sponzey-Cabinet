use cabinet_domain::webhook::{
    DeadLetterEntry, EventDeliveryJob, EventDeliveryJobEvent, EventDeliveryJobId,
    EventDeliveryJobState, EventEnvelope, EventSubscription, EventSubscriptionId,
    EventSubscriptionState, EventType, WebhookDestination, WebhookError, WebhookSignature,
    transition_event_delivery_job,
};
use cabinet_ports::webhook::EventLogPort;
use cabinet_ports::webhook::{
    DeadLetterStorePort, EventSubscriptionRepositoryPort, WebhookDeliveryPortError,
    WebhookEventLogPortError, WebhookRepositoryError, WebhookTransportPort,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateEventSubscriptionInput {
    id: EventSubscriptionId,
    workspace_id_hash: String,
    destination: WebhookDestination,
    event_types: Vec<EventType>,
}

impl CreateEventSubscriptionInput {
    pub fn new(
        id: EventSubscriptionId,
        workspace_id_hash: &str,
        destination: WebhookDestination,
        event_types: Vec<EventType>,
    ) -> Self {
        Self {
            id,
            workspace_id_hash: workspace_id_hash.to_string(),
            destination,
            event_types,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisableEventSubscriptionInput {
    id: EventSubscriptionId,
}

impl DisableEventSubscriptionInput {
    pub fn new(id: EventSubscriptionId) -> Self {
        Self { id }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEventSubscriptionsInput {
    workspace_id_hash: String,
}

impl ListEventSubscriptionsInput {
    pub fn new(workspace_id_hash: &str) -> Self {
        Self {
            workspace_id_hash: workspace_id_hash.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSubscriptionOutput {
    subscription: EventSubscription,
}

impl EventSubscriptionOutput {
    fn new(subscription: EventSubscription) -> Self {
        Self { subscription }
    }

    pub fn subscription(&self) -> &EventSubscription {
        &self.subscription
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEventSubscriptionsOutput {
    subscriptions: Vec<EventSubscription>,
}

impl ListEventSubscriptionsOutput {
    fn new(subscriptions: Vec<EventSubscription>) -> Self {
        Self { subscriptions }
    }

    pub fn subscriptions(&self) -> &[EventSubscription] {
        &self.subscriptions
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateEventSubscriptionUsecase;

impl CreateEventSubscriptionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: CreateEventSubscriptionInput,
        repository: &mut impl EventSubscriptionRepositoryPort,
    ) -> Result<EventSubscriptionOutput, WebhookSubscriptionUsecaseError> {
        let subscription = EventSubscription::new(
            input.id,
            &input.workspace_id_hash,
            input.destination,
            input.event_types,
        )
        .map_err(WebhookSubscriptionUsecaseError::from_domain_error)?;
        repository
            .save_subscription(subscription.clone())
            .map_err(WebhookSubscriptionUsecaseError::from_repository_error)?;
        Ok(EventSubscriptionOutput::new(subscription))
    }
}

impl Default for CreateEventSubscriptionUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisableEventSubscriptionUsecase;

impl DisableEventSubscriptionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: DisableEventSubscriptionInput,
        repository: &mut impl EventSubscriptionRepositoryPort,
    ) -> Result<EventSubscriptionOutput, WebhookSubscriptionUsecaseError> {
        let subscription = repository
            .find_subscription(&input.id)
            .map_err(WebhookSubscriptionUsecaseError::from_repository_error)?
            .ok_or(WebhookSubscriptionUsecaseError::SubscriptionNotFound)?;
        let disabled = subscription.disable();
        repository
            .save_subscription(disabled.clone())
            .map_err(WebhookSubscriptionUsecaseError::from_repository_error)?;
        Ok(EventSubscriptionOutput::new(disabled))
    }
}

impl Default for DisableEventSubscriptionUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListEventSubscriptionsUsecase;

impl ListEventSubscriptionsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ListEventSubscriptionsInput,
        repository: &impl EventSubscriptionRepositoryPort,
    ) -> Result<ListEventSubscriptionsOutput, WebhookSubscriptionUsecaseError> {
        let subscriptions = repository
            .list_subscriptions(&input.workspace_id_hash)
            .map_err(WebhookSubscriptionUsecaseError::from_repository_error)?;
        Ok(ListEventSubscriptionsOutput::new(subscriptions))
    }
}

impl Default for ListEventSubscriptionsUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendIntegrationEventInput {
    event: EventEnvelope,
}

impl AppendIntegrationEventInput {
    pub fn new(event: EventEnvelope) -> Self {
        Self { event }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendIntegrationEventOutput {
    event_id: String,
    workspace_id_hash: String,
}

impl AppendIntegrationEventOutput {
    fn new(event: &EventEnvelope) -> Self {
        Self {
            event_id: event.event_id().to_string(),
            workspace_id_hash: event.workspace_id_hash().to_string(),
        }
    }

    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn workspace_id_hash(&self) -> &str {
        &self.workspace_id_hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListDeadLettersInput {
    workspace_id_hash: String,
}

impl ListDeadLettersInput {
    pub fn new(workspace_id_hash: &str) -> Self {
        Self {
            workspace_id_hash: workspace_id_hash.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListDeadLettersOutput {
    entries: Vec<DeadLetterEntry>,
}

impl ListDeadLettersOutput {
    fn new(entries: Vec<DeadLetterEntry>) -> Self {
        Self { entries }
    }

    pub fn entries(&self) -> &[DeadLetterEntry] {
        &self.entries
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppendIntegrationEventUsecase;

impl AppendIntegrationEventUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: AppendIntegrationEventInput,
        event_log: &mut impl EventLogPort,
    ) -> Result<AppendIntegrationEventOutput, WebhookEventUsecaseError> {
        let output = AppendIntegrationEventOutput::new(&input.event);
        event_log
            .append_event(input.event)
            .map_err(WebhookEventUsecaseError::from_event_log_error)?;
        Ok(output)
    }
}

impl Default for AppendIntegrationEventUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListDeadLettersUsecase;

impl ListDeadLettersUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ListDeadLettersInput,
        dead_letter_store: &impl DeadLetterStorePort,
    ) -> Result<ListDeadLettersOutput, WebhookEventUsecaseError> {
        if input.workspace_id_hash.trim().is_empty()
            || input.workspace_id_hash.chars().any(char::is_control)
        {
            return Err(WebhookEventUsecaseError::InvalidInput);
        }

        let entries = dead_letter_store
            .list_dead_letters(&input.workspace_id_hash)
            .map_err(WebhookEventUsecaseError::from_dead_letter_store_error)?;
        Ok(ListDeadLettersOutput::new(entries))
    }
}

impl Default for ListDeadLettersUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliverWebhookEventInput {
    job_id: EventDeliveryJobId,
    subscription: EventSubscription,
    event: EventEnvelope,
    signature: WebhookSignature,
    attempt: u32,
    max_attempts: u32,
}

impl DeliverWebhookEventInput {
    pub fn new(
        job_id: EventDeliveryJobId,
        subscription: EventSubscription,
        event: EventEnvelope,
        signature: WebhookSignature,
        attempt: u32,
        max_attempts: u32,
    ) -> Self {
        Self {
            job_id,
            subscription,
            event,
            signature,
            attempt,
            max_attempts,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeliverWebhookEventOutput {
    state: Option<EventDeliveryJobState>,
    skipped_disabled_subscription: bool,
}

impl DeliverWebhookEventOutput {
    fn new(state: Option<EventDeliveryJobState>, skipped_disabled_subscription: bool) -> Self {
        Self {
            state,
            skipped_disabled_subscription,
        }
    }

    pub const fn state(self) -> Option<EventDeliveryJobState> {
        self.state
    }

    pub const fn skipped_disabled_subscription(self) -> bool {
        self.skipped_disabled_subscription
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeliverWebhookEventUsecase;

impl DeliverWebhookEventUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: DeliverWebhookEventInput,
        transport: &impl WebhookTransportPort,
        dead_letter_store: &mut impl DeadLetterStorePort,
    ) -> Result<DeliverWebhookEventOutput, WebhookDeliveryUsecaseError> {
        if input.attempt == 0 || input.max_attempts == 0 || input.attempt > input.max_attempts {
            return Err(WebhookDeliveryUsecaseError::InvalidRetryPolicy);
        }

        if input.subscription.state() == EventSubscriptionState::Disabled {
            return Ok(DeliverWebhookEventOutput::new(None, true));
        }

        let job = EventDeliveryJob::new(
            input.job_id.clone(),
            input.event.clone(),
            input.max_attempts,
        )
        .map_err(WebhookDeliveryUsecaseError::from_domain_error)?;
        let signing = transition_event_delivery_job(job.state(), EventDeliveryJobEvent::Sign)
            .map_err(WebhookDeliveryUsecaseError::from_domain_error)?;
        let sending = transition_event_delivery_job(signing, EventDeliveryJobEvent::Send)
            .map_err(WebhookDeliveryUsecaseError::from_domain_error)?;

        match transport.send_event(
            job.event(),
            input.subscription.destination(),
            &input.signature,
        ) {
            Ok(()) => {
                let delivered =
                    transition_event_delivery_job(sending, EventDeliveryJobEvent::Deliver)
                        .map_err(WebhookDeliveryUsecaseError::from_domain_error)?;
                Ok(DeliverWebhookEventOutput::new(Some(delivered), false))
            }
            Err(WebhookDeliveryPortError::TransportUnavailable) => {
                if input.attempt < job.max_attempts() {
                    let retry =
                        transition_event_delivery_job(sending, EventDeliveryJobEvent::Retry)
                            .map_err(WebhookDeliveryUsecaseError::from_domain_error)?;
                    return Ok(DeliverWebhookEventOutput::new(Some(retry), false));
                }

                let dead_lettered =
                    transition_event_delivery_job(sending, EventDeliveryJobEvent::DeadLetter)
                        .map_err(WebhookDeliveryUsecaseError::from_domain_error)?;
                let dead_letter = DeadLetterEntry::new_for_workspace(
                    &format!("dead-letter:{}", job.id().as_str()),
                    job.id().clone(),
                    job.event().workspace_id_hash(),
                    "webhook.delivery.transport_unavailable",
                )
                .map_err(WebhookDeliveryUsecaseError::from_domain_error)?;
                dead_letter_store
                    .save_dead_letter(dead_letter)
                    .map_err(WebhookDeliveryUsecaseError::from_port_error)?;
                Ok(DeliverWebhookEventOutput::new(Some(dead_lettered), false))
            }
            Err(WebhookDeliveryPortError::StoreUnavailable) => {
                Err(WebhookDeliveryUsecaseError::StoreUnavailable)
            }
        }
    }
}

impl Default for DeliverWebhookEventUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookSubscriptionUsecaseError {
    InvalidInput,
    SubscriptionNotFound,
    StoreUnavailable,
}

impl WebhookSubscriptionUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "webhook_subscription.invalid_input",
            Self::SubscriptionNotFound => "webhook_subscription.subscription_not_found",
            Self::StoreUnavailable => "webhook_subscription.store_unavailable",
        }
    }

    fn from_domain_error(_error: WebhookError) -> Self {
        Self::InvalidInput
    }

    fn from_repository_error(error: WebhookRepositoryError) -> Self {
        match error {
            WebhookRepositoryError::StoreUnavailable => Self::StoreUnavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookDeliveryUsecaseError {
    InvalidRetryPolicy,
    InvalidInput,
    InvalidTransition,
    StoreUnavailable,
}

impl WebhookDeliveryUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidRetryPolicy => "webhook_delivery.invalid_retry_policy",
            Self::InvalidInput => "webhook_delivery.invalid_input",
            Self::InvalidTransition => "webhook_delivery.invalid_transition",
            Self::StoreUnavailable => "webhook_delivery.store_unavailable",
        }
    }

    fn from_domain_error(error: WebhookError) -> Self {
        match error {
            WebhookError::InvalidRetryPolicy => Self::InvalidRetryPolicy,
            WebhookError::InvalidDeliveryTransition => Self::InvalidTransition,
            WebhookError::InvalidId
            | WebhookError::InvalidMetadata
            | WebhookError::SensitiveMetadata
            | WebhookError::InvalidDestinationReference
            | WebhookError::InvalidSecretReference
            | WebhookError::InvalidSignatureReference
            | WebhookError::EmptyEventTypes => Self::InvalidInput,
        }
    }

    fn from_port_error(error: WebhookDeliveryPortError) -> Self {
        match error {
            WebhookDeliveryPortError::TransportUnavailable => Self::InvalidTransition,
            WebhookDeliveryPortError::StoreUnavailable => Self::StoreUnavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookEventUsecaseError {
    InvalidInput,
    EventLogUnavailable,
    DeadLetterStoreUnavailable,
}

impl WebhookEventUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "webhook_event.invalid_input",
            Self::EventLogUnavailable => "webhook_event.event_log_unavailable",
            Self::DeadLetterStoreUnavailable => "webhook_event.dead_letter_store_unavailable",
        }
    }

    fn from_event_log_error(error: WebhookEventLogPortError) -> Self {
        match error {
            WebhookEventLogPortError::StoreUnavailable => Self::EventLogUnavailable,
        }
    }

    fn from_dead_letter_store_error(error: WebhookDeliveryPortError) -> Self {
        match error {
            WebhookDeliveryPortError::TransportUnavailable
            | WebhookDeliveryPortError::StoreUnavailable => Self::DeadLetterStoreUnavailable,
        }
    }
}
