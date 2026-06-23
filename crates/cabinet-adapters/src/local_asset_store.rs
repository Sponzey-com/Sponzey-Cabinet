use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use cabinet_domain::asset::{AssetFileName, AssetId, AssetMediaType, AssetMetadata};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_store::{
    AssetObject, AssetRecord, AssetStore, AssetStoreError, AssetStorePutOutcome,
};

use crate::local_atomic_file::{
    write_bytes_atomically as atomic_write_bytes, write_text_atomically as atomic_write_text,
};

pub const ASSET_METADATA_DIR: &str = "metadata";
pub const ASSET_OBJECTS_DIR: &str = "objects";
pub const ASSET_METADATA_FILE: &str = "metadata.txt";
pub const ASSET_OBJECT_EXTENSION: &str = "bin";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAssetStore {
    asset_store_root: PathBuf,
}

impl LocalAssetStore {
    pub fn new(asset_store_root: PathBuf) -> Self {
        Self { asset_store_root }
    }

    fn workspace_dir(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.asset_store_root
            .join(encode_path_segment(workspace_id.as_str()))
    }

    fn metadata_path(&self, workspace_id: &WorkspaceId, asset_id: &AssetId) -> PathBuf {
        self.workspace_dir(workspace_id)
            .join(ASSET_METADATA_DIR)
            .join(asset_id.as_str())
            .join(ASSET_METADATA_FILE)
    }

    fn object_path(&self, workspace_id: &WorkspaceId, asset_id: &AssetId) -> PathBuf {
        self.workspace_dir(workspace_id)
            .join(ASSET_OBJECTS_DIR)
            .join(&asset_id.as_str()[0..2])
            .join(format!("{}.{}", asset_id.as_str(), ASSET_OBJECT_EXTENSION))
    }
}

impl AssetStore for LocalAssetStore {
    fn put_asset(
        &mut self,
        workspace_id: &WorkspaceId,
        record: AssetRecord,
    ) -> Result<AssetStorePutOutcome, AssetStoreError> {
        let metadata_path = self.metadata_path(workspace_id, record.asset_id());
        let object_path = self.object_path(workspace_id, record.asset_id());

        if metadata_path.exists() && object_path.exists() {
            return Ok(AssetStorePutOutcome::AlreadyPresent);
        }
        if metadata_path.exists() && !object_path.exists() {
            return Err(AssetStoreError::MissingObject);
        }

        write_text_atomically(metadata_path, metadata_content(record.metadata()))?;
        write_bytes_atomically(object_path, record.object().bytes())?;
        Ok(AssetStorePutOutcome::Created)
    }

    fn get_metadata(
        &self,
        workspace_id: &WorkspaceId,
        asset_id: &AssetId,
    ) -> Result<Option<AssetMetadata>, AssetStoreError> {
        let metadata_path = self.metadata_path(workspace_id, asset_id);
        if !metadata_path.exists() {
            return Ok(None);
        }
        read_metadata(&metadata_path).map(Some)
    }

    fn get_object(
        &self,
        workspace_id: &WorkspaceId,
        asset_id: &AssetId,
    ) -> Result<Option<AssetObject>, AssetStoreError> {
        let object_path = self.object_path(workspace_id, asset_id);
        if !object_path.exists() {
            if self.metadata_path(workspace_id, asset_id).exists() {
                return Err(AssetStoreError::MissingObject);
            }
            return Ok(None);
        }

        let bytes = fs::read(object_path).map_err(|_| AssetStoreError::StorageUnavailable)?;
        AssetObject::new(asset_id.clone(), bytes)
            .map(Some)
            .map_err(|_| AssetStoreError::CorruptedMetadata)
    }

    fn remove_asset(
        &mut self,
        workspace_id: &WorkspaceId,
        asset_id: &AssetId,
    ) -> Result<(), AssetStoreError> {
        remove_file_if_exists(self.object_path(workspace_id, asset_id))?;
        let metadata_dir = self
            .metadata_path(workspace_id, asset_id)
            .parent()
            .ok_or(AssetStoreError::StorageUnavailable)?
            .to_path_buf();
        if metadata_dir.exists() {
            fs::remove_dir_all(metadata_dir).map_err(|_| AssetStoreError::StorageUnavailable)?;
        }
        Ok(())
    }
}

fn metadata_content(metadata: &AssetMetadata) -> String {
    format!(
        "id={}\nfile_name={}\nmedia_type={}\nbyte_size={}\n",
        metadata.id().as_str(),
        metadata.file_name().as_str(),
        metadata.media_type().as_str(),
        metadata.byte_size()
    )
}

fn read_metadata(path: &Path) -> Result<AssetMetadata, AssetStoreError> {
    let content = fs::read_to_string(path).map_err(|_| AssetStoreError::CorruptedMetadata)?;
    let mut id = None;
    let mut file_name = None;
    let mut media_type = None;
    let mut byte_size = None;

    for line in content.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(AssetStoreError::CorruptedMetadata)?;
        match key {
            "id" => id = Some(value),
            "file_name" => file_name = Some(value),
            "media_type" => media_type = Some(value),
            "byte_size" => byte_size = Some(value),
            _ => return Err(AssetStoreError::CorruptedMetadata),
        }
    }

    AssetMetadata::new(
        AssetId::from_sha256_hex(id.ok_or(AssetStoreError::CorruptedMetadata)?)
            .map_err(|_| AssetStoreError::CorruptedMetadata)?,
        AssetFileName::new(file_name.ok_or(AssetStoreError::CorruptedMetadata)?)
            .map_err(|_| AssetStoreError::CorruptedMetadata)?,
        AssetMediaType::new(media_type.ok_or(AssetStoreError::CorruptedMetadata)?)
            .map_err(|_| AssetStoreError::CorruptedMetadata)?,
        byte_size
            .ok_or(AssetStoreError::CorruptedMetadata)?
            .parse::<u64>()
            .map_err(|_| AssetStoreError::CorruptedMetadata)?,
    )
    .map_err(|_| AssetStoreError::CorruptedMetadata)
}

fn write_text_atomically(path: PathBuf, content: impl AsRef<str>) -> Result<(), AssetStoreError> {
    atomic_write_text(&path, content)
        .map(|_| ())
        .map_err(|_| AssetStoreError::StorageUnavailable)
}

fn write_bytes_atomically(path: PathBuf, bytes: &[u8]) -> Result<(), AssetStoreError> {
    atomic_write_bytes(&path, bytes)
        .map(|_| ())
        .map_err(|_| AssetStoreError::StorageUnavailable)
}

fn remove_file_if_exists(path: PathBuf) -> Result<(), AssetStoreError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(AssetStoreError::StorageUnavailable),
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
