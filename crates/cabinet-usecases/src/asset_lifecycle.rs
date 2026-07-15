use cabinet_domain::asset::{AssetAssociation, AssetCatalogRecord, AssetId};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::{
    AssetAssociationCatalog, AssetAssociationCatalogError, AssetAssociationUnlinkOutcome,
};
use cabinet_ports::asset_metadata_catalog::{AssetMetadataCatalog, AssetMetadataCatalogError};
use cabinet_ports::document_existence::{DocumentExistenceError, DocumentExistenceReader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListWorkspaceAssetsInput {
    workspace_id: WorkspaceId,
    cursor: Option<String>,
    limit: usize,
}
impl ListWorkspaceAssetsInput {
    pub fn new(
        workspace: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Self, AssetLifecycleError> {
        if limit == 0 || limit > 500 {
            return Err(AssetLifecycleError::InvalidInput);
        }
        let cursor = cursor
            .map(|value| {
                AssetId::from_sha256_hex(value)
                    .map(|_| value.to_string())
                    .map_err(|_| AssetLifecycleError::InvalidInput)
            })
            .transpose()?;
        Ok(Self {
            workspace_id: WorkspaceId::new(workspace)
                .map_err(|_| AssetLifecycleError::InvalidInput)?,
            cursor,
            limit,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListWorkspaceAssetsOutput {
    records: Vec<AssetCatalogRecord>,
    next_cursor: Option<String>,
}
impl ListWorkspaceAssetsOutput {
    pub fn records(&self) -> &[AssetCatalogRecord] {
        &self.records
    }
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}

pub struct ListWorkspaceAssetsUsecase;
impl ListWorkspaceAssetsUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<M: AssetMetadataCatalog>(
        &self,
        input: ListWorkspaceAssetsInput,
        metadata: &M,
    ) -> Result<ListWorkspaceAssetsOutput, AssetLifecycleError> {
        let page = metadata
            .list(&input.workspace_id, input.cursor.as_deref(), input.limit)
            .map_err(AssetLifecycleError::Metadata)?;
        Ok(ListWorkspaceAssetsOutput {
            records: page.records().to_vec(),
            next_cursor: page.next_cursor().map(str::to_string),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetAssetDetailInput {
    workspace_id: WorkspaceId,
    asset_id: AssetId,
    document_limit: usize,
}
impl GetAssetDetailInput {
    pub fn new(
        workspace: &str,
        asset: &str,
        document_limit: usize,
    ) -> Result<Self, AssetLifecycleError> {
        if document_limit == 0 || document_limit > 500 {
            return Err(AssetLifecycleError::InvalidInput);
        }
        Ok(Self {
            workspace_id: WorkspaceId::new(workspace)
                .map_err(|_| AssetLifecycleError::InvalidInput)?,
            asset_id: AssetId::from_sha256_hex(asset)
                .map_err(|_| AssetLifecycleError::InvalidInput)?,
            document_limit,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetAssetDetailOutput {
    record: AssetCatalogRecord,
    linked_documents: Vec<AssetAssociation>,
    reference_count: u64,
}
impl GetAssetDetailOutput {
    pub fn record(&self) -> &AssetCatalogRecord {
        &self.record
    }
    pub fn linked_documents(&self) -> &[AssetAssociation] {
        &self.linked_documents
    }
    pub fn reference_count(&self) -> u64 {
        self.reference_count
    }
}

pub struct GetAssetDetailUsecase;
impl GetAssetDetailUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<M: AssetMetadataCatalog, A: AssetAssociationCatalog>(
        &self,
        input: GetAssetDetailInput,
        metadata: &M,
        associations: &A,
    ) -> Result<GetAssetDetailOutput, AssetLifecycleError> {
        let record = metadata
            .get(&input.workspace_id, &input.asset_id)
            .map_err(AssetLifecycleError::Metadata)?
            .ok_or(AssetLifecycleError::AssetNotFound)?;
        let linked_documents = associations
            .list_documents(&input.workspace_id, &input.asset_id, input.document_limit)
            .map_err(AssetLifecycleError::Association)?;
        let reference_count = associations
            .reference_count(&input.workspace_id, &input.asset_id)
            .map_err(AssetLifecycleError::Association)?;
        Ok(GetAssetDetailOutput {
            record,
            linked_documents,
            reference_count,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlinkAssetInput {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    asset_id: AssetId,
}
impl UnlinkAssetInput {
    pub fn new(workspace: &str, document: &str, asset: &str) -> Result<Self, AssetLifecycleError> {
        Ok(Self {
            workspace_id: WorkspaceId::new(workspace)
                .map_err(|_| AssetLifecycleError::InvalidInput)?,
            document_id: DocumentId::new(document)
                .map_err(|_| AssetLifecycleError::InvalidInput)?,
            asset_id: AssetId::from_sha256_hex(asset)
                .map_err(|_| AssetLifecycleError::InvalidInput)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnlinkAssetOutput {
    removed: bool,
    remaining_references: u64,
}
impl UnlinkAssetOutput {
    pub fn removed(self) -> bool {
        self.removed
    }
    pub fn remaining_references(self) -> u64 {
        self.remaining_references
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetLifecycleProductEvent {
    Linked { document_id: String },
    Unlinked { document_id: String },
}
pub trait AssetLifecycleProductLogger {
    fn write_product(&mut self, event: AssetLifecycleProductEvent);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkAssetInput {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    asset_id: AssetId,
    label: String,
}
impl LinkAssetInput {
    pub fn new(
        workspace: &str,
        document: &str,
        asset: &str,
        label: &str,
    ) -> Result<Self, AssetLifecycleError> {
        let asset_id =
            AssetId::from_sha256_hex(asset).map_err(|_| AssetLifecycleError::InvalidInput)?;
        let document_id =
            DocumentId::new(document).map_err(|_| AssetLifecycleError::InvalidInput)?;
        AssetAssociation::new(asset_id.clone(), document_id.clone(), label)
            .map_err(|_| AssetLifecycleError::InvalidInput)?;
        Ok(Self {
            workspace_id: WorkspaceId::new(workspace)
                .map_err(|_| AssetLifecycleError::InvalidInput)?,
            document_id,
            asset_id,
            label: label.to_string(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkAssetOutput {
    linked: bool,
    reference_count: u64,
}
impl LinkAssetOutput {
    pub fn linked(self) -> bool {
        self.linked
    }
    pub fn reference_count(self) -> u64 {
        self.reference_count
    }
}

pub struct LinkAssetUsecase;
impl LinkAssetUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<D, M, A, L>(
        &self,
        input: LinkAssetInput,
        documents: &D,
        metadata: &M,
        associations: &mut A,
        logger: &mut L,
    ) -> Result<LinkAssetOutput, AssetLifecycleError>
    where
        D: DocumentExistenceReader,
        M: AssetMetadataCatalog,
        A: AssetAssociationCatalog,
        L: AssetLifecycleProductLogger,
    {
        match documents.exists(&input.workspace_id, &input.document_id) {
            Ok(true) => {}
            Ok(false) => return Err(AssetLifecycleError::DocumentNotFound),
            Err(error) => return Err(AssetLifecycleError::Document(error)),
        }
        if metadata
            .get(&input.workspace_id, &input.asset_id)
            .map_err(AssetLifecycleError::Metadata)?
            .is_none()
        {
            return Err(AssetLifecycleError::AssetNotFound);
        }
        let outcome = associations
            .link(
                &input.workspace_id,
                AssetAssociation::new(
                    input.asset_id.clone(),
                    input.document_id.clone(),
                    &input.label,
                )
                .map_err(|_| AssetLifecycleError::InvalidInput)?,
            )
            .map_err(AssetLifecycleError::Association)?;
        let linked = outcome
            == cabinet_ports::asset_association_catalog::AssetAssociationLinkOutcome::Linked;
        let reference_count = associations
            .reference_count(&input.workspace_id, &input.asset_id)
            .map_err(AssetLifecycleError::Association)?;
        if linked {
            logger.write_product(AssetLifecycleProductEvent::Linked {
                document_id: input.document_id.as_str().to_string(),
            });
        }
        Ok(LinkAssetOutput {
            linked,
            reference_count,
        })
    }
}

pub struct UnlinkAssetUsecase;
impl UnlinkAssetUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<D, M, A, L>(
        &self,
        input: UnlinkAssetInput,
        documents: &D,
        metadata: &M,
        associations: &mut A,
        logger: &mut L,
    ) -> Result<UnlinkAssetOutput, AssetLifecycleError>
    where
        D: DocumentExistenceReader,
        M: AssetMetadataCatalog,
        A: AssetAssociationCatalog,
        L: AssetLifecycleProductLogger,
    {
        match documents.exists(&input.workspace_id, &input.document_id) {
            Ok(true) => {}
            Ok(false) => return Err(AssetLifecycleError::DocumentNotFound),
            Err(error) => return Err(AssetLifecycleError::Document(error)),
        }
        if metadata
            .get(&input.workspace_id, &input.asset_id)
            .map_err(AssetLifecycleError::Metadata)?
            .is_none()
        {
            return Err(AssetLifecycleError::AssetNotFound);
        }
        let outcome = associations
            .unlink(&input.workspace_id, &input.asset_id, &input.document_id)
            .map_err(AssetLifecycleError::Association)?;
        let removed = outcome == AssetAssociationUnlinkOutcome::Unlinked;
        let remaining_references = associations
            .reference_count(&input.workspace_id, &input.asset_id)
            .map_err(AssetLifecycleError::Association)?;
        if removed {
            logger.write_product(AssetLifecycleProductEvent::Unlinked {
                document_id: input.document_id.as_str().to_string(),
            });
        }
        Ok(UnlinkAssetOutput {
            removed,
            remaining_references,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetLifecycleError {
    InvalidInput,
    AssetNotFound,
    DocumentNotFound,
    Document(DocumentExistenceError),
    Metadata(AssetMetadataCatalogError),
    Association(AssetAssociationCatalogError),
}
impl AssetLifecycleError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "asset_lifecycle.invalid_input",
            Self::AssetNotFound => "asset_lifecycle.asset_not_found",
            Self::DocumentNotFound => "asset_lifecycle.document_not_found",
            Self::Document(error) => error.code(),
            Self::Metadata(error) => error.code(),
            Self::Association(error) => error.code(),
        }
    }
}
