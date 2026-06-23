use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_asset_store::{
    ASSET_METADATA_DIR, ASSET_METADATA_FILE, ASSET_OBJECT_EXTENSION, ASSET_OBJECTS_DIR,
    LocalAssetStore,
};
use cabinet_domain::asset::{AssetFileName, AssetId, AssetMediaType, AssetMetadata};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_store::{
    AssetObject, AssetRecord, AssetStore, AssetStoreError, AssetStorePutOutcome,
};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

struct TempAssetRoot {
    path: PathBuf,
}

impl TempAssetRoot {
    fn new(test_name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = PathBuf::from("/tmp").join(format!(
            "sponzey-cabinet-asset-store-{test_name}-{}-{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temp asset root");
        Self { path }
    }
}

impl Drop for TempAssetRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn local_asset_store_reads_metadata_even_when_object_file_is_missing() {
    let temp = TempAssetRoot::new("metadata-only");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = asset_record(HASH_A, "diagram.png", "image/png", vec![1, 2, 3, 4]);
    let asset_id = record.asset_id().clone();
    let mut store = LocalAssetStore::new(temp.path.clone());

    store.put_asset(&workspace_id, record).expect("put asset");
    fs::remove_file(object_path(&temp, &workspace_id, &asset_id)).expect("remove object");
    let metadata = store
        .get_metadata(&workspace_id, &asset_id)
        .expect("get metadata")
        .expect("metadata");

    assert_eq!(metadata.file_name().as_str(), "diagram.png");
    assert_eq!(metadata.byte_size(), 4);
}

#[test]
fn local_asset_store_reads_object_bytes_separately_from_metadata() {
    let temp = TempAssetRoot::new("object");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = asset_record(HASH_A, "diagram.png", "image/png", vec![1, 2, 3, 4]);
    let asset_id = record.asset_id().clone();
    let mut store = LocalAssetStore::new(temp.path.clone());

    store.put_asset(&workspace_id, record).expect("put asset");
    let object = store
        .get_object(&workspace_id, &asset_id)
        .expect("get object")
        .expect("object");

    assert_eq!(object.bytes(), &[1, 2, 3, 4]);
    assert!(metadata_path(&temp, &workspace_id, &asset_id).is_file());
    assert!(object_path(&temp, &workspace_id, &asset_id).is_file());
}

#[test]
fn local_asset_store_reports_duplicate_registration_as_already_present() {
    let temp = TempAssetRoot::new("duplicate");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let mut store = LocalAssetStore::new(temp.path.clone());

    let first = store
        .put_asset(
            &workspace_id,
            asset_record(HASH_A, "diagram.png", "image/png", vec![1, 2, 3, 4]),
        )
        .expect("first put");
    let second = store
        .put_asset(
            &workspace_id,
            asset_record(HASH_A, "diagram.png", "image/png", vec![1, 2, 3, 4]),
        )
        .expect("second put");

    assert_eq!(first, AssetStorePutOutcome::Created);
    assert_eq!(second, AssetStorePutOutcome::AlreadyPresent);
}

#[test]
fn local_asset_store_reports_missing_object_when_metadata_exists() {
    let temp = TempAssetRoot::new("missing-object");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = asset_record(HASH_A, "diagram.png", "image/png", vec![1, 2, 3, 4]);
    let asset_id = record.asset_id().clone();
    let mut store = LocalAssetStore::new(temp.path.clone());

    store.put_asset(&workspace_id, record).expect("put asset");
    fs::remove_file(object_path(&temp, &workspace_id, &asset_id)).expect("remove object");
    let error = store
        .get_object(&workspace_id, &asset_id)
        .expect_err("missing object must fail");

    assert_eq!(error, AssetStoreError::MissingObject);
}

#[test]
fn local_asset_store_reports_corrupted_metadata() {
    let temp = TempAssetRoot::new("corrupt");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = asset_record(HASH_A, "diagram.png", "image/png", vec![1, 2, 3, 4]);
    let asset_id = record.asset_id().clone();
    let mut store = LocalAssetStore::new(temp.path.clone());

    store.put_asset(&workspace_id, record).expect("put asset");
    fs::write(
        metadata_path(&temp, &workspace_id, &asset_id),
        "not valid metadata",
    )
    .expect("write corrupt metadata");
    let error = store
        .get_metadata(&workspace_id, &asset_id)
        .expect_err("corrupt metadata must fail");

    assert_eq!(error, AssetStoreError::CorruptedMetadata);
}

fn asset_record(hash: &str, file_name: &str, media_type: &str, bytes: Vec<u8>) -> AssetRecord {
    AssetRecord::new(
        AssetMetadata::new(
            asset_id(hash),
            AssetFileName::new(file_name).expect("file name"),
            AssetMediaType::new(media_type).expect("media type"),
            bytes.len() as u64,
        )
        .expect("metadata"),
        AssetObject::new(asset_id(hash), bytes).expect("object"),
    )
    .expect("asset record")
}

fn asset_id(hash: &str) -> AssetId {
    AssetId::from_sha256_hex(hash).expect("asset id")
}

fn metadata_path(temp: &TempAssetRoot, workspace_id: &WorkspaceId, asset_id: &AssetId) -> PathBuf {
    temp.path
        .join(workspace_id.as_str())
        .join(ASSET_METADATA_DIR)
        .join(asset_id.as_str())
        .join(ASSET_METADATA_FILE)
}

fn object_path(temp: &TempAssetRoot, workspace_id: &WorkspaceId, asset_id: &AssetId) -> PathBuf {
    temp.path
        .join(workspace_id.as_str())
        .join(ASSET_OBJECTS_DIR)
        .join(&asset_id.as_str()[0..2])
        .join(format!("{}.{}", asset_id.as_str(), ASSET_OBJECT_EXTENSION))
}
