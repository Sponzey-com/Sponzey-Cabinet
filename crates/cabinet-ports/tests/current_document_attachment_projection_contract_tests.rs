use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::document::DocumentId;
use cabinet_domain::version::DocumentRevisionNumber;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_attachment_projection::{
    CurrentDocumentAttachmentProjectionError, CurrentDocumentAttachmentProjectionRequest,
};

#[test]
fn projection_request_accepts_sorted_full_set_and_empty_set() {
    let request = CurrentDocumentAttachmentProjectionRequest::new(
        workspace(),
        document(),
        DocumentRevisionNumber::new(2).unwrap(),
        vec![reference('a'), reference('b')],
    )
    .expect("sorted request");
    assert_eq!(request.references(), &[reference('a'), reference('b')]);

    let empty = CurrentDocumentAttachmentProjectionRequest::new(
        workspace(),
        document(),
        DocumentRevisionNumber::new(3).unwrap(),
        Vec::new(),
    )
    .expect("explicit empty replacement");
    assert!(empty.references().is_empty());
}

#[test]
fn projection_request_rejects_unsorted_and_duplicate_asset_ids() {
    for references in [
        vec![reference('b'), reference('a')],
        vec![reference('a'), reference('a')],
    ] {
        let error = CurrentDocumentAttachmentProjectionRequest::new(
            workspace(),
            document(),
            DocumentRevisionNumber::new(2).unwrap(),
            references,
        )
        .unwrap_err();
        assert_eq!(
            error,
            CurrentDocumentAttachmentProjectionError::InvalidRequest
        );
    }
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").unwrap()
}

fn document() -> DocumentId {
    DocumentId::new("doc-1").unwrap()
}

fn reference(character: char) -> AssetReference {
    let label = character.to_string();
    AssetReference::new(
        AssetId::from_sha256_hex(&std::iter::repeat_n(character, 64).collect::<String>()).unwrap(),
        &label,
    )
    .unwrap()
}
