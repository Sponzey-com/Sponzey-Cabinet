use cabinet_domain::asset::{AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType, AssetMetadata, AssetPreviewCapability};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_metadata_catalog::{AssetMetadataCatalog, AssetMetadataCatalogError, AssetMetadataPage, AssetMetadataPutOutcome};
use cabinet_ports::asset_preview::{AssetPreviewReadError, AssetPreviewReader};
use cabinet_usecases::asset_preview::{GetAssetPreviewInput, GetAssetPreviewUsecase, AssetPreviewResult};

#[test]
fn supported_text_preview_returns_bounded_content() {
    let metadata = Catalog::new("text/plain", 7);
    let output = GetAssetPreviewUsecase::new().execute(
        GetAssetPreviewInput::new("workspace-1", &"a".repeat(64), 1024).expect("input"),
        &metadata,
        &Reader(Ok(b"preview".to_vec())),
    ).expect("preview");
    assert!(matches!(output.result(), AssetPreviewResult::Content(bytes) if bytes == b"preview"));
    assert_eq!(output.capability(), AssetPreviewCapability::Text);
}

#[test]
fn unsupported_preview_does_not_read_object() {
    let output = GetAssetPreviewUsecase::new().execute(
        GetAssetPreviewInput::new("workspace-1", &"a".repeat(64), 1024).expect("input"),
        &Catalog::new("application/octet-stream", 7),
        &Reader(Err(AssetPreviewReadError::StorageUnavailable)),
    ).expect("unsupported is a result");
    assert_eq!(output.result(), &AssetPreviewResult::Unsupported);
}

#[test]
fn oversized_metadata_is_rejected_before_object_read() {
    let error = GetAssetPreviewUsecase::new().execute(
        GetAssetPreviewInput::new("workspace-1", &"a".repeat(64), 4).expect("input"),
        &Catalog::new("text/plain", 7),
        &Reader(Ok(b"preview".to_vec())),
    ).expect_err("oversized");
    assert_eq!(error.code(), "asset_preview.too_large");
}

struct Reader(Result<Vec<u8>, AssetPreviewReadError>);
impl AssetPreviewReader for Reader {
    fn read(&self, _: &WorkspaceId, _: &AssetId, _: usize) -> Result<Vec<u8>, AssetPreviewReadError> { self.0.clone() }
}

struct Catalog(AssetCatalogRecord);
impl Catalog {
    fn new(media_type: &str, byte_size: u64) -> Self {
        let metadata = AssetMetadata::new(
            AssetId::from_sha256_hex(&"a".repeat(64)).expect("asset"),
            AssetFileName::new("fixture.txt").expect("name"),
            AssetMediaType::new(media_type).expect("media"),
            byte_size,
        ).expect("metadata");
        Self(AssetCatalogRecord::new(metadata, 1, AssetPreviewCapability::for_media_type(&AssetMediaType::new(media_type).expect("media")), AssetExtractionStatus::NotRequested).expect("record"))
    }
}
impl AssetMetadataCatalog for Catalog {
    fn put(&mut self, _: &WorkspaceId, _: AssetCatalogRecord) -> Result<AssetMetadataPutOutcome, AssetMetadataCatalogError> { unreachable!() }
    fn get(&self, _: &WorkspaceId, _: &AssetId) -> Result<Option<AssetCatalogRecord>, AssetMetadataCatalogError> { Ok(Some(self.0.clone())) }
    fn list(&self, _: &WorkspaceId, _: Option<&str>, _: usize) -> Result<AssetMetadataPage, AssetMetadataCatalogError> { unreachable!() }
}
