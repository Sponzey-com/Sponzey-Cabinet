use cabinet_domain::asset::{
    AssetError, AssetFileName, AssetId, AssetMediaType, AssetMetadata, AssetReference,
};

const VALID_SHA256: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn asset_id_accepts_only_sha256_hex_identity() {
    let id = AssetId::from_sha256_hex(VALID_SHA256).expect("asset id should be valid");

    assert_eq!(id.as_str(), VALID_SHA256);
    assert_eq!(
        AssetId::from_sha256_hex("abc").expect_err("short hash must fail"),
        AssetError::InvalidContentHash
    );
    assert_eq!(
        AssetId::from_sha256_hex(
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        )
        .expect_err("non hex hash must fail"),
        AssetError::InvalidContentHash
    );
}

#[test]
fn asset_metadata_validates_file_name_media_type_and_size() {
    let metadata = AssetMetadata::new(
        AssetId::from_sha256_hex(VALID_SHA256).expect("id"),
        AssetFileName::new("diagram.png").expect("file name"),
        AssetMediaType::new("image/png").expect("media type"),
        1024,
    )
    .expect("metadata should be valid");

    assert_eq!(metadata.file_name().as_str(), "diagram.png");
    assert_eq!(metadata.media_type().as_str(), "image/png");
    assert_eq!(metadata.byte_size(), 1024);
    assert_eq!(
        AssetFileName::new("../secret.png").expect_err("path name must fail"),
        AssetError::InvalidFileName
    );
    assert_eq!(
        AssetMediaType::new("image").expect_err("invalid media type must fail"),
        AssetError::InvalidMediaType
    );
    assert_eq!(
        AssetMetadata::new(
            AssetId::from_sha256_hex(VALID_SHA256).expect("id"),
            AssetFileName::new("empty.png").expect("file name"),
            AssetMediaType::new("image/png").expect("media type"),
            0,
        )
        .expect_err("zero size must fail"),
        AssetError::InvalidByteSize
    );
}

#[test]
fn asset_reference_points_to_asset_without_embedding_original_bytes() {
    let id = AssetId::from_sha256_hex(VALID_SHA256).expect("id");
    let reference =
        AssetReference::new(id.clone(), "Architecture diagram").expect("reference should be valid");

    assert_eq!(reference.asset_id(), &id);
    assert_eq!(reference.label(), "Architecture diagram");
    assert_eq!(
        AssetReference::new(id, "  ").expect_err("empty label must fail"),
        AssetError::EmptyReferenceLabel
    );
}
