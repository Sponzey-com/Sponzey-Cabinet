use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::document::DocumentId;
use cabinet_domain::document_lock::{DocumentLock, DocumentLockId, DocumentLockTimestamp};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_lock::{
    DocumentLockClock, DocumentLockRepository, DocumentLockRepositoryError,
};

#[test]
fn document_lock_repository_port_keeps_domain_lock_without_storage_schema() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let owner = UserId::new("owner-1").expect("owner id");
    let lock = DocumentLock::new(
        DocumentLockId::new("lock-1").expect("lock id"),
        document_id.clone(),
        owner,
        DocumentLockTimestamp::from_millis(1_000),
        DocumentLockTimestamp::from_millis(31_000),
    )
    .expect("valid lock");
    let mut repository = FakeLockRepository::default();

    repository
        .save_document_lock(&workspace_id, lock)
        .expect("save lock");
    let stored = repository
        .get_document_lock(&workspace_id, &document_id)
        .expect("get lock")
        .expect("lock exists");

    assert_eq!(stored.lock_id().as_str(), "lock-1");
    assert_eq!(stored.document_id(), &document_id);
    assert_eq!(
        stored.expires_at(),
        DocumentLockTimestamp::from_millis(31_000)
    );

    let deleted = repository
        .delete_document_lock(&workspace_id, &document_id)
        .expect("delete lock")
        .expect("deleted lock");
    assert_eq!(deleted.lock_id().as_str(), "lock-1");
    assert!(
        repository
            .get_document_lock(&workspace_id, &document_id)
            .expect("get after delete")
            .is_none()
    );
}

#[test]
fn document_lock_clock_port_is_explicitly_injected() {
    let clock = FakeClock::new(7_000);

    assert_eq!(clock.now(), DocumentLockTimestamp::from_millis(7_000));
    assert_eq!(clock.read_count.get(), 1);
}

#[test]
fn document_lock_port_errors_expose_stable_codes() {
    assert_eq!(
        DocumentLockRepositoryError::StorageUnavailable.code(),
        "document_lock_repository.storage_unavailable"
    );
    assert_eq!(
        DocumentLockRepositoryError::Conflict.code(),
        "document_lock_repository.conflict"
    );
    assert_eq!(
        DocumentLockRepositoryError::CorruptedState.code(),
        "document_lock_repository.corrupted_state"
    );
}

#[derive(Default)]
struct FakeLockRepository {
    locks: HashMap<String, DocumentLock>,
}

impl DocumentLockRepository for FakeLockRepository {
    fn get_document_lock(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentLock>, DocumentLockRepositoryError> {
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
        Ok(self
            .locks
            .remove(&lock_key(workspace_id.as_str(), document_id.as_str())))
    }
}

struct FakeClock {
    now: DocumentLockTimestamp,
    read_count: Cell<usize>,
}

impl FakeClock {
    fn new(millis: u64) -> Self {
        Self {
            now: DocumentLockTimestamp::from_millis(millis),
            read_count: Cell::new(0),
        }
    }
}

impl DocumentLockClock for FakeClock {
    fn now(&self) -> DocumentLockTimestamp {
        self.read_count.set(self.read_count.get() + 1);
        self.now
    }
}

fn lock_key(workspace_id: &str, document_id: &str) -> String {
    format!("{workspace_id}:{document_id}")
}
