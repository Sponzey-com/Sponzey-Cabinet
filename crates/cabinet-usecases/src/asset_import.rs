use cabinet_domain::asset::{
    AssetAssociation, AssetCatalogRecord, AssetExtractionStatus, AssetId, AssetImportHandle,
    AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::asset_import_operation::{
    AssetImportEvent, AssetImportOperation, AssetImportOperationId,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::{
    AssetAssociationCatalog, AssetAssociationCatalogError,
};
use cabinet_ports::asset_import_operation_repository::{
    AssetImportOperationRepository, AssetImportOperationRepositoryError,
};
use cabinet_ports::asset_import_source::{AssetImportSource, AssetImportSourceError};
use cabinet_ports::asset_metadata_catalog::{AssetMetadataCatalog, AssetMetadataCatalogError};
use cabinet_ports::asset_object_publisher::{AssetObjectPublishError, AssetObjectPublisher};
use cabinet_ports::asset_staging::{AssetStagingError, AssetStagingWriter, StagedAsset};
use cabinet_ports::document_existence::{DocumentExistenceError, DocumentExistenceReader};

pub struct StageAssetImportInput {
    workspace_id: WorkspaceId,
    operation_id: AssetImportOperationId,
    handle: AssetImportHandle,
    chunk_bytes: usize,
}
impl StageAssetImportInput {
    pub fn new(
        workspace: &str,
        operation: &str,
        handle: &str,
        chunk_bytes: usize,
    ) -> Result<Self, StageAssetImportError> {
        if chunk_bytes == 0 {
            return Err(StageAssetImportError::InvalidInput);
        }
        Ok(Self {
            workspace_id: WorkspaceId::new(workspace)
                .map_err(|_| StageAssetImportError::InvalidInput)?,
            operation_id: AssetImportOperationId::new(operation)
                .map_err(|_| StageAssetImportError::InvalidInput)?,
            handle: AssetImportHandle::new(handle)
                .map_err(|_| StageAssetImportError::InvalidInput)?,
            chunk_bytes,
        })
    }
}

pub struct StageAssetImportUsecase;
impl StageAssetImportUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<S: AssetImportSource, W: AssetStagingWriter>(
        &self,
        input: StageAssetImportInput,
        source: &S,
        writer: &mut W,
    ) -> Result<StagedAsset, StageAssetImportError> {
        let descriptor = source
            .describe(&input.handle)
            .map_err(StageAssetImportError::Source)?;
        writer
            .begin(&input.workspace_id, &input.operation_id)
            .map_err(StageAssetImportError::Staging)?;
        let result = stream(&input, descriptor.byte_size(), source, writer);
        if result.is_err() {
            let _ = writer.cleanup(&input.workspace_id, &input.operation_id);
        }
        result
    }
}

fn stream<S: AssetImportSource, W: AssetStagingWriter>(
    input: &StageAssetImportInput,
    expected: u64,
    source: &S,
    writer: &mut W,
) -> Result<StagedAsset, StageAssetImportError> {
    let mut offset = 0_u64;
    let mut saw_eof = false;
    while offset < expected {
        let chunk = source
            .read_chunk(&input.handle, offset, input.chunk_bytes)
            .map_err(StageAssetImportError::Source)?;
        if chunk.offset() != offset || chunk.bytes().is_empty() {
            return Err(StageAssetImportError::SizeMismatch);
        }
        writer
            .append(
                &input.workspace_id,
                &input.operation_id,
                offset,
                chunk.bytes(),
            )
            .map_err(StageAssetImportError::Staging)?;
        offset = offset
            .checked_add(chunk.bytes().len() as u64)
            .ok_or(StageAssetImportError::SizeMismatch)?;
        saw_eof = chunk.is_eof();
        if saw_eof && offset != expected {
            return Err(StageAssetImportError::SizeMismatch);
        }
    }
    if !saw_eof || offset != expected {
        return Err(StageAssetImportError::SizeMismatch);
    }
    writer
        .finalize(&input.workspace_id, &input.operation_id, expected)
        .map_err(StageAssetImportError::Staging)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageAssetImportError {
    InvalidInput,
    SizeMismatch,
    Source(AssetImportSourceError),
    Staging(AssetStagingError),
}
impl StageAssetImportError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "asset_import.invalid_input",
            Self::SizeMismatch => "asset_import.size_mismatch",
            Self::Source(error) => error.code(),
            Self::Staging(error) => error.code(),
        }
    }
}

pub struct ImportAssetInput {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    operation_id: AssetImportOperationId,
    handle: AssetImportHandle,
    label: String,
    chunk_bytes: usize,
}
impl ImportAssetInput {
    pub fn new(
        workspace: &str,
        document: &str,
        operation: &str,
        handle: &str,
        label: &str,
        chunk_bytes: usize,
    ) -> Result<Self, ImportAssetError> {
        let label = label.trim();
        if label.is_empty() || chunk_bytes == 0 {
            return Err(ImportAssetError::InvalidInput);
        }
        Ok(Self {
            workspace_id: WorkspaceId::new(workspace)
                .map_err(|_| ImportAssetError::InvalidInput)?,
            document_id: DocumentId::new(document).map_err(|_| ImportAssetError::InvalidInput)?,
            operation_id: AssetImportOperationId::new(operation)
                .map_err(|_| ImportAssetError::InvalidInput)?,
            handle: AssetImportHandle::new(handle).map_err(|_| ImportAssetError::InvalidInput)?,
            label: label.to_string(),
            chunk_bytes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportAssetOutput {
    asset_id: AssetId,
    operation_id: AssetImportOperationId,
}
impl ImportAssetOutput {
    pub fn asset_id(&self) -> &AssetId {
        &self.asset_id
    }
    pub fn operation_id(&self) -> &AssetImportOperationId {
        &self.operation_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportAssetProductEvent {
    Completed {
        operation_id: String,
        document_id: String,
    },
    Failed {
        operation_id: String,
        error_code: &'static str,
    },
}
pub trait ImportAssetProductLogger {
    fn write_product(&mut self, event: ImportAssetProductEvent);
}

pub struct ImportAssetUsecase;
impl ImportAssetUsecase {
    pub const fn new() -> Self {
        Self
    }
    #[allow(clippy::too_many_arguments)]
    pub fn execute<D, S, W, P, M, A, R, L>(
        &self,
        input: ImportAssetInput,
        documents: &D,
        source: &S,
        writer: &mut W,
        publisher: &mut P,
        metadata: &mut M,
        associations: &mut A,
        operations: &mut R,
        logger: &mut L,
    ) -> Result<ImportAssetOutput, ImportAssetError>
    where
        D: DocumentExistenceReader,
        S: AssetImportSource,
        W: AssetStagingWriter,
        P: AssetObjectPublisher,
        M: AssetMetadataCatalog,
        A: AssetAssociationCatalog,
        R: AssetImportOperationRepository,
        L: ImportAssetProductLogger,
    {
        match documents.exists(&input.workspace_id, &input.document_id) {
            Ok(true) => {}
            Ok(false) => {
                log_initial_failure(
                    logger,
                    &input.operation_id,
                    "asset_import.document_not_found",
                );
                return Err(ImportAssetError::DocumentNotFound);
            }
            Err(error) => {
                log_initial_failure(logger, &input.operation_id, error.code());
                return Err(ImportAssetError::Document(error));
            }
        }
        let descriptor = match source.describe(&input.handle) {
            Ok(descriptor) => descriptor,
            Err(error) => {
                log_initial_failure(logger, &input.operation_id, error.code());
                return Err(ImportAssetError::Source(error));
            }
        };
        let mut operation = AssetImportOperation::new(
            input.operation_id.clone(),
            input.workspace_id.clone(),
            input.document_id.clone(),
            descriptor.byte_size(),
        )
        .map_err(|_| ImportAssetError::InvalidInput)?;
        operations
            .create(operation.clone())
            .map_err(ImportAssetError::Repository)?;
        advance(&mut operation, AssetImportEvent::Begin, 0, operations)?;
        advance(
            &mut operation,
            AssetImportEvent::ValidationSucceeded,
            0,
            operations,
        )?;
        let stage_input = StageAssetImportInput::new(
            input.workspace_id.as_str(),
            input.operation_id.as_str(),
            input.handle.as_str(),
            input.chunk_bytes,
        )
        .map_err(|_| ImportAssetError::InvalidInput)?;
        if let Err(error) = StageAssetImportUsecase::new().execute(stage_input, source, writer) {
            return fail(
                &mut operation,
                AssetImportEvent::StagingFailed,
                error.code(),
                operations,
                logger,
                ImportAssetError::Staging(error),
            );
        }
        advance(
            &mut operation,
            AssetImportEvent::StagingSucceeded,
            descriptor.byte_size(),
            operations,
        )?;
        advance(
            &mut operation,
            AssetImportEvent::HashingSucceeded,
            descriptor.byte_size(),
            operations,
        )?;
        let published = match publisher.publish(
            &input.workspace_id,
            &input.operation_id,
            descriptor.byte_size(),
        ) {
            Ok(value) => value,
            Err(error) => {
                return fail(
                    &mut operation,
                    AssetImportEvent::ObjectPublishFailed,
                    error.code(),
                    operations,
                    logger,
                    ImportAssetError::Publish(error),
                );
            }
        };
        advance(
            &mut operation,
            AssetImportEvent::ObjectPublished,
            descriptor.byte_size(),
            operations,
        )?;
        let asset_metadata = AssetMetadata::new(
            published.asset_id().clone(),
            descriptor.file_name().clone(),
            descriptor.media_type().clone(),
            descriptor.byte_size(),
        )
        .map_err(|_| ImportAssetError::InvalidInput)?;
        let preview = AssetPreviewCapability::for_media_type(asset_metadata.media_type());
        let extraction = if preview == AssetPreviewCapability::Unsupported {
            AssetExtractionStatus::Unsupported
        } else {
            AssetExtractionStatus::NotRequested
        };
        let record = AssetCatalogRecord::new(asset_metadata, 1, preview, extraction)
            .map_err(|_| ImportAssetError::InvalidInput)?;
        if let Err(error) = metadata.put(&input.workspace_id, record) {
            return fail(
                &mut operation,
                AssetImportEvent::MetadataPersistFailed,
                error.code(),
                operations,
                logger,
                ImportAssetError::Metadata(error),
            );
        }
        advance(
            &mut operation,
            AssetImportEvent::MetadataPersisted,
            descriptor.byte_size(),
            operations,
        )?;
        let association = AssetAssociation::new(
            published.asset_id().clone(),
            input.document_id.clone(),
            &input.label,
        )
        .map_err(|_| ImportAssetError::InvalidInput)?;
        if let Err(error) = associations.link(&input.workspace_id, association) {
            return fail(
                &mut operation,
                AssetImportEvent::LinkFailed,
                error.code(),
                operations,
                logger,
                ImportAssetError::Association(error),
            );
        }
        advance(
            &mut operation,
            AssetImportEvent::LinkSucceeded,
            descriptor.byte_size(),
            operations,
        )?;
        logger.write_product(ImportAssetProductEvent::Completed {
            operation_id: input.operation_id.as_str().to_string(),
            document_id: input.document_id.as_str().to_string(),
        });
        Ok(ImportAssetOutput {
            asset_id: published.asset_id().clone(),
            operation_id: input.operation_id,
        })
    }
}

fn advance<R: AssetImportOperationRepository>(
    operation: &mut AssetImportOperation,
    event: AssetImportEvent,
    progress: u64,
    repository: &mut R,
) -> Result<(), ImportAssetError> {
    let expected = operation.state();
    operation
        .apply(event, progress)
        .map_err(|_| ImportAssetError::InvalidState)?;
    repository
        .replace(operation.clone(), expected)
        .map_err(ImportAssetError::Repository)
}
fn fail<R: AssetImportOperationRepository, L: ImportAssetProductLogger>(
    operation: &mut AssetImportOperation,
    event: AssetImportEvent,
    error_code: &'static str,
    repository: &mut R,
    logger: &mut L,
    result: ImportAssetError,
) -> Result<ImportAssetOutput, ImportAssetError> {
    advance(operation, event, operation.completed_bytes(), repository)?;
    logger.write_product(ImportAssetProductEvent::Failed {
        operation_id: operation.operation_id().as_str().to_string(),
        error_code,
    });
    Err(result)
}

fn log_initial_failure<L: ImportAssetProductLogger>(
    logger: &mut L,
    operation_id: &AssetImportOperationId,
    error_code: &'static str,
) {
    logger.write_product(ImportAssetProductEvent::Failed {
        operation_id: operation_id.as_str().to_string(),
        error_code,
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportAssetError {
    InvalidInput,
    InvalidState,
    DocumentNotFound,
    Document(DocumentExistenceError),
    Source(AssetImportSourceError),
    Staging(StageAssetImportError),
    Publish(AssetObjectPublishError),
    Metadata(AssetMetadataCatalogError),
    Association(AssetAssociationCatalogError),
    Repository(AssetImportOperationRepositoryError),
}
impl ImportAssetError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "asset_import.invalid_input",
            Self::InvalidState => "asset_import.invalid_state",
            Self::DocumentNotFound => "asset_import.document_not_found",
            Self::Document(error) => error.code(),
            Self::Source(error) => error.code(),
            Self::Staging(error) => error.code(),
            Self::Publish(error) => error.code(),
            Self::Metadata(error) => error.code(),
            Self::Association(error) => error.code(),
            Self::Repository(error) => error.code(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListCatalogDocumentAssetsInput {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    limit: usize,
}

impl ListCatalogDocumentAssetsInput {
    pub fn new(
        workspace_id: &str,
        document_id: &str,
        limit: usize,
    ) -> Result<Self, ListCatalogDocumentAssetsError> {
        if limit == 0 || limit > 500 {
            return Err(ListCatalogDocumentAssetsError::InvalidInput);
        }
        Ok(Self {
            workspace_id: WorkspaceId::new(workspace_id)
                .map_err(|_| ListCatalogDocumentAssetsError::InvalidInput)?,
            document_id: DocumentId::new(document_id)
                .map_err(|_| ListCatalogDocumentAssetsError::InvalidInput)?,
            limit,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogDocumentAsset {
    association: AssetAssociation,
    record: AssetCatalogRecord,
}
impl CatalogDocumentAsset {
    pub fn asset_id(&self) -> &AssetId {
        self.association.asset_id()
    }
    pub fn label(&self) -> &str {
        self.association.label()
    }
    pub fn record(&self) -> &AssetCatalogRecord {
        &self.record
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListCatalogDocumentAssetsOutput {
    assets: Vec<CatalogDocumentAsset>,
}
impl ListCatalogDocumentAssetsOutput {
    pub fn assets(&self) -> &[CatalogDocumentAsset] {
        &self.assets
    }
}

pub struct ListCatalogDocumentAssetsUsecase;
impl ListCatalogDocumentAssetsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute<D, A, M>(
        &self,
        input: ListCatalogDocumentAssetsInput,
        documents: &D,
        associations: &A,
        metadata: &M,
    ) -> Result<ListCatalogDocumentAssetsOutput, ListCatalogDocumentAssetsError>
    where
        D: DocumentExistenceReader,
        A: AssetAssociationCatalog,
        M: AssetMetadataCatalog,
    {
        match documents.exists(&input.workspace_id, &input.document_id) {
            Ok(true) => {}
            Ok(false) => return Err(ListCatalogDocumentAssetsError::DocumentNotFound),
            Err(error) => return Err(ListCatalogDocumentAssetsError::Document(error)),
        }
        let mut links = associations
            .list_assets(&input.workspace_id, &input.document_id, input.limit)
            .map_err(ListCatalogDocumentAssetsError::Association)?;
        links.sort_by(|left, right| left.asset_id().as_str().cmp(right.asset_id().as_str()));
        let assets = links
            .into_iter()
            .map(|association| {
                let record = metadata
                    .get(&input.workspace_id, association.asset_id())
                    .map_err(ListCatalogDocumentAssetsError::Metadata)?
                    .ok_or(ListCatalogDocumentAssetsError::DanglingMetadata)?;
                Ok(CatalogDocumentAsset {
                    association,
                    record,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ListCatalogDocumentAssetsOutput { assets })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListCatalogDocumentAssetsError {
    InvalidInput,
    DocumentNotFound,
    DanglingMetadata,
    Document(DocumentExistenceError),
    Association(AssetAssociationCatalogError),
    Metadata(AssetMetadataCatalogError),
}
impl ListCatalogDocumentAssetsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "asset_query.invalid_input",
            Self::DocumentNotFound => "asset_query.document_not_found",
            Self::DanglingMetadata => "asset_query.dangling_metadata",
            Self::Document(error) => error.code(),
            Self::Association(error) => error.code(),
            Self::Metadata(error) => error.code(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoverAssetImportsInput {
    workspace_id: WorkspaceId,
    limit: usize,
}
impl RecoverAssetImportsInput {
    pub fn new(workspace: &str, limit: usize) -> Result<Self, RecoverAssetImportsError> {
        if limit == 0 || limit > 500 {
            return Err(RecoverAssetImportsError::InvalidInput);
        }
        Ok(Self {
            workspace_id: WorkspaceId::new(workspace)
                .map_err(|_| RecoverAssetImportsError::InvalidInput)?,
            limit,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoverAssetImportsOutput {
    cancelled: usize,
    cleanup_required: usize,
}
impl RecoverAssetImportsOutput {
    pub fn cancelled(self) -> usize {
        self.cancelled
    }
    pub fn cleanup_required(self) -> usize {
        self.cleanup_required
    }
}

pub struct RecoverAssetImportsUsecase;
impl RecoverAssetImportsUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R, W, L>(
        &self,
        input: RecoverAssetImportsInput,
        operations: &mut R,
        staging: &mut W,
        logger: &mut L,
    ) -> Result<RecoverAssetImportsOutput, RecoverAssetImportsError>
    where
        R: AssetImportOperationRepository,
        W: AssetStagingWriter,
        L: ImportAssetProductLogger,
    {
        let active = operations
            .list_active(&input.workspace_id, input.limit)
            .map_err(RecoverAssetImportsError::Repository)?;
        let mut output = RecoverAssetImportsOutput {
            cancelled: 0,
            cleanup_required: 0,
        };
        for mut operation in active {
            let expected = operation.state();
            operation
                .apply(
                    AssetImportEvent::CancelRequested,
                    operation.completed_bytes(),
                )
                .map_err(|_| RecoverAssetImportsError::InvalidState)?;
            operations
                .replace(operation.clone(), expected)
                .map_err(RecoverAssetImportsError::Repository)?;
            let expected = operation.state();
            let cleanup = staging.cleanup(operation.workspace_id(), operation.operation_id());
            let (event, error_code) = if cleanup.is_ok() {
                output.cancelled = output.cancelled.saturating_add(1);
                (AssetImportEvent::CleanupSucceeded, "asset.import.cancelled")
            } else {
                output.cleanup_required = output.cleanup_required.saturating_add(1);
                (
                    AssetImportEvent::CleanupFailed,
                    "asset.import.cleanup_required",
                )
            };
            operation
                .apply(event, operation.completed_bytes())
                .map_err(|_| RecoverAssetImportsError::InvalidState)?;
            operations
                .replace(operation.clone(), expected)
                .map_err(RecoverAssetImportsError::Repository)?;
            logger.write_product(ImportAssetProductEvent::Failed {
                operation_id: operation.operation_id().as_str().to_string(),
                error_code,
            });
        }
        Ok(output)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoverAssetImportsError {
    InvalidInput,
    InvalidState,
    Repository(AssetImportOperationRepositoryError),
}
impl RecoverAssetImportsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "asset_recovery.invalid_input",
            Self::InvalidState => "asset_recovery.invalid_state",
            Self::Repository(error) => error.code(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelAssetImportInput {
    workspace_id: WorkspaceId,
    operation_id: AssetImportOperationId,
}
impl CancelAssetImportInput {
    pub fn new(workspace: &str, operation: &str) -> Result<Self, CancelAssetImportError> {
        Ok(Self {
            workspace_id: WorkspaceId::new(workspace)
                .map_err(|_| CancelAssetImportError::InvalidInput)?,
            operation_id: AssetImportOperationId::new(operation)
                .map_err(|_| CancelAssetImportError::InvalidInput)?,
        })
    }
}

pub struct CancelAssetImportUsecase;
impl CancelAssetImportUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R, W, L>(
        &self,
        input: CancelAssetImportInput,
        operations: &mut R,
        staging: &mut W,
        logger: &mut L,
    ) -> Result<AssetImportOperation, CancelAssetImportError>
    where
        R: AssetImportOperationRepository,
        W: AssetStagingWriter,
        L: ImportAssetProductLogger,
    {
        let mut operation = operations
            .get(&input.operation_id)
            .map_err(CancelAssetImportError::Repository)?
            .ok_or(CancelAssetImportError::NotFound)?;
        if operation.workspace_id() != &input.workspace_id {
            return Err(CancelAssetImportError::NotFound);
        }
        if operation.state().is_terminal() {
            return Ok(operation);
        }
        let expected = operation.state();
        operation
            .apply(
                AssetImportEvent::CancelRequested,
                operation.completed_bytes(),
            )
            .map_err(|_| CancelAssetImportError::InvalidState)?;
        operations
            .replace(operation.clone(), expected)
            .map_err(CancelAssetImportError::Repository)?;
        let expected = operation.state();
        let (event, error_code) =
            match staging.cleanup(operation.workspace_id(), operation.operation_id()) {
                Ok(()) => (AssetImportEvent::CleanupSucceeded, "asset.import.cancelled"),
                Err(_) => (
                    AssetImportEvent::CleanupFailed,
                    "asset.import.cleanup_required",
                ),
            };
        operation
            .apply(event, operation.completed_bytes())
            .map_err(|_| CancelAssetImportError::InvalidState)?;
        operations
            .replace(operation.clone(), expected)
            .map_err(CancelAssetImportError::Repository)?;
        logger.write_product(ImportAssetProductEvent::Failed {
            operation_id: operation.operation_id().as_str().to_string(),
            error_code,
        });
        Ok(operation)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelAssetImportError {
    InvalidInput,
    InvalidState,
    NotFound,
    Repository(AssetImportOperationRepositoryError),
}
impl CancelAssetImportError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "asset_cancel.invalid_input",
            Self::InvalidState => "asset_cancel.invalid_state",
            Self::NotFound => "asset_cancel.not_found",
            Self::Repository(error) => error.code(),
        }
    }
}
