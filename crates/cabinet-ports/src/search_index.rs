use cabinet_domain::document::{DocumentBody, DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;

pub const MAX_SEARCH_LIMIT: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDocumentRecord {
    document_id: DocumentId,
    title: DocumentTitle,
    path: DocumentPath,
    body: DocumentBody,
}

impl SearchDocumentRecord {
    pub fn new(
        document_id: DocumentId,
        title: DocumentTitle,
        path: DocumentPath,
        body: DocumentBody,
    ) -> Self {
        Self {
            document_id,
            title,
            path,
            body,
        }
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn title(&self) -> &DocumentTitle {
        &self.title
    }

    pub fn path(&self) -> &DocumentPath {
        &self.path
    }

    pub fn body(&self) -> &DocumentBody {
        &self.body
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    text: String,
    limit: usize,
}

impl SearchQuery {
    pub fn new(text: &str, limit: usize) -> Result<Self, SearchIndexError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(SearchIndexError::InvalidQuery);
        }
        if limit == 0 || limit > MAX_SEARCH_LIMIT {
            return Err(SearchIndexError::InvalidLimit);
        }
        Ok(Self {
            text: trimmed.to_string(),
            limit,
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn limit(&self) -> usize {
        self.limit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    document_id: DocumentId,
    title: DocumentTitle,
    path: DocumentPath,
    snippet: String,
    score: u32,
}

impl SearchResult {
    pub fn new(
        document_id: DocumentId,
        title: DocumentTitle,
        path: DocumentPath,
        snippet: &str,
        score: u32,
    ) -> Result<Self, SearchIndexError> {
        let snippet = snippet.trim();
        if snippet.is_empty() {
            return Err(SearchIndexError::InvalidSnippet);
        }
        Ok(Self {
            document_id,
            title,
            path,
            snippet: snippet.to_string(),
            score,
        })
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn title(&self) -> &DocumentTitle {
        &self.title
    }

    pub fn path(&self) -> &DocumentPath {
        &self.path
    }

    pub fn snippet(&self) -> &str {
        &self.snippet
    }

    pub fn score(&self) -> u32 {
        self.score
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPage {
    results: Vec<SearchResult>,
}

impl SearchPage {
    pub fn new(results: Vec<SearchResult>) -> Self {
        Self { results }
    }

    pub fn results(&self) -> &[SearchResult] {
        &self.results
    }
}

pub trait SearchIndex {
    fn upsert_document(
        &mut self,
        workspace_id: &WorkspaceId,
        record: SearchDocumentRecord,
    ) -> Result<(), SearchIndexError>;

    fn delete_document(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<(), SearchIndexError>;

    fn search(
        &self,
        workspace_id: &WorkspaceId,
        query: SearchQuery,
    ) -> Result<SearchPage, SearchIndexError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchIndexError {
    InvalidQuery,
    InvalidLimit,
    InvalidSnippet,
    StorageUnavailable,
    CorruptedIndex,
}

impl SearchIndexError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidQuery => "search_index.invalid_query",
            Self::InvalidLimit => "search_index.invalid_limit",
            Self::InvalidSnippet => "search_index.invalid_snippet",
            Self::StorageUnavailable => "search_index.storage_unavailable",
            Self::CorruptedIndex => "search_index.corrupted_index",
        }
    }
}
