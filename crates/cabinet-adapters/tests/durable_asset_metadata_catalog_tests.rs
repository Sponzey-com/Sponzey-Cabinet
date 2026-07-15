use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_metadata_catalog::DurableAssetMetadataCatalog;
use cabinet_domain::asset::{
    AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType,
    AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_metadata_catalog::{
    AssetMetadataCatalog, AssetMetadataCatalogError, AssetMetadataPutOutcome,
};

#[test]
fn catalog_survives_restart_is_idempotent_and_lists_stable_cursor_pages() {
    let root = temp_root("restart");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let mut catalog = DurableAssetMetadataCatalog::new(root.clone());
    assert_eq!(
        catalog.put(&workspace, record('a')).expect("put a"),
        AssetMetadataPutOutcome::Created
    );
    assert_eq!(
        catalog.put(&workspace, record('b')).expect("put b"),
        AssetMetadataPutOutcome::Created
    );
    assert_eq!(
        catalog.put(&workspace, record('a')).expect("duplicate"),
        AssetMetadataPutOutcome::AlreadyPresent
    );
    let restarted = DurableAssetMetadataCatalog::new(root.clone());
    let first = restarted.list(&workspace, None, 1).expect("first");
    assert_eq!(first.records().len(), 1);
    assert!(first.next_cursor().is_some());
    let second = restarted
        .list(&workspace, first.next_cursor(), 1)
        .expect("second");
    assert_eq!(second.records().len(), 1);
    assert_ne!(
        first.records()[0].metadata().id(),
        second.records()[0].metadata().id()
    );
    assert_eq!(
        restarted
            .get(&workspace, first.records()[0].metadata().id())
            .expect("get"),
        Some(first.records()[0].clone())
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn catalog_reports_invalid_limit_and_corruption() {
    let root = temp_root("corrupt");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let mut catalog = DurableAssetMetadataCatalog::new(root.clone());
    catalog.put(&workspace, record('a')).expect("put");
    assert_eq!(
        catalog.list(&workspace, None, 0).expect_err("limit"),
        AssetMetadataCatalogError::InvalidLimit
    );
    let record_path = walk_first_file(&root);
    fs::write(record_path, "schema\t2\n").expect("corrupt");
    assert_eq!(
        catalog.list(&workspace, None, 10).expect_err("schema"),
        AssetMetadataCatalogError::UnsupportedSchema
    );
    let _ = fs::remove_dir_all(root);
}

fn record(fill: char) -> AssetCatalogRecord {
    let id = fill.to_string().repeat(64);
    AssetCatalogRecord::new(
        AssetMetadata::new(
            AssetId::from_sha256_hex(&id).expect("id"),
            AssetFileName::new(&format!("{fill}.pdf")).expect("name"),
            AssetMediaType::new("application/pdf").expect("media"),
            10,
        )
        .expect("metadata"),
        1,
        AssetPreviewCapability::Pdf,
        AssetExtractionStatus::NotRequested,
    )
    .expect("record")
}
fn temp_root(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "sponzey-asset-catalog-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    root
}
fn walk_first_file(root: &std::path::Path) -> std::path::PathBuf {
    let workspace = fs::read_dir(root.join("assets/metadata"))
        .expect("workspace")
        .next()
        .expect("workspace entry")
        .expect("entry")
        .path();
    fs::read_dir(workspace)
        .expect("records")
        .next()
        .expect("record")
        .expect("entry")
        .path()
}
