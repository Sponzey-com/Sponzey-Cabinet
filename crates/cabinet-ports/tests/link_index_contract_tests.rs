use cabinet_domain::document::{DocumentId, DocumentPath, DocumentSlug, DocumentTitle};
use cabinet_domain::link::{Backlink, DocumentLink, LinkTarget, SourceRange};
use cabinet_ports::link_index::{LinkIndexError, LinkProjectionRecord};

#[test]
fn link_projection_record_rejects_backlink_with_different_source_document() {
    let source = document_id("source-doc");
    let backlink = Backlink::new(
        document_id("other-source"),
        document_id("target-doc"),
        SourceRange::new(0, 10).expect("range"),
    );

    let error = LinkProjectionRecord::new(source, vec![backlink], Vec::new())
        .expect_err("mismatched source must fail");

    assert_eq!(error, LinkIndexError::MismatchedSourceDocument);
    assert_eq!(error.code(), "link_index.mismatched_source_document");
}

#[test]
fn link_projection_record_rejects_resolved_link_in_unresolved_list() {
    let source = document_id("source-doc");
    let resolved_link = DocumentLink::new(
        source.clone(),
        LinkTarget::resolved(DocumentPath::new("docs/target.md").expect("path")),
        SourceRange::new(0, 10).expect("range"),
    );

    let error = LinkProjectionRecord::new(source, Vec::new(), vec![resolved_link])
        .expect_err("resolved link must fail");

    assert_eq!(error, LinkIndexError::ResolvedLinkInUnresolvedProjection);
}

#[test]
fn link_projection_record_accepts_backlinks_and_unresolved_links_for_same_source() {
    let source = document_id("source-doc");
    let record = LinkProjectionRecord::new(
        source.clone(),
        vec![Backlink::new(
            source.clone(),
            document_id("target-doc"),
            SourceRange::new(0, 10).expect("range"),
        )],
        vec![DocumentLink::new(
            source.clone(),
            LinkTarget::unresolved(slug("Missing Page")),
            SourceRange::new(20, 34).expect("range"),
        )],
    )
    .expect("record");

    assert_eq!(record.source_document_id(), &source);
    assert_eq!(record.backlinks().len(), 1);
    assert_eq!(record.unresolved_links().len(), 1);
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}

fn slug(title: &str) -> DocumentSlug {
    DocumentSlug::from_title(&DocumentTitle::new(title).expect("title")).expect("slug")
}
