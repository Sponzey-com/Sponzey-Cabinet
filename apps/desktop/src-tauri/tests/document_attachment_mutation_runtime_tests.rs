use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_association_catalog::DurableAssetAssociationCatalog;
use cabinet_adapters::durable_asset_metadata_catalog::DurableAssetMetadataCatalog;
use cabinet_adapters::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT,
};
use cabinet_adapters::local_current_document_revision_projection::LocalCurrentDocumentRevisionProjectionWriter;
use cabinet_adapters::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_desktop_shell::{
    DesktopDocumentAttachmentMutationRequestDto, DesktopDocumentAttachmentMutationRuntime,
};
use cabinet_domain::asset::{
    AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType,
    AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor,
    VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::AssetAssociationCatalog;
use cabinet_ports::asset_metadata_catalog::AssetMetadataCatalog;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
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
