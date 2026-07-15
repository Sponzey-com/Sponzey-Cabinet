use cabinet_domain::webhook::DeadLetterEntry;
use cabinet_ports::webhook::{DeadLetterStorePort, WebhookDeliveryPortError};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LocalDeadLetterStore {
    entries: Vec<DeadLetterEntry>,
}

impl LocalDeadLetterStore {
    pub fn entries(&self) -> &[DeadLetterEntry] {
        &self.entries
    }
}

impl DeadLetterStorePort for LocalDeadLetterStore {
    fn save_dead_letter(&mut self, entry: DeadLetterEntry) -> Result<(), WebhookDeliveryPortError> {
        self.entries.push(entry);
        Ok(())
    }

    fn list_dead_letters(
        &self,
        workspace_id_hash: &str,
    ) -> Result<Vec<DeadLetterEntry>, WebhookDeliveryPortError> {
        Ok(self
            .entries
            .iter()
            .filter(|entry| entry.workspace_id_hash() == Some(workspace_id_hash))
            .cloned()
            .collect())
    }
}
