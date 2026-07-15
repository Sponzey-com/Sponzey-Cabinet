use cabinet_domain::asset::{AssetImportDescriptor, AssetImportError, AssetImportHandle};

#[test]
fn opaque_import_handle_accepts_a_platform_token_without_path_semantics() {
    let handle = AssetImportHandle::new("picker:7f00-token").expect("opaque handle");

    assert_eq!(handle.as_str(), "picker:7f00-token");
}

#[test]
fn opaque_import_handle_rejects_empty_control_and_oversized_values() {
    assert_eq!(
        AssetImportHandle::new("  ").expect_err("empty"),
        AssetImportError::InvalidHandle
    );
    assert_eq!(
        AssetImportHandle::new("picker:\nsecret").expect_err("control"),
        AssetImportError::InvalidHandle
    );
    assert_eq!(
        AssetImportHandle::new(&"a".repeat(257)).expect_err("oversized"),
        AssetImportError::HandleTooLong
    );
}

#[test]
fn import_descriptor_reuses_safe_asset_metadata_rules() {
    let descriptor = AssetImportDescriptor::new(
        AssetImportHandle::new("picker:one").expect("handle"),
        "architecture.pdf",
        "application/pdf",
        4096,
    )
    .expect("descriptor");

    assert_eq!(descriptor.handle().as_str(), "picker:one");
    assert_eq!(descriptor.file_name().as_str(), "architecture.pdf");
    assert_eq!(descriptor.media_type().as_str(), "application/pdf");
    assert_eq!(descriptor.byte_size(), 4096);

    assert_eq!(
        AssetImportDescriptor::new(
            AssetImportHandle::new("picker:two").expect("handle"),
            "../private.txt",
            "text/plain",
            1,
        )
        .expect_err("unsafe name"),
        AssetImportError::InvalidFileName
    );
    assert_eq!(
        AssetImportDescriptor::new(
            AssetImportHandle::new("picker:three").expect("handle"),
            "file.bin",
            "not-a-mime",
            1,
        )
        .expect_err("invalid mime"),
        AssetImportError::InvalidMediaType
    );
    assert_eq!(
        AssetImportDescriptor::new(
            AssetImportHandle::new("picker:four").expect("handle"),
            "empty.txt",
            "text/plain",
            0,
        )
        .expect_err("zero byte"),
        AssetImportError::InvalidByteSize
    );
}
