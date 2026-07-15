use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_workspace_home_projection::LocalWorkspaceHomeProjectionStore;
use cabinet_desktop_shell::{
    DesktopLocalCommandPayloadDto, DesktopLocalCommandRequestDto, DesktopWorkspaceHomeRuntime,
};
use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::workspace_home::{
    WorkspaceHomeBackupStatus, WorkspaceHomeDocumentProjection, WorkspaceHomeHealthStatus,
    WorkspaceHomeProjection,
};

#[test]
fn durable_workspace_home_runtime_returns_camel_case_ready_data() {
    let temp = TempRoot::new("ready");
    LocalWorkspaceHomeProjectionStore::new(temp.path.clone())
        .replace_projection(
            &WorkspaceId::new("workspace-1").expect("workspace"),
            &WorkspaceHomeProjection::new(
                vec![document("doc-1", "Source", "notes/source.md")],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                WorkspaceHomeBackupStatus::Fresh,
                WorkspaceHomeHealthStatus::Healthy,
            ),
        )
        .expect("seed projection");
    let runtime = DesktopWorkspaceHomeRuntime::new(temp.path.clone());

    let response = runtime.execute(home_request(12));
    let json = serde_json::to_string(&response).expect("serialize response");

    assert!(response.ok);
    let data = response.data.expect("success data");
    assert_eq!(data.workspace_id, "workspace-1");
    assert_eq!(data.state, "Ready");
    assert_eq!(data.recent_documents[0].document_id, "doc-1");
    assert_eq!(data.backup_status, "Fresh");
    assert!(json.contains("\"workspaceId\""));
    assert!(json.contains("\"recentDocuments\""));
    assert!(!json.contains("workspace_id"));
    assert!(!json.contains(&temp.path.display().to_string()));
    assert!(!json.contains("raw document body"));
}

#[test]
fn durable_workspace_home_runtime_returns_healthy_empty_for_missing_snapshot() {
    let temp = TempRoot::new("empty");
    let runtime = DesktopWorkspaceHomeRuntime::new(temp.path.clone());

    let response = runtime.execute(home_request(10));

    assert!(response.ok);
    let data = response.data.expect("empty data");
    assert_eq!(data.state, "Empty");
    assert!(data.recent_documents.is_empty());
    assert_eq!(data.backup_status, "NeverCreated");
    assert_eq!(data.health_status, "Healthy");
}

#[test]
fn durable_workspace_home_runtime_returns_safe_invalid_and_corrupt_failures() {
    let temp = TempRoot::new("failures");
    let store = LocalWorkspaceHomeProjectionStore::new(temp.path.clone());
    store
        .replace_projection(
            &WorkspaceId::new("workspace-1").expect("workspace"),
            &WorkspaceHomeProjection::empty(
                WorkspaceHomeBackupStatus::Fresh,
                WorkspaceHomeHealthStatus::Healthy,
            ),
        )
        .expect("seed projection");
    let snapshot = fs::read_dir(temp.path.join("home-projections"))
        .expect("snapshot dir")
        .next()
        .expect("snapshot entry")
        .expect("snapshot path")
        .path();
    fs::write(snapshot, "schema\t999\nprivate-body\n").expect("corrupt snapshot");
    let runtime = DesktopWorkspaceHomeRuntime::new(temp.path.clone());

    let invalid = runtime.execute(home_request(0));
    let corrupt = runtime.execute(home_request(10));
    let debug = format!("{corrupt:?}");

    assert!(!invalid.ok);
    assert_eq!(invalid.error_code.as_deref(), Some("COMMAND_INVALID_INPUT"));
    assert!(!invalid.retryable);
    assert!(!corrupt.ok);
    assert_eq!(
        corrupt.error_code.as_deref(),
        Some("WORKSPACE_HOME_PROJECTION_UNAVAILABLE")
    );
    assert!(corrupt.retryable);
    assert!(!debug.contains("private-body"));
    assert!(!debug.contains(&temp.path.display().to_string()));
}

fn home_request(limit: u16) -> DesktopLocalCommandRequestDto {
    DesktopLocalCommandRequestDto {
        command_name: "local_workspace_home".to_string(),
        payload: DesktopLocalCommandPayloadDto::WorkspaceHome {
            workspace_id: "workspace-1".to_string(),
            recent_documents: limit,
            favorites: 8,
            tags: 10,
            recent_changes: 14,
            unfinished_items: 6,
        },
    }
}

fn document(id: &str, title: &str, path: &str) -> WorkspaceHomeDocumentProjection {
    WorkspaceHomeDocumentProjection::new(
        DocumentId::new(id).expect("id"),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
    )
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
            "sponzey-cabinet-phase011-workspace-home-{label}-{}-{nonce}",
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
