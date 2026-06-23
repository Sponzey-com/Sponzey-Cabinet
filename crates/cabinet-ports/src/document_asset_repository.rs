use cabinet_domain::asset::{AssetId, AssetMetadata, AssetReference};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentAssetRecord {
    reference: AssetReference,
    metadata: AssetMetadata,
}

impl DocumentAssetRecord {
    pub fn new(
        reference: AssetReference,
        metadata: AssetMetadata,
    ) -> Result<Self, DocumentAssetRepositoryError> {
        if reference.asset_id() != metadata.id() {
            return Err(DocumentAssetRepositoryError::MismatchedAssetReference);
        }
        Ok(Self {
            reference,
            metadata,
        })
    }

    pub fn reference(&self) -> &AssetReference {
        &self.reference
    }

    pub fn metadata(&self) -> &AssetMetadata {
        &self.metadata
    }

    pub fn asset_id(&self) -> &AssetId {
        self.metadata.id()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentAssetAttachOutcome {
    Attached,
    AlreadyAttached,
}

pub trait DocumentAssetRepository {
    fn attach_asset(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        record: DocumentAssetRecord,
    ) -> Result<DocumentAssetAttachOutcome, DocumentAssetRepositoryError>;

    fn list_assets(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Vec<DocumentAssetRecord>, DocumentAssetRepositoryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentAssetRepositoryError {
    MismatchedAssetReference,
    InvalidAssociation,
    StorageUnavailable,
    CorruptedMetadata,
    Conflict,
}

impl DocumentAssetRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::MismatchedAssetReference => {
                "document_asset_repository.mismatched_asset_reference"
            }
            Self::InvalidAssociation => "document_asset_repository.invalid_association",
            Self::StorageUnavailable => "document_asset_repository.storage_unavailable",
            Self::CorruptedMetadata => "document_asset_repository.corrupted_metadata",
            Self::Conflict => "document_asset_repository.conflict",
        }
    }
}
