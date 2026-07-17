use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_asset_availability_resolver::LocalAssetAvailabilityResolver;
use cabinet_domain::asset::AssetId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_availability::{
    AssetAvailability, AssetAvailabilityBatchResolver, AssetAvailabilityResolveError,
};

#[test]
fn resolves_regular_and_missing_objects_in_request_order() {
    let temp = TempRoot::new("mixed");
    let workspace = workspace();
    let available = asset_id('a');
    let missing = asset_id('b');
    let path = object_path(&temp.path, &workspace, &available);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"object-bytes").unwrap();

    let records = LocalAssetAvailabilityResolver::new(temp.path.clone())
        .resolve_batch(&workspace, &[missing.clone(), available.clone()])
        .unwrap();

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].asset_id(), &missing);
    assert_eq!(records[0].availability(), AssetAvailability::Missing);
    assert_eq!(records[1].asset_id(), &available);
    assert_eq!(records[1].availability(), AssetAvailability::Available);
}

#[test]
fn empty_batch_performs_no_required_filesystem_setup() {
    let temp = TempRoot::new("empty");
    let records = LocalAssetAvailabilityResolver::new(temp.path.join("not-created"))
        .resolve_batch(&workspace(), &[])
        .unwrap();
    assert!(records.is_empty());
}

#[test]
fn non_regular_object_is_corrupted_instead_of_available() {
    let temp = TempRoot::new("directory");
    let asset = asset_id('a');
    fs::create_dir_all(object_path(&temp.path, &workspace(), &asset)).unwrap();

    assert_eq!(
        LocalAssetAvailabilityResolver::new(temp.path.clone())
            .resolve_batch(&workspace(), &[asset])
            .unwrap_err(),
        AssetAvailabilityResolveError::CorruptedData
    );
}

#[test]
fn filesystem_traversal_failure_is_storage_unavailable() {
    let temp = TempRoot::new("broken-root");
    let broken_root = temp.path.join("not-a-directory");
    fs::write(&broken_root, b"file").unwrap();

    assert_eq!(
        LocalAssetAvailabilityResolver::new(broken_root)
            .resolve_batch(&workspace(), &[asset_id('a')])
            .unwrap_err(),
        AssetAvailabilityResolveError::StorageUnavailable
    );
}

fn object_path(root: &Path, workspace: &WorkspaceId, asset: &AssetId) -> PathBuf {
    root.join("assets/objects")
        .join(hex(workspace.as_str()))
        .join(&asset.as_str()[..2])
        .join(format!("{}.bin", asset.as_str()))
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").unwrap()
}

fn asset_id(seed: char) -> AssetId {
    AssetId::from_sha256_hex(&seed.to_string().repeat(64)).unwrap()
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cabinet-asset-availability-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
