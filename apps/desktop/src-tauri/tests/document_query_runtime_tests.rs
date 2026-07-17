use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_version_store::VERSION_ENTRY_FILE;
use cabinet_desktop_shell::{
    DesktopDocumentMutationRequestDto, DesktopDocumentMutationRuntime,
    DesktopDocumentQueryRequestDto, DesktopDocumentQueryRuntime,
};

#[test]
fn authoritative_query_reads_current_and_version_without_storage_metadata() {
    let temp = TempRoot::new("current-version");
    let mutation = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).unwrap();
    let query = DesktopDocumentQueryRuntime::new(temp.path.clone(), 4096).unwrap();

    let created = mutation.execute(create_request("operation-1", "# 첫 제목\n본문 1"));
    let first_version = created.data.unwrap().current_version_id;
    let updated = mutation.execute(update_request(
        "operation-2",
        &first_version,
        "두 번째 제목\n본문 2",
    ));
    let current_version = updated.data.unwrap().current_version_id;

    let current = query.execute(DesktopDocumentQueryRequestDto::Current {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
    });
    assert!(current.ok);
    let current_data = current.data.unwrap();
    assert_eq!(current_data.kind, "current");
    assert_eq!(
        current_data.current_version_token.as_deref(),
        Some(current_version.as_str())
    );
    assert_eq!(current_data.revision_number, Some(2));
    assert_eq!(current_data.title.as_deref(), Some("두 번째 제목"));
    assert_eq!(current_data.body.as_deref(), Some("두 번째 제목\n본문 2"));

    let version = query.execute(DesktopDocumentQueryRequestDto::Version {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        version_token: first_version,
    });
    assert!(version.ok);
    let version_data = version.data.unwrap();
    assert_eq!(version_data.kind, "version");
    assert_eq!(version_data.revision_number, Some(1));
    assert_eq!(version_data.title.as_deref(), Some("첫 제목"));
    assert_eq!(version_data.body.as_deref(), Some("# 첫 제목\n본문 1"));

    let json = serde_json::to_value(version_data).unwrap();
    let object = json.as_object().unwrap();
    assert!(!object.contains_key("path"));
    assert!(!object.contains_key("snapshotRef"));
}

#[test]
fn authoritative_history_supports_bounded_cursor_pagination() {
    let temp = TempRoot::new("history");
    let mutation = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).unwrap();
    let query = DesktopDocumentQueryRuntime::new(temp.path.clone(), 4096).unwrap();

    let first = mutation.execute(create_request("operation-1", "제목 1\n본문"));
    let first_version = first.data.unwrap().current_version_id;
    let second = mutation.execute(update_request(
        "operation-2",
        &first_version,
        "제목 2\n본문",
    ));
    let second_version = second.data.unwrap().current_version_id;
    let third = mutation.execute(update_request(
        "operation-3",
        &second_version,
        "제목 3\n본문",
    ));
    assert!(third.ok);

    let first_page = query.execute(DesktopDocumentQueryRequestDto::History {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        cursor: None,
        limit: 2,
    });
    assert!(first_page.ok);
    let first_data = first_page.data.unwrap();
    assert_eq!(first_data.entries.len(), 2);
    assert_eq!(first_data.entries[0].revision_number, 1);
    assert_eq!(first_data.entries[1].revision_number, 2);
    assert!(first_data.has_more);
    let cursor = first_data.next_cursor.expect("opaque next cursor");

    let second_page = query.execute(DesktopDocumentQueryRequestDto::History {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        cursor: Some(cursor),
        limit: 2,
    });
    assert!(second_page.ok);
    let second_data = second_page.data.unwrap();
    assert_eq!(second_data.entries.len(), 1);
    assert_eq!(second_data.entries[0].revision_number, 3);
    assert!(!second_data.has_more);
    assert!(second_data.next_cursor.is_none());

    let invalid_limit = query.execute(DesktopDocumentQueryRequestDto::History {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        cursor: None,
        limit: 0,
    });
    assert_eq!(
        invalid_limit.error_code.as_deref(),
        Some("DOCUMENT_QUERY_INVALID_INPUT")
    );
    assert!(!invalid_limit.retryable);
}

#[test]
fn query_maps_missing_and_corrupt_authoritative_records_without_partial_data() {
    let temp = TempRoot::new("errors");
    let mutation = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).unwrap();
    let query = DesktopDocumentQueryRuntime::new(temp.path.clone(), 4096).unwrap();
    let created = mutation.execute(create_request("operation-1", "제목\n본문"));
    assert!(created.ok);
    let created_version = created.data.unwrap().current_version_id;

    let missing = query.execute(DesktopDocumentQueryRequestDto::Version {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        version_token: "missing-version".into(),
    });
    assert!(!missing.ok);
    assert!(missing.data.is_none());
    assert_eq!(
        missing.error_code.as_deref(),
        Some("DOCUMENT_QUERY_NOT_FOUND")
    );

    let wrong_document = query.execute(DesktopDocumentQueryRequestDto::Version {
        workspace_id: "workspace-1".into(),
        document_id: "doc-2".into(),
        version_token: created_version.clone(),
    });
    assert_eq!(
        wrong_document.error_code.as_deref(),
        Some("DOCUMENT_QUERY_NOT_FOUND")
    );

    let corrupted_count =
        corrupt_named_files(&temp.path.join("document-versions"), VERSION_ENTRY_FILE);
    assert!(corrupted_count > 0);
    let corrupt = query.execute(DesktopDocumentQueryRequestDto::Current {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
    });
    assert!(!corrupt.ok);
    assert!(corrupt.data.is_none());
    assert_eq!(
        corrupt.error_code.as_deref(),
        Some("DOCUMENT_QUERY_CORRUPTED_DATA")
    );
    assert!(!corrupt.retryable);
    assert!(corrupt.repair_required);
}

#[test]
fn tauri_main_registers_authoritative_document_query_command() {
    let source = include_str!("../src/main.rs");
    assert!(source.contains("execute_desktop_document_query"));
    assert!(source.contains("DesktopDocumentQueryRuntime"));
}

fn create_request(operation_id: &str, body: &str) -> DesktopDocumentMutationRequestDto {
    DesktopDocumentMutationRequestDto::Create {
        operation_id: operation_id.into(),
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        body: body.into(),
        author: "local-user".into(),
        summary: "Create".into(),
    }
}

fn update_request(
    operation_id: &str,
    expected_current_version_id: &str,
    body: &str,
) -> DesktopDocumentMutationRequestDto {
    DesktopDocumentMutationRequestDto::Update {
        operation_id: operation_id.into(),
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        expected_current_version_id: expected_current_version_id.into(),
        body: body.into(),
        author: "local-user".into(),
        summary: "Update".into(),
    }
}

fn corrupt_named_files(root: &Path, file_name: &str) -> usize {
    let mut count = 0;
    for entry in fs::read_dir(root).expect("version directory") {
        let path = entry.expect("version entry").path();
        if path.is_dir() {
            count += corrupt_named_files(&path, file_name);
        } else if path.file_name().and_then(|value| value.to_str()) == Some(file_name) {
            fs::write(path, "corrupted\n").expect("corrupt entry fixture");
            count += 1;
        }
    }
    count
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cabinet-desktop-query-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
