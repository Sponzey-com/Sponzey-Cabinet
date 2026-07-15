use std::collections::BTreeMap;

use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::object_storage::{
    ObjectContent, ObjectKey, ObjectMetadata, ObjectRecord, ObjectStorage,
    ObjectStorageDeleteOutcome, ObjectStorageError, ObjectStorageFieldDebugEvent,
    ObjectStorageHealth, ObjectStorageProductEvent, ObjectStoragePutOutcome,
    ObjectStorageRetryCount,
};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn object_key_is_hash_based_and_rejects_original_file_name_shape() {
    assert!(ObjectKey::from_sha256_hex(HASH_A).is_ok());

    let invalid = ObjectKey::from_sha256_hex("diagram.png")
        .expect_err("original file name shape must not be an object key");

    assert_eq!(invalid, ObjectStorageError::InvalidObjectKey);
    assert_eq!(invalid.code(), "object_storage.invalid_object_key");
}

#[test]
fn object_record_requires_matching_key_and_byte_size() {
    let metadata =
        ObjectMetadata::new(object_key(HASH_A), 4, "image/png", HASH_A).expect("metadata");
    let content = ObjectContent::new(object_key(HASH_B), vec![1, 2, 3, 4]).expect("content");

    let error = ObjectRecord::new(metadata, content).expect_err("mismatched key must fail");

    assert_eq!(error, ObjectStorageError::MismatchedObjectContent);
    assert_eq!(
        ObjectStorageError::MismatchedObjectContent.code(),
        "object_storage.mismatched_object_content"
    );
}

#[test]
fn object_storage_roundtrips_metadata_and_content_separately() {
    let workspace_id = workspace_id();
    let mut storage = FakeObjectStorage::default();
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

    assert_eq!(first, ObjectStoragePutOutcome::Created);
    assert_eq!(second, ObjectStoragePutOutcome::AlreadyPresent);
    assert_eq!(metadata.byte_size(), 4);
    assert_eq!(metadata.media_type(), "text/markdown");
    assert_eq!(content.bytes(), &[1, 2, 3, 4]);
    assert_eq!(storage.content_read_count, 1);
}

#[test]
fn metadata_read_does_not_require_content_bytes() {
    let workspace_id = workspace_id();
    let mut storage = FakeObjectStorage::default();
    let record = object_record(HASH_A, "image/png", vec![1, 2, 3, 4]);
    let key = record.key().clone();

    storage
        .put_object(&workspace_id, record)
        .expect("put object");
    storage
        .content_by_key
        .remove(&workspace_key(&workspace_id, &key));
    let metadata = storage
        .get_metadata(&workspace_id, &key)
        .expect("metadata read should not require content")
        .expect("metadata");

    assert_eq!(metadata.key(), &key);
    assert_eq!(storage.content_read_count, 0);
}

#[test]
fn content_read_reports_missing_content_with_stable_error_code() {
    let workspace_id = workspace_id();
    let mut storage = FakeObjectStorage::default();
    let record = object_record(HASH_A, "image/png", vec![1, 2, 3, 4]);
    let key = record.key().clone();

    storage
        .put_object(&workspace_id, record)
        .expect("put object");
    storage
        .content_by_key
        .remove(&workspace_key(&workspace_id, &key));
    let error = storage
        .get_content(&workspace_id, &key)
        .expect_err("missing content must fail");

    assert_eq!(error, ObjectStorageError::MissingContent);
    assert_eq!(error.code(), "object_storage.missing_content");
}

#[test]
fn delete_object_is_idempotent() {
    let workspace_id = workspace_id();
    let mut storage = FakeObjectStorage::default();
    let record = object_record(HASH_A, "application/octet-stream", vec![1, 2, 3, 4]);
    let key = record.key().clone();

    storage
        .put_object(&workspace_id, record)
        .expect("put object");
    let deleted = storage
        .delete_object(&workspace_id, &key)
        .expect("delete object");
    let missing = storage
        .delete_object(&workspace_id, &key)
        .expect("delete object again");

    assert_eq!(deleted, ObjectStorageDeleteOutcome::Deleted);
    assert_eq!(missing, ObjectStorageDeleteOutcome::Missing);
}

#[test]
fn health_degraded_has_stable_error_and_log_event_name() {
    let storage = FakeObjectStorage {
        health: ObjectStorageHealth::degraded(
            "s3-compatible",
            ObjectStorageError::StorageUnavailable,
        ),
        ..FakeObjectStorage::default()
    };

    let health = storage.probe_health().expect("health probe");
    let product = ObjectStorageProductEvent::health_degraded(
        "s3-compatible",
        object_key(HASH_A).safe_hash(),
        ObjectStorageError::StorageUnavailable,
    );

    assert!(!health.is_healthy());
    assert_eq!(health.backend_type(), "s3-compatible");
    assert_eq!(
        health.error_code(),
        Some("object_storage.storage_unavailable")
    );
    assert_eq!(product.event_name(), "object_storage.health.degraded");
}

#[test]
fn object_storage_logs_do_not_include_credentials_content_or_original_file_name() {
    let key = object_key(HASH_A);
    let product = ObjectStorageProductEvent::operation_failed(
        "s3-compatible",
        "put_object",
        key.safe_hash(),
        ObjectStorageError::StorageUnavailable,
    );
    let field = ObjectStorageFieldDebugEvent::operation_attempt(
        "s3-compatible",
        "put_object",
        key.safe_hash(),
        ObjectStorageRetryCount::new(2),
        "0-50ms",
    )
    .expect("field debug event");

    let rendered = format!("{product:?}\n{field:?}");

    assert!(rendered.contains("s3-compatible"));
    assert!(rendered.contains("object_storage.operation.failed"));
    assert!(!rendered.contains("diagram.png"));
    assert!(!rendered.contains("secret-access-key"));
    assert!(!rendered.contains("raw file content"));
    assert!(!rendered.contains(HASH_A));
}

#[derive(Debug, Clone)]
struct FakeObjectStorage {
    metadata_by_key: BTreeMap<String, ObjectMetadata>,
    content_by_key: BTreeMap<String, ObjectContent>,
    health: ObjectStorageHealth,
    content_read_count: usize,
}

impl Default for FakeObjectStorage {
    fn default() -> Self {
        Self {
            metadata_by_key: BTreeMap::new(),
            content_by_key: BTreeMap::new(),
            health: ObjectStorageHealth::healthy("fake-object-storage"),
            content_read_count: 0,
        }
    }
}

impl ObjectStorage for FakeObjectStorage {
    fn put_object(
        &mut self,
        workspace_id: &WorkspaceId,
        record: ObjectRecord,
    ) -> Result<ObjectStoragePutOutcome, ObjectStorageError> {
        let key = workspace_key(workspace_id, record.key());
        if self.metadata_by_key.contains_key(&key) && self.content_by_key.contains_key(&key) {
            return Ok(ObjectStoragePutOutcome::AlreadyPresent);
        }
        if self.metadata_by_key.contains_key(&key) && !self.content_by_key.contains_key(&key) {
            return Err(ObjectStorageError::MissingContent);
        }

        self.metadata_by_key
            .insert(key.clone(), record.metadata().clone());
        self.content_by_key.insert(key, record.content().clone());
        Ok(ObjectStoragePutOutcome::Created)
    }

    fn get_metadata(
        &self,
        workspace_id: &WorkspaceId,
        key: &ObjectKey,
    ) -> Result<Option<ObjectMetadata>, ObjectStorageError> {
        Ok(self
            .metadata_by_key
            .get(&workspace_key(workspace_id, key))
            .cloned())
    }

    fn get_content(
        &mut self,
        workspace_id: &WorkspaceId,
        key: &ObjectKey,
    ) -> Result<Option<ObjectContent>, ObjectStorageError> {
        self.content_read_count += 1;
        let workspace_key = workspace_key(workspace_id, key);
        if let Some(content) = self.content_by_key.get(&workspace_key) {
            return Ok(Some(content.clone()));
        }
        if self.metadata_by_key.contains_key(&workspace_key) {
            return Err(ObjectStorageError::MissingContent);
        }
        Ok(None)
    }

    fn delete_object(
        &mut self,
        workspace_id: &WorkspaceId,
        key: &ObjectKey,
    ) -> Result<ObjectStorageDeleteOutcome, ObjectStorageError> {
        let workspace_key = workspace_key(workspace_id, key);
        let removed_metadata = self.metadata_by_key.remove(&workspace_key).is_some();
        let removed_content = self.content_by_key.remove(&workspace_key).is_some();
        if removed_metadata || removed_content {
            return Ok(ObjectStorageDeleteOutcome::Deleted);
        }
        Ok(ObjectStorageDeleteOutcome::Missing)
    }

    fn probe_health(&self) -> Result<ObjectStorageHealth, ObjectStorageError> {
        Ok(self.health.clone())
    }
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

fn workspace_key(workspace_id: &WorkspaceId, key: &ObjectKey) -> String {
    format!("{}:{}", workspace_id.as_str(), key.as_str())
}
