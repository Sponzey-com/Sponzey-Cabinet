use cabinet_domain::asset::{AssetId, AssetPreviewCapability};
use cabinet_domain::workspace::WorkspaceId;

pub trait AssetPreviewReader {
    fn read(
        &self,
        workspace: &WorkspaceId,
        asset: &AssetId,
        max_bytes: usize,
    ) -> Result<Vec<u8>, AssetPreviewReadError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetPreviewReadError {
    NotFound,
    TooLarge,
    Corrupted,
    StorageUnavailable,
}

impl AssetPreviewReadError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotFound => "asset_preview.not_found",
            Self::TooLarge => "asset_preview.too_large",
            Self::Corrupted => "asset_preview.corrupted",
            Self::StorageUnavailable => "asset_preview.storage_unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetPreviewContent {
    capability: AssetPreviewCapability,
    media_type: String,
    bytes: Vec<u8>,
}

impl AssetPreviewContent {
    pub fn new(capability: AssetPreviewCapability, media_type: String, bytes: Vec<u8>) -> Self {
        Self { capability, media_type, bytes }
    }
    pub fn capability(&self) -> AssetPreviewCapability { self.capability }
    pub fn media_type(&self) -> &str { &self.media_type }
    pub fn bytes(&self) -> &[u8] { &self.bytes }
}
