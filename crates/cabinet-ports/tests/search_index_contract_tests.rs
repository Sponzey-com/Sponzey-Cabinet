use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentPath, DocumentTitle,
};
use cabinet_ports::search_index::{
    SearchDocumentRecord, SearchIndexError, SearchQuery, SearchResult,
};

#[test]
fn search_query_rejects_empty_query_or_invalid_limit() {
    assert_eq!(
        SearchQuery::new("  ", 10).expect_err("empty query must fail"),
        SearchIndexError::InvalidQuery
    );
    assert_eq!(
        SearchQuery::new("cabinet", 0).expect_err("zero limit must fail"),
        SearchIndexError::InvalidLimit
    );
    assert_eq!(
        SearchQuery::new("cabinet", 101).expect_err("too large limit must fail"),
        SearchIndexError::InvalidLimit
    );
}

#[test]
fn search_document_record_exposes_metadata_and_content_for_indexing() {
    let record = SearchDocumentRecord::new(
        document_id("doc-1"),
        title("Architecture"),
        path("docs/architecture.md"),
        body("Knowledge base architecture"),
    );

    assert_eq!(record.document_id().as_str(), "doc-1");
    assert_eq!(record.title().as_str(), "Architecture");
    assert_eq!(record.path().as_str(), "docs/architecture.md");
    assert_eq!(record.body().as_str(), "Knowledge base architecture");
}

#[test]
fn search_result_rejects_empty_snippet() {
    let error = SearchResult::new(
        document_id("doc-1"),
        title("Architecture"),
        path("docs/architecture.md"),
        "  ",
        1,
    )
    .expect_err("empty snippet must fail");

    assert_eq!(error, SearchIndexError::InvalidSnippet);
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}

fn title(value: &str) -> DocumentTitle {
    DocumentTitle::new(value).expect("title")
}

fn path(value: &str) -> DocumentPath {
    DocumentPath::new(value).expect("path")
}

fn body(value: &str) -> DocumentBody {
    DocumentBody::new(value, DocumentBodyPolicy::new(1024).expect("policy")).expect("body")
}
