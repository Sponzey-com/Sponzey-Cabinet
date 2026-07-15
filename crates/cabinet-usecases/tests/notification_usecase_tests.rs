use cabinet_domain::notification::{ChangeNotificationEventType, ChangeNotificationTimestamp};
use cabinet_ports::notification::{
    NotificationDeliveryError, NotificationDeliveryReceipt, NotificationPort,
};
use cabinet_usecases::notification::{
    ChangeNotificationSideEffectRequest, NotificationFieldDebugEvent, NotificationProductEvent,
    NotificationUsecaseError, NotificationUsecaseLogger, SendChangeNotificationInput,
    SendChangeNotificationOutputStatus, SendChangeNotificationUsecase,
};

#[test]
fn side_effect_request_maps_document_comment_review_and_lock_events() {
    let occurred_at = ChangeNotificationTimestamp::from_millis(10_000);

    let document = ChangeNotificationSideEffectRequest::document_changed(
        "actor-1",
        "workspace-1",
        "doc-1",
        occurred_at,
        "corr-document",
    )
    .expect("document notification");
    assert_eq!(
        document.notification().event_type(),
        ChangeNotificationEventType::DocumentChanged
    );
    assert_eq!(document.notification().target().target_kind(), "document");
    assert_eq!(document.notification().target().target_id(), "doc-1");

    let comment = ChangeNotificationSideEffectRequest::comment_changed(
        "actor-1",
        "workspace-1",
        "doc-1",
        "thread-1",
        occurred_at,
        "corr-comment",
    )
    .expect("comment notification");
    assert_eq!(
        comment.notification().event_type(),
        ChangeNotificationEventType::CommentChanged
    );
    assert_eq!(
        comment.notification().target().target_kind(),
        "comment_thread"
    );
    assert_eq!(comment.notification().target().target_id(), "thread-1");

    let review = ChangeNotificationSideEffectRequest::review_state_changed(
        "actor-1",
        "workspace-1",
        "doc-1",
        "review-1",
        occurred_at,
        "corr-review",
    )
    .expect("review notification");
    assert_eq!(
        review.notification().event_type(),
        ChangeNotificationEventType::ReviewStateChanged
    );
    assert_eq!(
        review.notification().target().target_kind(),
        "review_request"
    );
    assert_eq!(review.notification().target().target_id(), "review-1");

    let lock = ChangeNotificationSideEffectRequest::lock_state_changed(
        "actor-1",
        "workspace-1",
        "doc-1",
        "lock-1",
        occurred_at,
        "corr-lock",
    )
    .expect("lock notification");
    assert_eq!(
        lock.notification().event_type(),
        ChangeNotificationEventType::LockStateChanged
    );
    assert_eq!(lock.notification().target().target_kind(), "document_lock");
    assert_eq!(lock.notification().target().target_id(), "lock-1");
}

#[test]
fn send_change_notification_delivers_through_port_and_writes_field_debug_summary() {
    let request = ChangeNotificationSideEffectRequest::lock_state_changed(
        "actor-1",
        "workspace-1",
        "doc-1",
        "lock-1",
        ChangeNotificationTimestamp::from_millis(10_000),
        "corr-lock",
    )
    .expect("notification request");
    let mut port = FakeNotificationPort::default();
    let mut logger = FakeNotificationLogger::default();

    let output = SendChangeNotificationUsecase::new()
        .execute(
            SendChangeNotificationInput::new(request),
            &mut port,
            &mut logger,
        )
        .expect("notification delivered");

    assert_eq!(
        output.status(),
        SendChangeNotificationOutputStatus::Delivered
    );
    assert_eq!(port.delivery_count, 1);
    assert!(logger.product_events.is_empty());
    assert_eq!(logger.field_debug_events.len(), 1);
    assert_eq!(
        logger.field_debug_events[0].event_type(),
        "lock.state_changed"
    );
    assert_eq!(
        logger.field_debug_events[0].masked_actor_id(),
        "masked:or-1"
    );
    assert_eq!(logger.field_debug_events[0].delivery_count(), 1);
    assert_eq!(logger.field_debug_events[0].retry_count(), 0);
}

#[test]
fn send_change_notification_logs_product_event_only_on_delivery_failure() {
    let request = ChangeNotificationSideEffectRequest::comment_changed(
        "actor-1",
        "workspace-1",
        "doc-1",
        "thread-1",
        ChangeNotificationTimestamp::from_millis(10_000),
        "corr-comment",
    )
    .expect("notification request");
    let mut port = FakeNotificationPort {
        error: Some(NotificationDeliveryError::DeliveryFailed),
        ..FakeNotificationPort::default()
    };
    let mut logger = FakeNotificationLogger::default();

    let error = SendChangeNotificationUsecase::new()
        .execute(
            SendChangeNotificationInput::new(request),
            &mut port,
            &mut logger,
        )
        .expect_err("delivery should fail");

    assert_eq!(error, NotificationUsecaseError::DeliveryFailed);
    assert_eq!(logger.product_events.len(), 1);
    assert!(matches!(
        logger.product_events[0],
        NotificationProductEvent::DeliveryFailed {
            masked_target_id: ref target,
            event_type: "comment.changed",
            error_code: "NOTIFICATION_DELIVERY_FAILED",
        } if target == "masked:ad-1"
    ));
}

#[test]
fn send_change_notification_maps_unsupported_and_duplicate_errors_to_stable_codes() {
    let cases = [
        (
            NotificationDeliveryError::UnsupportedEventType,
            NotificationUsecaseError::UnsupportedEventType,
            "NOTIFICATION_UNSUPPORTED_EVENT_TYPE",
        ),
        (
            NotificationDeliveryError::DuplicateEvent,
            NotificationUsecaseError::DuplicateEvent,
            "NOTIFICATION_DUPLICATE_EVENT",
        ),
    ];

    for (port_error, expected_error, expected_code) in cases {
        let request = ChangeNotificationSideEffectRequest::review_state_changed(
            "actor-1",
            "workspace-1",
            "doc-1",
            "review-1",
            ChangeNotificationTimestamp::from_millis(10_000),
            "corr-review",
        )
        .expect("notification request");
        let mut port = FakeNotificationPort {
            error: Some(port_error),
            ..FakeNotificationPort::default()
        };
        let mut logger = FakeNotificationLogger::default();

        let error = SendChangeNotificationUsecase::new()
            .execute(
                SendChangeNotificationInput::new(request),
                &mut port,
                &mut logger,
            )
            .expect_err("delivery should fail");

        assert_eq!(error, expected_error);
        assert!(matches!(
            logger.product_events.last(),
            Some(NotificationProductEvent::DeliveryFailed {
                event_type: "review.state_changed",
                error_code,
                ..
            }) if *error_code == expected_code
        ));
    }
}

#[test]
fn notification_request_and_logs_do_not_include_raw_document_comment_or_asset_content() {
    let request = ChangeNotificationSideEffectRequest::document_changed(
        "actor-1",
        "workspace-1",
        "doc-1",
        ChangeNotificationTimestamp::from_millis(10_000),
        "corr-document",
    )
    .expect("notification request");
    let event = NotificationProductEvent::DeliveryFailed {
        masked_target_id: "masked:doc-1".to_string(),
        event_type: "document.changed",
        error_code: "NOTIFICATION_DELIVERY_FAILED",
    };

    let request_debug = format!("{request:?}");
    let event_debug = format!("{event:?}");

    assert!(!request_debug.contains("secret document body"));
    assert!(!request_debug.contains("secret comment body"));
    assert!(!request_debug.contains("asset bytes"));
    assert!(!event_debug.contains("secret document body"));
    assert!(!event_debug.contains("secret comment body"));
    assert!(!event_debug.contains("asset bytes"));
}

#[derive(Default)]
struct FakeNotificationPort {
    delivery_count: usize,
    error: Option<NotificationDeliveryError>,
}

impl NotificationPort for FakeNotificationPort {
    fn deliver(
        &mut self,
        notification: cabinet_domain::notification::ChangeNotification,
    ) -> Result<NotificationDeliveryReceipt, NotificationDeliveryError> {
        self.delivery_count += 1;
        if let Some(error) = self.error {
            return Err(error);
        }
        Ok(NotificationDeliveryReceipt::delivered(
            notification,
            self.delivery_count,
            0,
        ))
    }
}

#[derive(Default)]
struct FakeNotificationLogger {
    product_events: Vec<NotificationProductEvent>,
    field_debug_events: Vec<NotificationFieldDebugEvent>,
}

impl NotificationUsecaseLogger for FakeNotificationLogger {
    fn write_product(&mut self, event: NotificationProductEvent) {
        self.product_events.push(event);
    }

    fn write_field_debug(&mut self, event: NotificationFieldDebugEvent) {
        self.field_debug_events.push(event);
    }
}
