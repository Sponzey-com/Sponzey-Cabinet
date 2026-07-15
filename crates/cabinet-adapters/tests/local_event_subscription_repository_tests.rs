use cabinet_adapters::local_event_subscription_repository::LocalEventSubscriptionRepository;
use cabinet_domain::webhook::{
    EventSubscription, EventSubscriptionId, EventSubscriptionState, EventType, WebhookDestination,
    WebhookDestinationReference, WebhookSecretReference,
};
use cabinet_ports::webhook::EventSubscriptionRepositoryPort;

#[test]
fn local_event_subscription_repository_saves_finds_and_lists_by_workspace() {
    let mut repository = LocalEventSubscriptionRepository::default();
    repository
        .save_subscription(subscription("subscription-1", "workspace-hash-1"))
        .expect("save");
    repository
        .save_subscription(subscription("subscription-2", "workspace-hash-2"))
        .expect("save");

    let found = repository
        .find_subscription(&EventSubscriptionId::new("subscription-1").expect("subscription id"))
        .expect("find")
        .expect("subscription");
    let workspace_one = repository
        .list_subscriptions("workspace-hash-1")
        .expect("list");

    assert_eq!(found.id().as_str(), "subscription-1");
    assert_eq!(workspace_one.len(), 1);
    assert_eq!(workspace_one[0].workspace_id_hash(), "workspace-hash-1");
}

#[test]
fn local_event_subscription_repository_replaces_disabled_subscription() {
    let mut repository = LocalEventSubscriptionRepository::default();
    let enabled = subscription("subscription-1", "workspace-hash-1");
    let disabled = enabled.disable();
    repository.save_subscription(enabled).expect("save");
    repository
        .save_subscription(disabled)
        .expect("replace disabled");

    let found = repository
        .find_subscription(&EventSubscriptionId::new("subscription-1").expect("subscription id"))
        .expect("find")
        .expect("subscription");

    assert_eq!(found.state(), EventSubscriptionState::Disabled);
}

fn subscription(id: &str, workspace_id_hash: &str) -> EventSubscription {
    EventSubscription::new(
        EventSubscriptionId::new(id).expect("subscription id"),
        workspace_id_hash,
        WebhookDestination::new(
            WebhookDestinationReference::new("destination:customer-support").expect("destination"),
            WebhookSecretReference::new("secret-ref:webhook-customer-support").expect("secret"),
        ),
        vec![EventType::DocumentUpdated],
    )
    .expect("subscription")
}
