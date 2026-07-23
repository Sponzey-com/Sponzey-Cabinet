use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_metadata_catalog::DurableAssetMetadataCatalog;
use cabinet_adapters::durable_canvas_repository::DurableCanvasRepository;
use cabinet_adapters::local_create_document_revision_runtime::LocalCreateDocumentRevisionRuntime;
use cabinet_adapters::local_workspace_home_projection::LocalWorkspaceHomeProjectionStore;
use cabinet_desktop_shell::{
    DesktopLocalCommandPayloadDto, DesktopLocalCommandRequestDto, DesktopWorkspaceHomeRuntime,
};
use cabinet_domain::asset::{
    AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType,
    AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::canvas::{Canvas, CanvasId, CanvasLifecycleState};
use cabinet_domain::document::{DocumentBodyPolicy, DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_metadata_catalog::AssetMetadataCatalog;
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository};
use cabinet_ports::workspace_home::{
    WorkspaceHomeBackupStatus, WorkspaceHomeDocumentProjection, WorkspaceHomeHealthStatus,
    WorkspaceHomeProjection,
};
use cabinet_usecases::create_document_revision::CreateDocumentRevisionInput;

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
fn durable_workspace_home_runtime_counts_actual_current_stores_and_excludes_archived_canvas() {
    let temp = TempRoot::new("actual-summary");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let other_workspace = WorkspaceId::new("workspace-2").expect("other workspace");
    LocalCreateDocumentRevisionRuntime::new(
        temp.path.clone(),
        DocumentBodyPolicy::new(1024).expect("body policy"),
    )
    .execute(CreateDocumentRevisionInput::new(
        "operation-1",
        workspace.as_str(),
        "doc-1",
        "# Source",
        "local-user",
        "Create document",
    ))
    .expect("create document");

    let mut assets = DurableAssetMetadataCatalog::new(temp.path.clone());
    assets.put(&workspace, asset_record('a')).expect("asset a");
    assets.put(&workspace, asset_record('b')).expect("asset b");
    assets
        .put(&other_workspace, asset_record('c'))
        .expect("other asset");

    let mut canvases = DurableCanvasRepository::new(temp.path.clone());
    canvases
        .create_canvas(
            &workspace,
            canvas_record("active", CanvasLifecycleState::Saved),
        )
        .expect("active canvas");
    canvases
        .create_canvas(
            &workspace,
            canvas_record("archived", CanvasLifecycleState::Archived),
        )
        .expect("archived canvas");
    canvases
        .create_canvas(
            &other_workspace,
            canvas_record("other", CanvasLifecycleState::Saved),
        )
        .expect("other canvas");

    let data = DesktopWorkspaceHomeRuntime::new(temp.path.clone())
        .execute(home_request(10))
        .data
        .expect("home data");
    assert_eq!(data.document_count, 1);
    assert_eq!(data.asset_count, 2);
    assert_eq!(data.canvas_count, 1);

    for workspace_entry in
        fs::read_dir(temp.path.join("assets/metadata")).expect("asset workspaces")
    {
        for asset_entry in
            fs::read_dir(workspace_entry.expect("workspace entry").path()).expect("asset files")
        {
            fs::write(
                asset_entry.expect("asset entry").path(),
                "schema\t999\nprivate-path",
            )
            .expect("corrupt asset catalog");
        }
    }

    let degraded = DesktopWorkspaceHomeRuntime::new(temp.path.clone()).execute(home_request(10));
    assert!(degraded.ok);
    let degraded = degraded.data.expect("degraded home data");
    assert_eq!(degraded.state, "Degraded");
    assert_eq!(degraded.document_count, 1);
    assert_eq!(degraded.asset_count, 0);
    assert_eq!(degraded.canvas_count, 1);
    assert_eq!(degraded.summary_unavailable, vec!["Assets"]);
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

fn asset_record(fill: char) -> AssetCatalogRecord {
    AssetCatalogRecord::new(
        AssetMetadata::new(
            AssetId::from_sha256_hex(&fill.to_string().repeat(64)).expect("asset id"),
            AssetFileName::new(&format!("{fill}.pdf")).expect("filename"),
            AssetMediaType::new("application/pdf").expect("media type"),
            10,
        )
        .expect("metadata"),
        1,
        AssetPreviewCapability::Pdf,
        AssetExtractionStatus::NotRequested,
    )
    .expect("record")
}

fn canvas_record(id: &str, state: CanvasLifecycleState) -> CanvasRecord {
    CanvasRecord::new(
        Canvas::new(
            CanvasId::new(id).expect("canvas id"),
            Vec::new(),
            Vec::new(),
            state,
        )
        .expect("canvas"),
    )
    .expect("canvas record")
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
