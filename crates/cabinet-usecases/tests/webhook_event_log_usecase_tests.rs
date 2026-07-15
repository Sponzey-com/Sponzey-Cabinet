use cabinet_domain::webhook::{
    DeadLetterEntry, EventDeliveryJobId, EventEnvelope, EventMetadataSummary, EventType,
    WebhookError,
};
use cabinet_ports::webhook::{
    DeadLetterStorePort, EventLogPort, WebhookDeliveryPortError, WebhookEventLogPortError,
};
use cabinet_usecases::webhook::{
    AppendIntegrationEventInput, AppendIntegrationEventUsecase, ListDeadLettersInput,
    ListDeadLettersUsecase, WebhookEventUsecaseError,
};

#[derive(Default)]
struct FakeEventLog {
    events: Vec<EventEnvelope>,
    fail: bool,
}

impl EventLogPort for FakeEventLog {
    fn append_event(&mut self, event: EventEnvelope) -> Result<(), WebhookEventLogPortError> {
        if self.fail {
            return Err(WebhookEventLogPortError::StoreUnavailable);
        }
        self.events.push(event);
        Ok(())
    }

    fn list_events(
        &self,
        workspace_id_hash: &str,
    ) -> Result<Vec<EventEnvelope>, WebhookEventLogPortError> {
        Ok(self
            .events
            .iter()
            .filter(|event| event.workspace_id_hash() == workspace_id_hash)
            .cloned()
            .collect())
    }
}

#[derive(Default)]
struct FakeDeadLetterStore {
    entries: Vec<DeadLetterEntry>,
    fail: bool,
}

impl DeadLetterStorePort for FakeDeadLetterStore {
    fn save_dead_letter(&mut self, entry: DeadLetterEntry) -> Result<(), WebhookDeliveryPortError> {
        self.entries.push(entry);
        Ok(())
    }

    fn list_dead_letters(
        &self,
        workspace_id_hash: &str,
    ) -> Result<Vec<DeadLetterEntry>, WebhookDeliveryPortError> {
        if self.fail {
            return Err(WebhookDeliveryPortError::StoreUnavailable);
        }
        Ok(self
            .entries
            .iter()
            .filter(|entry| entry.workspace_id_hash() == Some(workspace_id_hash))
            .cloned()
            .collect())
    }
}

#[test]
fn append_integration_event_stores_envelope_without_raw_payload() {
    let mut event_log = FakeEventLog::default();

    let output = AppendIntegrationEventUsecase::new()
        .execute(
            AppendIntegrationEventInput::new(envelope("event-1", "workspace-hash-1")),
            &mut event_log,
        )
        .expect("append event");

    assert_eq!(output.event_id(), "event-1");
    assert_eq!(output.workspace_id_hash(), "workspace-hash-1");
    assert_eq!(event_log.events.len(), 1);
    assert_eq!(
        event_log.events[0].metadata_summary().as_str(),
        "metadata:event.updated"
    );
    assert_eq!(
        EventMetadataSummary::new("metadata:webhook_payload_body_fixture"),
        Err(WebhookError::SensitiveMetadata),
    );
}

#[test]
fn append_integration_event_maps_store_failure_to_stable_error_code() {
    let mut event_log = FakeEventLog {
        events: Vec::new(),
        fail: true,
    };

    let error = AppendIntegrationEventUsecase::new()
        .execute(
            AppendIntegrationEventInput::new(envelope("event-1", "workspace-hash-1")),
            &mut event_log,
        )
        .expect_err("store failure");

    assert_eq!(error, WebhookEventUsecaseError::EventLogUnavailable);
    assert_eq!(error.code(), "webhook_event.event_log_unavailable");
}

#[test]
fn list_dead_letters_filters_by_workspace_hash() {
    let mut store = FakeDeadLetterStore::default();
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

    let output = ListDeadLettersUsecase::new()
        .execute(ListDeadLettersInput::new("workspace-hash-1"), &store)
        .expect("list dead letters");

    assert_eq!(output.entries().len(), 1);
    assert_eq!(output.entries()[0].id(), "dead-letter-1");
    assert_eq!(
        output.entries()[0].reason_code(),
        "webhook.delivery.transport_unavailable"
    );
}

#[test]
fn list_dead_letters_maps_store_failure_to_stable_error_code() {
    let store = FakeDeadLetterStore {
        entries: Vec::new(),
        fail: true,
    };

    let error = ListDeadLettersUsecase::new()
        .execute(ListDeadLettersInput::new("workspace-hash-1"), &store)
        .expect_err("store failure");

    assert_eq!(error, WebhookEventUsecaseError::DeadLetterStoreUnavailable);
    assert_eq!(error.code(), "webhook_event.dead_letter_store_unavailable");
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
