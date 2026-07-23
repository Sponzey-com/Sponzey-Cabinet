use cabinet_adapters::durable_document_link_catalog::DurableDocumentLinkCatalog;
use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_link_catalog::{DocumentLinkCatalog, DocumentLinkCatalogRecord};
use cabinet_ports::link_target_resolver::{
    DocumentLinkTargetResolver, LinkTargetResolution, LinkTargetResolverError,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn catalog_resolves_exact_title_slug_and_path_after_restart() {
    let t = Temp::new("restart");
    let w = WorkspaceId::new("w").unwrap();
    let mut c = DurableDocumentLinkCatalog::new(t.path.clone());
    c.upsert(&w, record("d1", "Known Document", "notes/known.md"))
        .unwrap();
    drop(c);
    let r = DurableDocumentLinkCatalog::new(t.path.clone());
    for target in ["Known Document", "known-document", "notes/known.md"] {
        match r.resolve(&w, target).unwrap() {
            LinkTargetResolution::Resolved(v) => assert_eq!(v.document_id().as_str(), "d1"),
            _ => panic!("resolved"),
        }
    }
}
#[test]
fn catalog_returns_unresolved_ambiguous_and_isolates_workspaces() {
    let t = Temp::new("ambiguity");
    let w = WorkspaceId::new("w").unwrap();
    let other = WorkspaceId::new("other").unwrap();
    let mut c = DurableDocumentLinkCatalog::new(t.path.clone());
    c.upsert(&w, record("d1", "Same", "a.md")).unwrap();
    c.upsert(&w, record("d2", "Same", "b.md")).unwrap();
    assert_eq!(
        c.resolve(&w, "Same"),
        Err(LinkTargetResolverError::Ambiguous)
    );
    assert!(matches!(
        c.resolve(&w, "Missing").unwrap(),
        LinkTargetResolution::Unresolved(_)
    ));
    assert!(matches!(
        c.resolve(&other, "Same").unwrap(),
        LinkTargetResolution::Unresolved(_)
    ));
}

#[test]
fn catalog_resolves_relative_paths_from_source_and_rejects_workspace_escape() {
    let t = Temp::new("relative");
    let workspace = WorkspaceId::new("w").unwrap();
    let mut catalog = DurableDocumentLinkCatalog::new(t.path.clone());
    catalog
        .upsert(
            &workspace,
            record("source", "Source", "area/current/source.md"),
        )
        .unwrap();
    catalog
        .upsert(
            &workspace,
            record("sibling", "Sibling", "area/current/sibling.md"),
        )
        .unwrap();
    catalog
        .upsert(
            &workspace,
            record("shared", "Shared", "area/shared/note.md"),
        )
        .unwrap();
    catalog
        .upsert(
            &workspace,
            record("duplicate", "Duplicate", "other/sibling.md"),
        )
        .unwrap();

    for (target, expected) in [
        ("sibling.md", "sibling"),
        ("../shared/note.md#details", "shared"),
    ] {
        match catalog
            .resolve_relative(&workspace, &DocumentId::new("source").unwrap(), target)
            .unwrap()
        {
            LinkTargetResolution::Resolved(value) => {
                assert_eq!(value.document_id().as_str(), expected)
            }
            _ => panic!("expected resolved relative target"),
        }
    }
    assert!(matches!(
        catalog
            .resolve_relative(
                &workspace,
                &DocumentId::new("source").unwrap(),
                "missing.md",
            )
            .unwrap(),
        LinkTargetResolution::Unresolved(_)
    ));
    assert_eq!(
        catalog.resolve_relative(
            &workspace,
            &DocumentId::new("source").unwrap(),
            "../../../outside.md",
        ),
        Err(LinkTargetResolverError::InvalidTarget)
    );
}

#[test]
fn catalog_remove_is_idempotent_isolated_and_durable_across_restart() {
    let t = Temp::new("remove");
    let workspace = WorkspaceId::new("w").unwrap();
    let other = WorkspaceId::new("other").unwrap();
    let document = DocumentId::new("d1").unwrap();
    let mut catalog = DurableDocumentLinkCatalog::new(t.path.clone());
    catalog
        .upsert(&workspace, record("d1", "Removed", "removed.md"))
        .unwrap();
    catalog
        .upsert(&workspace, record("d2", "Kept", "kept.md"))
        .unwrap();
    catalog
        .upsert(&other, record("d1", "Other", "other.md"))
        .unwrap();

    assert!(catalog.remove(&workspace, &document).unwrap());
    assert!(!catalog.remove(&workspace, &document).unwrap());
    drop(catalog);

    let restarted = DurableDocumentLinkCatalog::new(t.path.clone());
    assert!(matches!(
        restarted.resolve(&workspace, "Removed").unwrap(),
        LinkTargetResolution::Unresolved(_)
    ));
    assert!(matches!(
        restarted.resolve(&workspace, "Kept").unwrap(),
        LinkTargetResolution::Resolved(_)
    ));
    assert!(matches!(
        restarted.resolve(&other, "Other").unwrap(),
        LinkTargetResolution::Resolved(_)
    ));
}
#[test]
fn catalog_rejects_corruption_without_raw_content() {
    let t = Temp::new("corrupt");
    let w = WorkspaceId::new("w").unwrap();
    let mut c = DurableDocumentLinkCatalog::new(t.path.clone());
    c.upsert(&w, record("d1", "Known", "a.md")).unwrap();
    let p = fs::read_dir(t.path.join("document-link-catalog"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(p, "schema\t999\nprivate title").unwrap();
    assert_eq!(
        c.resolve(&w, "Known"),
        Err(LinkTargetResolverError::Unavailable)
    );
}
fn record(id: &str, title: &str, path: &str) -> DocumentLinkCatalogRecord {
    DocumentLinkCatalogRecord::new(
        DocumentId::new(id).unwrap(),
        DocumentTitle::new(title).unwrap(),
        DocumentPath::new(path).unwrap(),
    )
    .unwrap()
}
struct Temp {
    path: PathBuf,
}
impl Temp {
    fn new(l: &str) -> Self {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("cabinet-catalog-{l}-{}-{n}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
