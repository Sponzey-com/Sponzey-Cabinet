use std::fs;
use std::path::PathBuf;

use cabinet_adapters::local_document_asset_repository::LocalDocumentAssetRepository;
use cabinet_domain::asset::{
    AssetFileName, AssetId, AssetMediaType, AssetMetadata, AssetReference,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_asset_repository::{
    DocumentAssetAttachOutcome, DocumentAssetRecord, DocumentAssetRepository,
};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn local_document_asset_repository_attaches_and_lists_metadata_without_object_bytes() {
    let root = unique_root("attach-list");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let record = document_asset_record(HASH_A, "diagram.png", "image/png", "Diagram");
    let mut repository = LocalDocumentAssetRepository::new(root.clone());

    let outcome = repository
        .attach_asset(&workspace_id, &document_id, record.clone())
        .expect("attach");
    let listed = repository
        .list_assets(&workspace_id, &document_id)
        .expect("list");

    assert_eq!(outcome, DocumentAssetAttachOutcome::Attached);
    assert_eq!(listed, vec![record]);

    fs::remove_dir_all(root).ok();
}

#[test]
fn local_document_asset_repository_returns_already_attached_without_duplicate() {
    let root = unique_root("dedupe");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let record = document_asset_record(HASH_A, "diagram.png", "image/png", "Diagram");
    let mut repository = LocalDocumentAssetRepository::new(root.clone());

    repository
        .attach_asset(&workspace_id, &document_id, record.clone())
        .expect("first attach");
    let outcome = repository
        .attach_asset(&workspace_id, &document_id, record)
        .expect("second attach");
    let listed = repository
        .list_assets(&workspace_id, &document_id)
        .expect("list");

    assert_eq!(outcome, DocumentAssetAttachOutcome::AlreadyAttached);
    assert_eq!(listed.len(), 1);

    fs::remove_dir_all(root).ok();
}

#[test]
fn local_document_asset_repository_isolates_assets_by_document() {
    let root = unique_root("isolate");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let doc_a = DocumentId::new("doc-1").expect("document id");
    let doc_b = DocumentId::new("doc-2").expect("document id");
    let mut repository = LocalDocumentAssetRepository::new(root.clone());

    repository
        .attach_asset(
            &workspace_id,
            &doc_a,
            document_asset_record(HASH_A, "a.png", "image/png", "A"),
        )
        .expect("attach a");
    repository
        .attach_asset(
            &workspace_id,
            &doc_b,
            document_asset_record(HASH_B, "b.png", "image/png", "B"),
        )
        .expect("attach b");

    let listed = repository.list_assets(&workspace_id, &doc_a).expect("list");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].asset_id().as_str(), HASH_A);

    fs::remove_dir_all(root).ok();
}

fn document_asset_record(
    hash: &str,
    file_name: &str,
    media_type: &str,
    label: &str,
) -> DocumentAssetRecord {
    let asset_id = AssetId::from_sha256_hex(hash).expect("asset id");
    let metadata = AssetMetadata::new(
        asset_id.clone(),
        AssetFileName::new(file_name).expect("file name"),
        AssetMediaType::new(media_type).expect("media type"),
        4,
    )
    .expect("metadata");
    let reference = AssetReference::new(asset_id, label).expect("reference");
    DocumentAssetRecord::new(reference, metadata).expect("record")
}

fn unique_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "sponzey-cabinet-local-document-assets-{label}-{}",
        std::process::id()
    ));
    fs::remove_dir_all(&root).ok();
    root
}
