use cabinet_adapters::local_notification::{LocalNotificationStub, LocalNotificationStubPolicy};
use cabinet_domain::document::DocumentId;
use cabinet_domain::notification::{
    ChangeNotification, ChangeNotificationCorrelationId, ChangeNotificationEventType,
    ChangeNotificationTarget, ChangeNotificationTimestamp,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::notification::{
    NotificationDeliveryError, NotificationDeliveryStatus, NotificationPort,
};

#[test]
fn local_notification_stub_records_delivered_events_without_transport() {
    let mut stub = LocalNotificationStub::new(LocalNotificationStubPolicy::default());
    let notification = notification(ChangeNotificationEventType::DocumentChanged, "corr-1");

    let receipt = stub.deliver(notification.clone()).expect("delivered");

    assert_eq!(receipt.status(), NotificationDeliveryStatus::Delivered);
    assert_eq!(receipt.delivery_count(), 1);
    assert_eq!(receipt.retry_count(), 0);
    assert_eq!(stub.recorded_notifications(), &[notification]);
}

#[test]
fn local_notification_stub_rejects_duplicate_correlation_target_and_event() {
    let mut stub = LocalNotificationStub::new(LocalNotificationStubPolicy::default());
    let first = notification(ChangeNotificationEventType::CommentChanged, "corr-1");
    let duplicate = first.clone();

    stub.deliver(first).expect("first delivered");
    let error = stub.deliver(duplicate).expect_err("duplicate rejected");

    assert_eq!(error, NotificationDeliveryError::DuplicateEvent);
    assert_eq!(stub.recorded_notifications().len(), 1);
}

#[test]
fn local_notification_stub_can_reject_unsupported_event_type() {
    let mut stub = LocalNotificationStub::new(
        LocalNotificationStubPolicy::default()
            .with_unsupported_event_type(ChangeNotificationEventType::ReviewStateChanged),
    );

    let error = stub
        .deliver(notification(
            ChangeNotificationEventType::ReviewStateChanged,
            "corr-review",
        ))
        .expect_err("unsupported event type");

    assert_eq!(error, NotificationDeliveryError::UnsupportedEventType);
    assert!(stub.recorded_notifications().is_empty());
}

#[test]
fn local_notification_stub_can_simulate_delivery_failure_without_recording_payload() {
    let mut stub = LocalNotificationStub::new(
        LocalNotificationStubPolicy::default().with_delivery_failure(true),
    );

    let error = stub
        .deliver(notification(
            ChangeNotificationEventType::LockStateChanged,
            "corr-lock",
        ))
        .expect_err("delivery failed");

    assert_eq!(error, NotificationDeliveryError::DeliveryFailed);
    assert!(stub.recorded_notifications().is_empty());
}

fn notification(
    event_type: ChangeNotificationEventType,
    correlation_id: &str,
) -> ChangeNotification {
    ChangeNotification::new(
        WorkspaceId::new("workspace-1").expect("workspace id"),
        UserId::new("actor-1").expect("actor id"),
        ChangeNotificationTarget::document(DocumentId::new("doc-1").expect("document id")),
        event_type,
        ChangeNotificationTimestamp::from_millis(10_000),
        ChangeNotificationCorrelationId::new(correlation_id).expect("correlation id"),
    )
}
