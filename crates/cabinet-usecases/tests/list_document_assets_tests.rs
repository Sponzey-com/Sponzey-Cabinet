use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::asset::{
    AssetFileName, AssetId, AssetMediaType, AssetMetadata, AssetReference,
};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::CurrentDocumentSnapshot;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_asset_repository::{
    DocumentAssetAttachOutcome, DocumentAssetRecord, DocumentAssetRepository,
    DocumentAssetRepositoryError,
};
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_usecases::document::{
    ListDocumentAssetsError, ListDocumentAssetsInput, ListDocumentAssetsUsecase,
};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(Default)]
struct FakeDocumentRepository {
    current: HashMap<(String, String), CurrentDocumentRecord>,
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
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
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
struct FakeDocumentAssetRepository {
    records: Vec<DocumentAssetRecord>,
    fail_list: bool,
    list_count: Cell<usize>,
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
        self.list_count.set(self.list_count.get() + 1);
        if self.fail_list {
            return Err(DocumentAssetRepositoryError::StorageUnavailable);
        }
        Ok(self.records.clone())
    }
}

#[test]
fn list_document_assets_returns_metadata_and_reference_without_asset_object_store() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "body"));
    let asset_repository = FakeDocumentAssetRepository {
        records: vec![document_asset_record()],
        ..FakeDocumentAssetRepository::default()
    };
    let usecase = ListDocumentAssetsUsecase::new();

    let output = usecase
        .execute(
            ListDocumentAssetsInput::new("workspace-1", "doc-1"),
            &documents,
            &asset_repository,
        )
        .expect("list assets");

    assert_eq!(output.assets().len(), 1);
    assert_eq!(output.assets()[0].asset_id().as_str(), HASH_A);
    assert_eq!(
        output.assets()[0].metadata().file_name().as_str(),
        "diagram.png"
    );
    assert_eq!(output.assets()[0].reference().label(), "Diagram");
    assert_eq!(asset_repository.list_count.get(), 1);
}

#[test]
fn list_document_assets_reports_not_found_without_listing_associations() {
    let documents = FakeDocumentRepository::default();
    let asset_repository = FakeDocumentAssetRepository::default();
    let usecase = ListDocumentAssetsUsecase::new();

    let error = usecase
        .execute(
            ListDocumentAssetsInput::new("workspace-1", "doc-404"),
            &documents,
            &asset_repository,
        )
        .expect_err("missing document must fail");

    assert_eq!(error, ListDocumentAssetsError::NotFound);
    assert_eq!(asset_repository.list_count.get(), 0);
}

#[test]
fn list_document_assets_maps_association_repository_failure_to_storage_unavailable() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert("workspace-1", current_record("doc-1", "body"));
    let asset_repository = FakeDocumentAssetRepository {
        fail_list: true,
        ..FakeDocumentAssetRepository::default()
    };
    let usecase = ListDocumentAssetsUsecase::new();

    let error = usecase
        .execute(
            ListDocumentAssetsInput::new("workspace-1", "doc-1"),
            &documents,
            &asset_repository,
        )
        .expect_err("list failure must fail");

    assert_eq!(error, ListDocumentAssetsError::StorageUnavailable);
    assert_eq!(asset_repository.list_count.get(), 1);
}

fn document_asset_record() -> DocumentAssetRecord {
    let metadata = AssetMetadata::new(
        asset_id(),
        AssetFileName::new("diagram.png").expect("file name"),
        AssetMediaType::new("image/png").expect("media type"),
        4,
    )
    .expect("metadata");
    let reference = AssetReference::new(asset_id(), "Diagram").expect("reference");
    DocumentAssetRecord::new(reference, metadata).expect("record")
}

fn asset_id() -> AssetId {
    AssetId::from_sha256_hex(HASH_A).expect("asset id")
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
