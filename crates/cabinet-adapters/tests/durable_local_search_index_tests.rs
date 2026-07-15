use cabinet_adapters::durable_local_search_index::DurableLocalSearchIndex;
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentPath, DocumentTitle,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::search_index::{
    SearchDocumentRecord, SearchIndex, SearchIndexError, SearchQuery,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn durable_search_indexes_title_path_and_body_after_restart() {
    let temp = Temp::new("restart");
    let workspace = workspace("workspace-1");
    let mut index = build(&temp);
    index
        .upsert_document(
            &workspace,
            record(
                "doc-1",
                "Architecture Cabinet",
                "notes/architecture.md",
                "exact body needle",
            ),
        )
        .unwrap();
    drop(index);

    let restarted = build(&temp);
    for query in ["cabinet", "architecture.md", "needle"] {
        let page = restarted
            .search(&workspace, SearchQuery::new(query, 10).unwrap())
            .unwrap();
        assert_eq!(page.results().len(), 1);
        assert_eq!(page.results()[0].document_id().as_str(), "doc-1");
        assert!(!page.results()[0].snippet().is_empty());
    }
}

#[test]
fn durable_search_replaces_deletes_and_isolates_workspaces() {
    let temp = Temp::new("mutation");
    let workspace_id = workspace("workspace-1");
    let other = workspace("workspace-2");
    let mut index = build(&temp);
    index
        .upsert_document(&workspace_id, record("doc-1", "Old", "old.md", "old term"))
        .unwrap();
    index
        .upsert_document(
            &workspace_id,
            record("doc-1", "New", "new.md", "replacement term"),
        )
        .unwrap();
    index
        .upsert_document(&other, record("doc-2", "Other", "other.md", "old term"))
        .unwrap();
    assert!(
        index
            .search(&workspace_id, SearchQuery::new("old", 10).unwrap())
            .unwrap()
            .results()
            .is_empty()
    );
    assert_eq!(
        index
            .search(&workspace_id, SearchQuery::new("replacement", 10).unwrap())
            .unwrap()
            .results()
            .len(),
        1
    );
    index
        .delete_document(&workspace_id, &DocumentId::new("doc-1").unwrap())
        .unwrap();
    assert!(
        index
            .search(&workspace_id, SearchQuery::new("replacement", 10).unwrap())
            .unwrap()
            .results()
            .is_empty()
    );
    assert_eq!(
        index
            .search(&other, SearchQuery::new("old", 10).unwrap())
            .unwrap()
            .results()
            .len(),
        1
    );
}

#[test]
fn durable_search_rejects_unknown_schema_and_checksum_corruption() {
    let temp = Temp::new("corruption");
    let workspace = workspace("workspace-1");
    let mut index = build(&temp);
    index
        .upsert_document(
            &workspace,
            record("doc-1", "Private", "private.md", "secret"),
        )
        .unwrap();
    let path = fs::read_dir(temp.path.join("search-projections"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(&path, "schema\t999\nprivate title and body").unwrap();
    let error = index
        .search(&workspace, SearchQuery::new("private", 10).unwrap())
        .expect_err("schema must fail");
    assert_eq!(error, SearchIndexError::CorruptedIndex);
    assert!(!format!("{error:?}").contains("private title"));

    fs::write(&path, "schema\t1\nchecksum\t0000000000000000\nrecord\t00\n").unwrap();
    assert_eq!(
        index
            .search(&workspace, SearchQuery::new("private", 10).unwrap())
            .expect_err("checksum must fail"),
        SearchIndexError::CorruptedIndex
    );
}

fn build(temp: &Temp) -> DurableLocalSearchIndex {
    DurableLocalSearchIndex::new(temp.path.clone(), DocumentBodyPolicy::new(4096).unwrap())
}

fn record(id: &str, title: &str, path: &str, body: &str) -> SearchDocumentRecord {
    SearchDocumentRecord::new(
        DocumentId::new(id).unwrap(),
        DocumentTitle::new(title).unwrap(),
        DocumentPath::new(path).unwrap(),
        DocumentBody::new(body, DocumentBodyPolicy::new(4096).unwrap()).unwrap(),
    )
}

fn workspace(value: &str) -> WorkspaceId {
    WorkspaceId::new(value).unwrap()
}

struct Temp {
    path: PathBuf,
}

impl Temp {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cabinet-durable-search-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
