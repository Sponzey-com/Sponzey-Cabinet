use cabinet_domain::webhook::{
    DeadLetterEntry, EventDeliveryJob, EventDeliveryJobEvent, EventDeliveryJobId,
    EventDeliveryJobState, EventEnvelope, EventMetadataSummary, EventType, WebhookError,
    WebhookSignature, transition_event_delivery_job,
};

#[test]
fn webhook_signature_requires_signature_reference_not_raw_secret() {
    let signature = WebhookSignature::new("signature:delivery-1").expect("signature");

    assert_eq!(signature.as_str(), "signature:delivery-1");
    assert_eq!(
        WebhookSignature::new("webhook_secret_fixture"),
        Err(WebhookError::InvalidSignatureReference),
    );
}

#[test]
fn event_delivery_job_starts_queued_with_event_reference() {
    let job = EventDeliveryJob::new(
        EventDeliveryJobId::new("delivery-1").expect("job id"),
        envelope("event-1"),
        3,
    )
    .expect("job");

    assert_eq!(job.id().as_str(), "delivery-1");
    assert_eq!(job.event().event_id(), "event-1");
    assert_eq!(job.max_attempts(), 3);
    assert_eq!(job.state(), EventDeliveryJobState::Queued);
    assert_eq!(
        EventDeliveryJob::new(
            EventDeliveryJobId::new("delivery-2").expect("job id"),
            envelope("event-2"),
            0,
        ),
        Err(WebhookError::InvalidRetryPolicy),
    );
}

#[test]
fn event_delivery_state_machine_supports_success_retry_and_dead_letter_paths() {
    let signing =
        transition_event_delivery_job(EventDeliveryJobState::Queued, EventDeliveryJobEvent::Sign)
            .expect("signing");
    let sending =
        transition_event_delivery_job(signing, EventDeliveryJobEvent::Send).expect("sending");
    let delivered =
        transition_event_delivery_job(sending, EventDeliveryJobEvent::Deliver).expect("delivered");

    assert_eq!(signing, EventDeliveryJobState::Signing);
    assert_eq!(sending, EventDeliveryJobState::Sending);
    assert_eq!(delivered, EventDeliveryJobState::Delivered);
    assert_eq!(
        transition_event_delivery_job(EventDeliveryJobState::Sending, EventDeliveryJobEvent::Retry)
            .expect("retry"),
        EventDeliveryJobState::RetryScheduled,
    );
    assert_eq!(
        transition_event_delivery_job(
            EventDeliveryJobState::RetryScheduled,
            EventDeliveryJobEvent::Send,
        )
        .expect("resend"),
        EventDeliveryJobState::Sending,
    );
    assert_eq!(
        transition_event_delivery_job(
            EventDeliveryJobState::Sending,
            EventDeliveryJobEvent::DeadLetter,
        )
        .expect("dead-letter"),
        EventDeliveryJobState::DeadLettered,
    );
}

#[test]
fn event_delivery_state_machine_rejects_invalid_transitions() {
    assert_eq!(
        transition_event_delivery_job(
            EventDeliveryJobState::Queued,
            EventDeliveryJobEvent::Deliver
        ),
        Err(WebhookError::InvalidDeliveryTransition),
    );
    assert_eq!(
        transition_event_delivery_job(
            EventDeliveryJobState::Delivered,
            EventDeliveryJobEvent::Send
        ),
        Err(WebhookError::InvalidDeliveryTransition),
    );
}

#[test]
fn dead_letter_entry_uses_reason_code_not_payload_body() {
    let entry = DeadLetterEntry::new(
        "dead-letter-1",
        EventDeliveryJobId::new("delivery-1").expect("job id"),
        "webhook.delivery.retry_exhausted",
    )
    .expect("dead letter");

    assert_eq!(entry.id(), "dead-letter-1");
    assert_eq!(entry.delivery_job_id().as_str(), "delivery-1");
    assert_eq!(entry.reason_code(), "webhook.delivery.retry_exhausted");
    assert_eq!(
        DeadLetterEntry::new(
            "dead-letter-2",
            EventDeliveryJobId::new("delivery-1").expect("job id"),
            "document_body_fixture",
        ),
        Err(WebhookError::SensitiveMetadata),
    );
}

fn envelope(event_id: &str) -> EventEnvelope {
    EventEnvelope::new(
        event_id,
        "workspace-hash-1",
        "actor-hash-1",
        "document-1",
        EventType::DocumentUpdated,
        EventMetadataSummary::new("metadata:document.updated").expect("metadata"),
    )
    .expect("envelope")
}
