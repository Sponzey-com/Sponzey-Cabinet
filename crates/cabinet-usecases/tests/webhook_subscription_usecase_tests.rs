use std::collections::HashMap;

use cabinet_domain::webhook::{
    EventSubscription, EventSubscriptionId, EventSubscriptionState, EventType, WebhookDestination,
    WebhookDestinationReference, WebhookSecretReference,
};
use cabinet_ports::webhook::{EventSubscriptionRepositoryPort, WebhookRepositoryError};
use cabinet_usecases::webhook::{
    CreateEventSubscriptionInput, CreateEventSubscriptionUsecase, DisableEventSubscriptionInput,
    DisableEventSubscriptionUsecase, ListEventSubscriptionsInput, ListEventSubscriptionsUsecase,
    WebhookSubscriptionUsecaseError,
};

#[derive(Default)]
struct FakeSubscriptionRepository {
    records: HashMap<String, EventSubscription>,
}

impl EventSubscriptionRepositoryPort for FakeSubscriptionRepository {
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

#[test]
fn create_subscription_saves_and_list_returns_workspace_records() {
    let mut repository = FakeSubscriptionRepository::default();
    let created = CreateEventSubscriptionUsecase::new()
        .execute(
            create_input("subscription-1", "workspace-hash-1"),
            &mut repository,
        )
        .expect("created");

    assert_eq!(created.subscription().id().as_str(), "subscription-1");
    assert_eq!(
        created.subscription().state(),
        EventSubscriptionState::Enabled
    );

    let listed = ListEventSubscriptionsUsecase::new()
        .execute(
            ListEventSubscriptionsInput::new("workspace-hash-1"),
            &repository,
        )
        .expect("listed");

    assert_eq!(listed.subscriptions().len(), 1);
    assert_eq!(listed.subscriptions()[0].id().as_str(), "subscription-1");
}

#[test]
fn disable_subscription_updates_state_to_disabled() {
    let mut repository = FakeSubscriptionRepository::default();
    CreateEventSubscriptionUsecase::new()
        .execute(
            create_input("subscription-1", "workspace-hash-1"),
            &mut repository,
        )
        .expect("created");

    let disabled = DisableEventSubscriptionUsecase::new()
        .execute(
            DisableEventSubscriptionInput::new(
                EventSubscriptionId::new("subscription-1").expect("subscription id"),
            ),
            &mut repository,
        )
        .expect("disabled");

    assert_eq!(
        disabled.subscription().state(),
        EventSubscriptionState::Disabled
    );
    assert_eq!(
        repository
            .find_subscription(
                &EventSubscriptionId::new("subscription-1").expect("subscription id")
            )
            .expect("found")
            .expect("subscription")
            .state(),
        EventSubscriptionState::Disabled,
    );
}

#[test]
fn disable_missing_subscription_returns_stable_error() {
    let mut repository = FakeSubscriptionRepository::default();
    let error = DisableEventSubscriptionUsecase::new()
        .execute(
            DisableEventSubscriptionInput::new(
                EventSubscriptionId::new("missing").expect("subscription id"),
            ),
            &mut repository,
        )
        .expect_err("missing");

    assert_eq!(error, WebhookSubscriptionUsecaseError::SubscriptionNotFound);
    assert_eq!(error.code(), "webhook_subscription.subscription_not_found");
}

fn create_input(subscription_id: &str, workspace_id_hash: &str) -> CreateEventSubscriptionInput {
    CreateEventSubscriptionInput::new(
        EventSubscriptionId::new(subscription_id).expect("subscription id"),
        workspace_id_hash,
        WebhookDestination::new(
            WebhookDestinationReference::new("destination:customer-support").expect("destination"),
            WebhookSecretReference::new("secret-ref:webhook-customer-support").expect("secret"),
        ),
        vec![EventType::DocumentUpdated],
    )
}
