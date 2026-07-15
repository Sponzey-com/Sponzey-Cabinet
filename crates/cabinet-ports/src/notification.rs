use cabinet_domain::notification::ChangeNotification;

pub trait NotificationPort {
    fn deliver(
        &mut self,
        notification: ChangeNotification,
    ) -> Result<NotificationDeliveryReceipt, NotificationDeliveryError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationDeliveryReceipt {
    notification: ChangeNotification,
    status: NotificationDeliveryStatus,
    delivery_count: usize,
    retry_count: usize,
}

impl NotificationDeliveryReceipt {
    pub fn queued(
        notification: ChangeNotification,
        delivery_count: usize,
        retry_count: usize,
    ) -> Self {
        Self {
            notification,
            status: NotificationDeliveryStatus::Queued,
            delivery_count,
            retry_count,
        }
    }

    pub fn delivered(
        notification: ChangeNotification,
        delivery_count: usize,
        retry_count: usize,
    ) -> Self {
        Self {
            notification,
            status: NotificationDeliveryStatus::Delivered,
            delivery_count,
            retry_count,
        }
    }

    pub fn failed(
        notification: ChangeNotification,
        delivery_count: usize,
        retry_count: usize,
    ) -> Self {
        Self {
            notification,
            status: NotificationDeliveryStatus::Failed,
            delivery_count,
            retry_count,
        }
    }

    pub fn notification(&self) -> &ChangeNotification {
        &self.notification
    }

    pub const fn status(&self) -> NotificationDeliveryStatus {
        self.status
    }

    pub const fn delivery_count(&self) -> usize {
        self.delivery_count
    }

    pub const fn retry_count(&self) -> usize {
        self.retry_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationDeliveryStatus {
    Queued,
    Delivered,
    Failed,
}

impl NotificationDeliveryStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Delivered => "delivered",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationDeliveryError {
    UnsupportedEventType,
    DuplicateEvent,
    DeliveryFailed,
}

impl NotificationDeliveryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedEventType => "notification.unsupported_event_type",
            Self::DuplicateEvent => "notification.duplicate_event",
            Self::DeliveryFailed => "notification.delivery_failed",
        }
    }
}
