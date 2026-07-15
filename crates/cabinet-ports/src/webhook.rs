use cabinet_domain::webhook::{
    DeadLetterEntry, EventEnvelope, EventSubscription, EventSubscriptionId, WebhookDestination,
    WebhookSignature,
};

pub trait EventSubscriptionRepositoryPort {
    fn save_subscription(
        &mut self,
        subscription: EventSubscription,
    ) -> Result<(), WebhookRepositoryError>;

    fn find_subscription(
        &self,
        id: &EventSubscriptionId,
    ) -> Result<Option<EventSubscription>, WebhookRepositoryError>;

    fn list_subscriptions(
        &self,
        workspace_id_hash: &str,
    ) -> Result<Vec<EventSubscription>, WebhookRepositoryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookRepositoryError {
    StoreUnavailable,
}

pub trait WebhookTransportPort {
    fn send_event(
        &self,
        event: &EventEnvelope,
        destination: &WebhookDestination,
        signature: &WebhookSignature,
    ) -> Result<(), WebhookDeliveryPortError>;
}

pub trait DeadLetterStorePort {
    fn save_dead_letter(&mut self, entry: DeadLetterEntry) -> Result<(), WebhookDeliveryPortError>;

    fn list_dead_letters(
        &self,
        _workspace_id_hash: &str,
    ) -> Result<Vec<DeadLetterEntry>, WebhookDeliveryPortError> {
        Err(WebhookDeliveryPortError::StoreUnavailable)
    }
}

pub trait EventLogPort {
    fn append_event(&mut self, event: EventEnvelope) -> Result<(), WebhookEventLogPortError>;

    fn list_events(
        &self,
        workspace_id_hash: &str,
    ) -> Result<Vec<EventEnvelope>, WebhookEventLogPortError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookDeliveryPortError {
    TransportUnavailable,
    StoreUnavailable,
}

impl WebhookDeliveryPortError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::TransportUnavailable => "webhook_delivery_port.transport_unavailable",
            Self::StoreUnavailable => "webhook_delivery_port.store_unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookEventLogPortError {
    StoreUnavailable,
}

impl WebhookEventLogPortError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StoreUnavailable => "webhook_event_log.store_unavailable",
        }
    }
}

impl WebhookRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StoreUnavailable => "webhook_repository.store_unavailable",
        }
    }
}
