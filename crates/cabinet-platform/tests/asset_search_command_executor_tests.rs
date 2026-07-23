use cabinet_domain::asset::{AssetFileName, AssetId, AssetMediaType};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_platform::asset_search_command::{
    AssetSearchCommandRequest, execute_asset_search_command,
};
use cabinet_ports::asset_search_index::{
    AssetSearchError, AssetSearchIndex, AssetSearchPage, AssetSearchQuery, AssetSearchResult,
};

struct FakeAssetSearchIndex {
    result: Result<AssetSearchPage, AssetSearchError>,
}

impl AssetSearchIndex for FakeAssetSearchIndex {
    fn search_assets(
        &self,
        _workspace_id: &WorkspaceId,
        _query: AssetSearchQuery,
    ) -> Result<AssetSearchPage, AssetSearchError> {
        self.result.clone()
    }
}

#[test]
fn asset_search_executor_maps_results_to_owned_command_dto() {
    let index = FakeAssetSearchIndex {
        result: Ok(AssetSearchPage::new(vec![asset_result(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "제품 명세서.pdf",
            "application/pdf",
            1536,
            3,
        )])),
    };

    let result = execute_asset_search_command(
        AssetSearchCommandRequest {
            workspace_id: "workspace-1".to_string(),
            text: "명세서".to_string(),
            limit: 10,
        },
        &index,
    )
    .expect("asset search command");

    assert_eq!(result.workspace_id, "workspace-1");
    assert_eq!(result.text, "명세서");
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].file_name, "제품 명세서.pdf");
    assert_eq!(result.results[0].media_type, "application/pdf");
    assert_eq!(result.results[0].byte_size, 1536);
    assert_eq!(result.results[0].score, 3);
}

#[test]
fn asset_search_executor_maps_invalid_input_and_storage_failures_to_stable_errors() {
    let invalid = execute_asset_search_command(
        AssetSearchCommandRequest {
            workspace_id: "workspace-1".to_string(),
            text: " ".to_string(),
            limit: 10,
        },
        &FakeAssetSearchIndex {
            result: Ok(AssetSearchPage::new(vec![])),
        },
    )
    .expect_err("blank query fails");
    let unavailable = execute_asset_search_command(
        AssetSearchCommandRequest {
            workspace_id: "workspace-1".to_string(),
            text: "명세서".to_string(),
            limit: 10,
        },
        &FakeAssetSearchIndex {
            result: Err(AssetSearchError::StorageUnavailable),
        },
    )
    .expect_err("storage failure maps");

    assert_eq!(invalid.error_code, "ASSET_SEARCH_INVALID_INPUT");
    assert!(!invalid.retryable);
    assert_eq!(unavailable.error_code, "ASSET_SEARCH_STORAGE_UNAVAILABLE");
    assert!(unavailable.retryable);
}

fn asset_result(
    id: &str,
    file_name: &str,
    media_type: &str,
    byte_size: u64,
    score: u32,
) -> AssetSearchResult {
    AssetSearchResult::new(
        AssetId::from_sha256_hex(id).expect("asset id"),
        AssetFileName::new(file_name).expect("file name"),
        AssetMediaType::new(media_type).expect("media type"),
        byte_size,
        score,
    )
    .expect("asset result")
}
