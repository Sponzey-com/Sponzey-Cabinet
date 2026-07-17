use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_document_link_catalog::DurableDocumentLinkCatalog;
use cabinet_adapters::durable_projection_work_repository::DurableProjectionWorkRepository;
use cabinet_adapters::local_create_document_revision_runtime::LOCAL_DOCUMENT_VERSION_ROOT;
use cabinet_adapters::local_current_document_revision_projection::{
    LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT, LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT,
};
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_adapters::local_workspace_home_projection::LocalWorkspaceHomeProjectionStore;
use cabinet_desktop_shell::{
    DesktopDocumentMutationRequestDto, DesktopDocumentMutationRuntime, DesktopProjectionRuntime,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_link_catalog::DocumentLinkCatalog;
use cabinet_ports::document_repository::DocumentRepository;
use cabinet_ports::projection_work::ProjectionWorkRepository;
use cabinet_ports::version_store::{HistoryPageRequest, VersionStore};
use cabinet_ports::workspace_home::{WorkspaceHomeProjectionLimits, WorkspaceHomeProjectionPort};

#[test]
fn mutation_request_serialization_excludes_client_generated_storage_metadata() {
    let request = DesktopDocumentMutationRequestDto::Create {
        operation_id: "operation-1".into(),
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        body: "제목\n본문".into(),
        author: "local-user".into(),
        summary: "Create".into(),
    };

    let json = serde_json::to_value(request).expect("serialize request");
    let object = json.as_object().expect("request object");

    assert_eq!(
        object.get("kind").and_then(|value| value.as_str()),
        Some("create")
    );
    assert!(!object.contains_key("path"));
    assert!(!object.contains_key("versionId"));
    assert!(!object.contains_key("snapshotRef"));
}

#[test]
fn authoritative_desktop_runtime_creates_and_updates_current_projection() {
    let temp = TempRoot::new("create-update");
    let runtime = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096)
        .expect("desktop mutation runtime");

    let created = runtime.execute(create_request("operation-1", "첫 제목\n본문 1"));
    assert!(created.ok);
    let created_data = created.data.expect("created data");
    assert_eq!(created_data.kind, "created");
    assert!(!created_data.current_version_id.is_empty());
    let updated = runtime.execute(DesktopDocumentMutationRequestDto::Update {
        operation_id: "operation-2".into(),
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        expected_current_version_id: created_data.current_version_id,
        body: "두 번째 제목\n본문 2".into(),
        author: "local-user".into(),
        summary: "Update".into(),
    });

    assert!(updated.ok);
    assert_eq!(updated.data.as_ref().unwrap().kind, "updated");
    let current = current_projection(&temp).expect("current projection");
    assert_eq!(current.metadata().title().as_str(), "두 번째 제목");
    assert_eq!(current.body().as_str(), "두 번째 제목\n본문 2");
}

#[test]
fn maps_operation_conflict_and_projection_recovery_without_payload_leakage() {
    let temp = TempRoot::new("errors");
    let runtime = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).unwrap();
    assert!(
        runtime
            .execute(create_request("operation-1", "정상 제목\n본문"))
            .ok
    );

    let conflict = runtime.execute(create_request("operation-1", "변경된 payload"));
    assert!(!conflict.ok);
    assert_eq!(
        conflict.error_code.as_deref(),
        Some("DOCUMENT_REVISION_CONFLICT")
    );
    assert!(!conflict.retryable);
    assert!(!conflict.repair_required);

    let blocked = TempRoot::new("recovery");
    fs::write(
        blocked
            .path
            .join(LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT),
        "block projection",
    )
    .unwrap();
    let recovery_runtime = DesktopDocumentMutationRuntime::new(blocked.path.clone(), 4096).unwrap();
    let recovery = recovery_runtime.execute(create_request("operation-2", "복구 제목\n본문"));
    assert!(!recovery.ok);
    assert_eq!(
        recovery.error_code.as_deref(),
        Some("DOCUMENT_REVISION_RECOVERY_REQUIRED")
    );
    assert!(recovery.retryable);
    assert!(recovery.repair_required);
    assert!(!format!("{recovery:?}").contains("복구 제목"));
}

#[test]
fn authoritative_mutation_updates_home_and_enqueues_projection_work_for_each_revision() {
    let temp = TempRoot::new("fanout");
    let runtime = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).unwrap();
    let created = runtime.execute(create_request("operation-1", "첫 제목\n본문"));
    let first_version = created.data.unwrap().current_version_id;
    let created_work = DurableProjectionWorkRepository::new(temp.path.clone())
        .list_resumable(20)
        .unwrap();
    assert_eq!(created_work.len(), 3);
    assert!(created_work.iter().all(|item| {
        item.identity().document_id().as_str() == "doc-1"
            && item.identity().version_id().as_str() == first_version
    }));
    let projection = DesktopProjectionRuntime::new(temp.path.clone(), 4096, 20, 3).unwrap();
    let projected_create = projection.run_once();
    assert!(projected_create.ok);
    assert_eq!(projected_create.ready_count, 3);
    assert_eq!(projected_create.retry_scheduled_count, 0);
    assert_eq!(projected_create.failed_count, 0);

    let updated = runtime.execute(DesktopDocumentMutationRequestDto::Update {
        operation_id: "operation-2".into(),
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        expected_current_version_id: first_version.clone(),
        body: "바뀐 제목\n본문".into(),
        author: "local-user".into(),
        summary: "Update".into(),
    });
    assert!(updated.ok);
    let second_version = updated.data.unwrap().current_version_id;

    let home = LocalWorkspaceHomeProjectionStore::new(temp.path.clone())
        .load_workspace_home(
            &WorkspaceId::new("workspace-1").unwrap(),
            WorkspaceHomeProjectionLimits::new(10, 10, 10, 10, 10).unwrap(),
        )
        .unwrap();
    assert_eq!(home.recent_documents().len(), 1);
    assert_eq!(home.recent_documents()[0].title(), "바뀐 제목");
    let updated_work = DurableProjectionWorkRepository::new(temp.path.clone())
        .list_resumable(20)
        .unwrap();
    assert_eq!(updated_work.len(), 3);
    assert!(updated_work.iter().all(|item| {
        item.identity().document_id().as_str() == "doc-1"
            && item.identity().version_id().as_str() == second_version
    }));
    let projected_update = projection.run_once();
    assert!(projected_update.ok);
    assert_eq!(projected_update.ready_count, 3);
    assert_eq!(projected_update.retry_scheduled_count, 0);
    assert_eq!(projected_update.failed_count, 0);
    assert!(
        DurableProjectionWorkRepository::new(temp.path.clone())
            .list_resumable(20)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn fanout_failure_is_recovery_required_and_same_operation_retry_does_not_add_revision() {
    let temp = TempRoot::new("fanout-recovery");
    fs::write(temp.path.join("home-projections"), "blocked").unwrap();
    let runtime = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).unwrap();

    let failed = runtime.execute(create_request("operation-1", "복구 제목\n본문"));

    assert!(!failed.ok);
    assert_eq!(
        failed.error_code.as_deref(),
        Some("DOCUMENT_REVISION_RECOVERY_REQUIRED")
    );
    assert!(failed.retryable);
    assert!(failed.repair_required);
    fs::remove_file(temp.path.join("home-projections")).unwrap();
    let retried = runtime.execute(create_request("operation-1", "복구 제목\n본문"));
    assert!(retried.ok);
    let history = LocalVersionStore::new(temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT))
        .list_history(
            &WorkspaceId::new("workspace-1").unwrap(),
            &DocumentId::new("doc-1").unwrap(),
            HistoryPageRequest::first(10).unwrap(),
        )
        .unwrap();
    assert_eq!(history.entries().len(), 1);
    let home = LocalWorkspaceHomeProjectionStore::new(temp.path.clone())
        .load_workspace_home(
            &WorkspaceId::new("workspace-1").unwrap(),
            WorkspaceHomeProjectionLimits::new(10, 10, 10, 10, 10).unwrap(),
        )
        .unwrap();
    assert_eq!(home.recent_documents()[0].title(), "복구 제목");
}

#[test]
fn title_change_updates_link_catalog_and_reindexes_authoritative_dependents() {
    let temp = TempRoot::new("title-dependent-fanout");
    let runtime = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).unwrap();
    let target = runtime.execute(DesktopDocumentMutationRequestDto::Create {
        operation_id: "operation-target-create".into(),
        workspace_id: "workspace-1".into(),
        document_id: "target".into(),
        body: "Old title\nTarget body".into(),
        author: "local-user".into(),
        summary: "Create target".into(),
    });
    assert!(target.ok);
    let target_version = target.data.unwrap().current_version_id;
    assert!(
        runtime
            .execute(DesktopDocumentMutationRequestDto::Create {
                operation_id: "operation-source-create".into(),
                workspace_id: "workspace-1".into(),
                document_id: "source".into(),
                body: "Source title\n[[Old title]]".into(),
                author: "local-user".into(),
                summary: "Create source".into(),
            })
            .ok
    );
    let projection = DesktopProjectionRuntime::new(temp.path.clone(), 4096, 20, 3).unwrap();
    let initial_projection = projection.run_once();
    assert_eq!(initial_projection.ready_count, 6);
    assert_eq!(initial_projection.failed_count, 0);

    let updated = runtime.execute(DesktopDocumentMutationRequestDto::Update {
        operation_id: "operation-target-update".into(),
        workspace_id: "workspace-1".into(),
        document_id: "target".into(),
        expected_current_version_id: target_version,
        body: "New title\nTarget body".into(),
        author: "local-user".into(),
        summary: "Rename through first line".into(),
    });
    assert!(updated.ok);

    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let catalog = DurableDocumentLinkCatalog::new(temp.path.clone())
        .list(&workspace)
        .unwrap();
    let target_record = catalog
        .iter()
        .find(|record| record.document_id().as_str() == "target")
        .expect("target catalog record");
    assert_eq!(target_record.title().as_str(), "New title");

    let work = DurableProjectionWorkRepository::new(temp.path.clone())
        .list_resumable(20)
        .unwrap();
    assert_eq!(
        work.iter()
            .filter(|item| item.identity().document_id().as_str() == "target")
            .count(),
        3
    );
    assert_eq!(
        work.iter()
            .filter(|item| item.identity().document_id().as_str() == "source")
            .count(),
        3
    );
}

#[test]
fn tauri_main_registers_authoritative_document_mutation_command() {
    let main_source = include_str!("../src/main.rs");
    assert!(main_source.contains("execute_desktop_document_mutation"));
    assert!(main_source.contains("DesktopDocumentMutationRuntime"));
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

fn current_projection(
    temp: &TempRoot,
) -> Option<cabinet_ports::document_repository::CurrentDocumentRecord> {
    LocalDocumentRepository::new(temp.path.join(LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT))
        .get_current_by_id(
            &WorkspaceId::new("workspace-1").unwrap(),
            &DocumentId::new("doc-1").unwrap(),
        )
        .unwrap()
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
            "cabinet-desktop-mutation-{name}-{}-{nanos}",
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
