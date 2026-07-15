use cabinet_domain::document::DocumentId;
use cabinet_domain::notification::{
    ChangeNotification, ChangeNotificationCorrelationId, ChangeNotificationError,
    ChangeNotificationEventType, ChangeNotificationTarget, ChangeNotificationTargetId,
    ChangeNotificationTimestamp,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::notification::{
    NotificationDeliveryError, NotificationDeliveryReceipt, NotificationDeliveryStatus,
    NotificationPort,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeNotificationSideEffectRequest {
    notification: ChangeNotification,
}

impl ChangeNotificationSideEffectRequest {
    pub fn document_changed(
        actor_user_id: &str,
        workspace_id: &str,
        document_id: &str,
        occurred_at: ChangeNotificationTimestamp,
        correlation_id: &str,
    ) -> Result<Self, NotificationUsecaseError> {
        Self::new(
            actor_user_id,
            workspace_id,
            ChangeNotificationTarget::document(parse_document_id(document_id)?),
            ChangeNotificationEventType::DocumentChanged,
            occurred_at,
            correlation_id,
        )
    }

    pub fn comment_changed(
        actor_user_id: &str,
        workspace_id: &str,
        document_id: &str,
        thread_id: &str,
        occurred_at: ChangeNotificationTimestamp,
        correlation_id: &str,
    ) -> Result<Self, NotificationUsecaseError> {
        Self::new(
            actor_user_id,
            workspace_id,
            ChangeNotificationTarget::comment_thread(
                parse_document_id(document_id)?,
                parse_target_id(thread_id)?,
            ),
            ChangeNotificationEventType::CommentChanged,
            occurred_at,
            correlation_id,
        )
    }

    pub fn review_state_changed(
        actor_user_id: &str,
        workspace_id: &str,
        document_id: &str,
        review_request_id: &str,
        occurred_at: ChangeNotificationTimestamp,
        correlation_id: &str,
    ) -> Result<Self, NotificationUsecaseError> {
        Self::new(
            actor_user_id,
            workspace_id,
            ChangeNotificationTarget::review_request(
                parse_document_id(document_id)?,
                parse_target_id(review_request_id)?,
            ),
            ChangeNotificationEventType::ReviewStateChanged,
            occurred_at,
            correlation_id,
        )
    }

    pub fn lock_state_changed(
        actor_user_id: &str,
        workspace_id: &str,
        document_id: &str,
        lock_id: &str,
        occurred_at: ChangeNotificationTimestamp,
        correlation_id: &str,
    ) -> Result<Self, NotificationUsecaseError> {
        Self::new(
            actor_user_id,
            workspace_id,
            ChangeNotificationTarget::document_lock(
                parse_document_id(document_id)?,
                parse_target_id(lock_id)?,
            ),
            ChangeNotificationEventType::LockStateChanged,
            occurred_at,
            correlation_id,
        )
    }

    fn new(
        actor_user_id: &str,
        workspace_id: &str,
        target: ChangeNotificationTarget,
        event_type: ChangeNotificationEventType,
        occurred_at: ChangeNotificationTimestamp,
        correlation_id: &str,
    ) -> Result<Self, NotificationUsecaseError> {
        Ok(Self {
            notification: ChangeNotification::new(
                WorkspaceId::new(workspace_id)
                    .map_err(|_| NotificationUsecaseError::InvalidInput)?,
                UserId::new(actor_user_id).map_err(|_| NotificationUsecaseError::InvalidInput)?,
                target,
                event_type,
                occurred_at,
                ChangeNotificationCorrelationId::new(correlation_id)
                    .map_err(NotificationUsecaseError::from_notification_error)?,
            ),
        })
    }

    pub fn notification(&self) -> &ChangeNotification {
        &self.notification
    }

    fn into_notification(self) -> ChangeNotification {
        self.notification
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendChangeNotificationInput {
    request: ChangeNotificationSideEffectRequest,
}

impl SendChangeNotificationInput {
    pub fn new(request: ChangeNotificationSideEffectRequest) -> Self {
        Self { request }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendChangeNotificationOutput {
    status: SendChangeNotificationOutputStatus,
    receipt: NotificationDeliveryReceipt,
}

impl SendChangeNotificationOutput {
    pub const fn status(&self) -> SendChangeNotificationOutputStatus {
        self.status
    }

    pub fn receipt(&self) -> &NotificationDeliveryReceipt {
        &self.receipt
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendChangeNotificationOutputStatus {
    Queued,
    Delivered,
    Failed,
}

impl SendChangeNotificationOutputStatus {
    const fn from_delivery_status(status: NotificationDeliveryStatus) -> Self {
        match status {
            NotificationDeliveryStatus::Queued => Self::Queued,
            NotificationDeliveryStatus::Delivered => Self::Delivered,
            NotificationDeliveryStatus::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationUsecaseError {
    InvalidInput,
    UnsupportedEventType,
    DuplicateEvent,
    DeliveryFailed,
}

impl NotificationUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "NOTIFICATION_INVALID_INPUT",
            Self::UnsupportedEventType => "NOTIFICATION_UNSUPPORTED_EVENT_TYPE",
            Self::DuplicateEvent => "NOTIFICATION_DUPLICATE_EVENT",
            Self::DeliveryFailed => "NOTIFICATION_DELIVERY_FAILED",
        }
    }

    const fn from_delivery_error(error: NotificationDeliveryError) -> Self {
        match error {
            NotificationDeliveryError::UnsupportedEventType => Self::UnsupportedEventType,
            NotificationDeliveryError::DuplicateEvent => Self::DuplicateEvent,
            NotificationDeliveryError::DeliveryFailed => Self::DeliveryFailed,
        }
    }

    const fn from_notification_error(_error: ChangeNotificationError) -> Self {
        Self::InvalidInput
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationProductEvent {
    DeliveryFailed {
        masked_target_id: String,
        event_type: &'static str,
        error_code: &'static str,
    },
}

impl NotificationProductEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::DeliveryFailed { .. } => "notification.delivery.failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationFieldDebugEvent {
    event_type: &'static str,
    masked_actor_id: String,
    delivery_count: usize,
    retry_count: usize,
}

impl NotificationFieldDebugEvent {
    pub const fn event_type(&self) -> &'static str {
        self.event_type
    }

    pub fn masked_actor_id(&self) -> &str {
        &self.masked_actor_id
    }

    pub const fn delivery_count(&self) -> usize {
        self.delivery_count
    }

    pub const fn retry_count(&self) -> usize {
        self.retry_count
    }
}

pub trait NotificationUsecaseLogger {
    fn write_product(&mut self, event: NotificationProductEvent);
    fn write_field_debug(&mut self, event: NotificationFieldDebugEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendChangeNotificationUsecase;

impl SendChangeNotificationUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        self,
        input: SendChangeNotificationInput,
        notification_port: &mut impl NotificationPort,
        logger: &mut impl NotificationUsecaseLogger,
    ) -> Result<SendChangeNotificationOutput, NotificationUsecaseError> {
        let notification = input.request.into_notification();
        let event_type = notification.event_type().as_str();
        let masked_actor_id = mask_id(notification.actor_user_id().as_str());
        let masked_target_id = mask_id(notification.target().target_id());

        match notification_port.deliver(notification) {
            Ok(receipt) => {
                logger.write_field_debug(NotificationFieldDebugEvent {
                    event_type,
                    masked_actor_id,
                    delivery_count: receipt.delivery_count(),
                    retry_count: receipt.retry_count(),
                });
                Ok(SendChangeNotificationOutput {
                    status: SendChangeNotificationOutputStatus::from_delivery_status(
                        receipt.status(),
                    ),
                    receipt,
                })
            }
            Err(error) => {
                let usecase_error = NotificationUsecaseError::from_delivery_error(error);
                logger.write_product(NotificationProductEvent::DeliveryFailed {
                    masked_target_id,
                    event_type,
                    error_code: usecase_error.code(),
                });
                logger.write_field_debug(NotificationFieldDebugEvent {
                    event_type,
                    masked_actor_id,
                    delivery_count: 0,
                    retry_count: 0,
                });
                Err(usecase_error)
            }
        }
    }
}

fn parse_document_id(value: &str) -> Result<DocumentId, NotificationUsecaseError> {
    DocumentId::new(value).map_err(|_| NotificationUsecaseError::InvalidInput)
}

fn parse_target_id(value: &str) -> Result<ChangeNotificationTargetId, NotificationUsecaseError> {
    ChangeNotificationTargetId::new(value)
        .map_err(NotificationUsecaseError::from_notification_error)
}

fn mask_id(value: &str) -> String {
    match value.len() {
        0 => "masked:empty".to_string(),
        1..=4 => "masked:short".to_string(),
        len => format!("masked:{}", &value[len - 4..]),
    }
}
