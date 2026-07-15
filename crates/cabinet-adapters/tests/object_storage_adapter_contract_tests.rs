use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::fake_s3_object_storage::{FakeS3ObjectStorage, FakeS3ObjectStorageConfig};
use cabinet_adapters::local_object_storage::{
    LocalObjectStorage, OBJECT_CONTENT_EXTENSION, OBJECT_CONTENT_ROOT_DIR, OBJECT_METADATA_FILE,
    OBJECT_METADATA_ROOT_DIR,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::object_storage::{
    ObjectContent, ObjectKey, ObjectMetadata, ObjectRecord, ObjectStorage,
    ObjectStorageDeleteOutcome, ObjectStorageError, ObjectStorageHealth, ObjectStoragePutOutcome,
};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn local_object_storage_passes_contract() {
    let temp = TempObjectRoot::new("local-contract");
    let storage = LocalObjectStorage::new(temp.path.clone());

    run_object_storage_contract(storage);
}

#[test]
fn fake_s3_object_storage_passes_contract() {
    let storage = FakeS3ObjectStorage::new(s3_config());

    run_object_storage_contract(storage);
}

#[test]
fn local_object_storage_reads_metadata_when_content_file_is_missing() {
    let temp = TempObjectRoot::new("local-metadata-only");
    let workspace_id = workspace_id();
    let record = object_record(HASH_A, "text/markdown", vec![1, 2, 3, 4]);
    let key = record.key().clone();
    let mut storage = LocalObjectStorage::new(temp.path.clone());

    storage
        .put_object(&workspace_id, record)
        .expect("put object");
    fs::remove_file(content_path(&temp, &workspace_id, &key)).expect("remove content");
    let metadata = storage
        .get_metadata(&workspace_id, &key)
        .expect("get metadata")
        .expect("metadata");
    let content_error = storage
        .get_content(&workspace_id, &key)
        .expect_err("missing content must fail");

    assert_eq!(metadata.key(), &key);
    assert_eq!(content_error, ObjectStorageError::MissingContent);
}

#[test]
fn fake_s3_config_redacts_credentials_in_debug_output() {
    let config = s3_config();
    let rendered = format!("{config:?}");

    assert!(rendered.contains("s3-compatible"));
    assert!(rendered.contains("bucket-1"));
    assert!(!rendered.contains("access-key-id"));
    assert!(!rendered.contains("secret-access-key"));
}

#[test]
fn fake_s3_health_degraded_uses_backend_neutral_result() {
    let mut storage = FakeS3ObjectStorage::new(s3_config());
    storage.set_health(ObjectStorageHealth::degraded(
        "s3-compatible",
        ObjectStorageError::StorageUnavailable,
    ));

    let health = storage.probe_health().expect("health probe");

    assert!(!health.is_healthy());
    assert_eq!(health.backend_type(), "s3-compatible");
    assert_eq!(
        health.error_code(),
        Some("object_storage.storage_unavailable")
    );
}

fn run_object_storage_contract<T: ObjectStorage>(mut storage: T) {
    let workspace_id = workspace_id();
    let record = object_record(HASH_A, "text/markdown", vec![1, 2, 3, 4]);
    let key = record.key().clone();

    let first = storage
        .put_object(&workspace_id, record)
        .expect("first put");
    let second = storage
        .put_object(
            &workspace_id,
            object_record(HASH_A, "text/markdown", vec![1, 2, 3, 4]),
        )
        .expect("second put");
    let metadata = storage
        .get_metadata(&workspace_id, &key)
        .expect("get metadata")
        .expect("metadata");
    let content = storage
        .get_content(&workspace_id, &key)
        .expect("get content")
        .expect("content");
    let missing = storage
        .get_metadata(&workspace_id, &object_key(HASH_B))
        .expect("missing metadata");
    let deleted = storage
        .delete_object(&workspace_id, &key)
        .expect("delete object");
    let deleted_again = storage
        .delete_object(&workspace_id, &key)
        .expect("delete object again");
    let health = storage.probe_health().expect("health probe");

    assert_eq!(first, ObjectStoragePutOutcome::Created);
    assert_eq!(second, ObjectStoragePutOutcome::AlreadyPresent);
    assert_eq!(metadata.key(), &key);
    assert_eq!(metadata.byte_size(), 4);
    assert_eq!(content.bytes(), &[1, 2, 3, 4]);
    assert!(missing.is_none());
    assert_eq!(deleted, ObjectStorageDeleteOutcome::Deleted);
    assert_eq!(deleted_again, ObjectStorageDeleteOutcome::Missing);
    assert!(health.is_healthy());
}

struct TempObjectRoot {
    path: PathBuf,
}

impl TempObjectRoot {
    fn new(test_name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = PathBuf::from("/tmp").join(format!(
            "sponzey-cabinet-object-storage-{test_name}-{}-{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temp object root");
        Self { path }
    }
}

impl Drop for TempObjectRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn s3_config() -> FakeS3ObjectStorageConfig {
    FakeS3ObjectStorageConfig::new(
        "https://objects.example.test",
        "bucket-1",
        "access-key-id",
        "secret-access-key",
    )
    .expect("s3 config")
}

fn workspace_id() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace id")
}

fn object_key(hash: &str) -> ObjectKey {
    ObjectKey::from_sha256_hex(hash).expect("object key")
}

fn object_record(hash: &str, media_type: &str, bytes: Vec<u8>) -> ObjectRecord {
    let key = object_key(hash);
    ObjectRecord::new(
        ObjectMetadata::new(key.clone(), bytes.len() as u64, media_type, hash).expect("metadata"),
        ObjectContent::new(key, bytes).expect("content"),
    )
    .expect("object record")
}

fn content_path(temp: &TempObjectRoot, workspace_id: &WorkspaceId, key: &ObjectKey) -> PathBuf {
    temp.path
        .join(workspace_id.as_str())
        .join(OBJECT_CONTENT_ROOT_DIR)
        .join(&key.as_str()[0..2])
        .join(format!("{}.{}", key.as_str(), OBJECT_CONTENT_EXTENSION))
}

#[allow(dead_code)]
fn metadata_path(temp: &TempObjectRoot, workspace_id: &WorkspaceId, key: &ObjectKey) -> PathBuf {
    temp.path
        .join(workspace_id.as_str())
        .join(OBJECT_METADATA_ROOT_DIR)
        .join(key.as_str())
        .join(OBJECT_METADATA_FILE)
}
