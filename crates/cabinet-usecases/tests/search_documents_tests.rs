use std::cell::Cell;

use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::search_index::{
    SearchDocumentRecord, SearchIndex, SearchIndexError, SearchPage, SearchQuery, SearchResult,
};
use cabinet_usecases::search::{
    SearchDocumentsError, SearchDocumentsInput, SearchDocumentsUsecase,
};

#[derive(Default)]
struct FakeSearchIndex {
    fail_search: bool,
    search_count: Cell<usize>,
}

impl SearchIndex for FakeSearchIndex {
    fn upsert_document(
        &mut self,
        _workspace_id: &WorkspaceId,
        _record: SearchDocumentRecord,
    ) -> Result<(), SearchIndexError> {
        Ok(())
    }

    fn delete_document(
        &mut self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<(), SearchIndexError> {
        Ok(())
    }

    fn search(
        &self,
        _workspace_id: &WorkspaceId,
        query: SearchQuery,
    ) -> Result<SearchPage, SearchIndexError> {
        self.search_count.set(self.search_count.get() + 1);
        if self.fail_search {
            return Err(SearchIndexError::StorageUnavailable);
        }
        Ok(SearchPage::new(vec![search_result(
            "doc-1",
            "Architecture",
            "docs/architecture.md",
            query.text(),
            query.limit() as u32,
        )]))
    }
}

#[test]
fn search_documents_delegates_query_to_search_index() {
    let search_index = FakeSearchIndex::default();
    let usecase = SearchDocumentsUsecase::new();

    let output = usecase
        .execute(
            SearchDocumentsInput::new("workspace-1", "cabinet", 10),
            &search_index,
        )
        .expect("search");

    assert_eq!(output.page().results().len(), 1);
    assert_eq!(output.page().results()[0].document_id().as_str(), "doc-1");
    assert_eq!(search_index.search_count.get(), 1);
}

#[test]
fn search_documents_rejects_invalid_query_without_calling_search_index() {
    let search_index = FakeSearchIndex::default();
    let usecase = SearchDocumentsUsecase::new();

    let error = usecase
        .execute(
            SearchDocumentsInput::new("workspace-1", "  ", 10),
            &search_index,
        )
        .expect_err("invalid query must fail");

    assert_eq!(error, SearchDocumentsError::InvalidInput);
    assert_eq!(search_index.search_count.get(), 0);
}

#[test]
fn search_documents_maps_search_index_failure_to_storage_unavailable() {
    let search_index = FakeSearchIndex {
        fail_search: true,
        ..FakeSearchIndex::default()
    };
    let usecase = SearchDocumentsUsecase::new();

    let error = usecase
        .execute(
            SearchDocumentsInput::new("workspace-1", "cabinet", 10),
            &search_index,
        )
        .expect_err("search failure must fail");

    assert_eq!(error, SearchDocumentsError::StorageUnavailable);
    assert_eq!(search_index.search_count.get(), 1);
}

fn search_result(id: &str, title: &str, path: &str, snippet: &str, score: u32) -> SearchResult {
    SearchResult::new(
        document_id(id),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
        snippet,
        score,
    )
    .expect("result")
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}
