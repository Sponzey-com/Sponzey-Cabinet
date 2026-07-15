use cabinet_domain::asset::{
    AssetCatalogRecord, AssetError, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType,
    AssetMetadata, AssetPreviewCapability,
};

#[test]
fn catalog_record_keeps_explicit_version_preview_and_extraction_status() {
    let record = AssetCatalogRecord::new(
        metadata("application/pdf"),
        1,
        AssetPreviewCapability::Pdf,
        AssetExtractionStatus::NotRequested,
    )
    .expect("record");
    assert_eq!(record.version(), 1);
    assert_eq!(record.preview(), AssetPreviewCapability::Pdf);
    assert_eq!(record.extraction(), AssetExtractionStatus::NotRequested);
}

#[test]
fn catalog_record_rejects_zero_version_and_incompatible_preview() {
    assert_eq!(
        AssetCatalogRecord::new(
            metadata("text/plain"),
            0,
            AssetPreviewCapability::Text,
            AssetExtractionStatus::Ready
        )
        .expect_err("version"),
        AssetError::InvalidCatalogVersion
    );
    assert_eq!(
        AssetCatalogRecord::new(
            metadata("application/pdf"),
            1,
            AssetPreviewCapability::Image,
            AssetExtractionStatus::NotRequested
        )
        .expect_err("preview"),
        AssetError::IncompatiblePreviewCapability
    );
}

fn metadata(media_type: &str) -> AssetMetadata {
    AssetMetadata::new(
        AssetId::from_sha256_hex(&"a".repeat(64)).expect("id"),
        AssetFileName::new("file.pdf").expect("name"),
        AssetMediaType::new(media_type).expect("media"),
        10,
    )
    .expect("metadata")
}
