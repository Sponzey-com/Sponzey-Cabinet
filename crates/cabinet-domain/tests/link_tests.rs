use cabinet_domain::document::{DocumentId, DocumentPath, DocumentSlug};
use cabinet_domain::link::{
    Backlink, DocumentLink, LinkError, LinkStatus, LinkTarget, SourceRange,
};

#[test]
fn document_link_can_target_resolved_document_path_with_source_range() {
    let link = DocumentLink::new(
        DocumentId::new("source-doc").expect("source id"),
        LinkTarget::resolved(DocumentPath::new("docs/target.md").expect("target path")),
        SourceRange::new(12, 30).expect("range"),
    );

    assert_eq!(link.source_document_id().as_str(), "source-doc");
    assert_eq!(link.status(), LinkStatus::Resolved);
    assert_eq!(link.source_range().start(), 12);
    assert_eq!(link.source_range().end(), 30);
}

#[test]
fn document_link_can_hold_unresolved_target_slug() {
    let slug = DocumentSlug::from_title(
        &cabinet_domain::document::DocumentTitle::new("Missing Page").expect("title"),
    )
    .expect("slug");
    let link = DocumentLink::new(
        DocumentId::new("source-doc").expect("source id"),
        LinkTarget::unresolved(slug),
        SourceRange::new(0, 14).expect("range"),
    );

    assert_eq!(link.status(), LinkStatus::Unresolved);
}

#[test]
fn source_range_rejects_empty_or_reversed_range() {
    assert_eq!(
        SourceRange::new(5, 5).expect_err("empty range must fail"),
        LinkError::InvalidSourceRange
    );
    assert_eq!(
        SourceRange::new(8, 2).expect_err("reversed range must fail"),
        LinkError::InvalidSourceRange
    );
}

#[test]
fn backlink_points_from_source_document_to_target_document() {
    let backlink = Backlink::new(
        DocumentId::new("source-doc").expect("source id"),
        DocumentId::new("target-doc").expect("target id"),
        SourceRange::new(3, 18).expect("range"),
    );

    assert_eq!(backlink.source_document_id().as_str(), "source-doc");
    assert_eq!(backlink.target_document_id().as_str(), "target-doc");
    assert_eq!(backlink.source_range().start(), 3);
}
