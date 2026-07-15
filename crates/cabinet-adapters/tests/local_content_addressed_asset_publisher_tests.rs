use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_asset_staging_writer::LocalAssetStagingWriter;
use cabinet_adapters::local_content_addressed_asset_publisher::LocalContentAddressedAssetPublisher;
use cabinet_domain::asset_import_operation::AssetImportOperationId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_object_publisher::{
    AssetObjectPublishError, AssetObjectPublishOutcome, AssetObjectPublisher,
};
use cabinet_ports::asset_staging::AssetStagingWriter;

const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

#[test]
fn publisher_hashes_staging_and_reuses_duplicate_content_after_restart() {
    let root = temp_root("publish");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let first = operation("import-1");
    let second = operation("import-2");
    stage(&root, &workspace, &first, b"abc");
    let mut publisher =
        LocalContentAddressedAssetPublisher::new(root.clone(), 2).expect("publisher");
    let created = publisher.publish(&workspace, &first, 3).expect("created");
    assert_eq!(created.asset_id().as_str(), ABC_SHA256);
    assert_eq!(created.outcome(), AssetObjectPublishOutcome::Created);

    stage(&root, &workspace, &second, b"abc");
    let mut restarted = LocalContentAddressedAssetPublisher::new(root.clone(), 2).expect("restart");
    let duplicate = restarted
        .publish(&workspace, &second, 3)
        .expect("duplicate");
    assert_eq!(duplicate.asset_id(), created.asset_id());
    assert_eq!(
        duplicate.outcome(),
        AssetObjectPublishOutcome::AlreadyPresent
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn publisher_rejects_missing_staging_and_size_mismatch() {
    let root = temp_root("errors");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let operation = operation("import-1");
    let mut publisher =
        LocalContentAddressedAssetPublisher::new(root.clone(), 4).expect("publisher");
    assert_eq!(
        publisher
            .publish(&workspace, &operation, 1)
            .expect_err("missing"),
        AssetObjectPublishError::StagingNotFound
    );
    stage(&root, &workspace, &operation, b"abc");
    assert_eq!(
        publisher
            .publish(&workspace, &operation, 4)
            .expect_err("size"),
        AssetObjectPublishError::SizeMismatch
    );
    let _ = fs::remove_dir_all(root);
}

fn stage(
    root: &std::path::Path,
    workspace: &WorkspaceId,
    operation: &AssetImportOperationId,
    bytes: &[u8],
) {
    let mut writer = LocalAssetStagingWriter::new(root.to_path_buf());
    writer.begin(workspace, operation).expect("begin");
    writer
        .append(workspace, operation, 0, bytes)
        .expect("append");
}
fn operation(value: &str) -> AssetImportOperationId {
    AssetImportOperationId::new(value).expect("operation")
}
fn temp_root(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "sponzey-object-publish-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&path).expect("root");
    path
}
