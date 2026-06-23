use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::asset::{AssetFileName, AssetId, AssetMediaType, AssetMetadata};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_store::{
    AssetObject, AssetRecord, AssetStore, AssetStoreError, AssetStorePutOutcome,
};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[derive(Default)]
struct FakeAssetStore {
    metadata_by_id: HashMap<(String, String), AssetMetadata>,
    object_by_id: HashMap<(String, String), AssetObject>,
    object_read_count: Cell<usize>,
}

impl FakeAssetStore {
    fn object_read_count(&self) -> usize {
        self.object_read_count.get()
    }
}

impl AssetStore for FakeAssetStore {
    fn put_asset(
        &mut self,
        workspace_id: &WorkspaceId,
        record: AssetRecord,
    ) -> Result<AssetStorePutOutcome, AssetStoreError> {
        let key = (
            workspace_id.as_str().to_string(),
            record.asset_id().as_str().to_string(),
        );
        if self.metadata_by_id.contains_key(&key) {
            return Ok(AssetStorePutOutcome::AlreadyPresent);
        }
        self.metadata_by_id
            .insert(key.clone(), record.metadata().clone());
        self.object_by_id.insert(key, record.object().clone());
        Ok(AssetStorePutOutcome::Created)
    }

    fn get_metadata(
        &self,
        workspace_id: &WorkspaceId,
        asset_id: &AssetId,
    ) -> Result<Option<AssetMetadata>, AssetStoreError> {
        Ok(self
            .metadata_by_id
            .get(&(
                workspace_id.as_str().to_string(),
                asset_id.as_str().to_string(),
            ))
            .cloned())
    }

    fn get_object(
        &self,
        workspace_id: &WorkspaceId,
        asset_id: &AssetId,
    ) -> Result<Option<AssetObject>, AssetStoreError> {
        self.object_read_count.set(self.object_read_count.get() + 1);
        Ok(self
            .object_by_id
            .get(&(
                workspace_id.as_str().to_string(),
                asset_id.as_str().to_string(),
            ))
            .cloned())
    }

    fn remove_asset(
        &mut self,
        workspace_id: &WorkspaceId,
        asset_id: &AssetId,
    ) -> Result<(), AssetStoreError> {
        let key = (
            workspace_id.as_str().to_string(),
            asset_id.as_str().to_string(),
        );
        self.metadata_by_id.remove(&key);
        self.object_by_id.remove(&key);
        Ok(())
    }
}

#[test]
fn asset_record_rejects_mismatched_metadata_and_object_identity() {
    let metadata = asset_metadata(HASH_A, "diagram.png", "image/png", 4);
    let object = AssetObject::new(asset_id(HASH_B), vec![1, 2, 3, 4]).expect("object");

    let error = AssetRecord::new(metadata, object).expect_err("mismatched id must fail");

    assert_eq!(error, AssetStoreError::MismatchedAssetObject);
    assert_eq!(error.code(), "asset_store.mismatched_asset_object");
}

#[test]
fn asset_record_rejects_mismatched_metadata_byte_size_and_object_length() {
    let metadata = asset_metadata(HASH_A, "diagram.png", "image/png", 8);
    let object = AssetObject::new(asset_id(HASH_A), vec![1, 2, 3, 4]).expect("object");

    let error = AssetRecord::new(metadata, object).expect_err("mismatched size must fail");

    assert_eq!(error, AssetStoreError::MismatchedAssetObject);
}

#[test]
fn asset_store_contract_reads_metadata_without_reading_object_bytes() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let record = asset_record(HASH_A, "diagram.png", "image/png", vec![1, 2, 3, 4]);
    let asset_id = record.asset_id().clone();
    let mut store = FakeAssetStore::default();

    assert_eq!(
        store.put_asset(&workspace_id, record).expect("put asset"),
        AssetStorePutOutcome::Created
    );
    let metadata = store
        .get_metadata(&workspace_id, &asset_id)
        .expect("get metadata")
        .expect("metadata");

    assert_eq!(metadata.file_name().as_str(), "diagram.png");
    assert_eq!(metadata.byte_size(), 4);
    assert_eq!(store.object_read_count(), 0);
}

#[test]
fn asset_store_contract_reports_duplicate_registration_as_already_present() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let mut store = FakeAssetStore::default();

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

fn asset_record(hash: &str, file_name: &str, media_type: &str, bytes: Vec<u8>) -> AssetRecord {
    AssetRecord::new(
        asset_metadata(hash, file_name, media_type, bytes.len() as u64),
        AssetObject::new(asset_id(hash), bytes).expect("object"),
    )
    .expect("asset record")
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
