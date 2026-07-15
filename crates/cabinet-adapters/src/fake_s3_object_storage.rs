use std::collections::BTreeMap;
use std::fmt;

use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::object_storage::{
    ObjectContent, ObjectKey, ObjectMetadata, ObjectRecord, ObjectStorage,
    ObjectStorageDeleteOutcome, ObjectStorageError, ObjectStorageHealth, ObjectStoragePutOutcome,
};

#[derive(Clone, PartialEq, Eq)]
pub struct FakeS3ObjectStorageConfig {
    endpoint: String,
    bucket: String,
    access_key_id: String,
    secret_access_key: S3Secret,
}

impl FakeS3ObjectStorageConfig {
    pub fn new(
        endpoint: &str,
        bucket: &str,
        access_key_id: &str,
        secret_access_key: &str,
    ) -> Result<Self, ObjectStorageError> {
        Ok(Self {
            endpoint: validate_endpoint(endpoint)?,
            bucket: validate_required_text(bucket)?,
            access_key_id: validate_required_text(access_key_id)?,
            secret_access_key: S3Secret::new(secret_access_key)?,
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    pub fn access_key_id(&self) -> &str {
        &self.access_key_id
    }

    pub fn secret_access_key(&self) -> &S3Secret {
        &self.secret_access_key
    }
}

impl fmt::Debug for FakeS3ObjectStorageConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FakeS3ObjectStorageConfig")
            .field("backend_type", &"s3-compatible")
            .field("endpoint", &self.endpoint)
            .field("bucket", &self.bucket)
            .field("access_key_id", &"<redacted>")
            .field("secret_access_key", &self.secret_access_key)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct S3Secret {
    value: String,
}

impl S3Secret {
    fn new(value: &str) -> Result<Self, ObjectStorageError> {
        let trimmed = value.trim();
        if trimmed.len() < 16 || trimmed.chars().any(char::is_control) {
            return Err(ObjectStorageError::InvalidObjectMetadata);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn expose_secret(&self) -> &str {
        &self.value
    }
}

impl fmt::Debug for S3Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("S3Secret(<redacted>)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeS3ObjectStorage {
    config: FakeS3ObjectStorageConfig,
    metadata_by_key: BTreeMap<String, ObjectMetadata>,
    content_by_key: BTreeMap<String, ObjectContent>,
    health: ObjectStorageHealth,
}

impl FakeS3ObjectStorage {
    pub fn new(config: FakeS3ObjectStorageConfig) -> Self {
        Self {
            config,
            metadata_by_key: BTreeMap::new(),
            content_by_key: BTreeMap::new(),
            health: ObjectStorageHealth::healthy("s3-compatible"),
        }
    }

    pub fn config(&self) -> &FakeS3ObjectStorageConfig {
        &self.config
    }

    pub fn set_health(&mut self, health: ObjectStorageHealth) {
        self.health = health;
    }
}

impl ObjectStorage for FakeS3ObjectStorage {
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
        if !self.metadata_by_key.contains_key(&key) && self.content_by_key.contains_key(&key) {
            return Err(ObjectStorageError::Conflict);
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

fn validate_endpoint(value: &str) -> Result<String, ObjectStorageError> {
    let trimmed = validate_required_text(value)?;
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://"))
        || trimmed.contains(char::is_whitespace)
    {
        return Err(ObjectStorageError::InvalidObjectMetadata);
    }
    Ok(trimmed)
}

fn validate_required_text(value: &str) -> Result<String, ObjectStorageError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(ObjectStorageError::InvalidObjectMetadata);
    }
    Ok(trimmed.to_string())
}

fn workspace_key(workspace_id: &WorkspaceId, key: &ObjectKey) -> String {
    format!(
        "{}/{}/{}",
        workspace_id.as_str(),
        key.as_str()[0..2].to_string(),
        key.as_str()
    )
}
