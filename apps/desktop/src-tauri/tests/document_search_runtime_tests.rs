use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_local_search_index::DurableLocalSearchIndex;
use cabinet_desktop_shell::{DesktopDocumentSearchRequestDto, DesktopDocumentSearchRuntime};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentPath, DocumentTitle,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::search_index::{SearchDocumentRecord, SearchIndex};

#[test]
fn desktop_search_runtime_finds_body_text_and_returns_a_safe_camel_case_result() {
    let temp = TempRoot::new();
    let policy = DocumentBodyPolicy::new(1024 * 1024).expect("policy");
    let mut index = DurableLocalSearchIndex::new(temp.path.clone(), policy);
    index
        .upsert_document(
            &WorkspaceId::new("workspace-1").expect("workspace"),
            SearchDocumentRecord::new(
                DocumentId::new("doc-1").expect("document"),
                DocumentTitle::new("제목에는 없음").expect("title"),
                DocumentPath::new("notes/document.md").expect("path"),
                DocumentBody::new("본문 전용 키워드가 포함되어 있습니다.", policy).expect("body"),
            ),
        )
        .expect("seed search projection");
    let runtime = DesktopDocumentSearchRuntime::new(temp.path.clone(), policy);

    let response = runtime.execute(DesktopDocumentSearchRequestDto {
        workspace_id: "workspace-1".to_string(),
        text: "본문 전용 키워드".to_string(),
        limit: 50,
    });
    let json = serde_json::to_string(&response).expect("json");

    assert!(response.ok);
    let data = response.data.expect("data");
    assert_eq!(data.query_name, "search-documents");
    assert_eq!(data.results.len(), 1);
    assert_eq!(data.results[0].document_id, "doc-1");
    assert!(data.results[0].snippet.contains("본문 전용 키워드"));
    assert!(json.contains("\"queryName\""));
    assert!(json.contains("\"documentId\""));
    assert!(!json.contains(&temp.path.display().to_string()));
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-document-search-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp root");
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
