use cabinet_domain::document::DocumentId;
use cabinet_domain::notification::{
    ChangeNotification, ChangeNotificationCorrelationId, ChangeNotificationError,
    ChangeNotificationEventType, ChangeNotificationTarget, ChangeNotificationTargetId,
    ChangeNotificationTimestamp,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;

#[test]
fn change_notification_keeps_minimal_safe_event_contract() {
    let notification = ChangeNotification::new(
        WorkspaceId::new("workspace-1").expect("workspace id"),
        UserId::new("actor-1").expect("actor id"),
        ChangeNotificationTarget::comment_thread(
            DocumentId::new("doc-1").expect("document id"),
            ChangeNotificationTargetId::new("thread-1").expect("thread id"),
        ),
        ChangeNotificationEventType::CommentChanged,
        ChangeNotificationTimestamp::from_millis(10_000),
        ChangeNotificationCorrelationId::new("corr-1").expect("correlation id"),
    );

    assert_eq!(notification.workspace_id().as_str(), "workspace-1");
    assert_eq!(notification.actor_user_id().as_str(), "actor-1");
    assert_eq!(notification.target().target_id(), "thread-1");
    assert_eq!(notification.target().document_id().as_str(), "doc-1");
    assert_eq!(
        notification.event_type(),
        ChangeNotificationEventType::CommentChanged
    );
    assert_eq!(notification.event_type().as_str(), "comment.changed");
    assert_eq!(notification.occurred_at().as_millis(), 10_000);
    assert_eq!(notification.correlation_id().as_str(), "corr-1");
}

#[test]
fn change_notification_supports_required_event_types_and_targets() {
    let document_id = DocumentId::new("doc-1").expect("document id");

    let document_target = ChangeNotificationTarget::document(document_id.clone());
    assert_eq!(document_target.target_kind(), "document");
    assert_eq!(document_target.target_id(), "doc-1");

    let review_target = ChangeNotificationTarget::review_request(
        document_id.clone(),
        ChangeNotificationTargetId::new("review-1").expect("review id"),
    );
    assert_eq!(review_target.target_kind(), "review_request");
    assert_eq!(review_target.target_id(), "review-1");

    let lock_target = ChangeNotificationTarget::document_lock(
        document_id,
        ChangeNotificationTargetId::new("lock-1").expect("lock id"),
    );
    assert_eq!(lock_target.target_kind(), "document_lock");
    assert_eq!(lock_target.target_id(), "lock-1");

    assert_eq!(
        ChangeNotificationEventType::DocumentChanged.as_str(),
        "document.changed"
    );
    assert_eq!(
        ChangeNotificationEventType::CommentChanged.as_str(),
        "comment.changed"
    );
    assert_eq!(
        ChangeNotificationEventType::ReviewStateChanged.as_str(),
        "review.state_changed"
    );
    assert_eq!(
        ChangeNotificationEventType::LockStateChanged.as_str(),
        "lock.state_changed"
    );
}

#[test]
fn change_notification_value_objects_reject_empty_or_control_values() {
    assert_eq!(
        ChangeNotificationTargetId::new(" \t ").expect_err("empty target id"),
        ChangeNotificationError::EmptyTargetId
    );
    assert_eq!(
        ChangeNotificationTargetId::new("target\n1").expect_err("control target id"),
        ChangeNotificationError::InvalidTargetId
    );
    assert_eq!(
        ChangeNotificationCorrelationId::new("").expect_err("empty correlation id"),
        ChangeNotificationError::EmptyCorrelationId
    );
    assert_eq!(
        ChangeNotificationCorrelationId::new("corr\n1").expect_err("control correlation id"),
        ChangeNotificationError::InvalidCorrelationId
    );
}

#[test]
fn change_notification_debug_output_does_not_contain_raw_document_comment_or_asset_content() {
    let notification = ChangeNotification::new(
        WorkspaceId::new("workspace-1").expect("workspace id"),
        UserId::new("actor-1").expect("actor id"),
        ChangeNotificationTarget::document(DocumentId::new("doc-1").expect("document id")),
        ChangeNotificationEventType::DocumentChanged,
        ChangeNotificationTimestamp::from_millis(10_000),
        ChangeNotificationCorrelationId::new("corr-1").expect("correlation id"),
    );

    let debug = format!("{notification:?}");

    assert!(!debug.contains("secret document body"));
    assert!(!debug.contains("secret comment body"));
    assert!(!debug.contains("asset bytes"));
}
