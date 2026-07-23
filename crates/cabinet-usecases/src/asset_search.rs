use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_search_index::{
    AssetSearchError, AssetSearchIndex, AssetSearchPage, AssetSearchQuery,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAssetsInput {
    workspace_id: String,
    query: String,
    limit: usize,
}

impl SearchAssetsInput {
    pub fn new(workspace_id: &str, query: &str, limit: usize) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            query: query.to_string(),
            limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAssetsOutput {
    page: AssetSearchPage,
}

impl SearchAssetsOutput {
    pub fn page(&self) -> &AssetSearchPage {
        &self.page
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchAssetsUsecase;

impl SearchAssetsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: SearchAssetsInput,
        asset_search_index: &impl AssetSearchIndex,
    ) -> Result<SearchAssetsOutput, SearchAssetsError> {
        let workspace_id =
            WorkspaceId::new(&input.workspace_id).map_err(|_| SearchAssetsError::InvalidInput)?;
        let query = AssetSearchQuery::new(&input.query, input.limit)
            .map_err(SearchAssetsError::from_asset_search_error)?;
        let page = asset_search_index
            .search_assets(&workspace_id, query)
            .map_err(SearchAssetsError::from_asset_search_error)?;
        Ok(SearchAssetsOutput { page })
    }
}

impl Default for SearchAssetsUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAssetsError {
    InvalidInput,
    StorageUnavailable,
}

impl SearchAssetsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "asset_search.invalid_input",
            Self::StorageUnavailable => "asset_search.storage_unavailable",
        }
    }

    fn from_asset_search_error(error: AssetSearchError) -> Self {
        match error {
            AssetSearchError::InvalidQuery
            | AssetSearchError::InvalidLimit
            | AssetSearchError::InvalidAssetMetadata => Self::InvalidInput,
            AssetSearchError::StorageUnavailable | AssetSearchError::CorruptedIndex => {
                Self::StorageUnavailable
            }
        }
    }
}
