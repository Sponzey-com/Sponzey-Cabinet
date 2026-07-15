use std::fs;
use std::path::PathBuf;

use cabinet_adapters::local_document_lock_repository::LocalDocumentLockRepository;
use cabinet_domain::document::DocumentId;
use cabinet_domain::document_lock::{DocumentLock, DocumentLockId, DocumentLockTimestamp};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_lock::{DocumentLockRepository, DocumentLockRepositoryError};

#[test]
fn local_document_lock_repository_persists_lock_across_instances() {
    let root = unique_temp_dir("local-document-lock-persist");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let lock = document_lock("lock-1", &document_id, "owner-1", 1_000, 31_000);

    {
        let mut repository = LocalDocumentLockRepository::new(root.clone());
        repository
            .save_document_lock(&workspace_id, lock.clone())
            .expect("save lock");
    }

    let repository = LocalDocumentLockRepository::new(root.clone());
    let loaded = repository
        .get_document_lock(&workspace_id, &document_id)
        .expect("get lock")
        .expect("stored lock");

    assert_eq!(loaded.lock_id(), lock.lock_id());
    assert_eq!(loaded.owner_user_id(), lock.owner_user_id());
    assert_eq!(loaded.expires_at(), lock.expires_at());
    assert!(!format!("{repository:?}").contains("owner-1"));
    cleanup_temp_dir(root);
}

#[test]
fn local_document_lock_repository_deletes_and_replaces_lock_durably() {
    let root = unique_temp_dir("local-document-lock-delete");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let mut repository = LocalDocumentLockRepository::new(root.clone());

    repository
        .save_document_lock(
            &workspace_id,
            document_lock("lock-1", &document_id, "owner-1", 1_000, 31_000),
        )
        .expect("save lock");
    repository
        .save_document_lock(
            &workspace_id,
            document_lock("lock-2", &document_id, "owner-2", 2_000, 32_000),
        )
        .expect("replace lock");
    let deleted = repository
        .delete_document_lock(&workspace_id, &document_id)
        .expect("delete lock")
        .expect("deleted lock");

    assert_eq!(deleted.lock_id().as_str(), "lock-2");
    assert!(
        LocalDocumentLockRepository::new(root.clone())
            .get_document_lock(&workspace_id, &document_id)
            .expect("get deleted")
            .is_none()
    );
    assert!(
        repository
            .delete_document_lock(&workspace_id, &document_id)
            .expect("delete missing")
            .is_none()
    );
    cleanup_temp_dir(root);
}

#[test]
fn local_document_lock_repository_reports_corrupted_lock_file() {
    let root = unique_temp_dir("local-document-lock-corrupt");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let mut repository = LocalDocumentLockRepository::new(root.clone());
    repository
        .save_document_lock(
            &workspace_id,
            document_lock("lock-1", &document_id, "owner-1", 1_000, 31_000),
        )
        .expect("save lock");

    fs::write(
        first_file_under(&root.join("document-locks"), "lock"),
        "not-a-lock-record",
    )
    .expect("corrupt lock file");
    let error = repository
        .get_document_lock(&workspace_id, &document_id)
        .expect_err("corrupted lock must fail");

    assert_eq!(error, DocumentLockRepositoryError::CorruptedState);
    cleanup_temp_dir(root);
}

fn document_lock(
    lock_id: &str,
    document_id: &DocumentId,
    owner_id: &str,
    acquired_at: u64,
    expires_at: u64,
) -> DocumentLock {
    DocumentLock::new(
        DocumentLockId::new(lock_id).expect("lock id"),
        document_id.clone(),
        UserId::new(owner_id).expect("owner id"),
        DocumentLockTimestamp::from_millis(acquired_at),
        DocumentLockTimestamp::from_millis(expires_at),
    )
    .expect("lock")
}

fn first_file_under(root: &PathBuf, extension: &str) -> PathBuf {
    let mut stack = vec![root.clone()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(path).expect("read dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                return path;
            }
        }
    }
    panic!("file with extension {extension} not found");
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sponzey-cabinet-{name}-{}", std::process::id()));
    cleanup_temp_dir(dir.clone());
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn cleanup_temp_dir(dir: PathBuf) {
    if dir.exists() {
        fs::remove_dir_all(dir).expect("remove temp dir");
    }
}
