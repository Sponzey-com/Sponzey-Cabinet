use cabinet_domain::notification::{ChangeNotification, ChangeNotificationEventType};
use cabinet_ports::notification::{
    NotificationDeliveryError, NotificationDeliveryReceipt, NotificationPort,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocalNotificationStubPolicy {
    unsupported_event_types: Vec<ChangeNotificationEventType>,
    delivery_failure: bool,
}

impl LocalNotificationStubPolicy {
    pub fn with_unsupported_event_type(mut self, event_type: ChangeNotificationEventType) -> Self {
        self.unsupported_event_types.push(event_type);
        self
    }

    pub const fn with_delivery_failure(mut self, delivery_failure: bool) -> Self {
        self.delivery_failure = delivery_failure;
        self
    }

    fn supports(&self, event_type: ChangeNotificationEventType) -> bool {
        !self.unsupported_event_types.contains(&event_type)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalNotificationStub {
    policy: LocalNotificationStubPolicy,
    recorded_notifications: Vec<ChangeNotification>,
}

impl LocalNotificationStub {
    pub fn new(policy: LocalNotificationStubPolicy) -> Self {
        Self {
            policy,
            recorded_notifications: Vec::new(),
        }
    }

    pub fn recorded_notifications(&self) -> &[ChangeNotification] {
        &self.recorded_notifications
    }
}

impl NotificationPort for LocalNotificationStub {
    fn deliver(
        &mut self,
        notification: ChangeNotification,
    ) -> Result<NotificationDeliveryReceipt, NotificationDeliveryError> {
        if !self.policy.supports(notification.event_type()) {
            return Err(NotificationDeliveryError::UnsupportedEventType);
        }
        if self.policy.delivery_failure {
            return Err(NotificationDeliveryError::DeliveryFailed);
        }
        if self.is_duplicate(&notification) {
            return Err(NotificationDeliveryError::DuplicateEvent);
        }

        self.recorded_notifications.push(notification.clone());
        Ok(NotificationDeliveryReceipt::delivered(
            notification,
            self.recorded_notifications.len(),
            0,
        ))
    }
}

impl LocalNotificationStub {
    fn is_duplicate(&self, notification: &ChangeNotification) -> bool {
        self.recorded_notifications.iter().any(|recorded| {
            recorded.correlation_id() == notification.correlation_id()
                && recorded.event_type() == notification.event_type()
                && recorded.target().target_kind() == notification.target().target_kind()
                && recorded.target().target_id() == notification.target().target_id()
        })
    }
}
