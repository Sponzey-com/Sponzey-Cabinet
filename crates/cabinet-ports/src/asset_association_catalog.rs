use cabinet_domain::asset::{AssetAssociation, AssetId};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetAssociationLinkOutcome {
    Linked,
    AlreadyLinked,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetAssociationUnlinkOutcome {
    Unlinked,
    NotLinked,
}

pub trait DocumentAssetAssociationReader {
    fn list_document_assets(
        &self,
        workspace: &WorkspaceId,
        document: &DocumentId,
        limit: usize,
    ) -> Result<Vec<AssetAssociation>, AssetAssociationCatalogError>;
}

impl<T: AssetAssociationCatalog + ?Sized> DocumentAssetAssociationReader for T {
    fn list_document_assets(
        &self,
        workspace: &WorkspaceId,
        document: &DocumentId,
        limit: usize,
    ) -> Result<Vec<AssetAssociation>, AssetAssociationCatalogError> {
        self.list_assets(workspace, document, limit)
    }
}

pub trait AssetAssociationCatalog {
    fn link(
        &mut self,
        workspace: &WorkspaceId,
        association: AssetAssociation,
    ) -> Result<AssetAssociationLinkOutcome, AssetAssociationCatalogError>;
    fn unlink(
        &mut self,
        workspace: &WorkspaceId,
        asset: &AssetId,
        document: &DocumentId,
    ) -> Result<AssetAssociationUnlinkOutcome, AssetAssociationCatalogError>;
    fn list_documents(
        &self,
        workspace: &WorkspaceId,
        asset: &AssetId,
        limit: usize,
    ) -> Result<Vec<AssetAssociation>, AssetAssociationCatalogError>;
    fn list_assets(
        &self,
        workspace: &WorkspaceId,
        document: &DocumentId,
        limit: usize,
    ) -> Result<Vec<AssetAssociation>, AssetAssociationCatalogError>;
    fn reference_count(
        &self,
        workspace: &WorkspaceId,
        asset: &AssetId,
    ) -> Result<u64, AssetAssociationCatalogError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetAssociationCatalogError {
    InvalidLimit,
    Conflict,
    StorageUnavailable,
    CorruptedRecord,
    UnsupportedSchema,
}
impl AssetAssociationCatalogError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimit => "asset_association.invalid_limit",
            Self::Conflict => "asset_association.conflict",
            Self::StorageUnavailable => "asset_association.storage_unavailable",
            Self::CorruptedRecord => "asset_association.corrupted",
            Self::UnsupportedSchema => "asset_association.unsupported_schema",
        }
    }
}
