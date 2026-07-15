use cabinet_domain::document::DocumentId;
use cabinet_domain::document_lock::{
    DocumentLock, DocumentLockErrorCode, DocumentLockEvent, DocumentLockId, DocumentLockState,
    DocumentLockTimestamp, DocumentLockTransitionContext, transition_document_lock,
};
use cabinet_domain::user::UserId;

#[test]
fn document_lock_transitions_from_unlocked_to_locked() {
    let document_id = DocumentId::new("doc-1").expect("document id");
    let owner = UserId::new("user-1").expect("owner id");
    let lock_id = DocumentLockId::new("lock-1").expect("lock id");
    let now = DocumentLockTimestamp::from_millis(1_000);

    let transition = transition_document_lock(DocumentLockTransitionContext::lock_requested(
        None,
        owner.clone(),
        now,
    ))
    .expect("unlocked document can be locked");

    assert_eq!(transition.previous_state(), DocumentLockState::Unlocked);
    assert_eq!(transition.next_state(), DocumentLockState::Locked);
    assert_eq!(transition.event(), DocumentLockEvent::LockRequested);

    let lock = DocumentLock::new(lock_id, document_id, owner, now, now.plus_millis(30_000))
        .expect("valid lock");
    assert!(!lock.is_expired_at(now.plus_millis(29_999)));
    assert!(lock.is_expired_at(now.plus_millis(30_000)));
}

#[test]
fn document_lock_rejects_lock_when_active_lock_exists() {
    let current = lock("lock-1", "doc-1", "owner-1", 1_000, 31_000);
    let requester = UserId::new("user-2").expect("requester id");

    let failure = transition_document_lock(DocumentLockTransitionContext::lock_requested(
        Some(&current),
        requester,
        DocumentLockTimestamp::from_millis(2_000),
    ))
    .expect_err("active lock should conflict");

    assert_eq!(failure.error_code(), DocumentLockErrorCode::AlreadyLocked);
    assert_eq!(failure.current_state(), DocumentLockState::Locked);
}

#[test]
fn document_lock_allows_owner_unlock_and_rejects_non_owner_unlock() {
    let current = lock("lock-1", "doc-1", "owner-1", 1_000, 31_000);
    let owner = UserId::new("owner-1").expect("owner id");
    let other = UserId::new("user-2").expect("other id");

    let released = transition_document_lock(DocumentLockTransitionContext::unlock_requested(
        Some(&current),
        owner,
        DocumentLockTimestamp::from_millis(2_000),
    ))
    .expect("owner can unlock active lock");
    assert_eq!(released.next_state(), DocumentLockState::Unlocked);

    let failure = transition_document_lock(DocumentLockTransitionContext::unlock_requested(
        Some(&current),
        other,
        DocumentLockTimestamp::from_millis(2_000),
    ))
    .expect_err("non-owner cannot unlock");
    assert_eq!(failure.error_code(), DocumentLockErrorCode::NotOwner);
}

#[test]
fn document_lock_reports_expired_lock_on_unlock_or_expire_event() {
    let current = lock("lock-1", "doc-1", "owner-1", 1_000, 2_000);
    let owner = UserId::new("owner-1").expect("owner id");

    let unlock_failure = transition_document_lock(DocumentLockTransitionContext::unlock_requested(
        Some(&current),
        owner.clone(),
        DocumentLockTimestamp::from_millis(2_000),
    ))
    .expect_err("expired lock cannot be released as active");
    assert_eq!(unlock_failure.error_code(), DocumentLockErrorCode::Expired);

    let expired = transition_document_lock(DocumentLockTransitionContext::lock_expired(
        Some(&current),
        owner,
        DocumentLockTimestamp::from_millis(2_000),
    ))
    .expect("expired lock can transition to unlocked");
    assert_eq!(expired.previous_state(), DocumentLockState::Locked);
    assert_eq!(expired.next_state(), DocumentLockState::Unlocked);
    assert_eq!(expired.event(), DocumentLockEvent::LockExpired);
}

fn lock(
    lock_id: &str,
    document_id: &str,
    owner_user_id: &str,
    acquired_at_millis: u64,
    expires_at_millis: u64,
) -> DocumentLock {
    DocumentLock::new(
        DocumentLockId::new(lock_id).expect("lock id"),
        DocumentId::new(document_id).expect("document id"),
        UserId::new(owner_user_id).expect("owner id"),
        DocumentLockTimestamp::from_millis(acquired_at_millis),
        DocumentLockTimestamp::from_millis(expires_at_millis),
    )
    .expect("valid lock")
}
