use cabinet_domain::asset::AssetAssociation;
use cabinet_domain::workspace::WorkspaceId;

use crate::asset_association_catalog::{
    AssetAssociationCatalog, AssetAssociationCatalogError, AssetAssociationLinkOutcome,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportedAssetDocumentLinkOutcome {
    Linked,
    AlreadyLinked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportedAssetDocumentLinkError {
    InvalidInput,
    NotFound,
    LegacyBaselineRequired,
    Conflict,
    OperationConflict,
    CurrentConflict,
    RecoveryRequired,
    StorageUnavailable,
    CorruptedRecord,
    UnsupportedSchema,
}

impl ImportedAssetDocumentLinkError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "imported_asset_link.invalid_input",
            Self::NotFound => "imported_asset_link.not_found",
            Self::LegacyBaselineRequired => "imported_asset_link.legacy_baseline_required",
            Self::Conflict => "asset_association.conflict",
            Self::OperationConflict => "imported_asset_link.operation_conflict",
            Self::CurrentConflict => "imported_asset_link.current_conflict",
            Self::RecoveryRequired => "imported_asset_link.recovery_required",
            Self::StorageUnavailable => "asset_association.storage_unavailable",
            Self::CorruptedRecord => "asset_association.corrupted",
            Self::UnsupportedSchema => "asset_association.unsupported_schema",
        }
    }
}

pub trait ImportedAssetDocumentLinkPort {
    fn link_imported_asset(
        &mut self,
        workspace: &WorkspaceId,
        association: AssetAssociation,
    ) -> Result<ImportedAssetDocumentLinkOutcome, ImportedAssetDocumentLinkError>;
}

impl<T: AssetAssociationCatalog + ?Sized> ImportedAssetDocumentLinkPort for T {
    fn link_imported_asset(
        &mut self,
        workspace: &WorkspaceId,
        association: AssetAssociation,
    ) -> Result<ImportedAssetDocumentLinkOutcome, ImportedAssetDocumentLinkError> {
        self.link(workspace, association)
            .map(|outcome| match outcome {
                AssetAssociationLinkOutcome::Linked => ImportedAssetDocumentLinkOutcome::Linked,
                AssetAssociationLinkOutcome::AlreadyLinked => {
                    ImportedAssetDocumentLinkOutcome::AlreadyLinked
                }
            })
            .map_err(map_catalog_error)
    }
}

const fn map_catalog_error(error: AssetAssociationCatalogError) -> ImportedAssetDocumentLinkError {
    match error {
        AssetAssociationCatalogError::Conflict | AssetAssociationCatalogError::InvalidLimit => {
            ImportedAssetDocumentLinkError::Conflict
        }
        AssetAssociationCatalogError::StorageUnavailable => {
            ImportedAssetDocumentLinkError::StorageUnavailable
        }
        AssetAssociationCatalogError::CorruptedRecord => {
            ImportedAssetDocumentLinkError::CorruptedRecord
        }
        AssetAssociationCatalogError::UnsupportedSchema => {
            ImportedAssetDocumentLinkError::UnsupportedSchema
        }
    }
}
