use cabinet_domain::webhook::{
    EventEnvelope, EventMetadataSummary, EventSubscription, EventSubscriptionId,
    EventSubscriptionState, EventType, WebhookDestination, WebhookDestinationReference,
    WebhookError, WebhookSecretReference,
};

#[test]
fn event_envelope_uses_hash_reference_and_metadata_summary_only() {
    let envelope = EventEnvelope::new(
        "event-1",
        "workspace-hash-1",
        "actor-hash-1",
        "document-1",
        EventType::DocumentUpdated,
        EventMetadataSummary::new("metadata:document.updated").expect("metadata"),
    )
    .expect("envelope");

    assert_eq!(envelope.event_id(), "event-1");
    assert_eq!(envelope.workspace_id_hash(), "workspace-hash-1");
    assert_eq!(envelope.actor_id_hash(), "actor-hash-1");
    assert_eq!(envelope.resource_id(), "document-1");
    assert_eq!(envelope.event_type(), EventType::DocumentUpdated);
    assert_eq!(
        envelope.metadata_summary().as_str(),
        "metadata:document.updated"
    );
    assert_eq!(
        EventMetadataSummary::new("metadata:document_body_fixture"),
        Err(WebhookError::SensitiveMetadata),
    );
    assert_eq!(
        EventMetadataSummary::new("metadata:comment_body_fixture"),
        Err(WebhookError::SensitiveMetadata),
    );
}

#[test]
fn webhook_destination_uses_references_not_raw_url_or_secret() {
    let destination = WebhookDestination::new(
        WebhookDestinationReference::new("destination:customer-support").expect("destination"),
        WebhookSecretReference::new("secret-ref:webhook-customer-support").expect("secret"),
    );

    assert_eq!(
        destination.reference().as_str(),
        "destination:customer-support",
    );
    assert_eq!(
        destination.secret_reference().as_str(),
        "secret-ref:webhook-customer-support",
    );
    assert_eq!(
        WebhookDestinationReference::new("https://example.com/webhook"),
        Err(WebhookError::InvalidDestinationReference),
    );
    assert_eq!(
        WebhookSecretReference::new("webhook_secret_fixture"),
        Err(WebhookError::InvalidSecretReference),
    );
}

#[test]
fn event_subscription_requires_event_types_and_can_be_disabled() {
    let destination = destination();
    let subscription = EventSubscription::new(
        EventSubscriptionId::new("subscription-1").expect("subscription id"),
        "workspace-hash-1",
        destination.clone(),
        vec![EventType::DocumentUpdated, EventType::AiAnswerCompleted],
    )
    .expect("subscription");

    assert_eq!(subscription.id().as_str(), "subscription-1");
    assert_eq!(subscription.workspace_id_hash(), "workspace-hash-1");
    assert_eq!(subscription.destination(), &destination);
    assert_eq!(subscription.event_types().len(), 2);
    assert_eq!(subscription.state(), EventSubscriptionState::Enabled);
    assert_eq!(
        subscription.disable().state(),
        EventSubscriptionState::Disabled,
    );
    assert_eq!(
        EventSubscription::new(
            EventSubscriptionId::new("subscription-2").expect("subscription id"),
            "workspace-hash-1",
            destination,
            vec![],
        ),
        Err(WebhookError::EmptyEventTypes),
    );
}

fn destination() -> WebhookDestination {
    WebhookDestination::new(
        WebhookDestinationReference::new("destination:customer-support").expect("destination"),
        WebhookSecretReference::new("secret-ref:webhook-customer-support").expect("secret"),
    )
}
