use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use cabinet_desktop_shell::{
    DesktopDocumentAuthoringRequestDto, DesktopDocumentAuthoringRuntime,
    DesktopProjectionRepairOperationRuntime, DesktopProjectionRuntime,
};

#[test]
fn repair_operation_runtime_returns_identity_and_survives_restart() {
    let temp = Temp::new("restart");
    let runtime = DesktopProjectionRepairOperationRuntime::new(temp.path.clone());
    let started = runtime.start("workspace-1", "doc-1");
    assert!(started.ok);
    assert_eq!(started.state.as_deref(), Some("queued"));
    let id = started.operation_id.unwrap();
    drop(runtime);

    let restarted = DesktopProjectionRepairOperationRuntime::new(temp.path.clone());
    let status = restarted.status("workspace-1", &id);
    assert!(status.ok);
    assert_eq!(status.state.as_deref(), Some("queued"));
    assert_eq!(status.total_units, 3);

    let cancelled = restarted.cancel("workspace-1", &id);
    assert!(cancelled.ok);
    assert_eq!(cancelled.state.as_deref(), Some("cancelled"));
}

#[test]
fn repair_operation_runner_persists_success_after_projection_publish() {
    let temp = Temp::new("runner");
    let authoring = DesktopDocumentAuthoringRuntime::new(temp.path.clone(), 4096).unwrap();
    let created = authoring.execute(DesktopDocumentAuthoringRequestDto::Create {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        path: "doc.md".into(),
        body: "body".into(),
        version_id: "version-1".into(),
        snapshot_ref: "snapshot-1".into(),
        author: "local-user".into(),
        summary: "create".into(),
    });
    assert!(created.ok, "create failed: {created:?}");
    let projection = DesktopProjectionRuntime::new(temp.path.clone(), 4096, 20, 3).unwrap();
    let operations = DesktopProjectionRepairOperationRuntime::new(temp.path.clone());
    let started = operations.start("workspace-1", "doc-1");
    let completed = operations.run(
        "workspace-1",
        started.operation_id.as_deref().unwrap(),
        &projection,
    );
    assert!(completed.ok);
    assert_eq!(completed.state.as_deref(), Some("succeeded"));
    assert_eq!(
        projection
            .get_freshness("workspace-1", "doc-1")
            .state
            .as_deref(),
        Some("ready")
    );
    drop(operations);
    let restarted = DesktopProjectionRepairOperationRuntime::new(temp.path.clone());
    let status = restarted.status("workspace-1", started.operation_id.as_deref().unwrap());
    assert_eq!(status.state.as_deref(), Some("succeeded"));
}

#[test]
fn repair_operation_succeeds_for_current_version_after_create_and_update() {
    let temp = Temp::new("updated-current");
    let authoring = DesktopDocumentAuthoringRuntime::new(temp.path.clone(), 4096).unwrap();
    assert!(
        authoring
            .execute(DesktopDocumentAuthoringRequestDto::Create {
                workspace_id: "workspace-1".into(),
                document_id: "doc-1".into(),
                path: "doc.md".into(),
                body: "body one".into(),
                version_id: "version-1".into(),
                snapshot_ref: "snapshot-1".into(),
                author: "local-user".into(),
                summary: "create".into(),
            })
            .ok
    );
    assert!(
        authoring
            .execute(DesktopDocumentAuthoringRequestDto::Update {
                workspace_id: "workspace-1".into(),
                document_id: "doc-1".into(),
                body: "body two".into(),
                expected_version_id: "version-1".into(),
                version_id: "version-2".into(),
                snapshot_ref: "snapshot-2".into(),
                author: "local-user".into(),
                summary: "update".into(),
            })
            .ok
    );

    let projection = DesktopProjectionRuntime::new(temp.path.clone(), 4096, 20, 3).unwrap();
    let operations = DesktopProjectionRepairOperationRuntime::new(temp.path.clone());
    let started = operations.start("workspace-1", "doc-1");
    let completed = operations.run(
        "workspace-1",
        started.operation_id.as_deref().unwrap(),
        &projection,
    );

    assert!(completed.ok);
    assert_eq!(completed.state.as_deref(), Some("succeeded"));
    assert_eq!(completed.completed_units, 3);
    assert_eq!(
        projection
            .get_freshness("workspace-1", "doc-1")
            .state
            .as_deref(),
        Some("ready")
    );
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
            "cabinet-repair-runtime-{label}-{}-{nonce}",
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
