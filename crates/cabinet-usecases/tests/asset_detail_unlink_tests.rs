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
use cabinet_usecases::asset_lifecycle::{
    AssetLifecycleProductEvent, AssetLifecycleProductLogger, GetAssetDetailInput,
    GetAssetDetailUsecase, LinkAssetInput, LinkAssetUsecase, ListWorkspaceAssetsInput,
    ListWorkspaceAssetsUsecase, UnlinkAssetInput, UnlinkAssetUsecase,
};

#[test]
fn workspace_page_preserves_catalog_cursor_and_rejects_invalid_input() {
    let root = temp_root("workspace-page");
    let (metadata, _, _) = seed(&root);

    let page = ListWorkspaceAssetsUsecase::new()
        .execute(
            ListWorkspaceAssetsInput::new("workspace-1", Some(&"0".repeat(64)), 25).expect("input"),
            &metadata,
        )
        .expect("page");

    assert_eq!(page.records().len(), 1);
    assert_eq!(page.next_cursor(), Some("next-cursor"));
    assert!(ListWorkspaceAssetsInput::new("workspace-1", None, 0).is_err());
    assert!(ListWorkspaceAssetsInput::new("workspace-1", Some("unsafe"), 25).is_err());
}

#[test]
fn existing_asset_link_is_idempotent_validates_entities_and_logs_only_new_link() {
    let root = temp_root("link");
    let (metadata, mut associations, asset) = seed(&root);
    associations.0.clear();
    let mut logger = RecordingLogger::default();

    let first = LinkAssetUsecase::new()
        .execute(
            LinkAssetInput::new("workspace-1", "doc-1", asset.as_str(), "Spec").expect("input"),
            &ExistingDocument,
            &metadata,
            &mut associations,
            &mut logger,
        )
        .expect("link");
    let second = LinkAssetUsecase::new()
        .execute(
            LinkAssetInput::new("workspace-1", "doc-1", asset.as_str(), "Spec").expect("input"),
            &ExistingDocument,
            &metadata,
            &mut associations,
            &mut logger,
        )
        .expect("idempotent link");

    assert!(first.linked());
    assert!(!second.linked());
    assert_eq!(first.reference_count(), 1);
    assert_eq!(logger.events.len(), 1);
    assert!(matches!(
        logger.events[0],
        AssetLifecycleProductEvent::Linked { .. }
    ));
    let missing = LinkAssetUsecase::new().execute(
        LinkAssetInput::new("workspace-1", "doc-1", &"b".repeat(64), "Missing").expect("input"),
        &ExistingDocument,
        &metadata,
        &mut associations,
        &mut logger,
    );
    assert_eq!(
        missing.expect_err("missing asset").code(),
        "asset_lifecycle.asset_not_found"
    );
}

#[test]
fn detail_returns_capability_version_extraction_and_bounded_document_links() {
    let root = temp_root("detail");
    let (metadata, associations, asset) = seed(&root);

    let detail = GetAssetDetailUsecase::new()
        .execute(
            GetAssetDetailInput::new("workspace-1", asset.as_str(), 10).expect("input"),
            &metadata,
            &associations,
        )
        .expect("detail");

    assert_eq!(detail.record().version(), 1);
    assert_eq!(detail.record().preview(), AssetPreviewCapability::Pdf);
    assert_eq!(
        detail.record().extraction(),
        AssetExtractionStatus::NotRequested
    );
    assert_eq!(detail.reference_count(), 2);
    assert_eq!(detail.linked_documents().len(), 2);
}

#[test]
fn unlink_is_idempotent_preserves_other_links_and_emits_safe_product_event_once() {
    let root = temp_root("unlink");
    let (metadata, mut associations, asset) = seed(&root);
    let mut logger = RecordingLogger::default();
    let input = UnlinkAssetInput::new("workspace-1", "doc-1", asset.as_str()).expect("input");

    let first = UnlinkAssetUsecase::new()
        .execute(
            input,
            &ExistingDocument,
            &metadata,
            &mut associations,
            &mut logger,
        )
        .expect("unlink");
    let second = UnlinkAssetUsecase::new()
        .execute(
            UnlinkAssetInput::new("workspace-1", "doc-1", asset.as_str()).expect("input"),
            &ExistingDocument,
            &metadata,
            &mut associations,
            &mut logger,
        )
        .expect("idempotent unlink");

    assert!(first.removed());
    assert!(!second.removed());
    assert_eq!(second.remaining_references(), 1);
    assert!(
        metadata
            .get(&WorkspaceId::new("workspace-1").expect("workspace"), &asset)
            .expect("metadata")
            .is_some()
    );
    assert_eq!(logger.events.len(), 1);
    assert!(matches!(
        logger.events[0],
        AssetLifecycleProductEvent::Unlinked { .. }
    ));
}

struct ExistingDocument;
impl DocumentExistenceReader for ExistingDocument {
    fn exists(&self, _: &WorkspaceId, _: &DocumentId) -> Result<bool, DocumentExistenceError> {
        Ok(true)
    }
}

#[derive(Default)]
struct RecordingLogger {
    events: Vec<AssetLifecycleProductEvent>,
}
impl AssetLifecycleProductLogger for RecordingLogger {
    fn write_product(&mut self, event: AssetLifecycleProductEvent) {
        self.events.push(event);
    }
}

fn seed(_: &std::path::Path) -> (FakeMetadata, FakeAssociations, AssetId) {
    let asset = AssetId::from_sha256_hex(&"a".repeat(64)).expect("asset");
    let record = AssetCatalogRecord::new(
        AssetMetadata::new(
            asset.clone(),
            AssetFileName::new("spec.pdf").expect("name"),
            AssetMediaType::new("application/pdf").expect("type"),
            42,
        )
        .expect("metadata"),
        1,
        AssetPreviewCapability::Pdf,
        AssetExtractionStatus::NotRequested,
    )
    .expect("record");
    let metadata = FakeMetadata(vec![record]);
    let associations = FakeAssociations(
        ["doc-1", "doc-2"]
            .into_iter()
            .map(|document| {
                AssetAssociation::new(
                    asset.clone(),
                    DocumentId::new(document).expect("document"),
                    "Spec",
                )
                .expect("association")
            })
            .collect(),
    );
    (metadata, associations, asset)
}

struct FakeMetadata(Vec<AssetCatalogRecord>);
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
        cursor: Option<&str>,
        _: usize,
    ) -> Result<AssetMetadataPage, AssetMetadataCatalogError> {
        assert!(cursor.is_some_and(|value| value == "0".repeat(64)));
        Ok(AssetMetadataPage::new(
            self.0.clone(),
            Some("next-cursor".to_string()),
        ))
    }
}

struct FakeAssociations(Vec<AssetAssociation>);
impl AssetAssociationCatalog for FakeAssociations {
    fn link(
        &mut self,
        _: &WorkspaceId,
        association: AssetAssociation,
    ) -> Result<AssetAssociationLinkOutcome, AssetAssociationCatalogError> {
        if self.0.contains(&association) {
            Ok(AssetAssociationLinkOutcome::AlreadyLinked)
        } else {
            self.0.push(association);
            Ok(AssetAssociationLinkOutcome::Linked)
        }
    }
    fn unlink(
        &mut self,
        _: &WorkspaceId,
        asset: &AssetId,
        document: &DocumentId,
    ) -> Result<AssetAssociationUnlinkOutcome, AssetAssociationCatalogError> {
        let before = self.0.len();
        self.0
            .retain(|link| link.asset_id() != asset || link.document_id() != document);
        Ok(if self.0.len() == before {
            AssetAssociationUnlinkOutcome::NotLinked
        } else {
            AssetAssociationUnlinkOutcome::Unlinked
        })
    }
    fn list_documents(
        &self,
        _: &WorkspaceId,
        asset: &AssetId,
        limit: usize,
    ) -> Result<Vec<AssetAssociation>, AssetAssociationCatalogError> {
        Ok(self
            .0
            .iter()
            .filter(|link| link.asset_id() == asset)
            .take(limit)
            .cloned()
            .collect())
    }
    fn list_assets(
        &self,
        _: &WorkspaceId,
        document: &DocumentId,
        limit: usize,
    ) -> Result<Vec<AssetAssociation>, AssetAssociationCatalogError> {
        Ok(self
            .0
            .iter()
            .filter(|link| link.document_id() == document)
            .take(limit)
            .cloned()
            .collect())
    }
    fn reference_count(
        &self,
        _: &WorkspaceId,
        asset: &AssetId,
    ) -> Result<u64, AssetAssociationCatalogError> {
        Ok(self
            .0
            .iter()
            .filter(|link| link.asset_id() == asset)
            .count() as u64)
    }
}

fn temp_root(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "sponzey-asset-lifecycle-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("root");
    root
}
