use cabinet_ports::asset_search_index::AssetSearchIndex;
use cabinet_usecases::asset_search::{SearchAssetsError, SearchAssetsInput, SearchAssetsUsecase};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSearchCommandRequest {
    pub workspace_id: String,
    pub text: String,
    pub limit: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSearchCommandResult {
    pub workspace_id: String,
    pub text: String,
    pub results: Vec<AssetSearchCommandItem>,
    pub product_log_event_name: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSearchCommandItem {
    pub asset_id: String,
    pub file_name: String,
    pub media_type: String,
    pub byte_size: u64,
    pub score: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetSearchCommandFailure {
    pub error_code: &'static str,
    pub retryable: bool,
    pub product_log_event_name: Option<&'static str>,
}

pub fn execute_asset_search_command(
    request: AssetSearchCommandRequest,
    asset_search_index: &impl AssetSearchIndex,
) -> Result<AssetSearchCommandResult, AssetSearchCommandFailure> {
    let output = SearchAssetsUsecase::new()
        .execute(
            SearchAssetsInput::new(&request.workspace_id, &request.text, request.limit as usize),
            asset_search_index,
        )
        .map_err(map_error)?;

    Ok(AssetSearchCommandResult {
        workspace_id: request.workspace_id,
        text: request.text,
        results: output
            .page()
            .results()
            .iter()
            .map(|item| AssetSearchCommandItem {
                asset_id: item.asset_id().as_str().to_string(),
                file_name: item.file_name().as_str().to_string(),
                media_type: item.media_type().as_str().to_string(),
                byte_size: item.byte_size(),
                score: item.score(),
            })
            .collect(),
        product_log_event_name: None,
    })
}

const fn map_error(error: SearchAssetsError) -> AssetSearchCommandFailure {
    match error {
        SearchAssetsError::InvalidInput => AssetSearchCommandFailure {
            error_code: "ASSET_SEARCH_INVALID_INPUT",
            retryable: false,
            product_log_event_name: None,
        },
        SearchAssetsError::StorageUnavailable => AssetSearchCommandFailure {
            error_code: "ASSET_SEARCH_STORAGE_UNAVAILABLE",
            retryable: true,
            product_log_event_name: None,
        },
    }
}
