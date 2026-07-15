use cabinet_domain::document::DocumentId;
use cabinet_domain::notification::{
    ChangeNotification, ChangeNotificationCorrelationId, ChangeNotificationEventType,
    ChangeNotificationTarget, ChangeNotificationTimestamp,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::notification::{
    NotificationDeliveryError, NotificationDeliveryReceipt, NotificationDeliveryStatus,
    NotificationPort,
};

#[test]
fn notification_port_accepts_change_notification_without_transport_types() {
    let mut port = FakeNotificationPort::default();
    let notification = notification(ChangeNotificationEventType::DocumentChanged, "corr-1");

    let receipt = port
        .deliver(notification.clone())
        .expect("notification delivered");

    assert_eq!(receipt.notification(), &notification);
    assert_eq!(receipt.status(), NotificationDeliveryStatus::Delivered);
    assert_eq!(receipt.delivery_count(), 1);
    assert_eq!(receipt.retry_count(), 0);
    assert_eq!(port.delivered, vec![notification]);
}

#[test]
fn notification_delivery_receipt_exposes_limited_states() {
    let notification = notification(ChangeNotificationEventType::LockStateChanged, "corr-1");

    let queued = NotificationDeliveryReceipt::queued(notification.clone(), 0, 0);
    assert_eq!(queued.status(), NotificationDeliveryStatus::Queued);
    assert_eq!(queued.status().as_str(), "queued");

    let delivered = NotificationDeliveryReceipt::delivered(notification.clone(), 1, 0);
    assert_eq!(delivered.status(), NotificationDeliveryStatus::Delivered);
    assert_eq!(delivered.status().as_str(), "delivered");

    let failed = NotificationDeliveryReceipt::failed(notification, 1, 1);
    assert_eq!(failed.status(), NotificationDeliveryStatus::Failed);
    assert_eq!(failed.status().as_str(), "failed");
}

#[test]
fn notification_delivery_errors_expose_stable_codes() {
    assert_eq!(
        NotificationDeliveryError::UnsupportedEventType.code(),
        "notification.unsupported_event_type"
    );
    assert_eq!(
        NotificationDeliveryError::DuplicateEvent.code(),
        "notification.duplicate_event"
    );
    assert_eq!(
        NotificationDeliveryError::DeliveryFailed.code(),
        "notification.delivery_failed"
    );
}

#[derive(Default)]
struct FakeNotificationPort {
    delivered: Vec<ChangeNotification>,
}

impl NotificationPort for FakeNotificationPort {
    fn deliver(
        &mut self,
        notification: ChangeNotification,
    ) -> Result<NotificationDeliveryReceipt, NotificationDeliveryError> {
        self.delivered.push(notification.clone());
        Ok(NotificationDeliveryReceipt::delivered(notification, 1, 0))
    }
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
