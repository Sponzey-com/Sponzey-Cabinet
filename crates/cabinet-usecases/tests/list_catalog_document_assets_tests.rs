use cabinet_domain::asset::{
    AssetAssociation, AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId,
    AssetMediaType, AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::{
    AssetAssociationCatalog, AssetAssociationCatalogError, AssetAssociationLinkOutcome,
    AssetAssociationUnlinkOutcome,
};
use cabinet_ports::asset_metadata_catalog::{
    AssetMetadataCatalog, AssetMetadataCatalogError, AssetMetadataPage, AssetMetadataPutOutcome,
};
use cabinet_ports::document_existence::{DocumentExistenceError, DocumentExistenceReader};
use cabinet_usecases::asset_import::{
    ListCatalogDocumentAssetsError, ListCatalogDocumentAssetsInput,
    ListCatalogDocumentAssetsUsecase,
};

#[test]
fn catalog_query_joins_document_associations_with_metadata_in_stable_order() {
    let asset_a = asset_id('a');
    let asset_b = asset_id('b');
    let associations = FakeAssociations::new(vec![
        association(asset_b.clone(), "Beta"),
        association(asset_a.clone(), "Alpha"),
    ]);
    let metadata = FakeMetadata::new(vec![
        record(asset_a.clone(), "alpha.pdf"),
        record(asset_b.clone(), "beta.png"),
    ]);

    let output = ListCatalogDocumentAssetsUsecase::new()
        .execute(
            ListCatalogDocumentAssetsInput::new("workspace-1", "doc-1", 10).expect("input"),
            &ExistingDocument,
            &associations,
            &metadata,
        )
        .expect("query");

    assert_eq!(output.assets().len(), 2);
    assert_eq!(output.assets()[0].asset_id(), &asset_a);
    assert_eq!(output.assets()[0].label(), "Alpha");
    assert_eq!(output.assets()[1].asset_id(), &asset_b);
}

#[test]
fn catalog_query_rejects_dangling_association_and_invalid_limit() {
    let asset = asset_id('a');
    let associations = FakeAssociations::new(vec![association(asset, "Missing")]);
    let metadata = FakeMetadata::new(Vec::new());

    let error = ListCatalogDocumentAssetsUsecase::new()
        .execute(
            ListCatalogDocumentAssetsInput::new("workspace-1", "doc-1", 10).expect("input"),
            &ExistingDocument,
            &associations,
            &metadata,
        )
        .expect_err("dangling association");

    assert_eq!(error, ListCatalogDocumentAssetsError::DanglingMetadata);
    assert_eq!(
        ListCatalogDocumentAssetsInput::new("workspace-1", "doc-1", 0).expect_err("limit"),
        ListCatalogDocumentAssetsError::InvalidInput
    );
}

struct ExistingDocument;
impl DocumentExistenceReader for ExistingDocument {
    fn exists(&self, _: &WorkspaceId, _: &DocumentId) -> Result<bool, DocumentExistenceError> {
        Ok(true)
    }
}

struct FakeAssociations(Vec<AssetAssociation>);
impl FakeAssociations {
    fn new(values: Vec<AssetAssociation>) -> Self {
        Self(values)
    }
}
impl AssetAssociationCatalog for FakeAssociations {
    fn link(
        &mut self,
        _: &WorkspaceId,
        _: AssetAssociation,
    ) -> Result<AssetAssociationLinkOutcome, AssetAssociationCatalogError> {
        unreachable!()
    }
    fn unlink(
        &mut self,
        _: &WorkspaceId,
        _: &AssetId,
        _: &DocumentId,
    ) -> Result<AssetAssociationUnlinkOutcome, AssetAssociationCatalogError> {
        unreachable!()
    }
    fn list_documents(
        &self,
        _: &WorkspaceId,
        _: &AssetId,
        _: usize,
    ) -> Result<Vec<AssetAssociation>, AssetAssociationCatalogError> {
        unreachable!()
    }
    fn list_assets(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
        limit: usize,
    ) -> Result<Vec<AssetAssociation>, AssetAssociationCatalogError> {
        Ok(self.0.iter().take(limit).cloned().collect())
    }
    fn reference_count(
        &self,
        _: &WorkspaceId,
        _: &AssetId,
    ) -> Result<u64, AssetAssociationCatalogError> {
        unreachable!()
    }
}

struct FakeMetadata(Vec<AssetCatalogRecord>);
impl FakeMetadata {
    fn new(values: Vec<AssetCatalogRecord>) -> Self {
        Self(values)
    }
}
impl AssetMetadataCatalog for FakeMetadata {
    fn put(
        &mut self,
        _: &WorkspaceId,
        _: AssetCatalogRecord,
    ) -> Result<AssetMetadataPutOutcome, AssetMetadataCatalogError> {
        unreachable!()
    }
    fn get(
        &self,
        _: &WorkspaceId,
        asset: &AssetId,
    ) -> Result<Option<AssetCatalogRecord>, AssetMetadataCatalogError> {
        Ok(self
            .0
            .iter()
            .find(|record| record.metadata().id() == asset)
            .cloned())
    }
    fn list(
        &self,
        _: &WorkspaceId,
        _: Option<&str>,
        _: usize,
    ) -> Result<AssetMetadataPage, AssetMetadataCatalogError> {
        unreachable!()
    }
}

fn asset_id(value: char) -> AssetId {
    AssetId::from_sha256_hex(&value.to_string().repeat(64)).expect("asset id")
}
fn association(asset: AssetId, label: &str) -> AssetAssociation {
    AssetAssociation::new(asset, DocumentId::new("doc-1").expect("document"), label)
        .expect("association")
}
fn record(asset: AssetId, name: &str) -> AssetCatalogRecord {
    AssetCatalogRecord::new(
        AssetMetadata::new(
            asset,
            AssetFileName::new(name).expect("file name"),
            AssetMediaType::new("application/pdf").expect("media type"),
            42,
        )
        .expect("metadata"),
        1,
        AssetPreviewCapability::Pdf,
        AssetExtractionStatus::NotRequested,
    )
    .expect("record")
}
