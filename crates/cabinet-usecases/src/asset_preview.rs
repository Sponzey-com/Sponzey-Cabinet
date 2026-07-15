use cabinet_domain::asset::{AssetId, AssetPreviewCapability};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_metadata_catalog::{AssetMetadataCatalog, AssetMetadataCatalogError};
use cabinet_ports::asset_preview::{AssetPreviewReadError, AssetPreviewReader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetAssetPreviewInput {
    workspace: WorkspaceId,
    asset: AssetId,
    max_bytes: usize,
}
impl GetAssetPreviewInput {
    pub fn new(workspace: &str, asset: &str, max_bytes: usize) -> Result<Self, AssetPreviewError> {
        if max_bytes == 0 { return Err(AssetPreviewError::InvalidInput); }
        Ok(Self {
            workspace: WorkspaceId::new(workspace).map_err(|_| AssetPreviewError::InvalidInput)?,
            asset: AssetId::from_sha256_hex(asset).map_err(|_| AssetPreviewError::InvalidInput)?,
            max_bytes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetPreviewResult { Content(Vec<u8>), Unsupported }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetAssetPreviewOutput {
    capability: AssetPreviewCapability,
    media_type: String,
    result: AssetPreviewResult,
}
impl GetAssetPreviewOutput {
    pub fn capability(&self) -> AssetPreviewCapability { self.capability }
    pub fn media_type(&self) -> &str { &self.media_type }
    pub fn result(&self) -> &AssetPreviewResult { &self.result }
}

pub struct GetAssetPreviewUsecase;
impl GetAssetPreviewUsecase {
    pub const fn new() -> Self { Self }
    pub fn execute<M: AssetMetadataCatalog, R: AssetPreviewReader>(
        &self,
        input: GetAssetPreviewInput,
        metadata: &M,
        reader: &R,
    ) -> Result<GetAssetPreviewOutput, AssetPreviewError> {
        let record = metadata.get(&input.workspace, &input.asset)
            .map_err(AssetPreviewError::Metadata)?
            .ok_or(AssetPreviewError::NotFound)?;
        let capability = record.preview();
        let media_type = record.metadata().media_type().as_str().to_string();
        if capability == AssetPreviewCapability::Unsupported {
            return Ok(GetAssetPreviewOutput { capability, media_type, result: AssetPreviewResult::Unsupported });
        }
        if record.metadata().byte_size() > input.max_bytes as u64 {
            return Err(AssetPreviewError::Read(AssetPreviewReadError::TooLarge));
        }
        let bytes = reader.read(&input.workspace, &input.asset, input.max_bytes)
            .map_err(AssetPreviewError::Read)?;
        Ok(GetAssetPreviewOutput { capability, media_type, result: AssetPreviewResult::Content(bytes) })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetPreviewError {
    InvalidInput,
    NotFound,
    Metadata(AssetMetadataCatalogError),
    Read(AssetPreviewReadError),
}
impl AssetPreviewError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "asset_preview.invalid_input",
            Self::NotFound => "asset_preview.not_found",
            Self::Metadata(error) => error.code(),
            Self::Read(error) => error.code(),
        }
    }
    pub const fn retryable(self) -> bool {
        matches!(self, Self::Metadata(AssetMetadataCatalogError::StorageUnavailable) | Self::Read(AssetPreviewReadError::StorageUnavailable))
    }
}
