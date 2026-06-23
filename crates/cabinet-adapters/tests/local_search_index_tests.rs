use cabinet_adapters::local_search_index::LocalSearchIndex;
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentPath, DocumentTitle,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::search_index::{SearchDocumentRecord, SearchIndex, SearchQuery};

#[test]
fn local_search_index_returns_matching_documents_with_snippets() {
    let workspace = workspace_id();
    let mut index = LocalSearchIndex::default();
    index
        .upsert_document(
            &workspace,
            record(
                "doc-1",
                "Architecture",
                "docs/architecture.md",
                "Sponzey Cabinet knowledge base architecture",
            ),
        )
        .expect("upsert");

    let page = index
        .search(&workspace, SearchQuery::new("cabinet", 10).expect("query"))
        .expect("search");

    assert_eq!(page.results().len(), 1);
    assert_eq!(page.results()[0].document_id().as_str(), "doc-1");
    assert!(page.results()[0].snippet().contains("Cabinet"));
}

#[test]
fn local_search_index_respects_limit_and_orders_higher_score_first() {
    let workspace = workspace_id();
    let mut index = LocalSearchIndex::default();
    index
        .upsert_document(
            &workspace,
            record("doc-1", "Cabinet", "docs/one.md", "cabinet cabinet search"),
        )
        .expect("upsert");
    index
        .upsert_document(
            &workspace,
            record("doc-2", "Notes", "docs/two.md", "cabinet"),
        )
        .expect("upsert");

    let page = index
        .search(&workspace, SearchQuery::new("cabinet", 1).expect("query"))
        .expect("search");

    assert_eq!(page.results().len(), 1);
    assert_eq!(page.results()[0].document_id().as_str(), "doc-1");
    assert!(page.results()[0].score() > 1);
}

#[test]
fn local_search_index_delete_removes_document_from_results() {
    let workspace = workspace_id();
    let mut index = LocalSearchIndex::default();
    index
        .upsert_document(
            &workspace,
            record("doc-1", "Cabinet", "docs/one.md", "cabinet body"),
        )
        .expect("upsert");

    index
        .delete_document(&workspace, &document_id("doc-1"))
        .expect("delete");
    let page = index
        .search(&workspace, SearchQuery::new("cabinet", 10).expect("query"))
        .expect("search");

    assert!(page.results().is_empty());
}

fn workspace_id() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace id")
}

fn record(id: &str, title: &str, path: &str, body: &str) -> SearchDocumentRecord {
    SearchDocumentRecord::new(
        document_id(id),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
    )
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}
