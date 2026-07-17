use cabinet_domain::asset::{AssetFileName, AssetId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_external_open::{AssetExternalOpenError, AssetExternalOpener};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub trait ExternalPathLauncher: Send + Sync {
    fn launch(&self, path: &Path) -> Result<(), ()>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MacOsExternalPathLauncher;

impl ExternalPathLauncher for MacOsExternalPathLauncher {
    fn launch(&self, path: &Path) -> Result<(), ()> {
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg(path)
                .spawn()
                .map(|_| ())
                .map_err(|_| ())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = path;
            Err(())
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalAssetExternalOpener<L = MacOsExternalPathLauncher> {
    root: PathBuf,
    launcher: L,
}

impl LocalAssetExternalOpener<MacOsExternalPathLauncher> {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            launcher: MacOsExternalPathLauncher,
        }
    }
}

impl<L: ExternalPathLauncher> LocalAssetExternalOpener<L> {
    pub fn with_launcher(root: PathBuf, launcher: L) -> Self {
        Self { root, launcher }
    }

    fn object_path(&self, workspace: &WorkspaceId, asset: &AssetId) -> PathBuf {
        self.root
            .join("assets/objects")
            .join(hex(workspace.as_str()))
            .join(&asset.as_str()[..2])
            .join(format!("{}.bin", asset.as_str()))
    }

    fn open_copy_path(
        &self,
        workspace: &WorkspaceId,
        asset: &AssetId,
        file_name: &AssetFileName,
    ) -> PathBuf {
        self.root
            .join("asset-open-cache")
            .join(hex(workspace.as_str()))
            .join(asset.as_str())
            .join(file_name.as_str())
    }
}

impl<L: ExternalPathLauncher> AssetExternalOpener for LocalAssetExternalOpener<L> {
    fn open(
        &self,
        workspace: &WorkspaceId,
        asset: &AssetId,
        file_name: &AssetFileName,
    ) -> Result<(), AssetExternalOpenError> {
        let source = self.object_path(workspace, asset);
        let metadata = fs::symlink_metadata(&source).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AssetExternalOpenError::NotFound
            } else {
                AssetExternalOpenError::StorageUnavailable
            }
        })?;
        if !metadata.file_type().is_file() {
            return Err(AssetExternalOpenError::Corrupted);
        }

        let target = self.open_copy_path(workspace, asset, file_name);
        let parent = target.parent().ok_or(AssetExternalOpenError::Corrupted)?;
        fs::create_dir_all(parent).map_err(|_| AssetExternalOpenError::StorageUnavailable)?;
        match fs::symlink_metadata(&target) {
            Ok(metadata) if metadata.file_type().is_file() => {
                fs::remove_file(&target).map_err(|_| AssetExternalOpenError::StorageUnavailable)?;
            }
            Ok(_) => return Err(AssetExternalOpenError::Corrupted),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(AssetExternalOpenError::StorageUnavailable),
        }
        fs::copy(&source, &target).map_err(|_| AssetExternalOpenError::StorageUnavailable)?;
        let mut permissions = fs::metadata(&target)
            .map_err(|_| AssetExternalOpenError::StorageUnavailable)?
            .permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&target, permissions)
            .map_err(|_| AssetExternalOpenError::StorageUnavailable)?;

        self.launcher
            .launch(&target)
            .map_err(|_| AssetExternalOpenError::LauncherUnavailable)
    }
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
