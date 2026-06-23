use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::search_index::{SearchIndex, SearchIndexError, SearchPage, SearchQuery};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDocumentsInput {
    workspace_id: String,
    query: String,
    limit: usize,
}

impl SearchDocumentsInput {
    pub fn new(workspace_id: &str, query: &str, limit: usize) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            query: query.to_string(),
            limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDocumentsOutput {
    page: SearchPage,
}

impl SearchDocumentsOutput {
    pub fn page(&self) -> &SearchPage {
        &self.page
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchDocumentsUsecase;

impl SearchDocumentsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: SearchDocumentsInput,
        search_index: &impl SearchIndex,
    ) -> Result<SearchDocumentsOutput, SearchDocumentsError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| SearchDocumentsError::InvalidInput)?;
        let query = SearchQuery::new(&input.query, input.limit)
            .map_err(SearchDocumentsError::from_search_index_error)?;
        let page = search_index
            .search(&workspace_id, query)
            .map_err(SearchDocumentsError::from_search_index_error)?;
        Ok(SearchDocumentsOutput { page })
    }
}

impl Default for SearchDocumentsUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDocumentsError {
    InvalidInput,
    StorageUnavailable,
}

impl SearchDocumentsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "search.invalid_input",
            Self::StorageUnavailable => "search.storage_unavailable",
        }
    }

    fn from_search_index_error(error: SearchIndexError) -> Self {
        match error {
            SearchIndexError::InvalidQuery
            | SearchIndexError::InvalidLimit
            | SearchIndexError::InvalidSnippet => Self::InvalidInput,
            SearchIndexError::StorageUnavailable | SearchIndexError::CorruptedIndex => {
                Self::StorageUnavailable
            }
        }
    }
}
