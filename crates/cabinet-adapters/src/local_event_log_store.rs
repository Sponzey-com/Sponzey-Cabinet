use cabinet_domain::webhook::EventEnvelope;
use cabinet_ports::webhook::{EventLogPort, WebhookEventLogPortError};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LocalEventLogStore {
    events: Vec<EventEnvelope>,
}

impl LocalEventLogStore {
    pub fn events(&self) -> &[EventEnvelope] {
        &self.events
    }
}

impl EventLogPort for LocalEventLogStore {
    fn append_event(&mut self, event: EventEnvelope) -> Result<(), WebhookEventLogPortError> {
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
