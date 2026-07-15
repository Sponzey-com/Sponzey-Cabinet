use std::cell::Cell;

use cabinet_domain::webhook::{
    DeadLetterEntry, EventDeliveryJobId, EventDeliveryJobState, EventEnvelope,
    EventMetadataSummary, EventSubscription, EventSubscriptionId, EventType, WebhookDestination,
    WebhookDestinationReference, WebhookSecretReference, WebhookSignature,
};
use cabinet_ports::webhook::{DeadLetterStorePort, WebhookDeliveryPortError, WebhookTransportPort};
use cabinet_usecases::webhook::{
    DeliverWebhookEventInput, DeliverWebhookEventUsecase, WebhookDeliveryUsecaseError,
};

struct FakeTransport {
    fail: bool,
    call_count: Cell<usize>,
}

impl WebhookTransportPort for FakeTransport {
    fn send_event(
        &self,
        _event: &EventEnvelope,
        _destination: &WebhookDestination,
        _signature: &WebhookSignature,
    ) -> Result<(), WebhookDeliveryPortError> {
        self.call_count.set(self.call_count.get() + 1);
        if self.fail {
            return Err(WebhookDeliveryPortError::TransportUnavailable);
        }
        Ok(())
    }
}

#[derive(Default)]
struct FakeDeadLetterStore {
    entries: Vec<DeadLetterEntry>,
}

impl DeadLetterStorePort for FakeDeadLetterStore {
    fn save_dead_letter(&mut self, entry: DeadLetterEntry) -> Result<(), WebhookDeliveryPortError> {
        self.entries.push(entry);
        Ok(())
    }
}

#[test]
fn deliver_webhook_event_skips_disabled_subscription_without_transport_call() {
    let subscription = subscription().disable();
    let transport = FakeTransport {
        fail: false,
        call_count: Cell::new(0),
    };
    let mut dead_letters = FakeDeadLetterStore::default();

    let output = DeliverWebhookEventUsecase::new()
        .execute(input(subscription, 1, 3), &transport, &mut dead_letters)
        .expect("skipped");

    assert_eq!(output.state(), None);
    assert!(output.skipped_disabled_subscription());
    assert_eq!(transport.call_count.get(), 0);
    assert!(dead_letters.entries.is_empty());
}

#[test]
fn deliver_webhook_event_marks_delivered_on_transport_success() {
    let transport = FakeTransport {
        fail: false,
        call_count: Cell::new(0),
    };
    let mut dead_letters = FakeDeadLetterStore::default();

    let output = DeliverWebhookEventUsecase::new()
        .execute(input(subscription(), 1, 3), &transport, &mut dead_letters)
        .expect("delivered");

    assert_eq!(output.state(), Some(EventDeliveryJobState::Delivered));
    assert_eq!(transport.call_count.get(), 1);
    assert!(dead_letters.entries.is_empty());
}

#[test]
fn deliver_webhook_event_schedules_retry_before_limit() {
    let transport = FakeTransport {
        fail: true,
        call_count: Cell::new(0),
    };
    let mut dead_letters = FakeDeadLetterStore::default();

    let output = DeliverWebhookEventUsecase::new()
        .execute(input(subscription(), 1, 3), &transport, &mut dead_letters)
        .expect("retry");

    assert_eq!(output.state(), Some(EventDeliveryJobState::RetryScheduled));
    assert_eq!(transport.call_count.get(), 1);
    assert!(dead_letters.entries.is_empty());
}

#[test]
fn deliver_webhook_event_dead_letters_after_retry_limit() {
    let transport = FakeTransport {
        fail: true,
        call_count: Cell::new(0),
    };
    let mut dead_letters = FakeDeadLetterStore::default();

    let output = DeliverWebhookEventUsecase::new()
        .execute(input(subscription(), 3, 3), &transport, &mut dead_letters)
        .expect("dead letter");

    assert_eq!(output.state(), Some(EventDeliveryJobState::DeadLettered));
    assert_eq!(dead_letters.entries.len(), 1);
    assert_eq!(
        dead_letters.entries[0].reason_code(),
        "webhook.delivery.transport_unavailable",
    );
}

#[test]
fn deliver_webhook_event_rejects_invalid_attempt_policy() {
    let transport = FakeTransport {
        fail: false,
        call_count: Cell::new(0),
    };
    let mut dead_letters = FakeDeadLetterStore::default();

    let error = DeliverWebhookEventUsecase::new()
        .execute(input(subscription(), 4, 3), &transport, &mut dead_letters)
        .expect_err("invalid policy");

    assert_eq!(error, WebhookDeliveryUsecaseError::InvalidRetryPolicy);
    assert_eq!(error.code(), "webhook_delivery.invalid_retry_policy");
}

fn input(
    subscription: EventSubscription,
    attempt: u32,
    max_attempts: u32,
) -> DeliverWebhookEventInput {
    DeliverWebhookEventInput::new(
        EventDeliveryJobId::new("delivery-1").expect("job id"),
        subscription,
        envelope(),
        WebhookSignature::new("signature:delivery-1").expect("signature"),
        attempt,
        max_attempts,
    )
}

fn subscription() -> EventSubscription {
    EventSubscription::new(
        EventSubscriptionId::new("subscription-1").expect("subscription id"),
        "workspace-hash-1",
        WebhookDestination::new(
            WebhookDestinationReference::new("destination:customer-support").expect("destination"),
            WebhookSecretReference::new("secret-ref:webhook-customer-support").expect("secret"),
        ),
        vec![EventType::DocumentUpdated],
    )
    .expect("subscription")
}

fn envelope() -> EventEnvelope {
    EventEnvelope::new(
        "event-1",
        "workspace-hash-1",
        "actor-hash-1",
        "document-1",
        EventType::DocumentUpdated,
        EventMetadataSummary::new("metadata:document.updated").expect("metadata"),
    )
    .expect("envelope")
}
