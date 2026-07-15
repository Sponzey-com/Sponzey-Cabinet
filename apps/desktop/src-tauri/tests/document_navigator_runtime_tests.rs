use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_document_navigator_projection::LocalDocumentNavigatorProjectionStore;
use cabinet_desktop_shell::{DesktopDocumentNavigatorRequestDto, DesktopDocumentNavigatorRuntime};
use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_navigator::DocumentNavigatorItem;

#[test]
fn durable_navigator_runtime_returns_camel_case_ready_page_after_restart() {
    let temp = TempRoot::new("ready");
    let store = LocalDocumentNavigatorProjectionStore::new(temp.path.clone(), 100).expect("store");
    store
        .replace_workspace_items(&workspace(), vec![item()])
        .expect("seed projection");
    let runtime = DesktopDocumentNavigatorRuntime::new(temp.path.clone(), 100).expect("runtime");

    let response = runtime.execute(request("Collection", Some("work"), Some("arch"), 20));
    let json = serde_json::to_string(&response).expect("json");

    assert!(response.ok);
    let data = response.data.expect("data");
    assert_eq!(data.workspace_id, "workspace-1");
    assert_eq!(data.view, "Collection");
    assert_eq!(data.state, "Ready");
    assert_eq!(data.items[0].document_id, "doc-1");
    assert!(json.contains("\"workspaceId\""));
    assert!(json.contains("\"nextCursor\""));
    assert!(!json.contains("workspace_id"));
    assert!(!json.contains(&temp.path.display().to_string()));
    assert!(!json.contains("raw document body"));
}

#[test]
fn durable_navigator_runtime_returns_empty_and_safe_invalid_responses() {
    let temp = TempRoot::new("empty-invalid");
    let runtime = DesktopDocumentNavigatorRuntime::new(temp.path.clone(), 100).expect("runtime");

    let empty = runtime.execute(request("Tree", None, None, 20));
    let invalid_view = runtime.execute(request("Unknown", None, None, 20));
    let invalid_limit = runtime.execute(request("Tree", None, None, 0));

    assert!(empty.ok);
    assert_eq!(empty.data.expect("empty data").state, "EmptyResult");
    assert!(!invalid_view.ok);
    assert_eq!(
        invalid_view.error_code.as_deref(),
        Some("DOCUMENT_NAVIGATOR_INVALID_INPUT")
    );
    assert!(!invalid_limit.ok);
    assert!(!invalid_limit.retryable);
}

#[test]
fn durable_navigator_runtime_returns_retryable_sanitized_corruption_failure() {
    let temp = TempRoot::new("corrupt");
    let store = LocalDocumentNavigatorProjectionStore::new(temp.path.clone(), 100).expect("store");
    store
        .replace_workspace_items(&workspace(), vec![item()])
        .expect("seed");
    let snapshot = fs::read_dir(temp.path.join("navigator-projections"))
        .expect("directory")
        .next()
        .expect("entry")
        .expect("path")
        .path();
    fs::write(snapshot, "schema\t999\nprivate-body\n").expect("corrupt");
    let runtime = DesktopDocumentNavigatorRuntime::new(temp.path.clone(), 100).expect("runtime");

    let response = runtime.execute(request("Recent", None, None, 20));
    let debug = format!("{response:?}");

    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE")
    );
    assert!(response.retryable);
    assert!(!debug.contains("private-body"));
    assert!(!debug.contains(&temp.path.display().to_string()));
}

fn request(
    view: &str,
    view_key: Option<&str>,
    filter: Option<&str>,
    limit: u16,
) -> DesktopDocumentNavigatorRequestDto {
    DesktopDocumentNavigatorRequestDto {
        workspace_id: "workspace-1".to_string(),
        view: view.to_string(),
        view_key: view_key.map(str::to_string),
        filter: filter.map(str::to_string),
        limit,
        cursor: None,
    }
}

fn item() -> DocumentNavigatorItem {
    DocumentNavigatorItem::new(
        DocumentId::new("doc-1").expect("id"),
        DocumentTitle::new("Architecture").expect("title"),
        DocumentPath::new("notes/architecture.md").expect("path"),
        vec!["work".to_string()],
        vec!["rust".to_string()],
        true,
        1,
    )
    .expect("item")
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace")
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-phase011-document-navigator-{label}-{}-{nonce}",
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
