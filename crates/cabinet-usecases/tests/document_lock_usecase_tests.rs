use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::document::DocumentId;
use cabinet_domain::document_lock::{DocumentLock, DocumentLockId, DocumentLockTimestamp};
use cabinet_domain::permission::{
    Permission, PermissionDecision, PermissionDecisionReason, PolicySource,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_lock::{
    DocumentLockClock, DocumentLockPermissionCheckError, DocumentLockPermissionChecker,
    DocumentLockRepository, DocumentLockRepositoryError,
};
use cabinet_usecases::document_lock::{
    DocumentLockProductEvent, DocumentLockUsecaseError, DocumentLockUsecaseLogger,
    DocumentLockViewStatus, ExpireDocumentLockInput, ExpireDocumentLockUsecase,
    GetDocumentLockInput, GetDocumentLockUsecase, LockDocumentInput, LockDocumentPolicy,
    LockDocumentUsecase, UnlockDocumentInput, UnlockDocumentUsecase,
};

#[derive(Default)]
struct FakeDocumentLockRepository {
    locks: HashMap<String, DocumentLock>,
    get_count: Cell<usize>,
    save_count: Cell<usize>,
    delete_count: Cell<usize>,
}

impl FakeDocumentLockRepository {
    fn insert(&mut self, workspace_id: &str, lock: DocumentLock) {
        self.locks
            .insert(lock_key(workspace_id, lock.document_id().as_str()), lock);
    }
}

impl DocumentLockRepository for FakeDocumentLockRepository {
    fn get_document_lock(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentLock>, DocumentLockRepositoryError> {
        self.get_count.set(self.get_count.get() + 1);
        Ok(self
            .locks
            .get(&lock_key(workspace_id.as_str(), document_id.as_str()))
            .cloned())
    }

    fn save_document_lock(
        &mut self,
        workspace_id: &WorkspaceId,
        lock: DocumentLock,
    ) -> Result<(), DocumentLockRepositoryError> {
        self.save_count.set(self.save_count.get() + 1);
        self.locks.insert(
            lock_key(workspace_id.as_str(), lock.document_id().as_str()),
            lock,
        );
        Ok(())
    }

    fn delete_document_lock(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentLock>, DocumentLockRepositoryError> {
        self.delete_count.set(self.delete_count.get() + 1);
        Ok(self
            .locks
            .remove(&lock_key(workspace_id.as_str(), document_id.as_str())))
    }
}

#[derive(Default)]
struct FakePermissionChecker {
    decisions: Vec<(Permission, PermissionDecision)>,
}

impl FakePermissionChecker {
    fn allow(&mut self, permission: Permission) {
        self.decisions.push((
            permission,
            PermissionDecision::allowed(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleAllowsPermission,
            ),
        ));
    }

    fn deny(&mut self, permission: Permission) {
        self.decisions.push((
            permission,
            PermissionDecision::denied(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleDoesNotAllowPermission,
            ),
        ));
    }
}

impl DocumentLockPermissionChecker for FakePermissionChecker {
    fn check_document_permission(
        &self,
        _actor_user_id: &UserId,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        permission: Permission,
    ) -> Result<PermissionDecision, DocumentLockPermissionCheckError> {
        Ok(self
            .decisions
            .iter()
            .rev()
            .find_map(|(candidate, decision)| (*candidate == permission).then_some(*decision))
            .unwrap_or(PermissionDecision::denied(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleDoesNotAllowPermission,
            )))
    }
}

#[derive(Clone, Copy)]
struct FakeClock {
    now: DocumentLockTimestamp,
}

impl FakeClock {
    fn at(millis: u64) -> Self {
        Self {
            now: DocumentLockTimestamp::from_millis(millis),
        }
    }
}

impl DocumentLockClock for FakeClock {
    fn now(&self) -> DocumentLockTimestamp {
        self.now
    }
}

#[derive(Default)]
struct FakeDocumentLockLogger {
    product_events: Vec<DocumentLockProductEvent>,
}

impl DocumentLockUsecaseLogger for FakeDocumentLockLogger {
    fn write_product(&mut self, event: DocumentLockProductEvent) {
        self.product_events.push(event);
    }

    fn write_field_debug(
        &mut self,
        _event: cabinet_usecases::document_lock::DocumentLockFieldDebugEvent,
    ) {
    }
}

#[test]
fn authorized_actor_acquires_lock_with_injected_ttl_policy() {
    let mut repository = FakeDocumentLockRepository::default();
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Write);
    let clock = FakeClock::at(1_000);
    let policy = LockDocumentPolicy::new(30_000).expect("lock policy");
    let mut logger = FakeDocumentLockLogger::default();

    let output = LockDocumentUsecase::new(policy)
        .execute(
            LockDocumentInput::new("user-1", "workspace-1", "doc-1", "lock-1"),
            &checker,
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect("lock should be acquired");

    assert_eq!(output.status(), DocumentLockViewStatus::Locked);
    assert_eq!(output.lock().expect("lock").lock_id().as_str(), "lock-1");
    assert_eq!(
        output.lock().expect("lock").expires_at(),
        DocumentLockTimestamp::from_millis(31_000)
    );
    assert_eq!(repository.save_count.get(), 1);
    assert_eq!(
        logger.product_events[0].event_name(),
        "document.lock.acquired"
    );
}

#[test]
fn lock_document_reports_conflict_when_active_lock_exists() {
    let mut repository = FakeDocumentLockRepository::default();
    repository.insert(
        "workspace-1",
        lock("lock-1", "doc-1", "owner-1", 1_000, 31_000),
    );
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Write);
    let clock = FakeClock::at(2_000);
    let policy = LockDocumentPolicy::new(30_000).expect("lock policy");
    let mut logger = FakeDocumentLockLogger::default();

    let error = LockDocumentUsecase::new(policy)
        .execute(
            LockDocumentInput::new("user-2", "workspace-1", "doc-1", "lock-2"),
            &checker,
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect_err("active lock should conflict");

    assert_eq!(error, DocumentLockUsecaseError::AlreadyLocked);
    assert_eq!(repository.save_count.get(), 0);
    assert!(matches!(
        logger.product_events.last(),
        Some(DocumentLockProductEvent::LockConflict {
            error_code: "DOCUMENT_LOCK_ALREADY_LOCKED",
            ..
        })
    ));
}

#[test]
fn lock_document_expires_stale_lock_before_acquiring_new_lock() {
    let mut repository = FakeDocumentLockRepository::default();
    repository.insert(
        "workspace-1",
        lock("lock-old", "doc-1", "owner-1", 1_000, 2_000),
    );
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Write);
    let clock = FakeClock::at(2_000);
    let policy = LockDocumentPolicy::new(30_000).expect("lock policy");
    let mut logger = FakeDocumentLockLogger::default();

    let output = LockDocumentUsecase::new(policy)
        .execute(
            LockDocumentInput::new("user-2", "workspace-1", "doc-1", "lock-new"),
            &checker,
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect("expired lock should be cleaned before acquisition");

    assert_eq!(output.lock().expect("lock").lock_id().as_str(), "lock-new");
    assert_eq!(repository.delete_count.get(), 1);
    assert_eq!(
        logger
            .product_events
            .iter()
            .map(DocumentLockProductEvent::event_name)
            .collect::<Vec<_>>(),
        vec!["document.lock.expired", "document.lock.acquired"]
    );
    assert!(matches!(
        logger.product_events.first(),
        Some(DocumentLockProductEvent::LockExpired {
            masked_actor_id,
            ..
        }) if masked_actor_id == "masked:er-2"
    ));
}

#[test]
fn non_owner_unlock_is_rejected_without_deleting_lock() {
    let mut repository = FakeDocumentLockRepository::default();
    repository.insert(
        "workspace-1",
        lock("lock-1", "doc-1", "owner-1", 1_000, 31_000),
    );
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Write);
    let clock = FakeClock::at(2_000);
    let mut logger = FakeDocumentLockLogger::default();

    let error = UnlockDocumentUsecase::new()
        .execute(
            UnlockDocumentInput::new("user-2", "workspace-1", "doc-1"),
            &checker,
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect_err("non owner cannot unlock");

    assert_eq!(error, DocumentLockUsecaseError::NotOwner);
    assert_eq!(repository.delete_count.get(), 0);
    assert!(
        repository
            .locks
            .contains_key(&lock_key("workspace-1", "doc-1"))
    );
}

#[test]
fn owner_unlock_releases_lock_and_writes_product_log() {
    let mut repository = FakeDocumentLockRepository::default();
    repository.insert(
        "workspace-1",
        lock("lock-1", "doc-1", "owner-1", 1_000, 31_000),
    );
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Write);
    let clock = FakeClock::at(2_000);
    let mut logger = FakeDocumentLockLogger::default();

    let output = UnlockDocumentUsecase::new()
        .execute(
            UnlockDocumentInput::new("owner-1", "workspace-1", "doc-1"),
            &checker,
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect("owner unlocks");

    assert_eq!(output.status(), DocumentLockViewStatus::Unlocked);
    assert!(output.lock().is_none());
    assert_eq!(repository.delete_count.get(), 1);
    assert!(
        !repository
            .locks
            .contains_key(&lock_key("workspace-1", "doc-1"))
    );
    assert_eq!(
        logger.product_events[0].event_name(),
        "document.lock.released"
    );
}

#[test]
fn unauthorized_actor_cannot_acquire_lock_or_touch_repository() {
    let mut repository = FakeDocumentLockRepository::default();
    let mut checker = FakePermissionChecker::default();
    checker.deny(Permission::Write);
    let clock = FakeClock::at(1_000);
    let policy = LockDocumentPolicy::new(30_000).expect("lock policy");
    let mut logger = FakeDocumentLockLogger::default();

    let error = LockDocumentUsecase::new(policy)
        .execute(
            LockDocumentInput::new("user-1", "workspace-1", "doc-1", "lock-1"),
            &checker,
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect_err("permission denied");

    assert_eq!(error, DocumentLockUsecaseError::Unauthorized);
    assert_eq!(repository.get_count.get(), 0);
    assert_eq!(repository.save_count.get(), 0);
    assert!(matches!(
        logger.product_events.last(),
        Some(DocumentLockProductEvent::LockConflict {
            error_code: "DOCUMENT_LOCK_UNAUTHORIZED",
            ..
        })
    ));
}

#[test]
fn unlock_missing_lock_returns_stable_error_without_delete() {
    let mut repository = FakeDocumentLockRepository::default();
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Write);
    let clock = FakeClock::at(2_000);
    let mut logger = FakeDocumentLockLogger::default();

    let error = UnlockDocumentUsecase::new()
        .execute(
            UnlockDocumentInput::new("owner-1", "workspace-1", "doc-1"),
            &checker,
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect_err("missing lock should fail");

    assert_eq!(error, DocumentLockUsecaseError::LockNotFound);
    assert_eq!(repository.delete_count.get(), 0);
}

#[test]
fn get_document_lock_returns_expired_result_and_cleans_repository() {
    let mut repository = FakeDocumentLockRepository::default();
    repository.insert(
        "workspace-1",
        lock("lock-1", "doc-1", "owner-1", 1_000, 2_000),
    );
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Read);
    let clock = FakeClock::at(2_000);
    let mut logger = FakeDocumentLockLogger::default();

    let output = GetDocumentLockUsecase::new()
        .execute(
            GetDocumentLockInput::new("viewer-1", "workspace-1", "doc-1"),
            &checker,
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect("get lock");

    assert_eq!(output.status(), DocumentLockViewStatus::Expired);
    assert!(output.lock().is_none());
    assert_eq!(repository.delete_count.get(), 1);
    assert_eq!(
        logger.product_events[0].event_name(),
        "document.lock.expired"
    );
}

#[test]
fn expire_document_lock_requires_expired_lock() {
    let mut repository = FakeDocumentLockRepository::default();
    repository.insert(
        "workspace-1",
        lock("lock-1", "doc-1", "owner-1", 1_000, 31_000),
    );
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Write);
    let clock = FakeClock::at(2_000);
    let mut logger = FakeDocumentLockLogger::default();

    let error = ExpireDocumentLockUsecase::new()
        .execute(
            ExpireDocumentLockInput::new("owner-1", "workspace-1", "doc-1"),
            &checker,
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect_err("active lock should not expire");

    assert_eq!(error, DocumentLockUsecaseError::LockNotExpired);
    assert_eq!(repository.delete_count.get(), 0);
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

fn lock_key(workspace_id: &str, document_id: &str) -> String {
    format!("{workspace_id}:{document_id}")
}
