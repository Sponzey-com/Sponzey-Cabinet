use cabinet_domain::audit::{
    AuditAction, AuditActor, AuditError, AuditEvent, AuditEventId, AuditMetadata, AuditTarget,
    AuditTargetId, AuditTimestamp,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;

#[test]
fn audit_event_keeps_searchable_safe_structure() {
    let event = AuditEvent::new(
        AuditEventId::new("audit-1").expect("audit id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        AuditActor::user(UserId::new("actor-1").expect("actor id")),
        AuditAction::ReviewApproved,
        AuditTarget::review_request(
            DocumentId::new("doc-1").expect("document id"),
            AuditTargetId::new("review-1").expect("review id"),
        ),
        AuditMetadata::new([("workflow_state", "approved"), ("source", "review")])
            .expect("metadata"),
        AuditTimestamp::from_millis(10_000),
    );

    assert_eq!(event.event_id().as_str(), "audit-1");
    assert_eq!(event.workspace_id().as_str(), "workspace-1");
    assert_eq!(event.actor().actor_id(), "actor-1");
    assert_eq!(event.action(), AuditAction::ReviewApproved);
    assert_eq!(event.action().as_str(), "review.approved");
    assert_eq!(event.target().target_type(), "review_request");
    assert_eq!(event.target().target_id(), "review-1");
    assert_eq!(
        event.target().document_id().expect("document").as_str(),
        "doc-1"
    );
    assert_eq!(event.metadata().value("workflow_state"), Some("approved"));
    assert_eq!(event.occurred_at().as_millis(), 10_000);
}

#[test]
fn audit_actions_cover_permission_review_publish_lock_and_backup_events() {
    assert_eq!(AuditAction::PermissionDenied.as_str(), "permission.denied");
    assert_eq!(AuditAction::ReviewRequested.as_str(), "review.requested");
    assert_eq!(AuditAction::ReviewRejected.as_str(), "review.rejected");
    assert_eq!(
        AuditAction::DocumentPublished.as_str(),
        "document.published"
    );
    assert_eq!(AuditAction::LockAcquired.as_str(), "lock.acquired");
    assert_eq!(AuditAction::LockReleased.as_str(), "lock.released");
    assert_eq!(AuditAction::LockExpired.as_str(), "lock.expired");
    assert_eq!(AuditAction::BackupCreated.as_str(), "backup.created");
    assert_eq!(AuditAction::RestoreCompleted.as_str(), "restore.completed");
}

#[test]
fn audit_metadata_rejects_sensitive_keys_and_values() {
    assert_eq!(
        AuditMetadata::new([("document_body", "hello")]).expect_err("body key rejected"),
        AuditError::SensitiveMetadataKey
    );
    assert_eq!(
        AuditMetadata::new([("reason", "token=abc")]).expect_err("token value rejected"),
        AuditError::SensitiveMetadataValue
    );
    assert_eq!(
        AuditMetadata::new([("secret", "abc")]).expect_err("secret key rejected"),
        AuditError::SensitiveMetadataKey
    );
    assert_eq!(
        AuditMetadata::new([("safe_key", "comment body: hello")])
            .expect_err("comment body value rejected"),
        AuditError::SensitiveMetadataValue
    );
}

#[test]
fn audit_value_objects_reject_empty_or_control_values() {
    assert_eq!(
        AuditEventId::new(" ").expect_err("empty event id"),
        AuditError::EmptyEventId
    );
    assert_eq!(
        AuditEventId::new("audit\n1").expect_err("control event id"),
        AuditError::InvalidEventId
    );
    assert_eq!(
        AuditTargetId::new("").expect_err("empty target id"),
        AuditError::EmptyTargetId
    );
    assert_eq!(
        AuditTargetId::new("target\n1").expect_err("control target id"),
        AuditError::InvalidTargetId
    );
}

#[test]
fn audit_debug_output_does_not_contain_raw_document_comment_asset_or_secret_content() {
    let event = AuditEvent::new(
        AuditEventId::new("audit-1").expect("audit id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        AuditActor::user(UserId::new("actor-1").expect("actor id")),
        AuditAction::PermissionDenied,
        AuditTarget::document(DocumentId::new("doc-1").expect("document id")),
        AuditMetadata::new([("permission", "write")]).expect("metadata"),
        AuditTimestamp::from_millis(10_000),
    );

    let debug = format!("{event:?}");

    assert!(!debug.contains("secret document body"));
    assert!(!debug.contains("secret comment body"));
    assert!(!debug.contains("asset bytes"));
    assert!(!debug.contains("token="));
}
