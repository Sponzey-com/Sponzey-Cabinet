use std::collections::HashMap;

use cabinet_domain::asset::{AssetId, AssetMetadata};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::CurrentDocumentSnapshot;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_store::{
    AssetObject, AssetRecord, AssetStore, AssetStoreError, AssetStorePutOutcome,
};
use cabinet_ports::document_asset_repository::{
    DocumentAssetAttachOutcome, DocumentAssetRecord, DocumentAssetRepository,
    DocumentAssetRepositoryError,
};
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_usecases::document::{
    AttachFileToDocumentError, AttachFileToDocumentInput, AttachFileToDocumentUsecase,
    CreateDocumentProductEvent, DocumentChangeEvent, DocumentChangeEventPublisher,
    DocumentProductLogger,
};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(Default)]
struct FakeDocumentRepository {
    current: HashMap<(String, String), CurrentDocumentRecord>,
    put_count: usize,
}

impl FakeDocumentRepository {
    fn insert(&mut self, workspace_id: &str, record: CurrentDocumentRecord) {
        self.current.insert(
            (
                workspace_id.to_string(),
                record.document_id().as_str().to_string(),
            ),
            record,
        );
    }

    fn current_body(&self, workspace_id: &str, document_id: &str) -> String {
        self.current
            .get(&(workspace_id.to_string(), document_id.to_string()))
            .expect("current document")
            .body()
            .as_str()
            .to_string()
    }
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        self.put_count += 1;
        self.current.insert(
            (
                workspace_id.as_str().to_string(),
                record.document_id().as_str().to_string(),
            ),
            record,
        );
        Ok(())
    }

    fn get_current_by_id(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        Ok(self
            .current
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
            ))
            .cloned())
    }

    fn get_current_by_path(
        &self,
        _workspace_id: &WorkspaceId,
        _path: &DocumentPath,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        Ok(None)
    }

    fn delete_current(
        &mut self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<(), DocumentRepositoryError> {
        Ok(())
    }
}

#[derive(Default)]
struct FakeAssetStore {
    records: Vec<AssetRecord>,
    fail_put: bool,
}

impl AssetStore for FakeAssetStore {
    fn put_asset(
        &mut self,
        _workspace_id: &WorkspaceId,
        record: AssetRecord,
    ) -> Result<AssetStorePutOutcome, AssetStoreError> {
        if self.fail_put {
            return Err(AssetStoreError::StorageUnavailable);
        }
        self.records.push(record);
        Ok(AssetStorePutOutcome::Created)
    }

    fn get_metadata(
        &self,
        _workspace_id: &WorkspaceId,
        _asset_id: &AssetId,
    ) -> Result<Option<AssetMetadata>, AssetStoreError> {
        Ok(None)
    }

    fn get_object(
        &self,
        _workspace_id: &WorkspaceId,
        _asset_id: &AssetId,
    ) -> Result<Option<AssetObject>, AssetStoreError> {
        Ok(None)
    }

    fn remove_asset(
        &mut self,
        _workspace_id: &WorkspaceId,
        _asset_id: &AssetId,
    ) -> Result<(), AssetStoreError> {
        Ok(())
    }
}

#[derive(Default)]
struct FakeDocumentAssetRepository {
    records: Vec<DocumentAssetRecord>,
}

impl DocumentAssetRepository for FakeDocumentAssetRepository {
    fn attach_asset(
        &mut self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        record: DocumentAssetRecord,
    ) -> Result<DocumentAssetAttachOutcome, DocumentAssetRepositoryError> {
        self.records.push(record);
        Ok(DocumentAssetAttachOutcome::Attached)
    }

    fn list_assets(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<Vec<DocumentAssetRecord>, DocumentAssetRepositoryError> {
        Ok(self.records.clone())
    }
}

#[derive(Default)]
struct FakeEventPublisher {
    events: Vec<DocumentChangeEvent>,
}

impl DocumentChangeEventPublisher for FakeEventPublisher {
    fn publish(&mut self, event: DocumentChangeEvent) {
        self.events.push(event);
    }
}

#[derive(Default)]
struct FakeProductLogger {
    events: Vec<CreateDocumentProductEvent>,
}

impl DocumentProductLogger for FakeProductLogger {
    fn write_product(&mut self, event: CreateDocumentProductEvent) {
        self.events.push(event);
    }
}

#[test]
fn attach_file_to_document_stores_asset_and_association_without_changing_document_body() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "body"));
    let mut assets = FakeAssetStore::default();
    let mut document_assets = FakeDocumentAssetRepository::default();
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = AttachFileToDocumentUsecase::new();

    let output = usecase
        .execute(
            valid_input(),
            &documents,
            &mut assets,
            &mut document_assets,
            &mut publisher,
            &mut logger,
        )
        .expect("attach");

    assert_eq!(output.asset_id().as_str(), HASH_A);
    assert_eq!(assets.records.len(), 1);
    assert_eq!(
        assets.records[0].metadata().file_name().as_str(),
        "diagram.png"
    );
    assert_eq!(assets.records[0].object().bytes(), &[1, 2, 3, 4]);
    assert_eq!(document_assets.records.len(), 1);
    assert_eq!(
        document_assets.records[0].reference().label(),
        "Architecture diagram"
    );
    assert_eq!(documents.current_body("workspace-1", "doc-1"), "body");
    assert_eq!(documents.put_count, 0);
    assert_eq!(
        publisher.events,
        vec![DocumentChangeEvent::DocumentAssetAttached {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            asset_id: HASH_A.to_string(),
        }]
    );
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::DocumentAssetAttached {
            document_id: "doc-1".to_string(),
            asset_id: HASH_A.to_string(),
        }]
    );
}

#[test]
fn attach_file_to_document_skips_asset_storage_when_document_is_missing() {
    let documents = FakeDocumentRepository::default();
    let mut assets = FakeAssetStore::default();
    let mut document_assets = FakeDocumentAssetRepository::default();
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = AttachFileToDocumentUsecase::new();

    let error = usecase
        .execute(
            valid_input(),
            &documents,
            &mut assets,
            &mut document_assets,
            &mut publisher,
            &mut logger,
        )
        .expect_err("missing document must fail");

    assert_eq!(error, AttachFileToDocumentError::DocumentNotFound);
    assert!(assets.records.is_empty());
    assert!(document_assets.records.is_empty());
    assert!(publisher.events.is_empty());
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::UsecaseFailed {
            error_code: "document_asset.document_not_found",
        }]
    );
}

#[test]
fn attach_file_to_document_skips_association_when_asset_storage_fails() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "body"));
    let mut assets = FakeAssetStore {
        fail_put: true,
        ..FakeAssetStore::default()
    };
    let mut document_assets = FakeDocumentAssetRepository::default();
    let mut publisher = FakeEventPublisher::default();
    let mut logger = FakeProductLogger::default();
    let usecase = AttachFileToDocumentUsecase::new();

    let error = usecase
        .execute(
            valid_input(),
            &documents,
            &mut assets,
            &mut document_assets,
            &mut publisher,
            &mut logger,
        )
        .expect_err("asset storage failure must fail");

    assert_eq!(error, AttachFileToDocumentError::StorageUnavailable);
    assert_eq!(documents.current_body("workspace-1", "doc-1"), "body");
    assert_eq!(documents.put_count, 0);
    assert!(document_assets.records.is_empty());
    assert!(publisher.events.is_empty());
    assert_eq!(
        logger.events,
        vec![CreateDocumentProductEvent::UsecaseFailed {
            error_code: "document_asset.storage_unavailable",
        }]
    );
}

fn valid_input() -> AttachFileToDocumentInput {
    AttachFileToDocumentInput::new(
        "workspace-1",
        "doc-1",
        HASH_A,
        "diagram.png",
        "image/png",
        vec![1, 2, 3, 4],
        "Architecture diagram",
    )
}

fn current_record(document_id: &str, body: &str) -> CurrentDocumentRecord {
    let metadata = DocumentMetadata::new(
        DocumentId::new(document_id).expect("document id"),
        DocumentTitle::new("Title").expect("title"),
        DocumentPath::new("docs/title.md").expect("path"),
    )
    .expect("metadata");
    let snapshot = CurrentDocumentSnapshot::new(
        DocumentId::new(document_id).expect("document id"),
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
    );
    CurrentDocumentRecord::new(metadata, snapshot).expect("record")
}
