use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_asset_preview_reader::LocalAssetPreviewReader;
use cabinet_domain::asset::AssetId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_preview::{AssetPreviewReadError, AssetPreviewReader};
use sha2::{Digest, Sha256};

#[test]
fn reads_verified_bounded_content_addressed_object() {
    let root = temp_root("read");
    let bytes = b"preview content";
    let asset = AssetId::from_sha256_hex(&format!("{:x}", Sha256::digest(bytes))).expect("asset");
    write_object(&root, &asset, bytes);
    let result = LocalAssetPreviewReader::new(root.clone())
        .read(&workspace(), &asset, 1024)
        .expect("read");
    assert_eq!(result, bytes);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_oversized_and_corrupted_objects() {
    let root = temp_root("reject");
    let asset = AssetId::from_sha256_hex(&"a".repeat(64)).expect("asset");
    write_object(&root, &asset, b"corrupt");
    let reader = LocalAssetPreviewReader::new(root.clone());
    assert_eq!(
        reader.read(&workspace(), &asset, 2),
        Err(AssetPreviewReadError::TooLarge)
    );
    assert_eq!(
        reader.read(&workspace(), &asset, 1024),
        Err(AssetPreviewReadError::Corrupted)
    );
    let _ = fs::remove_dir_all(root);
}

fn write_object(root: &std::path::Path, asset: &AssetId, bytes: &[u8]) {
    let path = root
        .join("assets/objects")
        .join(hex("workspace-1"))
        .join(&asset.as_str()[..2])
        .join(format!("{}.bin", asset.as_str()));
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, bytes).expect("write");
}
fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace")
}
fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn temp_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "cabinet-preview-{name}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ))
}
