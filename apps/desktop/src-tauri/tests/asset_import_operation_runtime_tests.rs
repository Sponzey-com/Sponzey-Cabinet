use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_association_catalog::DurableAssetAssociationCatalog;
use cabinet_adapters::durable_asset_import_operation_repository::DurableAssetImportOperationRepository;
use cabinet_adapters::durable_asset_metadata_catalog::DurableAssetMetadataCatalog;
use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_adapters::durable_projection_work_repository::DurableProjectionWorkRepository;
use cabinet_adapters::local_asset_staging_writer::LocalAssetStagingWriter;
use cabinet_adapters::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT,
};
use cabinet_adapters::local_current_document_revision_projection::LocalCurrentDocumentRevisionProjectionWriter;
use cabinet_adapters::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_desktop_shell::{
    DesktopAssetImportRequestDto, DesktopAssetImportSelectionRuntime, DesktopDocumentAssetsRuntime,
    DesktopDocumentAuthoringRequestDto, DesktopDocumentAuthoringRuntime,
    DesktopLocalCommandPayloadDto, DesktopLocalCommandRequestDto, DesktopProjectionRuntime,
    DesktopRevisionGuardedAssetImportRequestDto,
};
use cabinet_domain::asset::AssetId;
use cabinet_domain::asset_import_operation::{
    AssetImportEvent, AssetImportOperation, AssetImportOperationId, AssetImportState,
};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::graph::GraphEdgeKind;
use cabinet_domain::version::{
    AttachmentSnapshotState, CurrentDocumentSnapshot, DocumentRevisionNumber, DocumentSnapshotRef,
    VersionAuthor, VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::AssetAssociationCatalog;
use cabinet_ports::asset_import_operation_repository::AssetImportOperationRepository;
use cabinet_ports::asset_metadata_catalog::AssetMetadataCatalog;
use cabinet_ports::asset_staging::AssetStagingWriter;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::document_repository::{CurrentDocumentRecord, DocumentRepository};
use cabinet_ports::graph_projection::GraphProjectionStore;
use cabinet_ports::projection_work::ProjectionWorkRepository;
use cabinet_ports::version_store::{HistoryPageRequest, VersionStore};
use cabinet_usecases::project_current_document_revision::{
    ProjectCurrentDocumentRevisionInput, ProjectCurrentDocumentRevisionUsecase,
};

#[test]
fn native_import_runtime_persists_completed_operation_metadata_and_association_across_restart() {
    let root = temp_root("complete");
    seed_document(&root, "doc-1");
    let selected_path = root.join("source.pdf");
    fs::write(&selected_path, b"durable-pdf-content").expect("source");
    let runtime =
        DesktopAssetImportSelectionRuntime::with_app_data_root(root.clone(), "workspace-1", 4)
            .expect("runtime");
    let selection = runtime.register_selected_paths(vec![selected_path]);
    let descriptor = &selection.data.expect("selection").files[0];

    let response = runtime.import(DesktopAssetImportRequestDto {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        handle: descriptor.handle.clone(),
        label: "Design PDF".into(),
    });

    assert!(response.ok, "response={response:?}");
    assert_eq!(response.state.as_deref(), Some("completed"));
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let document = DocumentId::new("doc-1").expect("document");
    let asset = AssetId::from_sha256_hex(response.asset_id.as_deref().expect("asset id"))
        .expect("asset id");
    let operation =
        AssetImportOperationId::new(response.operation_id.as_deref().expect("operation id"))
            .expect("operation id");

    let metadata = DurableAssetMetadataCatalog::new(root.clone())
        .get(&workspace, &asset)
        .expect("metadata read")
        .expect("metadata");
    let links = DurableAssetAssociationCatalog::new(root.clone())
        .list_assets(&workspace, &document, 10)
        .expect("links");
    let operation = DurableAssetImportOperationRepository::new(root.clone())
        .get(&operation)
        .expect("operation read")
        .expect("operation");

    assert_eq!(metadata.metadata().id(), &asset);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].asset_id(), &asset);
    assert_eq!(operation.state(), AssetImportState::Completed);
    let query = DesktopDocumentAssetsRuntime::new(root.clone(), 10 * 1024 * 1024)
        .expect("query runtime")
        .execute(DesktopLocalCommandRequestDto {
            command_name: "list_document_assets".into(),
            payload: DesktopLocalCommandPayloadDto::DocumentIdentity {
                workspace_id: "workspace-1".into(),
                document_id: "doc-1".into(),
            },
        });
    assert!(query.ok, "query={query:?}");
    let queried = query.data.expect("query data");
    assert_eq!(queried.assets.len(), 1);
    assert_eq!(queried.assets[0].asset_id, asset.as_str());
    assert_eq!(queried.assets[0].status, "available");
    let serialized = serde_json::to_string(&response).expect("response json");
    assert!(!serialized.contains(root.to_string_lossy().as_ref()));
    assert!(!serialized.contains("durable-pdf-content"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn revision_guarded_import_creates_attachment_revision_and_rejects_stale_second_link() {
    let root = temp_root("revision-guarded");
    seed_revisioned_document(&root, "doc-revisioned");
    let first_source = root.join("first.txt");
    fs::write(&first_source, b"first imported asset").unwrap();
    let runtime = DesktopAssetImportSelectionRuntime::with_app_data_root_and_body_policy(
        root.clone(),
        "workspace-1",
        4,
        DocumentBodyPolicy::new(10 * 1024 * 1024).unwrap(),
    )
    .unwrap();
    let first = runtime.register_selected_paths(vec![first_source]);
    let first_response =
        runtime.import_revision_guarded(DesktopRevisionGuardedAssetImportRequestDto {
            import: DesktopAssetImportRequestDto {
                workspace_id: "workspace-1".into(),
                document_id: "doc-revisioned".into(),
                handle: first.data.unwrap().files[0].handle.clone(),
                label: "First".into(),
            },
            attachment_operation_id: "attachment-import-first".into(),
            expected_current_version_token: "version-graph-1".into(),
        });
    assert!(first_response.ok, "first={first_response:?}");

    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let document = DocumentId::new("doc-revisioned").unwrap();
    let history = LocalVersionStore::new(root.join(LOCAL_DOCUMENT_VERSION_ROOT))
        .list_history(
            &workspace,
            &document,
            HistoryPageRequest::first(10).unwrap(),
        )
        .unwrap();
    assert_eq!(history.entries().len(), 2);
    assert_eq!(
        DurableAssetAssociationCatalog::new(root.clone())
            .list_assets(&workspace, &document, 10)
            .unwrap()
            .len(),
        1
    );

    let second_source = root.join("second.txt");
    fs::write(&second_source, b"second imported asset").unwrap();
    let second = runtime.register_selected_paths(vec![second_source]);
    let stale = runtime.import_revision_guarded(DesktopRevisionGuardedAssetImportRequestDto {
        import: DesktopAssetImportRequestDto {
            workspace_id: "workspace-1".into(),
            document_id: "doc-revisioned".into(),
            handle: second.data.unwrap().files[0].handle.clone(),
            label: "Second".into(),
        },
        attachment_operation_id: "attachment-import-second".into(),
        expected_current_version_token: "version-graph-1".into(),
    });
    assert!(!stale.ok);
    assert_eq!(
        stale.error_code.as_deref(),
        Some("imported_asset_link.current_conflict")
    );
    let history = LocalVersionStore::new(root.join(LOCAL_DOCUMENT_VERSION_ROOT))
        .list_history(
            &workspace,
            &document,
            HistoryPageRequest::first(10).unwrap(),
        )
        .unwrap();
    assert_eq!(history.entries().len(), 2);
    assert_eq!(
        DurableAssetAssociationCatalog::new(root.clone())
            .list_assets(&workspace, &document, 10)
            .unwrap()
            .len(),
        1
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_import_hands_completed_association_to_graph_projection_idempotently() {
    let root = temp_root("graph-handoff");
    seed_authored_document(&root, "doc-graph");
    let selected_path = root.join("graph-source.txt");
    fs::write(&selected_path, b"graph asset").expect("source");
    let runtime =
        DesktopAssetImportSelectionRuntime::with_app_data_root(root.clone(), "workspace-1", 4)
            .expect("runtime");
    let projection =
        DesktopProjectionRuntime::new(root.clone(), 10 * 1024 * 1024, 20, 3).expect("projection");
    assert!(projection.run_once().ok);
    assert_eq!(imported_attachment_edges(&root, "doc-graph"), 0);
    let selection = runtime.register_selected_paths(vec![selected_path]);
    let handle = selection.data.expect("selection").files[0].handle.clone();

    let imported = runtime.import(DesktopAssetImportRequestDto {
        workspace_id: "workspace-1".into(),
        document_id: "doc-graph".into(),
        handle,
        label: "Graph import".into(),
    });
    assert!(imported.ok, "imported={imported:?}");
    let operation_id = imported.operation_id.clone().expect("operation");

    assert!(projection.run_once().ok);
    assert_eq!(imported_attachment_edges(&root, "doc-graph"), 1);

    let status = runtime.status("workspace-1", &operation_id);
    assert!(status.ok, "status={status:?}");
    let first_pending = DurableProjectionWorkRepository::new(root.clone())
        .list_resumable(20)
        .expect("work");
    assert!(first_pending.is_empty());
    let repeated = runtime.status("workspace-1", &operation_id);
    assert!(repeated.ok, "repeated={repeated:?}");
    let second_pending = DurableProjectionWorkRepository::new(root.clone())
        .list_resumable(20)
        .expect("work");
    assert_eq!(second_pending.len(), first_pending.len());

    drop(projection);
    assert_eq!(imported_attachment_edges(&root, "doc-graph"), 1);
    let _ = fs::remove_dir_all(root);
}

fn imported_attachment_edges(root: &PathBuf, document_id: &str) -> usize {
    DurableLocalGraphProjectionStore::new(root.clone())
        .get_projection(
            &WorkspaceId::new("workspace-1").expect("workspace"),
            &DocumentId::new(document_id).expect("document"),
        )
        .expect("graph read")
        .expect("graph")
        .graph()
        .edges()
        .iter()
        .filter(|edge| edge.kind() == GraphEdgeKind::AttachmentReference)
        .count()
}

fn seed_authored_document(root: &PathBuf, document_id: &str) {
    let runtime =
        DesktopDocumentAuthoringRuntime::new(root.clone(), 10 * 1024 * 1024).expect("authoring");
    let response = runtime.execute(DesktopDocumentAuthoringRequestDto::Create {
        workspace_id: "workspace-1".into(),
        document_id: document_id.into(),
        path: format!("{document_id}.md"),
        body: "# Graph import target".into(),
        version_id: "version-graph-1".into(),
        snapshot_ref: "snapshot-graph-1".into(),
        author: "local-user".into(),
        summary: "Created".into(),
    });
    assert!(response.ok, "authoring={response:?}");
}

fn seed_revisioned_document(root: &PathBuf, document_id: &str) {
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let document = DocumentId::new(document_id).unwrap();
    let version = VersionId::new("version-graph-1").unwrap();
    let snapshot_ref = DocumentSnapshotRef::new("snapshot-import-base").unwrap();
    let policy = DocumentBodyPolicy::new(10 * 1024 * 1024).unwrap();
    let entry = VersionEntry::new(
        version.clone(),
        document.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("local-user").unwrap(),
        VersionSummary::new("Seed import target").unwrap(),
    )
    .unwrap()
    .with_created_at_epoch_ms(1)
    .unwrap()
    .with_revision_number(DocumentRevisionNumber::new(1).unwrap())
    .unwrap();
    let record = cabinet_ports::version_store::VersionRecord::new(
        entry,
        cabinet_ports::version_store::VersionSnapshot::with_attachment_state(
            document.clone(),
            snapshot_ref,
            DocumentBody::new("# Revisioned import target\n", policy).unwrap(),
            AttachmentSnapshotState::known(Vec::new()).unwrap(),
        ),
    )
    .unwrap();
    LocalVersionStore::with_body_policy(root.join(LOCAL_DOCUMENT_VERSION_ROOT), policy)
        .append_version(&workspace, record.clone())
        .unwrap();
    LocalCurrentDocumentVersionPointer::new(root.join(LOCAL_DOCUMENT_POINTER_ROOT))
        .compare_and_set_current_version(&workspace, &document, None, version)
        .unwrap();
    ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            ProjectCurrentDocumentRevisionInput::new(
                "workspace-1",
                &format!("{document_id}.md"),
                record,
            ),
            &mut LocalCurrentDocumentRevisionProjectionWriter::new(root.clone(), policy),
        )
        .unwrap();
}

#[test]
fn native_import_runtime_rejects_scope_mismatch_before_creating_operation() {
    let root = temp_root("scope");
    let source = root.join("source.txt");
    fs::write(&source, b"content").expect("source");
    let runtime =
        DesktopAssetImportSelectionRuntime::with_app_data_root(root.clone(), "workspace-1", 4)
            .expect("runtime");
    let selection = runtime.register_selected_paths(vec![source]);
    let handle = selection.data.expect("selection").files[0].handle.clone();

    let response = runtime.import(DesktopAssetImportRequestDto {
        workspace_id: "workspace-2".into(),
        document_id: "doc-1".into(),
        handle,
        label: "Source".into(),
    });

    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("asset_import.workspace_scope_mismatch")
    );
    let active = DurableAssetImportOperationRepository::new(root.clone())
        .list_active(&WorkspaceId::new("workspace-2").expect("workspace"), 10)
        .expect("active operations");
    assert!(active.is_empty());
    assert!(!root.join("staging/assets").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_import_runtime_rejects_unknown_handle_before_storage_mutation() {
    let root = temp_root("unknown-handle");
    seed_document(&root, "doc-1");
    let runtime =
        DesktopAssetImportSelectionRuntime::with_app_data_root(root.clone(), "workspace-1", 4)
            .expect("runtime");

    let response = runtime.import(DesktopAssetImportRequestDto {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        handle: "picker:unknown".into(),
        label: "Unknown".into(),
    });

    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("asset_import.handle_not_found")
    );
    let active = DurableAssetImportOperationRepository::new(root.clone())
        .list_active(&WorkspaceId::new("workspace-1").expect("workspace"), 10)
        .expect("active operations");
    assert!(active.is_empty());
    assert!(!root.join("staging/assets").exists());
    assert!(!root.join("assets/objects").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_import_runtime_startup_recovers_active_staging_operation() {
    let root = temp_root("startup-recovery");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let operation_id = AssetImportOperationId::new("interrupted-op").expect("operation id");
    let mut operation = AssetImportOperation::new(
        operation_id.clone(),
        workspace.clone(),
        DocumentId::new("doc-1").expect("document"),
        10,
    )
    .expect("operation");
    operation.apply(AssetImportEvent::Begin, 0).expect("begin");
    operation
        .apply(AssetImportEvent::ValidationSucceeded, 0)
        .expect("validated");
    DurableAssetImportOperationRepository::new(root.clone())
        .create(operation)
        .expect("create operation");
    let mut staging = LocalAssetStagingWriter::new(root.clone());
    staging
        .begin(&workspace, &operation_id)
        .expect("staging begin");
    staging
        .append(&workspace, &operation_id, 0, b"partial")
        .expect("staging append");

    let _runtime =
        DesktopAssetImportSelectionRuntime::with_app_data_root(root.clone(), "workspace-1", 4)
            .expect("runtime recovery");

    let recovered = DurableAssetImportOperationRepository::new(root.clone())
        .get(&operation_id)
        .expect("get")
        .expect("operation");
    assert_eq!(recovered.state(), AssetImportState::Cancelled);
    assert!(
        !root
            .join("staging/assets")
            .join(hex("workspace-1"))
            .join(format!("{}.part", hex("interrupted-op")))
            .exists()
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_import_runtime_start_returns_durable_selected_status_before_execution() {
    let root = temp_root("start-status");
    seed_document(&root, "doc-1");
    let source = root.join("source.txt");
    fs::write(&source, b"status-content").expect("source");
    let runtime =
        DesktopAssetImportSelectionRuntime::with_app_data_root(root.clone(), "workspace-1", 4)
            .expect("runtime");
    let selected = runtime.register_selected_paths(vec![source]);
    let request = DesktopAssetImportRequestDto {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        handle: selected.data.expect("selection").files[0].handle.clone(),
        label: "Source".into(),
    };

    let started = runtime.start(request.clone());
    assert!(started.ok);
    assert_eq!(started.state.as_deref(), Some("selected"));
    let operation_id = started.operation_id.expect("operation id");
    assert_eq!(
        runtime
            .status("workspace-1", &operation_id)
            .state
            .as_deref(),
        Some("selected")
    );

    let completed = runtime.run_started(request, &operation_id);
    assert!(completed.ok, "completed={completed:?}");
    assert_eq!(
        runtime
            .status("workspace-1", &operation_id)
            .state
            .as_deref(),
        Some("completed")
    );
    let _ = fs::remove_dir_all(root);
}

fn seed_document(root: &std::path::Path, id: &str) {
    let document_id = DocumentId::new(id).expect("document id");
    let metadata = DocumentMetadata::new(
        document_id.clone(),
        DocumentTitle::new("Asset Host").expect("title"),
        DocumentPath::new("asset-host.md").expect("path"),
    )
    .expect("metadata");
    let snapshot = CurrentDocumentSnapshot::new(
        document_id,
        DocumentBody::new(
            "# Asset Host",
            DocumentBodyPolicy::new(1024).expect("body policy"),
        )
        .expect("body"),
    );
    let mut repository = LocalDocumentRepository::new(root.join("authoring-current"));
    repository
        .put_current(
            &WorkspaceId::new("workspace-1").expect("workspace"),
            CurrentDocumentRecord::new(metadata, snapshot).expect("record"),
        )
        .expect("put document");
}

fn temp_root(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "sponzey-asset-operation-runtime-{name}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("root");
    root
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
