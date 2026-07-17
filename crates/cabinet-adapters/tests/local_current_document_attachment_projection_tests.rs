use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_association_catalog::DurableAssetAssociationCatalog;
use cabinet_adapters::local_current_document_attachment_projection::{
    LOCAL_CURRENT_DOCUMENT_ATTACHMENT_PROJECTION_MARKER_FILE,
    LOCAL_CURRENT_DOCUMENT_ATTACHMENT_PROJECTION_ROOT, LocalCurrentDocumentAttachmentProjection,
};
use cabinet_domain::asset::{AssetAssociation, AssetId, AssetReference};
use cabinet_domain::document::DocumentId;
use cabinet_domain::version::DocumentRevisionNumber;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::AssetAssociationCatalog;
use cabinet_ports::current_document_attachment_projection::{
    CurrentDocumentAttachmentProjectionError, CurrentDocumentAttachmentProjectionOutcome,
    CurrentDocumentAttachmentProjectionRequest, CurrentDocumentAttachmentProjectionWriter,
};

#[test]
fn initial_apply_survives_restart_and_becomes_already_current() {
    let temp = TempRoot::new("initial");
    let request = request(2, vec![reference('a', "A"), reference('b', "B")]);

    let applied = LocalCurrentDocumentAttachmentProjection::new(temp.path.clone())
        .replace_current_document_attachments(request.clone())
        .expect("apply");
    assert_eq!(applied, CurrentDocumentAttachmentProjectionOutcome::Applied);
    assert_associations(&temp, &[('a', "A"), ('b', "B")]);

    let restarted = LocalCurrentDocumentAttachmentProjection::new(temp.path.clone())
        .replace_current_document_attachments(request)
        .expect("restart");
    assert_eq!(
        restarted,
        CurrentDocumentAttachmentProjectionOutcome::AlreadyCurrent
    );
    assert_associations(&temp, &[('a', "A"), ('b', "B")]);
}

#[test]
fn newer_full_set_relabels_removes_and_links_while_stale_request_conflicts() {
    let temp = TempRoot::new("replace");
    let mut projection = LocalCurrentDocumentAttachmentProjection::new(temp.path.clone());
    projection
        .replace_current_document_attachments(request(
            2,
            vec![reference('a', "Old A"), reference('b', "B")],
        ))
        .unwrap();

    projection
        .replace_current_document_attachments(request(
            3,
            vec![reference('a', "New A"), reference('c', "C")],
        ))
        .expect("replace");
    assert_associations(&temp, &[('a', "New A"), ('c', "C")]);

    let stale = projection
        .replace_current_document_attachments(request(2, vec![reference('b', "B")]))
        .unwrap_err();
    assert_eq!(stale, CurrentDocumentAttachmentProjectionError::Conflict);
    assert_associations(&temp, &[('a', "New A"), ('c', "C")]);
}

#[test]
fn applying_marker_resumes_after_partial_catalog_failure() {
    let temp = TempRoot::new("resume");
    let mut projection = LocalCurrentDocumentAttachmentProjection::new(temp.path.clone());
    projection
        .replace_current_document_attachments(request(1, vec![reference('a', "A")]))
        .unwrap();
    let blocked_asset_root = association_asset_root(&temp, 'b');
    fs::create_dir_all(blocked_asset_root.parent().unwrap()).unwrap();
    fs::write(&blocked_asset_root, b"block directory creation").unwrap();
    let desired = request(2, vec![reference('b', "B")]);

    let failed = projection
        .replace_current_document_attachments(desired.clone())
        .unwrap_err();
    assert_eq!(
        failed,
        CurrentDocumentAttachmentProjectionError::StorageUnavailable
    );
    assert!(marker_path(&temp).exists());

    fs::remove_file(blocked_asset_root).unwrap();
    let resumed = LocalCurrentDocumentAttachmentProjection::new(temp.path.clone())
        .replace_current_document_attachments(desired.clone())
        .expect("resume applying");
    assert_eq!(resumed, CurrentDocumentAttachmentProjectionOutcome::Applied);
    assert_associations(&temp, &[('b', "B")]);

    let ready = LocalCurrentDocumentAttachmentProjection::new(temp.path.clone())
        .replace_current_document_attachments(desired)
        .unwrap();
    assert_eq!(
        ready,
        CurrentDocumentAttachmentProjectionOutcome::AlreadyCurrent
    );
}

#[test]
fn corrupted_marker_stops_before_catalog_mutation() {
    let temp = TempRoot::new("corrupt");
    let mut catalog = DurableAssetAssociationCatalog::new(temp.path.clone());
    catalog
        .link(&workspace(), association('a', "A"))
        .expect("seed association");
    fs::create_dir_all(marker_path(&temp).parent().unwrap()).unwrap();
    fs::write(marker_path(&temp), b"not-json").unwrap();

    let error = LocalCurrentDocumentAttachmentProjection::new(temp.path.clone())
        .replace_current_document_attachments(request(2, Vec::new()))
        .unwrap_err();

    assert_eq!(
        error,
        CurrentDocumentAttachmentProjectionError::CorruptedProjection
    );
    assert_associations(&temp, &[('a', "A")]);
}

fn request(
    revision: u64,
    references: Vec<AssetReference>,
) -> CurrentDocumentAttachmentProjectionRequest {
    CurrentDocumentAttachmentProjectionRequest::new(
        workspace(),
        document(),
        DocumentRevisionNumber::new(revision).unwrap(),
        references,
    )
    .unwrap()
}

fn assert_associations(temp: &TempRoot, expected: &[(char, &str)]) {
    let associations = DurableAssetAssociationCatalog::new(temp.path.clone())
        .list_assets(&workspace(), &document(), 500)
        .expect("list associations");
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

fn association(character: char, label: &str) -> AssetAssociation {
    AssetAssociation::new(asset_id(character), document(), label).unwrap()
}

fn reference(character: char, label: &str) -> AssetReference {
    AssetReference::new(asset_id(character), label).unwrap()
}

fn asset_id(character: char) -> AssetId {
    AssetId::from_sha256_hex(&std::iter::repeat_n(character, 64).collect::<String>()).unwrap()
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").unwrap()
}

fn document() -> DocumentId {
    DocumentId::new("doc-1").unwrap()
}

fn marker_path(temp: &TempRoot) -> PathBuf {
    temp.path
        .join(LOCAL_CURRENT_DOCUMENT_ATTACHMENT_PROJECTION_ROOT)
        .join(hex("workspace-1"))
        .join(hex("doc-1"))
        .join(LOCAL_CURRENT_DOCUMENT_ATTACHMENT_PROJECTION_MARKER_FILE)
}

fn association_asset_root(temp: &TempRoot, character: char) -> PathBuf {
    temp.path
        .join("assets/associations")
        .join(hex("workspace-1"))
        .join("by-asset")
        .join(asset_id(character).as_str())
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
            "sponzey-current-attachment-projection-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.path);
    }
}

fn remove_dir_all(path: &Path) -> std::io::Result<()> {
    fs::remove_dir_all(path)
}
