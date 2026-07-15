use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;

use cabinet_domain::asset::AssetId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_preview::{AssetPreviewReadError, AssetPreviewReader};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct LocalAssetPreviewReader { root: PathBuf }

impl LocalAssetPreviewReader {
    pub fn new(root: PathBuf) -> Self { Self { root } }
    fn object_path(&self, workspace: &WorkspaceId, asset: &AssetId) -> PathBuf {
        self.root.join("assets/objects").join(hex(workspace.as_str()))
            .join(&asset.as_str()[..2]).join(format!("{}.bin", asset.as_str()))
    }
}

impl AssetPreviewReader for LocalAssetPreviewReader {
    fn read(&self, workspace: &WorkspaceId, asset: &AssetId, max_bytes: usize) -> Result<Vec<u8>, AssetPreviewReadError> {
        if max_bytes == 0 { return Err(AssetPreviewReadError::TooLarge); }
        let path = self.object_path(workspace, asset);
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound { AssetPreviewReadError::NotFound } else { AssetPreviewReadError::StorageUnavailable }
        })?;
        if !metadata.file_type().is_file() { return Err(AssetPreviewReadError::Corrupted); }
        if metadata.len() > max_bytes as u64 { return Err(AssetPreviewReadError::TooLarge); }
        let file = File::open(path).map_err(|_| AssetPreviewReadError::StorageUnavailable)?;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.take(max_bytes as u64 + 1).read_to_end(&mut bytes).map_err(|_| AssetPreviewReadError::StorageUnavailable)?;
        if bytes.len() > max_bytes { return Err(AssetPreviewReadError::TooLarge); }
        let actual = format!("{:x}", Sha256::digest(&bytes));
        if actual != asset.as_str() { return Err(AssetPreviewReadError::Corrupted); }
        Ok(bytes)
    }
}

fn hex(value: &str) -> String {
    value.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect()
}
