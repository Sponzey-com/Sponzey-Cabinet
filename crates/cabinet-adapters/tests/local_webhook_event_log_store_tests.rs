use cabinet_adapters::local_dead_letter_store::LocalDeadLetterStore;
use cabinet_adapters::local_event_log_store::LocalEventLogStore;
use cabinet_domain::webhook::{
    DeadLetterEntry, EventDeliveryJobId, EventEnvelope, EventMetadataSummary, EventType,
    WebhookError,
};
use cabinet_ports::webhook::{DeadLetterStorePort, EventLogPort};

#[test]
fn local_event_log_store_appends_and_lists_by_workspace_without_raw_payload() {
    let mut store = LocalEventLogStore::default();

    store
        .append_event(envelope("event-1", "workspace-hash-1"))
        .expect("append first");
    store
        .append_event(envelope("event-2", "workspace-hash-2"))
        .expect("append second");

    let events = store
        .list_events("workspace-hash-1")
        .expect("list workspace events");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id(), "event-1");
    assert_eq!(
        events[0].metadata_summary().as_str(),
        "metadata:event.updated"
    );
    assert_eq!(
        EventMetadataSummary::new("metadata:webhook_payload_body_fixture"),
        Err(WebhookError::SensitiveMetadata),
    );
}

#[test]
fn local_dead_letter_store_lists_entries_by_workspace() {
    let mut store = LocalDeadLetterStore::default();

    store
        .save_dead_letter(dead_letter(
            "dead-letter-1",
            "delivery-1",
            "workspace-hash-1",
        ))
        .expect("save first");
    store
        .save_dead_letter(dead_letter(
            "dead-letter-2",
            "delivery-2",
            "workspace-hash-2",
        ))
        .expect("save second");

    let entries = store
        .list_dead_letters("workspace-hash-1")
        .expect("list workspace dead letters");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id(), "dead-letter-1");
    assert_eq!(
        entries[0].reason_code(),
        "webhook.delivery.transport_unavailable"
    );
}

fn envelope(event_id: &str, workspace_id_hash: &str) -> EventEnvelope {
    EventEnvelope::new(
        event_id,
        workspace_id_hash,
        "actor-hash-1",
        "document-1",
        EventType::DocumentUpdated,
        EventMetadataSummary::new("metadata:event.updated").expect("metadata"),
    )
    .expect("envelope")
}

fn dead_letter(id: &str, job_id: &str, workspace_id_hash: &str) -> DeadLetterEntry {
    DeadLetterEntry::new_for_workspace(
        id,
        EventDeliveryJobId::new(job_id).expect("job id"),
        workspace_id_hash,
        "webhook.delivery.transport_unavailable",
    )
    .expect("dead letter")
}
