use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use cabinet_domain::asset_import_operation::AssetImportOperationId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_staging::{AssetStagingError, AssetStagingWriter, StagedAsset};

#[derive(Debug, Clone)]
pub struct LocalAssetStagingWriter {
    root: PathBuf,
}
impl LocalAssetStagingWriter {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn path(&self, workspace: &WorkspaceId, operation: &AssetImportOperationId) -> PathBuf {
        staging_asset_path(&self.root, workspace, operation)
    }
}
impl AssetStagingWriter for LocalAssetStagingWriter {
    fn begin(
        &mut self,
        workspace: &WorkspaceId,
        operation: &AssetImportOperationId,
    ) -> Result<(), AssetStagingError> {
        let path = self.path(workspace, operation);
        fs::create_dir_all(path.parent().ok_or(AssetStagingError::StorageUnavailable)?)
            .map_err(|_| AssetStagingError::StorageUnavailable)?;
        fs::File::create(path)
            .map(|_| ())
            .map_err(|_| AssetStagingError::StorageUnavailable)
    }
    fn append(
        &mut self,
        workspace: &WorkspaceId,
        operation: &AssetImportOperationId,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), AssetStagingError> {
        if bytes.is_empty() {
            return Err(AssetStagingError::InvalidInput);
        }
        let path = self.path(workspace, operation);
        let size = fs::metadata(&path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    AssetStagingError::NotFound
                } else {
                    AssetStagingError::StorageUnavailable
                }
            })?
            .len();
        if size != offset {
            return Err(AssetStagingError::OffsetConflict);
        }
        OpenOptions::new()
            .append(true)
            .open(path)
            .and_then(|mut file| {
                file.write_all(bytes)?;
                file.sync_data()
            })
            .map_err(|_| AssetStagingError::StorageUnavailable)
    }
    fn finalize(
        &mut self,
        workspace: &WorkspaceId,
        operation: &AssetImportOperationId,
        expected: u64,
    ) -> Result<StagedAsset, AssetStagingError> {
        let size = fs::metadata(self.path(workspace, operation))
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    AssetStagingError::NotFound
                } else {
                    AssetStagingError::StorageUnavailable
                }
            })?
            .len();
        StagedAsset::new(operation.clone(), size, expected)
    }
    fn cleanup(
        &mut self,
        workspace: &WorkspaceId,
        operation: &AssetImportOperationId,
    ) -> Result<(), AssetStagingError> {
        match fs::remove_file(self.path(workspace, operation)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(AssetStagingError::StorageUnavailable),
        }
    }
}
fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn staging_asset_path(
    root: &std::path::Path,
    workspace: &WorkspaceId,
    operation: &AssetImportOperationId,
) -> PathBuf {
    root.join("staging/assets")
        .join(hex(workspace.as_str()))
        .join(format!("{}.part", hex(operation.as_str())))
}
