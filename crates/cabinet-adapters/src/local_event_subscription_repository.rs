use std::collections::HashMap;

use cabinet_domain::webhook::{EventSubscription, EventSubscriptionId};
use cabinet_ports::webhook::{EventSubscriptionRepositoryPort, WebhookRepositoryError};

#[derive(Debug, Default)]
pub struct LocalEventSubscriptionRepository {
    records: HashMap<String, EventSubscription>,
}

impl LocalEventSubscriptionRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl EventSubscriptionRepositoryPort for LocalEventSubscriptionRepository {
    fn save_subscription(
        &mut self,
        subscription: EventSubscription,
    ) -> Result<(), WebhookRepositoryError> {
        self.records
            .insert(subscription.id().as_str().to_string(), subscription);
        Ok(())
    }

    fn find_subscription(
        &self,
        id: &EventSubscriptionId,
    ) -> Result<Option<EventSubscription>, WebhookRepositoryError> {
        Ok(self.records.get(id.as_str()).cloned())
    }

    fn list_subscriptions(
        &self,
        workspace_id_hash: &str,
    ) -> Result<Vec<EventSubscription>, WebhookRepositoryError> {
        Ok(self
            .records
            .values()
            .filter(|subscription| subscription.workspace_id_hash() == workspace_id_hash)
            .cloned()
            .collect())
    }
}
