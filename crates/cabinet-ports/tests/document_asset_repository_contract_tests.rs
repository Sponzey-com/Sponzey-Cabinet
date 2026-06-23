use cabinet_domain::asset::{
    AssetFileName, AssetId, AssetMediaType, AssetMetadata, AssetReference,
};
use cabinet_ports::document_asset_repository::{DocumentAssetRecord, DocumentAssetRepositoryError};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn document_asset_record_rejects_mismatched_reference_and_metadata() {
    let reference = AssetReference::new(asset_id(HASH_A), "Diagram").expect("reference");
    let metadata = asset_metadata(HASH_B, "diagram.png", "image/png", 4);

    let error = DocumentAssetRecord::new(reference, metadata).expect_err("mismatch must fail");

    assert_eq!(
        error,
        DocumentAssetRepositoryError::MismatchedAssetReference
    );
    assert_eq!(
        error.code(),
        "document_asset_repository.mismatched_asset_reference"
    );
}

#[test]
fn document_asset_record_keeps_reference_and_metadata_without_object_bytes() {
    let reference = AssetReference::new(asset_id(HASH_A), "Diagram").expect("reference");
    let metadata = asset_metadata(HASH_A, "diagram.png", "image/png", 4);

    let record = DocumentAssetRecord::new(reference.clone(), metadata.clone()).expect("record");

    assert_eq!(record.reference(), &reference);
    assert_eq!(record.metadata(), &metadata);
    assert_eq!(record.asset_id(), metadata.id());
}

fn asset_metadata(hash: &str, file_name: &str, media_type: &str, byte_size: u64) -> AssetMetadata {
    AssetMetadata::new(
        asset_id(hash),
        AssetFileName::new(file_name).expect("file name"),
        AssetMediaType::new(media_type).expect("media type"),
        byte_size,
    )
    .expect("metadata")
}

fn asset_id(hash: &str) -> AssetId {
    AssetId::from_sha256_hex(hash).expect("asset id")
}
