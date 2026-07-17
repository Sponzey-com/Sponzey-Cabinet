use cabinet_domain::asset::{AssetFileName, AssetId};
use cabinet_domain::workspace::WorkspaceId;

pub trait AssetExternalOpener: Send + Sync {
    fn open(
        &self,
        workspace: &WorkspaceId,
        asset: &AssetId,
        file_name: &AssetFileName,
    ) -> Result<(), AssetExternalOpenError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetExternalOpenError {
    NotFound,
    Corrupted,
    StorageUnavailable,
    LauncherUnavailable,
}

impl AssetExternalOpenError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotFound => "asset_external_open.not_found",
            Self::Corrupted => "asset_external_open.corrupted",
            Self::StorageUnavailable => "asset_external_open.storage_unavailable",
            Self::LauncherUnavailable => "asset_external_open.launcher_unavailable",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(self, Self::StorageUnavailable | Self::LauncherUnavailable)
    }
}
