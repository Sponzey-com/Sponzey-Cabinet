use cabinet_adapters::local_link_index::LocalLinkIndex;
use cabinet_domain::document::{DocumentId, DocumentSlug, DocumentTitle};
use cabinet_domain::link::{Backlink, DocumentLink, LinkTarget, SourceRange};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::link_index::{LinkIndex, LinkProjectionRecord};

#[test]
fn local_link_index_replaces_source_projection_and_queries_backlinks() {
    let workspace = workspace_id();
    let source = document_id("source-doc");
    let target = document_id("target-doc");
    let mut index = LocalLinkIndex::default();

    index
        .replace_document_links(
            &workspace,
            LinkProjectionRecord::new(
                source.clone(),
                vec![Backlink::new(
                    source.clone(),
                    target.clone(),
                    SourceRange::new(0, 10).expect("range"),
                )],
                Vec::new(),
            )
            .expect("record"),
        )
        .expect("replace");
    index
        .replace_document_links(
            &workspace,
            LinkProjectionRecord::new(source, Vec::new(), Vec::new()).expect("record"),
        )
        .expect("replace with empty projection");

    let backlinks = index
        .list_backlinks(&workspace, &target)
        .expect("backlinks");

    assert!(backlinks.is_empty());
}

#[test]
fn local_link_index_returns_source_projection_by_document_id() {
    let workspace = workspace_id();
    let source = document_id("source-doc");
    let target = document_id("target-doc");
    let mut index = LocalLinkIndex::default();

    index
        .replace_document_links(
            &workspace,
            LinkProjectionRecord::new(
                source.clone(),
                vec![Backlink::new(
                    source.clone(),
                    target,
                    SourceRange::new(0, 10).expect("range"),
                )],
                Vec::new(),
            )
            .expect("record"),
        )
        .expect("replace");

    let projection = index
        .get_document_links(&workspace, &source)
        .expect("get projection")
        .expect("projection");

    assert_eq!(projection.source_document_id(), &source);
    assert_eq!(projection.backlinks().len(), 1);
}

#[test]
fn local_link_index_queries_unresolved_links_and_orphan_documents() {
    let workspace = workspace_id();
    let source = document_id("source-doc");
    let target = document_id("target-doc");
    let orphan = document_id("orphan-doc");
    let mut index = LocalLinkIndex::default();

    index
        .replace_document_links(
            &workspace,
            LinkProjectionRecord::new(
                source.clone(),
                vec![Backlink::new(
                    source.clone(),
                    target.clone(),
                    SourceRange::new(0, 10).expect("range"),
                )],
                vec![DocumentLink::new(
                    source,
                    LinkTarget::unresolved(slug("Missing Page")),
                    SourceRange::new(20, 34).expect("range"),
                )],
            )
            .expect("record"),
        )
        .expect("replace");

    let unresolved = index.list_unresolved_links(&workspace).expect("unresolved");
    let orphans = index
        .list_orphan_documents(&workspace, &[target.clone(), orphan.clone()])
        .expect("orphans");

    assert_eq!(unresolved.len(), 1);
    assert_eq!(orphans, vec![orphan]);
}

fn workspace_id() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace id")
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}

fn slug(title: &str) -> DocumentSlug {
    DocumentSlug::from_title(&DocumentTitle::new(title).expect("title")).expect("slug")
}
