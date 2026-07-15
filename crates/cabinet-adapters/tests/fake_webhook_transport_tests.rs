use cabinet_adapters::fake_webhook_transport::FakeWebhookTransport;
use cabinet_adapters::local_dead_letter_store::LocalDeadLetterStore;
use cabinet_domain::webhook::{
    DeadLetterEntry, EventDeliveryJobId, EventEnvelope, EventMetadataSummary, EventType,
    WebhookDestination, WebhookDestinationReference, WebhookSecretReference, WebhookSignature,
};
use cabinet_ports::webhook::{DeadLetterStorePort, WebhookDeliveryPortError, WebhookTransportPort};

#[test]
fn fake_webhook_transport_returns_configured_success_or_failure() {
    let success = FakeWebhookTransport::succeeding();
    success
        .send_event(&envelope(), &destination(), &signature())
        .expect("sent");
    assert_eq!(success.call_count(), 1);

    let failing = FakeWebhookTransport::failing(WebhookDeliveryPortError::TransportUnavailable);
    assert_eq!(
        failing.send_event(&envelope(), &destination(), &signature()),
        Err(WebhookDeliveryPortError::TransportUnavailable),
    );
    assert_eq!(failing.call_count(), 1);
}

#[test]
fn local_dead_letter_store_saves_entries_without_payload_body() {
    let mut store = LocalDeadLetterStore::default();
    let entry = DeadLetterEntry::new(
        "dead-letter-1",
        EventDeliveryJobId::new("delivery-1").expect("job id"),
        "webhook.delivery.transport_unavailable",
    )
    .expect("dead letter");

    store.save_dead_letter(entry.clone()).expect("save");

    assert_eq!(store.entries().len(), 1);
    assert_eq!(store.entries()[0], entry);
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

fn destination() -> WebhookDestination {
    WebhookDestination::new(
        WebhookDestinationReference::new("destination:customer-support").expect("destination"),
        WebhookSecretReference::new("secret-ref:webhook-customer-support").expect("secret"),
    )
}

fn signature() -> WebhookSignature {
    WebhookSignature::new("signature:delivery-1").expect("signature")
}
