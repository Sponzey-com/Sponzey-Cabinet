use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectKey {
    value: String,
}

impl ObjectKey {
    pub fn from_sha256_hex(value: &str) -> Result<Self, ObjectStorageError> {
        let trimmed = value.trim();
        if trimmed.len() != 64 || !trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ObjectStorageError::InvalidObjectKey);
        }
        Ok(Self {
            value: trimmed.to_ascii_lowercase(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn safe_hash(&self) -> String {
        format!("sha256:{}...", &self.value[0..12])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectMetadata {
    key: ObjectKey,
    byte_size: u64,
    media_type: String,
    content_hash: String,
}

impl ObjectMetadata {
    pub fn new(
        key: ObjectKey,
        byte_size: u64,
        media_type: &str,
        content_hash: &str,
    ) -> Result<Self, ObjectStorageError> {
        let media_type =
            validate_required_text(media_type, ObjectStorageError::InvalidObjectMetadata)?;
        let content_hash =
            validate_required_text(content_hash, ObjectStorageError::InvalidObjectMetadata)?;
        if byte_size == 0 {
            return Err(ObjectStorageError::InvalidObjectMetadata);
        }
        Ok(Self {
            key,
            byte_size,
            media_type,
            content_hash,
        })
    }

    pub fn key(&self) -> &ObjectKey {
        &self.key
    }

    pub fn byte_size(&self) -> u64 {
        self.byte_size
    }

    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectContent {
    key: ObjectKey,
    bytes: Vec<u8>,
}

impl ObjectContent {
    pub fn new(key: ObjectKey, bytes: Vec<u8>) -> Result<Self, ObjectStorageError> {
        if bytes.is_empty() {
            return Err(ObjectStorageError::InvalidObjectContent);
        }
        Ok(Self { key, bytes })
    }

    pub fn key(&self) -> &ObjectKey {
        &self.key
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectRecord {
    metadata: ObjectMetadata,
    content: ObjectContent,
}

impl ObjectRecord {
    pub fn new(
        metadata: ObjectMetadata,
        content: ObjectContent,
    ) -> Result<Self, ObjectStorageError> {
        if metadata.key() != content.key() || metadata.byte_size() != content.bytes().len() as u64 {
            return Err(ObjectStorageError::MismatchedObjectContent);
        }
        Ok(Self { metadata, content })
    }

    pub fn metadata(&self) -> &ObjectMetadata {
        &self.metadata
    }

    pub fn content(&self) -> &ObjectContent {
        &self.content
    }

    pub fn key(&self) -> &ObjectKey {
        self.metadata.key()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectStoragePutOutcome {
    Created,
    AlreadyPresent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectStorageDeleteOutcome {
    Deleted,
    Missing,
}

pub trait ObjectStorage {
    fn put_object(
        &mut self,
        workspace_id: &WorkspaceId,
        record: ObjectRecord,
    ) -> Result<ObjectStoragePutOutcome, ObjectStorageError>;

    fn get_metadata(
        &self,
        workspace_id: &WorkspaceId,
        key: &ObjectKey,
    ) -> Result<Option<ObjectMetadata>, ObjectStorageError>;

    fn get_content(
        &mut self,
        workspace_id: &WorkspaceId,
        key: &ObjectKey,
    ) -> Result<Option<ObjectContent>, ObjectStorageError>;

    fn delete_object(
        &mut self,
        workspace_id: &WorkspaceId,
        key: &ObjectKey,
    ) -> Result<ObjectStorageDeleteOutcome, ObjectStorageError>;

    fn probe_health(&self) -> Result<ObjectStorageHealth, ObjectStorageError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectStorageError {
    InvalidObjectKey,
    InvalidObjectMetadata,
    InvalidObjectContent,
    MismatchedObjectContent,
    MetadataUnavailable,
    ContentUnavailable,
    MissingContent,
    StorageUnavailable,
    HealthDegraded,
    Conflict,
}

impl ObjectStorageError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidObjectKey => "object_storage.invalid_object_key",
            Self::InvalidObjectMetadata => "object_storage.invalid_object_metadata",
            Self::InvalidObjectContent => "object_storage.invalid_object_content",
            Self::MismatchedObjectContent => "object_storage.mismatched_object_content",
            Self::MetadataUnavailable => "object_storage.metadata_unavailable",
            Self::ContentUnavailable => "object_storage.content_unavailable",
            Self::MissingContent => "object_storage.missing_content",
            Self::StorageUnavailable => "object_storage.storage_unavailable",
            Self::HealthDegraded => "object_storage.health_degraded",
            Self::Conflict => "object_storage.conflict",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectStorageHealth {
    backend_type: String,
    error_code: Option<&'static str>,
}

impl ObjectStorageHealth {
    pub fn healthy(backend_type: &str) -> Self {
        Self {
            backend_type: backend_type.to_string(),
            error_code: None,
        }
    }

    pub fn degraded(backend_type: &str, error: ObjectStorageError) -> Self {
        Self {
            backend_type: backend_type.to_string(),
            error_code: Some(error.code()),
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.error_code.is_none()
    }

    pub fn backend_type(&self) -> &str {
        &self.backend_type
    }

    pub fn error_code(&self) -> Option<&'static str> {
        self.error_code
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectStorageProductEvent {
    event_name: &'static str,
    backend_type: String,
    operation: String,
    object_key_hash: String,
    error_code: &'static str,
}

impl ObjectStorageProductEvent {
    pub fn health_degraded(
        backend_type: &str,
        object_key_hash: String,
        error: ObjectStorageError,
    ) -> Self {
        Self {
            event_name: "object_storage.health.degraded",
            backend_type: backend_type.to_string(),
            operation: "probe_health".to_string(),
            object_key_hash,
            error_code: error.code(),
        }
    }

    pub fn operation_failed(
        backend_type: &str,
        operation: &str,
        object_key_hash: String,
        error: ObjectStorageError,
    ) -> Self {
        Self {
            event_name: "object_storage.operation.failed",
            backend_type: backend_type.to_string(),
            operation: operation.to_string(),
            object_key_hash,
            error_code: error.code(),
        }
    }

    pub fn event_name(&self) -> &'static str {
        self.event_name
    }

    pub fn backend_type(&self) -> &str {
        &self.backend_type
    }

    pub fn operation(&self) -> &str {
        &self.operation
    }

    pub fn object_key_hash(&self) -> &str {
        &self.object_key_hash
    }

    pub fn error_code(&self) -> &'static str {
        self.error_code
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectStorageRetryCount {
    value: u16,
}

impl ObjectStorageRetryCount {
    pub const fn new(value: u16) -> Self {
        Self { value }
    }

    pub const fn value(self) -> u16 {
        self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectStorageFieldDebugEvent {
    backend_type: String,
    operation: String,
    object_key_hash: String,
    retry_count: ObjectStorageRetryCount,
    duration_bucket: String,
}

impl ObjectStorageFieldDebugEvent {
    pub fn operation_attempt(
        backend_type: &str,
        operation: &str,
        object_key_hash: String,
        retry_count: ObjectStorageRetryCount,
        duration_bucket: &str,
    ) -> Result<Self, ObjectStorageError> {
        Ok(Self {
            backend_type: validate_required_text(
                backend_type,
                ObjectStorageError::InvalidObjectMetadata,
            )?,
            operation: validate_required_text(
                operation,
                ObjectStorageError::InvalidObjectMetadata,
            )?,
            object_key_hash: validate_required_text(
                &object_key_hash,
                ObjectStorageError::InvalidObjectMetadata,
            )?,
            retry_count,
            duration_bucket: validate_required_text(
                duration_bucket,
                ObjectStorageError::InvalidObjectMetadata,
            )?,
        })
    }

    pub fn backend_type(&self) -> &str {
        &self.backend_type
    }

    pub fn operation(&self) -> &str {
        &self.operation
    }

    pub fn object_key_hash(&self) -> &str {
        &self.object_key_hash
    }

    pub fn retry_count(&self) -> ObjectStorageRetryCount {
        self.retry_count
    }

    pub fn duration_bucket(&self) -> &str {
        &self.duration_bucket
    }
}

fn validate_required_text(
    value: &str,
    error: ObjectStorageError,
) -> Result<String, ObjectStorageError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(error);
    }
    Ok(trimmed.to_string())
}
