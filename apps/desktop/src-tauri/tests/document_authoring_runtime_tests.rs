use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_document_link_catalog::DurableDocumentLinkCatalog;
use cabinet_adapters::durable_projection_work_repository::DurableProjectionWorkRepository;
use cabinet_adapters::local_workspace_home_projection::LocalWorkspaceHomeProjectionStore;
use cabinet_desktop_shell::{DesktopDocumentAuthoringRequestDto, DesktopDocumentAuthoringRuntime};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::link_target_resolver::{DocumentLinkTargetResolver, LinkTargetResolution};
use cabinet_ports::projection_work::ProjectionWorkRepository;
use cabinet_ports::workspace_home::{WorkspaceHomeProjectionLimits, WorkspaceHomeProjectionPort};

#[test]
fn durable_authoring_runtime_creates_reopens_updates_and_survives_restart() {
    let temp = TempRoot::new("restart");
    let runtime = build_runtime(&temp);

    let created = runtime.execute(create_request());
    let current = runtime.execute(get_request());
    let updated = runtime.execute(update_request("v1", "v2"));
    assert!(created.ok);
    assert_eq!(created.data.expect("created").current_version_id, "v1");
    assert_eq!(
        current.data.expect("current").body.as_deref(),
        Some("# Source\nbody one")
    );
    assert_eq!(updated.data.expect("updated").current_version_id, "v2");
    assert_eq!(runtime.product_event_count(), 2);
    drop(runtime);

    let restarted = build_runtime(&temp);
    let reopened = restarted.execute(get_request());
    let data = reopened.data.expect("reopened");
    assert_eq!(data.kind, "current");
    assert_eq!(data.current_version_id, "v2");
    assert_eq!(data.body.as_deref(), Some("body two"));
}

#[test]
fn durable_authoring_runtime_renames_metadata_and_preserves_identity_body_and_version() {
    let temp = TempRoot::new("rename");
    let runtime = build_runtime(&temp);
    assert!(runtime.execute(create_request()).ok);
    let renamed = runtime.execute(DesktopDocumentAuthoringRequestDto::Rename {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        current_version_id: "v1".to_string(),
        title: "새 제목".to_string(),
        path: "notes/source.md".to_string(),
    });
    assert!(renamed.ok);
    let current = runtime.execute(get_request()).data.expect("current");
    assert_eq!(current.document_id, "doc-1");
    assert_eq!(current.current_version_id, "v1");
    assert_eq!(current.title.as_deref(), Some("새 제목"));
    assert_eq!(current.body.as_deref(), Some("# Source\nbody one"));
}

#[test]
fn durable_authoring_runtime_rejects_stale_or_empty_rename_without_mutation() {
    let temp = TempRoot::new("rename-guard");
    let runtime = build_runtime(&temp);
    assert!(runtime.execute(create_request()).ok);

    let stale = runtime.execute(DesktopDocumentAuthoringRequestDto::Rename {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        current_version_id: "stale".to_string(),
        title: "잘못된 제목".to_string(),
        path: "notes/source.md".to_string(),
    });
    let empty = runtime.execute(DesktopDocumentAuthoringRequestDto::Rename {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        current_version_id: "v1".to_string(),
        title: "   ".to_string(),
        path: "notes/source.md".to_string(),
    });

    assert!(!stale.ok);
    assert_eq!(
        stale.error_code.as_deref(),
        Some("DOCUMENT_AUTHORING_VERSION_CONFLICT")
    );
    assert!(!empty.ok);
    assert_eq!(
        empty.error_code.as_deref(),
        Some("DOCUMENT_AUTHORING_INVALID_INPUT")
    );
    let current = runtime.execute(get_request()).data.expect("current");
    assert_eq!(current.title.as_deref(), Some("Source"));
    assert_eq!(current.current_version_id, "v1");
}

#[test]
fn durable_authoring_runtime_returns_persisted_history_creation_time() {
    let temp = TempRoot::new("history-created-at");
    let runtime = build_runtime(&temp);
    assert!(runtime.execute(create_request()).ok);

    let history = runtime.execute(DesktopDocumentAuthoringRequestDto::GetHistory {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        limit: 10,
    });
    let created_at = &history.data.expect("history").entries[0].created_at;

    assert_ne!(created_at, "local-version");
    assert!(created_at.parse::<u64>().expect("epoch milliseconds") > 0);
}

#[test]
fn durable_authoring_runtime_returns_camel_case_safe_conflict_response() {
    let temp = TempRoot::new("conflict");
    let runtime = build_runtime(&temp);
    runtime.execute(create_request());

    let response = runtime.execute(update_request("stale", "v2"));
    let json = serde_json::to_string(&response).expect("json");
    let debug = format!("{response:?}");

    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("DOCUMENT_AUTHORING_VERSION_CONFLICT")
    );
    assert!(!response.retryable);
    assert!(!response.repair_required);
    assert!(json.contains("\"errorCode\""));
    assert!(json.contains("\"repairRequired\""));
    assert!(!json.contains("error_code"));
    assert!(!debug.contains("body two"));
    assert!(!debug.contains("notes/source.md"));
    assert!(!debug.contains(&temp.path.display().to_string()));

    let current = runtime.execute(get_request()).data.expect("current");
    assert_eq!(current.current_version_id, "v1");
    assert_eq!(current.body.as_deref(), Some("# Source\nbody one"));
}

#[test]
fn durable_authoring_runtime_validates_startup_policy_and_redacts_current_debug() {
    let temp = TempRoot::new("policy-debug");
    assert_eq!(
        DesktopDocumentAuthoringRuntime::new(temp.path.clone(), 0)
            .err()
            .expect("invalid policy"),
        "DOCUMENT_AUTHORING_INVALID_BODY_POLICY"
    );
    let runtime = build_runtime(&temp);
    runtime.execute(create_request());

    let current = runtime.execute(get_request());
    let json = serde_json::to_string(&current).expect("json");
    let debug = format!("{current:?}");

    assert!(json.contains("\"currentVersionId\":\"v1\""));
    assert!(json.contains("\"body\":\"# Source\\nbody one\""));
    assert!(!debug.contains("# Source"));
    assert!(!debug.contains("notes/source.md"));
}

#[test]
fn durable_authoring_runtime_lists_history_previews_and_blocks_stale_restore() {
    let temp = TempRoot::new("history-restore");
    let runtime = build_runtime(&temp);
    runtime.execute(create_request());
    runtime.execute(update_request("v1", "v2"));

    let history = runtime.execute(DesktopDocumentAuthoringRequestDto::GetHistory {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        limit: 20,
    });
    let version = runtime.execute(DesktopDocumentAuthoringRequestDto::GetVersion {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        version_id: "v1".to_string(),
    });
    let preview = runtime.execute(DesktopDocumentAuthoringRequestDto::PreviewRestore {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        target_version_id: "v1".to_string(),
        expected_current_version_id: "v2".to_string(),
    });

    assert_eq!(history.data.expect("history").entries.len(), 2);
    assert_eq!(
        version.data.expect("version").body.as_deref(),
        Some("# Source\nbody one")
    );
    let preview_data = preview.data.expect("preview");
    assert_eq!(preview_data.target_version_id.as_deref(), Some("v1"));
    assert_eq!(
        preview_data.expected_current_version_id.as_deref(),
        Some("v2")
    );
    assert_eq!(preview_data.can_restore, Some(true));
    assert!(!preview_data.lines.is_empty());

    runtime.execute(update_request("v2", "v3"));
    let stale_restore = runtime.execute(DesktopDocumentAuthoringRequestDto::Restore {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        target_version_id: "v1".to_string(),
        expected_current_version_id: "v2".to_string(),
        restored_version_id: "v4".to_string(),
        restored_snapshot_ref: "snapshot-v4".to_string(),
        author: "local-user".to_string(),
        summary: "Restore v1".to_string(),
    });
    let current_after_stale = runtime.execute(get_request()).data.expect("current");

    assert!(!stale_restore.ok);
    assert_eq!(
        stale_restore.error_code.as_deref(),
        Some("DOCUMENT_RESTORE_VERSION_CONFLICT")
    );
    assert_eq!(current_after_stale.current_version_id, "v3");
    assert_eq!(current_after_stale.body.as_deref(), Some("body two"));

    let applied = runtime.execute(DesktopDocumentAuthoringRequestDto::Restore {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        target_version_id: "v1".to_string(),
        expected_current_version_id: "v3".to_string(),
        restored_version_id: "v4".to_string(),
        restored_snapshot_ref: "snapshot-v4".to_string(),
        author: "local-user".to_string(),
        summary: "Restore v1".to_string(),
    });
    let current_after_restore = runtime
        .execute(get_request())
        .data
        .expect("restored current");

    assert!(applied.ok);
    assert_eq!(
        applied
            .data
            .expect("applied")
            .restored_version_id
            .as_deref(),
        Some("v4")
    );
    assert_eq!(current_after_restore.current_version_id, "v4");
    assert_eq!(
        current_after_restore.body.as_deref(),
        Some("# Source\nbody one")
    );
    drop(runtime);

    let restarted = build_runtime(&temp);
    let reopened = restarted.execute(get_request()).data.expect("reopened restored current");
    assert_eq!(reopened.current_version_id, "v4");
    assert_eq!(reopened.body.as_deref(), Some("# Source\nbody one"));
}

#[test]
fn durable_authoring_runtime_enqueues_projection_work_that_survives_restart() {
    let temp = TempRoot::new("projection-work");
    let runtime = build_runtime(&temp);
    assert!(runtime.execute(create_request()).ok);
    assert!(runtime.execute(update_request("v1", "v2")).ok);
    drop(runtime);

    let repository = DurableProjectionWorkRepository::new(temp.path.clone());
    let work = repository.list_resumable(20).expect("restart work");

    assert_eq!(work.len(), 6);
    assert_eq!(
        work.iter()
            .filter(|item| item.identity().version_id().as_str() == "v1")
            .count(),
        3
    );
    assert_eq!(
        work.iter()
            .filter(|item| item.identity().version_id().as_str() == "v2")
            .count(),
        3
    );
}

#[test]
fn durable_authoring_runtime_catalogs_created_document_for_restart_resolution() {
    let temp = TempRoot::new("document-link-catalog");
    let runtime = build_runtime(&temp);
    assert!(runtime.execute(create_request()).ok);
    drop(runtime);

    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace");
    let catalog = DurableDocumentLinkCatalog::new(temp.path.clone());
    for target in ["Source", "source", "notes/source.md"] {
        let LinkTargetResolution::Resolved(resolved) = catalog
            .resolve(&workspace_id, target)
            .expect("resolve after restart")
        else {
            panic!("created document should resolve");
        };
        assert_eq!(resolved.document_id().as_str(), "doc-1");
    }

    let work = DurableProjectionWorkRepository::new(temp.path.clone())
        .list_resumable(20)
        .expect("projection work");
    assert_eq!(work.len(), 3);
}

#[test]
fn durable_authoring_runtime_updates_restart_safe_home_projection_after_create() {
    let temp = TempRoot::new("home-projection");
    let runtime = build_runtime(&temp);
    assert!(runtime.execute(create_request()).ok);
    drop(runtime);

    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let projection = LocalWorkspaceHomeProjectionStore::new(temp.path.clone())
        .load_workspace_home(
            &workspace,
            WorkspaceHomeProjectionLimits::new(20, 20, 20, 20, 20).expect("limits"),
        )
        .expect("restart home projection");

    assert_eq!(projection.recent_documents().len(), 1);
    assert_eq!(projection.recent_documents()[0].document_id(), "doc-1");
    assert_eq!(projection.recent_documents()[0].title(), "Source");
    assert_eq!(projection.recent_documents()[0].path(), "notes/source.md");
}

fn build_runtime(temp: &TempRoot) -> DesktopDocumentAuthoringRuntime {
    DesktopDocumentAuthoringRuntime::new(temp.path.clone(), 1024).expect("runtime")
}

fn create_request() -> DesktopDocumentAuthoringRequestDto {
    DesktopDocumentAuthoringRequestDto::Create {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        path: "notes/source.md".to_string(),
        body: "# Source\nbody one".to_string(),
        version_id: "v1".to_string(),
        snapshot_ref: "snapshot-v1".to_string(),
        author: "local-user".to_string(),
        summary: "Created".to_string(),
    }
}

fn update_request(
    expected_version_id: &str,
    version_id: &str,
) -> DesktopDocumentAuthoringRequestDto {
    DesktopDocumentAuthoringRequestDto::Update {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        body: "body two".to_string(),
        expected_version_id: expected_version_id.to_string(),
        version_id: version_id.to_string(),
        snapshot_ref: format!("snapshot-{version_id}"),
        author: "local-user".to_string(),
        summary: "Updated".to_string(),
    }
}

fn get_request() -> DesktopDocumentAuthoringRequestDto {
    DesktopDocumentAuthoringRequestDto::GetCurrent {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
    }
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
            "sponzey-phase011-authoring-{label}-{}-{nonce}",
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
