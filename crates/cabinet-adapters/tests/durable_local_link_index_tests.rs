use cabinet_adapters::durable_local_link_index::DurableLocalLinkIndex;
use cabinet_domain::document::{DocumentId, DocumentSlug, DocumentTitle};
use cabinet_domain::link::{Backlink, DocumentLink, LinkTarget, SourceRange};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::link_index::{
    BacklinkPageReader, BacklinkPageRequest, LinkIndex, LinkIndexError, LinkProjectionRecord,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn durable_link_index_survives_restart_and_replaces_removed_relations() {
    let temp = Temp::new("restart");
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let source = DocumentId::new("doc-1").unwrap();
    let mut writer = DurableLocalLinkIndex::new(temp.path.clone());
    writer
        .replace_document_links(&workspace, record(true))
        .unwrap();
    drop(writer);
    let mut restarted = DurableLocalLinkIndex::new(temp.path.clone());
    assert_eq!(
        restarted
            .list_backlinks(&workspace, &DocumentId::new("doc-2").unwrap())
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        restarted.list_unresolved_links(&workspace).unwrap().len(),
        1
    );
    restarted
        .replace_document_links(
            &workspace,
            LinkProjectionRecord::new(source, vec![], vec![]).unwrap(),
        )
        .unwrap();
    drop(restarted);
    let final_reader = DurableLocalLinkIndex::new(temp.path.clone());
    assert!(
        final_reader
            .list_backlinks(&workspace, &DocumentId::new("doc-2").unwrap())
            .unwrap()
            .is_empty()
    );
    assert!(
        final_reader
            .list_unresolved_links(&workspace)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn durable_link_index_isolates_workspaces_and_reports_missing() {
    let temp = Temp::new("isolation");
    let a = WorkspaceId::new("a").unwrap();
    let b = WorkspaceId::new("b").unwrap();
    let source = DocumentId::new("doc-1").unwrap();
    let mut store = DurableLocalLinkIndex::new(temp.path.clone());
    store.replace_document_links(&a, record(true)).unwrap();
    assert!(store.get_document_links(&b, &source).unwrap().is_none());
    assert_eq!(store.list_unresolved_links(&b).unwrap(), vec![]);
}

#[test]
fn durable_link_index_delete_is_idempotent_and_survives_restart() {
    let temp = Temp::new("delete");
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let source = DocumentId::new("doc-1").unwrap();
    let mut store = DurableLocalLinkIndex::new(temp.path.clone());
    store
        .replace_document_links(&workspace, record(true))
        .unwrap();
    store.delete_document_links(&workspace, &source).unwrap();
    store.delete_document_links(&workspace, &source).unwrap();
    drop(store);

    assert!(
        DurableLocalLinkIndex::new(temp.path.clone())
            .get_document_links(&workspace, &source)
            .unwrap()
            .is_none()
    );
}

#[test]
fn durable_link_index_rejects_corrupt_and_unknown_schema_without_raw_content() {
    let temp = Temp::new("corrupt");
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let source = DocumentId::new("doc-1").unwrap();
    let mut store = DurableLocalLinkIndex::new(temp.path.clone());
    store
        .replace_document_links(&workspace, record(true))
        .unwrap();
    let path = find(&temp.path);
    fs::write(&path, "schema\t999\nprivate body\n").unwrap();
    let error = store.get_document_links(&workspace, &source).unwrap_err();
    assert_eq!(error, LinkIndexError::CorruptedProjection);
    assert!(!format!("{error:?}").contains("private body"));
}

#[test]
fn durable_link_index_pages_backlinks_without_materializing_every_projection() {
    let temp = Temp::new("page");
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let target = DocumentId::new("target-doc").unwrap();
    let mut store = DurableLocalLinkIndex::new(temp.path.clone());
    for source_index in 0..3 {
        let source = DocumentId::new(&format!("source-{source_index}")).unwrap();
        let backlinks = (0..3)
            .map(|offset| {
                Backlink::new(
                    source.clone(),
                    target.clone(),
                    SourceRange::new(offset, offset + 1).unwrap(),
                )
            })
            .collect();
        store
            .replace_document_links(
                &workspace,
                LinkProjectionRecord::new(source, backlinks, vec![]).unwrap(),
            )
            .unwrap();
    }
    drop(store);
    let reader = DurableLocalLinkIndex::new(temp.path.clone());
    let first = reader
        .list_backlinks_page(&workspace, &target, BacklinkPageRequest::new(0, 5).unwrap())
        .unwrap();
    assert_eq!(first.records().len(), 5);
    assert_eq!(first.next_offset(), Some(5));
    let second = reader
        .list_backlinks_page(&workspace, &target, BacklinkPageRequest::new(5, 5).unwrap())
        .unwrap();
    assert_eq!(second.records().len(), 4);
    assert_eq!(second.next_offset(), None);
}
fn record(with_links: bool) -> LinkProjectionRecord {
    let source = DocumentId::new("doc-1").unwrap();
    let range = SourceRange::new(1, 4).unwrap();
    let backlinks = if with_links {
        vec![Backlink::new(
            source.clone(),
            DocumentId::new("doc-2").unwrap(),
            range,
        )]
    } else {
        vec![]
    };
    let unresolved = if with_links {
        vec![DocumentLink::new(
            source.clone(),
            LinkTarget::unresolved(
                DocumentSlug::from_title(&DocumentTitle::new("Missing").unwrap()).unwrap(),
            ),
            range,
        )]
    } else {
        vec![]
    };
    LinkProjectionRecord::new(source, backlinks, unresolved).unwrap()
}
fn find(root: &PathBuf) -> PathBuf {
    let workspace = fs::read_dir(root.join("link-projections"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::read_dir(workspace)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
}
struct Temp {
    path: PathBuf,
}
impl Temp {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cabinet-link-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
