use std::cell::RefCell;

use cabinet_domain::asset::{AssetFileName, AssetId, AssetMediaType};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_search_index::{
    AssetSearchError, AssetSearchIndex, AssetSearchPage, AssetSearchQuery, AssetSearchResult,
};
use cabinet_usecases::asset_search::{SearchAssetsError, SearchAssetsInput, SearchAssetsUsecase};

#[test]
fn search_assets_usecase_validates_input_and_returns_index_results() {
    let index = FakeAssetSearchIndex {
        result: Ok(AssetSearchPage::new(vec![asset_result(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "제품 명세서.pdf",
        )])),
        calls: RefCell::new(Vec::new()),
    };

    let output = SearchAssetsUsecase::new()
        .execute(
            SearchAssetsInput::new("workspace-1", " 명세서 ", 20),
            &index,
        )
        .expect("search assets");

    assert_eq!(index.recorded_workspace().as_str(), "workspace-1");
    assert_eq!(index.recorded_query().text(), "명세서");
    assert_eq!(index.recorded_query().limit(), 20);
    assert_eq!(
        output.page().results()[0].file_name().as_str(),
        "제품 명세서.pdf"
    );
}

#[test]
fn search_assets_usecase_rejects_blank_query_and_invalid_limit() {
    let index = FakeAssetSearchIndex {
        result: Ok(AssetSearchPage::new(Vec::new())),
        calls: RefCell::new(Vec::new()),
    };
    let usecase = SearchAssetsUsecase::new();

    assert_eq!(
        usecase.execute(SearchAssetsInput::new("workspace-1", " ", 20), &index),
        Err(SearchAssetsError::InvalidInput),
    );
    assert_eq!(
        usecase.execute(SearchAssetsInput::new("workspace-1", "asset", 0), &index),
        Err(SearchAssetsError::InvalidInput),
    );
}

#[test]
fn search_assets_usecase_maps_index_failures_to_stable_errors() {
    let usecase = SearchAssetsUsecase::new();
    let unavailable = FakeAssetSearchIndex {
        result: Err(AssetSearchError::StorageUnavailable),
        calls: RefCell::new(Vec::new()),
    };
    let corrupted = FakeAssetSearchIndex {
        result: Err(AssetSearchError::CorruptedIndex),
        calls: RefCell::new(Vec::new()),
    };

    assert_eq!(
        usecase.execute(
            SearchAssetsInput::new("workspace-1", "asset", 20),
            &unavailable
        ),
        Err(SearchAssetsError::StorageUnavailable),
    );
    assert_eq!(
        usecase.execute(
            SearchAssetsInput::new("workspace-1", "asset", 20),
            &corrupted
        ),
        Err(SearchAssetsError::StorageUnavailable),
    );
    assert_eq!(
        SearchAssetsError::StorageUnavailable.code(),
        "asset_search.storage_unavailable"
    );
}

struct FakeAssetSearchIndex {
    result: Result<AssetSearchPage, AssetSearchError>,
    calls: RefCell<Vec<(WorkspaceId, AssetSearchQuery)>>,
}

impl FakeAssetSearchIndex {
    fn recorded_workspace(&self) -> WorkspaceId {
        self.calls.borrow()[0].0.clone()
    }

    fn recorded_query(&self) -> AssetSearchQuery {
        self.calls.borrow()[0].1.clone()
    }
}

impl AssetSearchIndex for FakeAssetSearchIndex {
    fn search_assets(
        &self,
        _workspace_id: &WorkspaceId,
        query: AssetSearchQuery,
    ) -> Result<AssetSearchPage, AssetSearchError> {
        self.calls.borrow_mut().push((_workspace_id.clone(), query));
        self.result.clone()
    }
}

fn asset_result(id: &str, file_name: &str) -> AssetSearchResult {
    AssetSearchResult::new(
        AssetId::from_sha256_hex(id).expect("asset id"),
        AssetFileName::new(file_name).expect("file name"),
        AssetMediaType::new("application/pdf").expect("media type"),
        1536,
        100,
    )
    .expect("asset search result")
}
