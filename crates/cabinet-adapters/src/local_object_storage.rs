use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::object_storage::{
    ObjectContent, ObjectKey, ObjectMetadata, ObjectRecord, ObjectStorage,
    ObjectStorageDeleteOutcome, ObjectStorageError, ObjectStorageHealth, ObjectStoragePutOutcome,
};

use crate::local_atomic_file::{
    write_bytes_atomically as atomic_write_bytes, write_text_atomically as atomic_write_text,
};

pub const OBJECT_METADATA_ROOT_DIR: &str = "metadata";
pub const OBJECT_CONTENT_ROOT_DIR: &str = "content";
pub const OBJECT_METADATA_FILE: &str = "metadata.txt";
pub const OBJECT_CONTENT_EXTENSION: &str = "bin";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalObjectStorage {
    root: PathBuf,
}

impl LocalObjectStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn workspace_dir(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.root.join(encode_path_segment(workspace_id.as_str()))
    }

    fn metadata_path(&self, workspace_id: &WorkspaceId, key: &ObjectKey) -> PathBuf {
        self.workspace_dir(workspace_id)
            .join(OBJECT_METADATA_ROOT_DIR)
            .join(key.as_str())
            .join(OBJECT_METADATA_FILE)
    }

    fn content_path(&self, workspace_id: &WorkspaceId, key: &ObjectKey) -> PathBuf {
        self.workspace_dir(workspace_id)
            .join(OBJECT_CONTENT_ROOT_DIR)
            .join(&key.as_str()[0..2])
            .join(format!("{}.{}", key.as_str(), OBJECT_CONTENT_EXTENSION))
    }
}

impl ObjectStorage for LocalObjectStorage {
    fn put_object(
        &mut self,
        workspace_id: &WorkspaceId,
        record: ObjectRecord,
    ) -> Result<ObjectStoragePutOutcome, ObjectStorageError> {
        let metadata_path = self.metadata_path(workspace_id, record.key());
        let content_path = self.content_path(workspace_id, record.key());

        if metadata_path.exists() && content_path.exists() {
            return Ok(ObjectStoragePutOutcome::AlreadyPresent);
        }
        if metadata_path.exists() && !content_path.exists() {
            return Err(ObjectStorageError::MissingContent);
        }
        if !metadata_path.exists() && content_path.exists() {
            return Err(ObjectStorageError::Conflict);
        }

        write_bytes_atomically(content_path, record.content().bytes())?;
        write_text_atomically(metadata_path, metadata_content(record.metadata()))?;
        Ok(ObjectStoragePutOutcome::Created)
    }

    fn get_metadata(
        &self,
        workspace_id: &WorkspaceId,
        key: &ObjectKey,
    ) -> Result<Option<ObjectMetadata>, ObjectStorageError> {
        let metadata_path = self.metadata_path(workspace_id, key);
        if !metadata_path.exists() {
            return Ok(None);
        }
        read_metadata(&metadata_path).map(Some)
    }

    fn get_content(
        &mut self,
        workspace_id: &WorkspaceId,
        key: &ObjectKey,
    ) -> Result<Option<ObjectContent>, ObjectStorageError> {
        let content_path = self.content_path(workspace_id, key);
        if !content_path.exists() {
            if self.metadata_path(workspace_id, key).exists() {
                return Err(ObjectStorageError::MissingContent);
            }
            return Ok(None);
        }

        let bytes = fs::read(content_path).map_err(|_| ObjectStorageError::ContentUnavailable)?;
        ObjectContent::new(key.clone(), bytes).map(Some)
    }

    fn delete_object(
        &mut self,
        workspace_id: &WorkspaceId,
        key: &ObjectKey,
    ) -> Result<ObjectStorageDeleteOutcome, ObjectStorageError> {
        let removed_content = remove_file_if_exists(self.content_path(workspace_id, key))?;
        let metadata_dir = self
            .metadata_path(workspace_id, key)
            .parent()
            .ok_or(ObjectStorageError::MetadataUnavailable)?
            .to_path_buf();
        let removed_metadata = remove_dir_if_exists(metadata_dir)?;

        if removed_content || removed_metadata {
            return Ok(ObjectStorageDeleteOutcome::Deleted);
        }
        Ok(ObjectStorageDeleteOutcome::Missing)
    }

    fn probe_health(&self) -> Result<ObjectStorageHealth, ObjectStorageError> {
        fs::create_dir_all(&self.root).map_err(|_| ObjectStorageError::StorageUnavailable)?;
        Ok(ObjectStorageHealth::healthy("local-disk"))
    }
}

fn metadata_content(metadata: &ObjectMetadata) -> String {
    format!(
        "key={}\nbyte_size={}\nmedia_type={}\ncontent_hash={}\n",
        metadata.key().as_str(),
        metadata.byte_size(),
        metadata.media_type(),
        metadata.content_hash()
    )
}

fn read_metadata(path: &Path) -> Result<ObjectMetadata, ObjectStorageError> {
    let content = fs::read_to_string(path).map_err(|_| ObjectStorageError::MetadataUnavailable)?;
    let mut key = None;
    let mut byte_size = None;
    let mut media_type = None;
    let mut content_hash = None;

    for line in content.lines() {
        let (field, value) = line
            .split_once('=')
            .ok_or(ObjectStorageError::MetadataUnavailable)?;
        match field {
            "key" => key = Some(value),
            "byte_size" => byte_size = Some(value),
            "media_type" => media_type = Some(value),
            "content_hash" => content_hash = Some(value),
            _ => return Err(ObjectStorageError::MetadataUnavailable),
        }
    }

    ObjectMetadata::new(
        ObjectKey::from_sha256_hex(key.ok_or(ObjectStorageError::MetadataUnavailable)?)?,
        byte_size
            .ok_or(ObjectStorageError::MetadataUnavailable)?
            .parse::<u64>()
            .map_err(|_| ObjectStorageError::MetadataUnavailable)?,
        media_type.ok_or(ObjectStorageError::MetadataUnavailable)?,
        content_hash.ok_or(ObjectStorageError::MetadataUnavailable)?,
    )
    .map_err(|_| ObjectStorageError::MetadataUnavailable)
}

fn write_text_atomically(
    path: PathBuf,
    content: impl AsRef<str>,
) -> Result<(), ObjectStorageError> {
    atomic_write_text(&path, content)
        .map(|_| ())
        .map_err(|_| ObjectStorageError::MetadataUnavailable)
}

fn write_bytes_atomically(path: PathBuf, bytes: &[u8]) -> Result<(), ObjectStorageError> {
    atomic_write_bytes(&path, bytes)
        .map(|_| ())
        .map_err(|_| ObjectStorageError::ContentUnavailable)
}

fn remove_file_if_exists(path: PathBuf) -> Result<bool, ObjectStorageError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(_) => Err(ObjectStorageError::ContentUnavailable),
    }
}

fn remove_dir_if_exists(path: PathBuf) -> Result<bool, ObjectStorageError> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(_) => Err(ObjectStorageError::MetadataUnavailable),
    }
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' {
            encoded.push(byte as char);
        } else {
            encoded.push('~');
            encoded.push_str(&format!("{byte:02x}"));
        }
    }
    encoded
}
