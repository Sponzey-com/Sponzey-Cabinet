use cabinet_domain::asset::AssetId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_external_open::{AssetExternalOpenError, AssetExternalOpener};
use cabinet_ports::asset_metadata_catalog::{AssetMetadataCatalog, AssetMetadataCatalogError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAssetExternallyInput {
    workspace: WorkspaceId,
    asset: AssetId,
}

impl OpenAssetExternallyInput {
    pub fn new(workspace: &str, asset: &str) -> Result<Self, OpenAssetExternallyError> {
        Ok(Self {
            workspace: WorkspaceId::new(workspace)
                .map_err(|_| OpenAssetExternallyError::InvalidInput)?,
            asset: AssetId::from_sha256_hex(asset)
                .map_err(|_| OpenAssetExternallyError::InvalidInput)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenAssetExternallyOutput;

impl OpenAssetExternallyOutput {
    pub const fn opened(&self) -> bool {
        true
    }
}

pub struct OpenAssetExternallyUsecase;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetExternalOpenProductEvent {
    Failed { error_code: &'static str },
}

impl AssetExternalOpenProductEvent {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Failed { .. } => "document.attachment.open_failed",
        }
    }
}

pub trait AssetExternalOpenProductLogger {
    fn write_product(&mut self, event: AssetExternalOpenProductEvent);
}

impl OpenAssetExternallyUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute<
        M: AssetMetadataCatalog,
        O: AssetExternalOpener + ?Sized,
        L: AssetExternalOpenProductLogger,
    >(
        &self,
        input: OpenAssetExternallyInput,
        metadata: &M,
        opener: &O,
        product_logger: &mut L,
    ) -> Result<OpenAssetExternallyOutput, OpenAssetExternallyError> {
        let result: Result<OpenAssetExternallyOutput, OpenAssetExternallyError> = (|| {
            let record = metadata
                .get(&input.workspace, &input.asset)
                .map_err(OpenAssetExternallyError::Metadata)?
                .ok_or(OpenAssetExternallyError::NotFound)?;
            opener
                .open(
                    &input.workspace,
                    &input.asset,
                    record.metadata().file_name(),
                )
                .map_err(OpenAssetExternallyError::Open)?;
            Ok(OpenAssetExternallyOutput)
        })();
        if let Err(error) = result {
            product_logger.write_product(AssetExternalOpenProductEvent::Failed {
                error_code: error.code(),
            });
        }
        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenAssetExternallyError {
    InvalidInput,
    NotFound,
    Metadata(AssetMetadataCatalogError),
    Open(AssetExternalOpenError),
}

impl OpenAssetExternallyError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "asset_external_open.invalid_input",
            Self::NotFound => "asset_external_open.not_found",
            Self::Metadata(error) => error.code(),
            Self::Open(error) => error.code(),
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(
            self,
            Self::Metadata(AssetMetadataCatalogError::StorageUnavailable)
        ) || matches!(self, Self::Open(error) if error.retryable())
    }
}
