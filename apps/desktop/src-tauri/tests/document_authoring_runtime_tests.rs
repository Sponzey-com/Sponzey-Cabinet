use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_document_link_catalog::DurableDocumentLinkCatalog;
use cabinet_adapters::durable_projection_work_repository::DurableProjectionWorkRepository;
use cabinet_adapters::local_current_document_revision_projection::LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT;
use cabinet_adapters::local_mutate_document_attachments_runtime::LocalMutateDocumentAttachmentsRuntime;
use cabinet_adapters::local_restore_document_revision_runtime::LocalRestoreDocumentRevisionRuntime;
use cabinet_adapters::local_workspace_home_projection::LocalWorkspaceHomeProjectionStore;
use cabinet_desktop_shell::{
    DesktopDocumentAuthoringRequestDto, DesktopDocumentAuthoringRuntime,
    DesktopDocumentMutationRequestDto, DesktopDocumentMutationRuntime,
    DesktopDocumentQueryRequestDto, DesktopDocumentQueryRuntime, DesktopProjectionRuntime,
};
use cabinet_domain::asset::AssetId;
use cabinet_domain::document::DocumentBodyPolicy;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::link_target_resolver::{DocumentLinkTargetResolver, LinkTargetResolution};
use cabinet_ports::projection_work::ProjectionWorkRepository;
use cabinet_ports::workspace_home::{WorkspaceHomeProjectionLimits, WorkspaceHomeProjectionPort};
use cabinet_usecases::mutate_document_attachments::MutateDocumentAttachmentsInput;
use cabinet_usecases::restore_document_revision::{
    RestoreDocumentRevisionError, RestoreDocumentRevisionInput,
};

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
    let data = history.data.expect("history");
    let created_at = &data.entries[0].created_at;
    let revision_number = data.entries[0].revision_number;

    assert_ne!(created_at, "local-version");
    assert!(created_at.parse::<u64>().expect("epoch milliseconds") > 0);
    assert_eq!(revision_number, 1);
}

#[test]
fn desktop_runtime_startup_migrates_legacy_revision_numbers_idempotently() {
    let temp = TempRoot::new("startup-revision-migration");
    let runtime = build_runtime(&temp);
    assert!(runtime.execute(create_request()).ok);
    drop(runtime);
    remove_revision_number(&authoring_entry_path(&temp, "doc-1", "v1"));

    let restarted = build_runtime(&temp);
    let history = restarted.execute(DesktopDocumentAuthoringRequestDto::GetHistory {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        limit: 10,
    });
    assert_eq!(history.data.expect("history").entries[0].revision_number, 1);
    drop(restarted);
    let after_first = fs::read(authoring_entry_path(&temp, "doc-1", "v1")).expect("entry");

    drop(build_runtime(&temp));
    let after_second = fs::read(authoring_entry_path(&temp, "doc-1", "v1")).expect("entry");

    assert_eq!(after_first, after_second);
}

#[test]
fn desktop_runtimes_reject_corrupt_revision_number_at_startup() {
    let temp = TempRoot::new("startup-revision-corruption");
    let runtime = build_runtime(&temp);
    assert!(runtime.execute(create_request()).ok);
    drop(runtime);
    replace_revision_number(&authoring_entry_path(&temp, "doc-1", "v1"), 2);

    assert_eq!(
        DesktopDocumentAuthoringRuntime::new(temp.path.clone(), 1024)
            .err()
            .expect("authoring startup must fail"),
        "DOCUMENT_AUTHORING_REVISION_MIGRATION_FAILED"
    );
    assert_eq!(
        DesktopProjectionRuntime::new(temp.path.clone(), 1024, 20, 3)
            .err()
            .expect("projection startup must fail"),
        "PROJECTION_REVISION_MIGRATION_FAILED"
    );
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
fn durable_authoring_runtime_lists_history_and_blocks_stale_restore() {
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
    assert_eq!(history.data.expect("history").entries.len(), 2);
    assert_eq!(
        version.data.expect("version").body.as_deref(),
        Some("# Source\nbody one")
    );
}

#[test]
fn restore_preview_reads_authoritative_current_pointer_and_rich_diff() {
    let temp = TempRoot::new("authoritative-restore-preview");
    let mutation = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).expect("mutation");
    let created = mutation.execute(DesktopDocumentMutationRequestDto::Create {
        operation_id: "create-restore-preview".to_string(),
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        body: "# 과거 제목\n과거 본문".to_string(),
        author: "local-user".to_string(),
        summary: "Create".to_string(),
    });
    let target_version = created.data.expect("created").current_version_id;
    let updated = mutation.execute(DesktopDocumentMutationRequestDto::Update {
        operation_id: "update-restore-preview".to_string(),
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        expected_current_version_id: target_version.clone(),
        body: "# 현재 제목\n현재 본문".to_string(),
        author: "local-user".to_string(),
        summary: "Update".to_string(),
    });
    let current_version = updated.data.expect("updated").current_version_id;
    let runtime = DesktopDocumentAuthoringRuntime::new(temp.path.clone(), 4096).expect("authoring");

    let preview = runtime.execute(DesktopDocumentAuthoringRequestDto::PreviewRestore {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        target_version_id: target_version.clone(),
    });

    let preview_json = serde_json::to_value(&preview).expect("serialize preview response");
    assert_eq!(
        preview_json["data"]["missingAssetLabels"],
        serde_json::json!([])
    );
    assert!(preview_json["data"]["lines"].is_array());

    let data = preview.data.expect("preview");
    assert_eq!(
        data.expected_current_version_id.as_deref(),
        Some(current_version.as_str())
    );
    assert_eq!(
        data.target_version_id.as_deref(),
        Some(target_version.as_str())
    );
    assert_eq!(data.can_restore, Some(true));
    let diff = data.restore_diff.expect("rich restore diff");
    assert_eq!(diff.kind, "complete");
    assert_eq!(diff.left_version_token, current_version);
    assert_eq!(diff.right_version_token, target_version);
    assert!(!diff.hunks.is_empty());

    let restored = runtime.execute(DesktopDocumentAuthoringRequestDto::Restore {
        operation_id: "restore-authoritative-1".to_string(),
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        target_version_id: target_version.clone(),
        expected_current_version_id: current_version.clone(),
        author: "local-user".to_string(),
        summary: "Restore".to_string(),
    });
    let restored_data = restored.data.expect("restored");
    assert_eq!(
        runtime.restore_product_event_names(),
        vec![
            "document.restore.requested",
            "document.restore.primary_committed",
            "document.restore.completed",
        ]
    );
    assert_ne!(
        restored_data.restored_version_id.as_deref(),
        Some(target_version.as_str())
    );
    assert_eq!(
        restored_data.restored_version_id.as_deref(),
        Some(restored_data.current_version_id.as_str())
    );
    let query = DesktopDocumentQueryRuntime::new(temp.path.clone(), 4096).expect("query");
    let current = query.execute(DesktopDocumentQueryRequestDto::Current {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
    });
    assert_eq!(
        current.data.expect("current").body.as_deref(),
        Some("# 과거 제목\n과거 본문")
    );
    assert_eq!(
        DurableProjectionWorkRepository::new(temp.path.clone())
            .list_resumable(20)
            .expect("restore projection work")
            .len(),
        9
    );

    let stale = runtime.execute(DesktopDocumentAuthoringRequestDto::Restore {
        operation_id: "restore-authoritative-stale".to_string(),
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        target_version_id: target_version,
        expected_current_version_id: current_version,
        author: "local-user".to_string(),
        summary: "Restore".to_string(),
    });
    assert!(!stale.ok);
    assert_eq!(
        stale.error_code.as_deref(),
        Some("DOCUMENT_RESTORE_VERSION_CONFLICT")
    );
    assert_eq!(
        runtime.restore_product_event_names(),
        vec![
            "document.restore.requested",
            "document.restore.primary_committed",
            "document.restore.completed",
            "document.restore.requested",
            "document.restore.conflict",
        ]
    );
}

#[test]
fn restore_preview_and_apply_block_unchanged_missing_target_asset_without_writes() {
    let temp = TempRoot::new("authoritative-restore-missing-asset");
    let mutation = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).expect("mutation");
    let created = mutation.execute(DesktopDocumentMutationRequestDto::Create {
        operation_id: "create-restore-missing".to_string(),
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        body: "# 과거 제목\n과거 본문".to_string(),
        author: "local-user".to_string(),
        summary: "Create".to_string(),
    });
    let created_version = created.data.expect("created").current_version_id;
    let missing_asset = AssetId::from_sha256_hex(&"a".repeat(64)).expect("asset id");
    let mut attachments = LocalMutateDocumentAttachmentsRuntime::new(
        temp.path.clone(),
        DocumentBodyPolicy::new(4096).expect("body policy"),
    );
    let target = attachments
        .execute(MutateDocumentAttachmentsInput::link(
            "link-restore-missing",
            "workspace-1",
            "doc-1",
            &created_version,
            missing_asset.as_str(),
            "누락 자료",
            "local-user",
            "Attach",
        ))
        .expect("link missing reference");
    let target_version = target.version_id().as_str().to_string();
    let updated = mutation.execute(DesktopDocumentMutationRequestDto::Update {
        operation_id: "update-restore-missing".to_string(),
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        expected_current_version_id: target_version.clone(),
        body: "# 현재 제목\n현재 본문".to_string(),
        author: "local-user".to_string(),
        summary: "Update".to_string(),
    });
    let current_version = updated.data.expect("updated").current_version_id;
    let runtime = DesktopDocumentAuthoringRuntime::new(temp.path.clone(), 4096).expect("authoring");

    let preview = runtime.execute(DesktopDocumentAuthoringRequestDto::PreviewRestore {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        target_version_id: target_version.clone(),
    });
    let preview_data = preview.data.expect("blocked preview");
    assert_eq!(preview_data.can_restore, Some(false));
    assert_eq!(preview_data.missing_asset_labels, vec!["누락 자료"]);
    assert!(
        preview_data
            .restore_diff
            .expect("diff")
            .attachment_diff
            .added
            .is_empty()
    );

    let blocked = runtime.execute(DesktopDocumentAuthoringRequestDto::Restore {
        operation_id: "restore-missing".to_string(),
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        target_version_id: target_version,
        expected_current_version_id: current_version.clone(),
        author: "local-user".to_string(),
        summary: "Restore".to_string(),
    });
    assert!(!blocked.ok);
    assert_eq!(
        blocked.error_code.as_deref(),
        Some("DOCUMENT_RESTORE_MISSING_DEPENDENCY")
    );
    assert!(!blocked.retryable);

    let query = DesktopDocumentQueryRuntime::new(temp.path.clone(), 4096).expect("query");
    let current = query.execute(DesktopDocumentQueryRequestDto::Current {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
    });
    assert_eq!(
        current
            .data
            .expect("current")
            .current_version_token
            .as_deref(),
        Some(current_version.as_str())
    );
    let history = query.execute(DesktopDocumentQueryRequestDto::History {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        cursor: None,
        limit: 20,
    });
    assert_eq!(history.data.expect("history").entries.len(), 3);
}

#[test]
fn authoring_startup_recovers_committed_restore_projection_and_enqueues_derived_work() {
    let temp = TempRoot::new("startup-restore-recovery");
    let mutation = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).expect("mutation");
    let target = mutation.execute(DesktopDocumentMutationRequestDto::Create {
        operation_id: "create-startup-recovery".to_string(),
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        body: "# 과거 제목\n과거 본문".to_string(),
        author: "local-user".to_string(),
        summary: "Create".to_string(),
    });
    let target_version = target.data.expect("target").current_version_id;
    let current = mutation.execute(DesktopDocumentMutationRequestDto::Update {
        operation_id: "update-startup-recovery".to_string(),
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        expected_current_version_id: target_version.clone(),
        body: "# 현재 제목\n현재 본문".to_string(),
        author: "local-user".to_string(),
        summary: "Update".to_string(),
    });
    let current_version = current.data.expect("current").current_version_id;
    let blocker = current_projection_identity_path(&temp);
    fs::remove_file(&blocker).unwrap();
    fs::create_dir(&blocker).unwrap();
    let error = LocalRestoreDocumentRevisionRuntime::new(
        temp.path.clone(),
        DocumentBodyPolicy::new(4096).unwrap(),
    )
    .execute(RestoreDocumentRevisionInput::new(
        "restore-startup-recovery",
        "workspace-1",
        "doc-1",
        &target_version,
        &current_version,
        "local-user",
        "Restore",
    ))
    .expect_err("projection failure");
    assert_eq!(error, RestoreDocumentRevisionError::RecoveryRequired);
    fs::remove_dir(blocker).unwrap();

    let _runtime =
        DesktopDocumentAuthoringRuntime::new(temp.path.clone(), 4096).expect("startup recovery");
    let query = DesktopDocumentQueryRuntime::new(temp.path.clone(), 4096).expect("query");
    let current = query.execute(DesktopDocumentQueryRequestDto::Current {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
    });
    assert_eq!(
        current.data.expect("recovered current").body.as_deref(),
        Some("# 과거 제목\n과거 본문")
    );
    let history = query.execute(DesktopDocumentQueryRequestDto::History {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        cursor: None,
        limit: 20,
    });
    assert_eq!(history.data.expect("history").entries.len(), 3);
    assert_eq!(
        DurableProjectionWorkRepository::new(temp.path.clone())
            .list_resumable(20)
            .expect("projection work")
            .len(),
        9
    );
    let _second_restart = DesktopDocumentAuthoringRuntime::new(temp.path.clone(), 4096)
        .expect("idempotent startup recovery");
    assert_eq!(
        DurableProjectionWorkRepository::new(temp.path.clone())
            .list_resumable(20)
            .expect("idempotent projection work")
            .len(),
        9
    );
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

fn authoring_entry_path(temp: &TempRoot, document_id: &str, version_id: &str) -> PathBuf {
    temp.path
        .join("authoring-versions")
        .join("workspace-1")
        .join("documents")
        .join(document_id)
        .join("snapshots")
        .join(version_id)
        .join("entry.txt")
}

fn current_projection_identity_path(temp: &TempRoot) -> PathBuf {
    temp.path
        .join(LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT)
        .join(hex("workspace-1"))
        .join(hex("doc-1"))
        .join("current.projection")
}

fn hex(value: &str) -> String {
    value.bytes().map(|byte| format!("{byte:02x}")).collect()
}

fn remove_revision_number(path: &PathBuf) {
    let content = fs::read_to_string(path).expect("entry");
    let legacy = content
        .lines()
        .filter(|line| !line.starts_with("revision_number="))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(path, legacy).expect("legacy entry");
}

fn replace_revision_number(path: &PathBuf, revision_number: u64) {
    let content = fs::read_to_string(path).expect("entry");
    let mut lines = content
        .lines()
        .filter(|line| !line.starts_with("revision_number="))
        .map(str::to_string)
        .collect::<Vec<_>>();
    lines.push(format!("revision_number={revision_number}"));
    fs::write(path, lines.join("\n") + "\n").expect("corrupt entry");
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
