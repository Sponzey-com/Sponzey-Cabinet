use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use cabinet_domain::asset::AssetId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_availability::{
    AssetAvailability, AssetAvailabilityBatchResolver, AssetAvailabilityRecord,
    AssetAvailabilityResolveError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAssetAvailabilityResolver {
    root: PathBuf,
}

impl LocalAssetAvailabilityResolver {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn object_path(&self, workspace_id: &WorkspaceId, asset_id: &AssetId) -> PathBuf {
        self.root
            .join("assets/objects")
            .join(hex(workspace_id.as_str()))
            .join(&asset_id.as_str()[..2])
            .join(format!("{}.bin", asset_id.as_str()))
    }
}

impl AssetAvailabilityBatchResolver for LocalAssetAvailabilityResolver {
    fn resolve_batch(
        &self,
        workspace_id: &WorkspaceId,
        asset_ids: &[AssetId],
    ) -> Result<Vec<AssetAvailabilityRecord>, AssetAvailabilityResolveError> {
        asset_ids
            .iter()
            .map(|asset_id| {
                let availability =
                    match fs::symlink_metadata(self.object_path(workspace_id, asset_id)) {
                        Ok(metadata) if metadata.file_type().is_file() => {
                            AssetAvailability::Available
                        }
                        Ok(_) => return Err(AssetAvailabilityResolveError::CorruptedData),
                        Err(error) if error.kind() == ErrorKind::NotFound => {
                            AssetAvailability::Missing
                        }
                        Err(_) => return Err(AssetAvailabilityResolveError::StorageUnavailable),
                    };
                Ok(AssetAvailabilityRecord::new(asset_id.clone(), availability))
            })
            .collect()
    }
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
