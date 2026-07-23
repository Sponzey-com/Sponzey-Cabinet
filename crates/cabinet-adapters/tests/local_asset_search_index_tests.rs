use cabinet_adapters::local_asset_search_index::LocalAssetSearchIndex;
use cabinet_domain::asset::{
    AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType,
    AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_search_index::{AssetSearchIndex, AssetSearchQuery};

#[test]
fn local_asset_search_index_returns_matching_assets_by_file_name() {
    let workspace = workspace_id("workspace-1");
    let mut index = LocalAssetSearchIndex::default();
    index.upsert_asset(
        &workspace,
        asset_record(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "제품 명세서.pdf",
            "application/pdf",
            1536,
        ),
    );

    let page = index
        .search_assets(
            &workspace,
            AssetSearchQuery::new("명세서", 10).expect("query"),
        )
        .expect("search");

    assert_eq!(page.results().len(), 1);
    assert_eq!(page.results()[0].file_name().as_str(), "제품 명세서.pdf");
    assert_eq!(page.results()[0].media_type().as_str(), "application/pdf");
}

#[test]
fn local_asset_search_index_respects_limit_and_orders_higher_score_first() {
    let workspace = workspace_id("workspace-1");
    let mut index = LocalAssetSearchIndex::default();
    index.upsert_asset(
        &workspace,
        asset_record(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "cabinet cabinet.pdf",
            "application/pdf",
            1536,
        ),
    );
    index.upsert_asset(
        &workspace,
        asset_record(
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "image.png",
            "image/cabinet",
            2048,
        ),
    );

    let page = index
        .search_assets(
            &workspace,
            AssetSearchQuery::new("cabinet", 1).expect("query"),
        )
        .expect("search");

    assert_eq!(page.results().len(), 1);
    assert_eq!(
        page.results()[0].file_name().as_str(),
        "cabinet cabinet.pdf"
    );
    assert!(page.results()[0].score() > 1);
}

#[test]
fn local_asset_search_index_deletes_assets_and_isolates_workspaces() {
    let workspace = workspace_id("workspace-1");
    let other_workspace = workspace_id("workspace-2");
    let mut index = LocalAssetSearchIndex::default();
    let asset_id = AssetId::from_sha256_hex(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .expect("asset id");
    index.upsert_asset(
        &workspace,
        asset_record(asset_id.as_str(), "cabinet.pdf", "application/pdf", 1536),
    );
    index.upsert_asset(
        &other_workspace,
        asset_record(
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "cabinet-private.pdf",
            "application/pdf",
            1536,
        ),
    );

    index.delete_asset(&workspace, &asset_id);

    let page = index
        .search_assets(
            &workspace,
            AssetSearchQuery::new("cabinet", 10).expect("query"),
        )
        .expect("search");
    let other_page = index
        .search_assets(
            &other_workspace,
            AssetSearchQuery::new("cabinet", 10).expect("query"),
        )
        .expect("search");

    assert!(page.results().is_empty());
    assert_eq!(other_page.results().len(), 1);
}

fn workspace_id(value: &str) -> WorkspaceId {
    WorkspaceId::new(value).expect("workspace id")
}

fn asset_record(id: &str, file_name: &str, media_type: &str, byte_size: u64) -> AssetCatalogRecord {
    let media_type = AssetMediaType::new(media_type).expect("media type");
    AssetCatalogRecord::new(
        AssetMetadata::new(
            AssetId::from_sha256_hex(id).expect("asset id"),
            AssetFileName::new(file_name).expect("file name"),
            media_type.clone(),
            byte_size,
        )
        .expect("metadata"),
        1,
        AssetPreviewCapability::for_media_type(&media_type),
        AssetExtractionStatus::NotRequested,
    )
    .expect("record")
}
