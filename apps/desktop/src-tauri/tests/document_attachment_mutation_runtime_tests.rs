use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_association_catalog::DurableAssetAssociationCatalog;
use cabinet_adapters::durable_asset_metadata_catalog::DurableAssetMetadataCatalog;
use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_adapters::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT,
};
use cabinet_adapters::local_current_document_revision_projection::LocalCurrentDocumentRevisionProjectionWriter;
use cabinet_adapters::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_desktop_shell::{
    DesktopAssetDetailRequestDto, DesktopCanvasRequestDto, DesktopCanvasRuntime,
    DesktopDocumentAssetsRuntime, DesktopDocumentAttachmentMutationRequestDto,
    DesktopDocumentAttachmentMutationRuntime, DesktopDocumentAuthoringRequestDto,
    DesktopDocumentAuthoringRuntime, DesktopDocumentMutationRequestDto,
    DesktopDocumentMutationRuntime, DesktopLocalCommandPayloadDto, DesktopLocalCommandRequestDto,
    DesktopProjectionRuntime,
};
use cabinet_domain::asset::{
    AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType,
    AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::graph::GraphEdgeKind;
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor,
    VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::AssetAssociationCatalog;
use cabinet_ports::asset_metadata_catalog::AssetMetadataCatalog;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::graph_projection::GraphProjectionStore;
use cabinet_ports::version_store::{
    HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
};
use cabinet_usecases::project_current_document_revision::{
    ProjectCurrentDocumentRevisionInput, ProjectCurrentDocumentRevisionUsecase,
};

#[test]
fn desktop_link_is_revisioned_and_same_operation_replays_without_internal_token_response() {
    let temp = TempRoot::new("link-replay");
    seed_current(&temp);
    seed_asset(&temp, 'a', "spec.pdf");
    let runtime = runtime(&temp);
    let request = link_request("operation-link", "version-1", 'a', "설계 자료");

    let fresh = runtime.execute(request.clone());
    assert!(fresh.ok, "fresh={fresh:?}");
    assert_eq!(fresh.outcome.as_deref(), Some("fresh"));
    assert_eq!(fresh.delta.as_deref(), Some("linked"));
    assert_eq!(fresh.revision_number, Some(2));
    assert_eq!(history_count(&temp), 2);
    assert_associations(&temp, &[('a', "설계 자료")]);

    let replayed = runtime.execute(request);
    assert!(replayed.ok, "replayed={replayed:?}");
    assert_eq!(replayed.outcome.as_deref(), Some("replayed"));
    assert_eq!(history_count(&temp), 2);
    let json = serde_json::to_string(&replayed).unwrap();
    for forbidden in ["version-", "snapshot", "notes/", "첫 번째 문서"] {
        assert!(!json.contains(forbidden), "forbidden {forbidden}: {json}");
    }
}

#[test]
fn desktop_unlink_uses_current_guard_and_preserves_asset_metadata() {
    let temp = TempRoot::new("unlink");
    seed_current(&temp);
    seed_asset(&temp, 'a', "spec.pdf");
    let runtime = runtime(&temp);
    assert!(
        runtime
            .execute(link_request("operation-link", "version-1", 'a', "A"))
            .ok
    );
    let expected = current_version(&temp);

    let unlinked = runtime.execute(DesktopDocumentAttachmentMutationRequestDto::Unlink {
        operation_id: "operation-unlink".into(),
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        expected_current_version_token: expected.as_str().to_string(),
        asset_id: asset_id('a').as_str().to_string(),
    });

    assert!(unlinked.ok, "unlinked={unlinked:?}");
    assert_eq!(unlinked.delta.as_deref(), Some("unlinked"));
    assert_eq!(unlinked.revision_number, Some(3));
    assert_associations(&temp, &[]);
    assert!(
        DurableAssetMetadataCatalog::new(temp.path.clone())
            .get(&workspace(), &asset_id('a'))
            .unwrap()
            .is_some()
    );
}

#[test]
fn shared_asset_unlink_preserves_other_document_object_and_canvas_across_restart() {
    let temp = TempRoot::new("shared-unlink-restart");
    seed_current(&temp);
    let document_mutations = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).unwrap();
    let second = document_mutations.execute(DesktopDocumentMutationRequestDto::Create {
        operation_id: "create-doc-2".into(),
        workspace_id: "workspace-1".into(),
        document_id: "doc-2".into(),
        body: "두 번째 문서\n공유 첨부를 유지합니다.\n".into(),
        author: "local-user".into(),
        summary: "두 번째 문서 생성".into(),
    });
    assert!(second.ok, "second={second:?}");
    let second_version = second.data.expect("second data").current_version_id;
    seed_asset(&temp, 'a', "spec.pdf");
    seed_asset_object(&temp, 'a');
    let original_asset_bytes = fs::read(asset_object_path(&temp, 'a')).unwrap();

    let mutations = runtime(&temp);
    let linked_first = mutations.execute(link_request(
        "shared-link-doc-1",
        "version-1",
        'a',
        "공유 설계 자료",
    ));
    assert!(linked_first.ok, "linked_first={linked_first:?}");
    let linked_second = mutations.execute(DesktopDocumentAttachmentMutationRequestDto::Link {
        operation_id: "shared-link-doc-2".into(),
        workspace_id: "workspace-1".into(),
        document_id: "doc-2".into(),
        expected_current_version_token: second_version,
        asset_id: asset_id('a').as_str().into(),
        label: "두 번째 문서의 공유 자료".into(),
    });
    assert!(linked_second.ok, "linked_second={linked_second:?}");

    let canvas = DesktopCanvasRuntime::new(temp.path.clone()).unwrap();
    assert!(
        canvas
            .execute(DesktopCanvasRequestDto::Create {
                workspace_id: "workspace-1".into(),
                canvas_id: "shared-canvas".into(),
                title: "공유 첨부 지도".into(),
            })
            .ok
    );
    let canvas_asset = canvas.execute(DesktopCanvasRequestDto::AddAssetNode {
        workspace_id: "workspace-1".into(),
        canvas_id: "shared-canvas".into(),
        expected_revision: 1,
        node_id: "shared-asset-node".into(),
        asset_id: asset_id('a').as_str().into(),
        x: 20,
        y: 20,
        width: 320,
        height: 180,
        operation_id: "shared-canvas-add-asset".into(),
    });
    assert!(canvas_asset.ok, "canvas_asset={canvas_asset:?}");

    let unlinked = mutations.execute(DesktopDocumentAttachmentMutationRequestDto::Unlink {
        operation_id: "shared-unlink-doc-1".into(),
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        expected_current_version_token: current_version(&temp).as_str().into(),
        asset_id: asset_id('a').as_str().into(),
    });
    assert!(unlinked.ok, "unlinked={unlinked:?}");
    assert_eq!(unlinked.delta.as_deref(), Some("unlinked"));

    drop(canvas);
    drop(mutations);
    drop(document_mutations);

    let assets = DesktopDocumentAssetsRuntime::new(temp.path.clone(), 4096).unwrap();
    let first_assets = assets.execute(document_assets_request("doc-1"));
    assert!(first_assets.ok, "first_assets={first_assets:?}");
    assert!(first_assets.data.unwrap().assets.is_empty());
    let second_assets = assets.execute(document_assets_request("doc-2"));
    assert!(second_assets.ok, "second_assets={second_assets:?}");
    assert_eq!(second_assets.data.unwrap().assets.len(), 1);

    let detail = assets.detail(DesktopAssetDetailRequestDto {
        workspace_id: "workspace-1".into(),
        asset_id: asset_id('a').as_str().into(),
    });
    assert!(detail.ok, "detail={detail:?}");
    let detail = detail.data.unwrap();
    assert_eq!(detail.reference_count, 1);
    assert_eq!(detail.preview_capability, "pdf");
    assert_eq!(detail.linked_document_ids, vec!["doc-2"]);
    assert_eq!(
        detail.linked_documents[0].title.as_deref(),
        Some("두 번째 문서")
    );
    assert_eq!(detail.linked_documents[0].state, "available");
    assert!(asset_object_path(&temp, 'a').is_file());
    assert_eq!(
        fs::read(asset_object_path(&temp, 'a')).unwrap(),
        original_asset_bytes
    );

    let restarted_canvas = DesktopCanvasRuntime::new(temp.path.clone()).unwrap();
    let canvas_readback = restarted_canvas.execute(DesktopCanvasRequestDto::Get {
        workspace_id: "workspace-1".into(),
        canvas_id: "shared-canvas".into(),
    });
    assert!(canvas_readback.ok, "canvas_readback={canvas_readback:?}");
    let canvas_readback = canvas_readback.data.unwrap();
    let asset_node = canvas_readback
        .nodes
        .iter()
        .find(|node| node.node_id == "shared-asset-node")
        .expect("shared asset node");
    assert_eq!(asset_node.target_status, "available");
    assert_eq!(asset_node.display_label, "spec.pdf");

    let serialized = serde_json::to_string(&(detail, canvas_readback)).unwrap();
    for forbidden in [
        temp.path.to_string_lossy().as_ref(),
        "assets/objects",
        "object_key",
    ] {
        assert!(!serialized.contains(forbidden), "forbidden={forbidden}");
    }
}

#[test]
fn stale_and_missing_asset_fail_without_new_revision() {
    let temp = TempRoot::new("failures");
    seed_current(&temp);
    seed_asset(&temp, 'a', "spec.pdf");
    let runtime = runtime(&temp);
    let linked = runtime.execute(link_request("operation-link", "version-1", 'a', "A"));
    assert!(linked.ok);
    assert_eq!(history_count(&temp), 2);

    let stale = runtime.execute(link_request("operation-stale", "version-1", 'a', "Changed"));
    assert!(!stale.ok);
    assert_eq!(
        stale.error_code.as_deref(),
        Some("DOCUMENT_ATTACHMENT_CONFLICT")
    );
    assert!(!stale.retryable);
    assert_eq!(history_count(&temp), 2);

    let missing = runtime.execute(link_request(
        "operation-missing",
        current_version(&temp).as_str(),
        'b',
        "Missing",
    ));
    assert!(!missing.ok);
    assert_eq!(
        missing.error_code.as_deref(),
        Some("DOCUMENT_ATTACHMENT_ASSET_NOT_FOUND")
    );
    assert_eq!(history_count(&temp), 2);
}

#[test]
fn attachment_restore_converges_association_graph_and_object_across_restart() {
    let temp = TempRoot::new("restore-convergence");
    seed_current(&temp);
    seed_asset(&temp, 'a', "spec.pdf");
    seed_asset_object(&temp, 'a');
    let mutations = runtime(&temp);

    let linked = mutations.execute(link_request(
        "operation-link",
        "version-1",
        'a',
        "설계 자료",
    ));
    assert!(linked.ok, "linked={linked:?}");
    let attached_version = current_version(&temp);
    let projection = DesktopProjectionRuntime::new(temp.path.clone(), 4096, 20, 3).unwrap();
    assert!(projection.run_once().ok);
    assert_eq!(attachment_edge_count(&temp), 1);

    let authoring = DesktopDocumentAuthoringRuntime::new(temp.path.clone(), 4096).unwrap();
    let restored_without_attachment =
        authoring.execute(DesktopDocumentAuthoringRequestDto::Restore {
            operation_id: "restore-without-attachment".into(),
            workspace_id: "workspace-1".into(),
            document_id: "doc-1".into(),
            target_version_id: "version-1".into(),
            expected_current_version_id: attached_version.as_str().into(),
            author: "local-user".into(),
            summary: "첨부 전 버전 복원".into(),
        });
    assert!(
        restored_without_attachment.ok,
        "restored_without_attachment={restored_without_attachment:?}"
    );
    assert!(projection.run_once().ok);
    assert_associations(&temp, &[]);
    assert_eq!(attachment_edge_count(&temp), 0);
    assert!(asset_object_path(&temp, 'a').is_file());

    let restored_without_attachment_version = current_version(&temp);
    let restored_with_attachment = authoring.execute(DesktopDocumentAuthoringRequestDto::Restore {
        operation_id: "restore-with-attachment".into(),
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        target_version_id: attached_version.as_str().into(),
        expected_current_version_id: restored_without_attachment_version.as_str().into(),
        author: "local-user".into(),
        summary: "첨부 버전 복원".into(),
    });
    assert!(
        restored_with_attachment.ok,
        "restored_with_attachment={restored_with_attachment:?}"
    );
    assert!(projection.run_once().ok);
    assert_associations(&temp, &[('a', "설계 자료")]);
    assert_eq!(attachment_edge_count(&temp), 1);
    assert!(asset_object_path(&temp, 'a').is_file());
    assert_eq!(
        authoring.restore_product_event_names(),
        vec![
            "document.restore.requested",
            "document.restore.primary_committed",
            "document.restore.completed",
            "document.restore.requested",
            "document.restore.primary_committed",
            "document.restore.completed",
        ]
    );

    drop(projection);
    drop(authoring);
    let restarted_projection =
        DesktopProjectionRuntime::new(temp.path.clone(), 4096, 20, 3).unwrap();
    assert!(restarted_projection.run_once().ok);
    assert_associations(&temp, &[('a', "설계 자료")]);
    assert_eq!(attachment_edge_count(&temp), 1);
    assert!(asset_object_path(&temp, 'a').is_file());
}

#[test]
fn attachment_projection_enqueue_failure_recovers_on_runtime_restart_without_user_retry() {
    let temp = TempRoot::new("startup-graph-recovery");
    seed_current(&temp);
    seed_asset(&temp, 'a', "spec.pdf");
    let blocker = temp.path.join("operations/projection");
    fs::create_dir_all(blocker.parent().unwrap()).unwrap();
    fs::write(&blocker, b"block projection work directory").unwrap();

    let response = runtime(&temp).execute(link_request(
        "operation-link",
        "version-1",
        'a',
        "설계 자료",
    ));
    assert!(!response.ok, "response={response:?}");
    assert_eq!(
        response.error_code.as_deref(),
        Some("DOCUMENT_ATTACHMENT_RECOVERY_REQUIRED")
    );
    assert!(response.retryable);
    assert!(response.repair_required);
    assert_eq!(history_count(&temp), 2);
    assert_associations(&temp, &[('a', "설계 자료")]);
    assert_eq!(attachment_edge_count(&temp), 0);

    fs::remove_file(blocker).unwrap();
    let _restarted_mutations = runtime(&temp);
    let projection = DesktopProjectionRuntime::new(temp.path.clone(), 4096, 20, 3).unwrap();
    assert!(projection.run_once().ok);

    assert_eq!(history_count(&temp), 2);
    assert_associations(&temp, &[('a', "설계 자료")]);
    assert_eq!(attachment_edge_count(&temp), 1);
}

fn runtime(temp: &TempRoot) -> DesktopDocumentAttachmentMutationRuntime {
    DesktopDocumentAttachmentMutationRuntime::new(temp.path.clone(), 4096).unwrap()
}

fn link_request(
    operation: &str,
    expected: &str,
    asset: char,
    label: &str,
) -> DesktopDocumentAttachmentMutationRequestDto {
    DesktopDocumentAttachmentMutationRequestDto::Link {
        operation_id: operation.to_string(),
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        expected_current_version_token: expected.to_string(),
        asset_id: asset_id(asset).as_str().to_string(),
        label: label.to_string(),
    }
}

fn document_assets_request(document_id: &str) -> DesktopLocalCommandRequestDto {
    DesktopLocalCommandRequestDto {
        command_name: "list_document_assets".into(),
        payload: DesktopLocalCommandPayloadDto::DocumentIdentity {
            workspace_id: "workspace-1".into(),
            document_id: document_id.into(),
        },
    }
}

fn seed_current(temp: &TempRoot) {
    let snapshot_ref = DocumentSnapshotRef::new("snapshot-1").unwrap();
    let entry = VersionEntry::new(
        version_id("version-1"),
        document(),
        snapshot_ref.clone(),
        VersionAuthor::new("local-user").unwrap(),
        VersionSummary::new("Seed").unwrap(),
    )
    .unwrap()
    .with_created_at_epoch_ms(1)
    .unwrap()
    .with_revision_number(DocumentRevisionNumber::new(1).unwrap())
    .unwrap();
    let record = VersionRecord::new(
        entry,
        VersionSnapshot::with_attachment_state(
            document(),
            snapshot_ref,
            DocumentBody::new("첫 번째 문서\n본문\n", body_policy()).unwrap(),
            AttachmentSnapshotState::known(Vec::new()).unwrap(),
        ),
    )
    .unwrap();
    let mut versions = LocalVersionStore::with_body_policy(
        temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT),
        body_policy(),
    );
    versions
        .append_version(&workspace(), record.clone())
        .unwrap();
    LocalCurrentDocumentVersionPointer::new(temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT))
        .compare_and_set_current_version(&workspace(), &document(), None, version_id("version-1"))
        .unwrap();
    ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            ProjectCurrentDocumentRevisionInput::new("workspace-1", "notes/original.md", record),
            &mut LocalCurrentDocumentRevisionProjectionWriter::new(
                temp.path.clone(),
                body_policy(),
            ),
        )
        .unwrap();
}

fn seed_asset(temp: &TempRoot, character: char, file_name: &str) {
    let metadata = AssetMetadata::new(
        asset_id(character),
        AssetFileName::new(file_name).unwrap(),
        AssetMediaType::new("application/pdf").unwrap(),
        42,
    )
    .unwrap();
    let record = AssetCatalogRecord::new(
        metadata,
        1,
        AssetPreviewCapability::Pdf,
        AssetExtractionStatus::NotRequested,
    )
    .unwrap();
    DurableAssetMetadataCatalog::new(temp.path.clone())
        .put(&workspace(), record)
        .unwrap();
}

fn seed_asset_object(temp: &TempRoot, character: char) {
    let path = asset_object_path(temp, character);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"asset-object").unwrap();
}

fn asset_object_path(temp: &TempRoot, character: char) -> PathBuf {
    let id = asset_id(character);
    temp.path
        .join("assets/objects")
        .join(hex("workspace-1"))
        .join(&id.as_str()[..2])
        .join(format!("{}.bin", id.as_str()))
}

fn attachment_edge_count(temp: &TempRoot) -> usize {
    DurableLocalGraphProjectionStore::new(temp.path.clone())
        .get_projection(&workspace(), &document())
        .unwrap()
        .map(|projection| {
            projection
                .graph()
                .edges()
                .iter()
                .filter(|edge| edge.kind() == GraphEdgeKind::AttachmentReference)
                .count()
        })
        .unwrap_or(0)
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn assert_associations(temp: &TempRoot, expected: &[(char, &str)]) {
    let associations = DurableAssetAssociationCatalog::new(temp.path.clone())
        .list_assets(&workspace(), &document(), 500)
        .unwrap();
    let actual = associations
        .iter()
        .map(|association| {
            (
                association.asset_id().as_str().to_string(),
                association.label().to_string(),
            )
        })
        .collect::<Vec<_>>();
    let expected = expected
        .iter()
        .map(|(asset, label)| (asset_id(*asset).as_str().to_string(), (*label).to_string()))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

fn current_version(temp: &TempRoot) -> VersionId {
    LocalCurrentDocumentVersionPointer::new(temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT))
        .load_current_version(&workspace(), &document())
        .unwrap()
        .unwrap()
}

fn history_count(temp: &TempRoot) -> usize {
    LocalVersionStore::with_body_policy(temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT), body_policy())
        .list_history(
            &workspace(),
            &document(),
            HistoryPageRequest::first(20).unwrap(),
        )
        .unwrap()
        .entries()
        .len()
}

fn asset_id(character: char) -> AssetId {
    AssetId::from_sha256_hex(&std::iter::repeat_n(character, 64).collect::<String>()).unwrap()
}

fn version_id(value: &str) -> VersionId {
    VersionId::new(value).unwrap()
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").unwrap()
}

fn document() -> DocumentId {
    DocumentId::new("doc-1").unwrap()
}

fn body_policy() -> DocumentBodyPolicy {
    DocumentBodyPolicy::new(4096).unwrap()
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-desktop-attachment-mutation-{label}-{}-{nonce}",
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
