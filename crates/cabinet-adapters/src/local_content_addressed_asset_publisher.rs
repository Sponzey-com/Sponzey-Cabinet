use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use cabinet_domain::asset::AssetId;
use cabinet_domain::asset_import_operation::AssetImportOperationId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_object_publisher::{
    AssetObjectPublishError, AssetObjectPublishOutcome, AssetObjectPublisher, PublishedAssetObject,
};
use sha2::{Digest, Sha256};

use crate::local_asset_staging_writer::staging_asset_path;

#[derive(Debug, Clone)]
pub struct LocalContentAddressedAssetPublisher {
    root: PathBuf,
    hash_chunk_bytes: usize,
}
impl LocalContentAddressedAssetPublisher {
    pub fn new(root: PathBuf, hash_chunk_bytes: usize) -> Result<Self, AssetObjectPublishError> {
        if hash_chunk_bytes == 0 {
            return Err(AssetObjectPublishError::InvalidConfig);
        }
        Ok(Self {
            root,
            hash_chunk_bytes,
        })
    }
    fn final_path(&self, workspace: &WorkspaceId, asset_id: &AssetId) -> PathBuf {
        self.root
            .join("assets/objects")
            .join(hex(workspace.as_str()))
            .join(&asset_id.as_str()[0..2])
            .join(format!("{}.bin", asset_id.as_str()))
    }
}

impl AssetObjectPublisher for LocalContentAddressedAssetPublisher {
    fn publish(
        &mut self,
        workspace: &WorkspaceId,
        operation: &AssetImportOperationId,
        expected_size: u64,
    ) -> Result<PublishedAssetObject, AssetObjectPublishError> {
        if expected_size == 0 {
            return Err(AssetObjectPublishError::SizeMismatch);
        }
        let staging = staging_asset_path(&self.root, workspace, operation);
        let metadata = fs::metadata(&staging).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AssetObjectPublishError::StagingNotFound
            } else {
                AssetObjectPublishError::StorageUnavailable
            }
        })?;
        if metadata.len() != expected_size {
            return Err(AssetObjectPublishError::SizeMismatch);
        }
        let asset_id = hash_file(&staging, self.hash_chunk_bytes)?;
        let final_path = self.final_path(workspace, &asset_id);
        fs::create_dir_all(
            final_path
                .parent()
                .ok_or(AssetObjectPublishError::StorageUnavailable)?,
        )
        .map_err(|_| AssetObjectPublishError::StorageUnavailable)?;
        let outcome = match fs::hard_link(&staging, &final_path) {
            Ok(()) => AssetObjectPublishOutcome::Created,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = fs::metadata(&final_path)
                    .map_err(|_| AssetObjectPublishError::StorageUnavailable)?;
                if existing.len() != expected_size {
                    return Err(AssetObjectPublishError::Conflict);
                }
                AssetObjectPublishOutcome::AlreadyPresent
            }
            Err(_) => return Err(AssetObjectPublishError::StorageUnavailable),
        };
        fs::remove_file(staging).map_err(|_| AssetObjectPublishError::StorageUnavailable)?;
        PublishedAssetObject::new(asset_id, expected_size, outcome)
    }
}

fn hash_file(path: &Path, chunk_bytes: usize) -> Result<AssetId, AssetObjectPublishError> {
    let file = File::open(path).map_err(|_| AssetObjectPublishError::StorageUnavailable)?;
    let mut reader = BufReader::with_capacity(chunk_bytes, file);
    let mut buffer = vec![0_u8; chunk_bytes];
    let mut hasher = Sha256::new();
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| AssetObjectPublishError::StorageUnavailable)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    AssetId::from_sha256_hex(&format!("{:x}", hasher.finalize()))
        .map_err(|_| AssetObjectPublishError::InvalidHash)
}
fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
