use cabinet_domain::asset::{
    AssetAssociation, AssetCatalogRecord, AssetId, AssetImportDescriptor, AssetImportHandle,
};
use cabinet_domain::asset_import_operation::{
    AssetImportOperation, AssetImportOperationId, AssetImportState,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::{
    AssetAssociationCatalog, AssetAssociationCatalogError, AssetAssociationLinkOutcome,
    AssetAssociationUnlinkOutcome,
};
use cabinet_ports::asset_import_operation_repository::{
    AssetImportOperationCreateOutcome, AssetImportOperationRepository,
    AssetImportOperationRepositoryError,
};
use cabinet_ports::asset_import_source::{
    AssetImportChunk, AssetImportSource, AssetImportSourceError,
};
use cabinet_ports::asset_metadata_catalog::{
    AssetMetadataCatalog, AssetMetadataCatalogError, AssetMetadataPage, AssetMetadataPutOutcome,
};
use cabinet_ports::asset_object_publisher::{
    AssetObjectPublishError, AssetObjectPublishOutcome, AssetObjectPublisher, PublishedAssetObject,
};
use cabinet_ports::asset_staging::{AssetStagingError, AssetStagingWriter, StagedAsset};
use cabinet_ports::document_existence::{DocumentExistenceError, DocumentExistenceReader};
use cabinet_usecases::asset_import::{
    ImportAssetInput, ImportAssetProductEvent, ImportAssetProductLogger, ImportAssetUsecase,
};

const HASH: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

#[test]
fn import_usecase_completes_object_metadata_association_operation_and_log() {
    let source = Source;
    let mut writer = Writer::default();
    let mut publisher = Publisher { fail: false };
    let mut metadata = Metadata::default();
    let mut associations = Associations::default();
    let mut operations = Operations::default();
    let mut logger = Logger::default();
    let output = ImportAssetUsecase::new()
        .execute(
            input(),
            &Documents,
            &source,
            &mut writer,
            &mut publisher,
            &mut metadata,
            &mut associations,
            &mut operations,
            &mut logger,
        )
        .expect("import");
    assert_eq!(output.asset_id().as_str(), HASH);
    assert_eq!(
        operations.current.as_ref().expect("operation").state(),
        AssetImportState::Completed
    );
    assert_eq!(metadata.records.len(), 1);
    assert_eq!(associations.records.len(), 1);
    assert_eq!(
        logger.events,
        vec![ImportAssetProductEvent::Completed {
            operation_id: "import-1".into(),
            document_id: "doc-1".into()
        }]
    );
}

#[test]
fn import_usecase_persists_publish_failure_and_safe_product_log() {
    let mut operations = Operations::default();
    let mut logger = Logger::default();
    let error = ImportAssetUsecase::new()
        .execute(
            input(),
            &Documents,
            &Source,
            &mut Writer::default(),
            &mut Publisher { fail: true },
            &mut Metadata::default(),
            &mut Associations::default(),
            &mut operations,
            &mut logger,
        )
        .expect_err("publish failure");
    assert_eq!(error.code(), "asset_publish.storage_unavailable");
    assert_eq!(
        operations.current.expect("operation").state(),
        AssetImportState::ObjectPublishFailed
    );
    assert_eq!(
        logger.events,
        vec![ImportAssetProductEvent::Failed {
            operation_id: "import-1".into(),
            error_code: "asset_publish.storage_unavailable"
        }]
    );
}

#[test]
fn import_usecase_maps_staging_metadata_and_link_failures_to_exact_states() {
    let mut staging_operations = Operations::default();
    let mut staging_logs = Logger::default();
    ImportAssetUsecase::new()
        .execute(
            input(),
            &Documents,
            &Source,
            &mut Writer {
                fail: true,
                ..Writer::default()
            },
            &mut Publisher { fail: false },
            &mut Metadata::default(),
            &mut Associations::default(),
            &mut staging_operations,
            &mut staging_logs,
        )
        .expect_err("staging");
    assert_eq!(
        staging_operations
            .current
            .expect("staging operation")
            .state(),
        AssetImportState::StagingFailed
    );

    let mut metadata_operations = Operations::default();
    let mut metadata_logs = Logger::default();
    ImportAssetUsecase::new()
        .execute(
            input(),
            &Documents,
            &Source,
            &mut Writer::default(),
            &mut Publisher { fail: false },
            &mut Metadata {
                fail: true,
                ..Metadata::default()
            },
            &mut Associations::default(),
            &mut metadata_operations,
            &mut metadata_logs,
        )
        .expect_err("metadata");
    assert_eq!(
        metadata_operations
            .current
            .expect("metadata operation")
            .state(),
        AssetImportState::MetadataPersistFailed
    );

    let mut link_operations = Operations::default();
    let mut link_logs = Logger::default();
    ImportAssetUsecase::new()
        .execute(
            input(),
            &Documents,
            &Source,
            &mut Writer::default(),
            &mut Publisher { fail: false },
            &mut Metadata::default(),
            &mut Associations {
                fail: true,
                ..Associations::default()
            },
            &mut link_operations,
            &mut link_logs,
        )
        .expect_err("link");
    assert_eq!(
        link_operations.current.expect("link operation").state(),
        AssetImportState::LinkFailed
    );
    assert_eq!(staging_logs.events.len(), 1);
    assert_eq!(metadata_logs.events.len(), 1);
    assert_eq!(link_logs.events.len(), 1);
}

#[test]
fn import_usecase_rejects_missing_document_before_storage_mutation() {
    let mut operations = Operations::default();
    let mut logger = Logger::default();
    let error = ImportAssetUsecase::new()
        .execute(
            input(),
            &MissingDocuments,
            &Source,
            &mut Writer::default(),
            &mut Publisher { fail: false },
            &mut Metadata::default(),
            &mut Associations::default(),
            &mut operations,
            &mut logger,
        )
        .expect_err("missing document");
    assert_eq!(error.code(), "asset_import.document_not_found");
    assert!(operations.current.is_none());
    assert_eq!(logger.events.len(), 1);
}

struct Source;
struct Documents;
struct MissingDocuments;
impl DocumentExistenceReader for Documents {
    fn exists(&self, _: &WorkspaceId, _: &DocumentId) -> Result<bool, DocumentExistenceError> {
        Ok(true)
    }
}
impl DocumentExistenceReader for MissingDocuments {
    fn exists(&self, _: &WorkspaceId, _: &DocumentId) -> Result<bool, DocumentExistenceError> {
        Ok(false)
    }
}
impl AssetImportSource for Source {
    fn describe(
        &self,
        handle: &AssetImportHandle,
    ) -> Result<AssetImportDescriptor, AssetImportSourceError> {
        AssetImportDescriptor::new(handle.clone(), "notes.txt", "text/plain", 3)
            .map_err(|_| AssetImportSourceError::UnsafeSource)
    }
    fn read_chunk(
        &self,
        _handle: &AssetImportHandle,
        offset: u64,
        max: usize,
    ) -> Result<AssetImportChunk, AssetImportSourceError> {
        let bytes: Vec<u8> = b"abc"[offset as usize..]
            .iter()
            .copied()
            .take(max)
            .collect();
        let eof = offset + bytes.len() as u64 == 3;
        AssetImportChunk::new(offset, bytes, eof, max)
    }
}
#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
    fail: bool,
}
impl AssetStagingWriter for Writer {
    fn begin(
        &mut self,
        _: &WorkspaceId,
        _: &AssetImportOperationId,
    ) -> Result<(), AssetStagingError> {
        if self.fail {
            return Err(AssetStagingError::StorageUnavailable);
        }
        self.bytes.clear();
        Ok(())
    }
    fn append(
        &mut self,
        _: &WorkspaceId,
        _: &AssetImportOperationId,
        _: u64,
        bytes: &[u8],
    ) -> Result<(), AssetStagingError> {
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn finalize(
        &mut self,
        _: &WorkspaceId,
        op: &AssetImportOperationId,
        expected: u64,
    ) -> Result<StagedAsset, AssetStagingError> {
        StagedAsset::new(op.clone(), self.bytes.len() as u64, expected)
    }
    fn cleanup(
        &mut self,
        _: &WorkspaceId,
        _: &AssetImportOperationId,
    ) -> Result<(), AssetStagingError> {
        Ok(())
    }
}
struct Publisher {
    fail: bool,
}
impl AssetObjectPublisher for Publisher {
    fn publish(
        &mut self,
        _: &WorkspaceId,
        _: &AssetImportOperationId,
        size: u64,
    ) -> Result<PublishedAssetObject, AssetObjectPublishError> {
        if self.fail {
            Err(AssetObjectPublishError::StorageUnavailable)
        } else {
            PublishedAssetObject::new(
                AssetId::from_sha256_hex(HASH).unwrap(),
                size,
                AssetObjectPublishOutcome::Created,
            )
        }
    }
}
#[derive(Default)]
struct Metadata {
    records: Vec<AssetCatalogRecord>,
    fail: bool,
}
impl AssetMetadataCatalog for Metadata {
    fn put(
        &mut self,
        _: &WorkspaceId,
        record: AssetCatalogRecord,
    ) -> Result<AssetMetadataPutOutcome, AssetMetadataCatalogError> {
        if self.fail {
            return Err(AssetMetadataCatalogError::StorageUnavailable);
        }
        self.records.push(record);
        Ok(AssetMetadataPutOutcome::Created)
    }
    fn get(
        &self,
        _: &WorkspaceId,
        _: &AssetId,
    ) -> Result<Option<AssetCatalogRecord>, AssetMetadataCatalogError> {
        Ok(None)
    }
    fn list(
        &self,
        _: &WorkspaceId,
        _: Option<&str>,
        _: usize,
    ) -> Result<AssetMetadataPage, AssetMetadataCatalogError> {
        Ok(AssetMetadataPage::new(vec![], None))
    }
}
#[derive(Default)]
struct Associations {
    records: Vec<AssetAssociation>,
    fail: bool,
}
impl AssetAssociationCatalog for Associations {
    fn link(
        &mut self,
        _: &WorkspaceId,
        record: AssetAssociation,
    ) -> Result<AssetAssociationLinkOutcome, AssetAssociationCatalogError> {
        if self.fail {
            return Err(AssetAssociationCatalogError::StorageUnavailable);
        }
        self.records.push(record);
        Ok(AssetAssociationLinkOutcome::Linked)
    }
    fn unlink(
        &mut self,
        _: &WorkspaceId,
        _: &AssetId,
        _: &DocumentId,
    ) -> Result<AssetAssociationUnlinkOutcome, AssetAssociationCatalogError> {
        Ok(AssetAssociationUnlinkOutcome::NotLinked)
    }
    fn list_documents(
        &self,
        _: &WorkspaceId,
        _: &AssetId,
        _: usize,
    ) -> Result<Vec<AssetAssociation>, AssetAssociationCatalogError> {
        Ok(vec![])
    }
    fn list_assets(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
        _: usize,
    ) -> Result<Vec<AssetAssociation>, AssetAssociationCatalogError> {
        Ok(vec![])
    }
    fn reference_count(
        &self,
        _: &WorkspaceId,
        _: &AssetId,
    ) -> Result<u64, AssetAssociationCatalogError> {
        Ok(0)
    }
}
#[derive(Default)]
struct Operations {
    current: Option<AssetImportOperation>,
    history: Vec<AssetImportState>,
}
impl AssetImportOperationRepository for Operations {
    fn create(
        &mut self,
        op: AssetImportOperation,
    ) -> Result<AssetImportOperationCreateOutcome, AssetImportOperationRepositoryError> {
        self.history.push(op.state());
        self.current = Some(op);
        Ok(AssetImportOperationCreateOutcome::Created)
    }
    fn get(
        &self,
        _: &AssetImportOperationId,
    ) -> Result<Option<AssetImportOperation>, AssetImportOperationRepositoryError> {
        Ok(self.current.clone())
    }
    fn replace(
        &mut self,
        op: AssetImportOperation,
        expected: AssetImportState,
    ) -> Result<(), AssetImportOperationRepositoryError> {
        if self.current.as_ref().map(|v| v.state()) != Some(expected) {
            return Err(AssetImportOperationRepositoryError::Conflict);
        }
        self.history.push(op.state());
        self.current = Some(op);
        Ok(())
    }
    fn list_active(
        &self,
        _: &WorkspaceId,
        _: usize,
    ) -> Result<Vec<AssetImportOperation>, AssetImportOperationRepositoryError> {
        Ok(vec![])
    }
}
#[derive(Default)]
struct Logger {
    events: Vec<ImportAssetProductEvent>,
}
impl ImportAssetProductLogger for Logger {
    fn write_product(&mut self, event: ImportAssetProductEvent) {
        self.events.push(event);
    }
}
fn input() -> ImportAssetInput {
    ImportAssetInput::new("workspace-1", "doc-1", "import-1", "picker:1", "Notes", 2)
        .expect("input")
}
