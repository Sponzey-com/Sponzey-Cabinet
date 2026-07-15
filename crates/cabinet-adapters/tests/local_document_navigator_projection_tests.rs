use std::fs;

use cabinet_adapters::local_document_navigator_projection::LocalDocumentNavigatorProjectionStore;
use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_navigator::{
    DocumentNavigatorItem, DocumentNavigatorProjectionError, DocumentNavigatorProjectionPort,
    DocumentNavigatorProjectionQuery, NavigatorViewKind,
};

#[test]
fn local_navigator_projection_persists_restart_deduplicates_and_caps_items() {
    let root = temp_root("restart-capacity");
    let workspace = workspace("workspace-1");
    let store = LocalDocumentNavigatorProjectionStore::new(root.clone(), 3).expect("store");
    store
        .replace_workspace_items(
            &workspace,
            vec![
                item("doc-1", "One", "a/one.md", false, 3, &["work"], &["rust"]),
                item(
                    "doc-1",
                    "Duplicate",
                    "z/duplicate.md",
                    true,
                    0,
                    &["work"],
                    &["rust"],
                ),
                item("doc-2", "Two", "a/two.md", true, 1, &["work"], &["rust"]),
                item(
                    "doc-3",
                    "Three",
                    "b/three.md",
                    false,
                    2,
                    &["personal"],
                    &["notes"],
                ),
                item(
                    "doc-4",
                    "Four",
                    "c/four.md",
                    false,
                    4,
                    &["personal"],
                    &["notes"],
                ),
            ],
        )
        .expect("replace projection");

    let restarted = LocalDocumentNavigatorProjectionStore::new(root.clone(), 3).expect("restart");
    let page = restarted
        .load_navigator_page(
            &workspace,
            &query(NavigatorViewKind::Tree, None, None, 0, 100),
        )
        .expect("tree page");

    assert_eq!(page.items().len(), 3);
    assert_eq!(page.items()[0].document_id(), "doc-1");
    assert_eq!(page.items()[0].title(), "One");
    assert_eq!(page.items()[1].document_id(), "doc-2");
    let entries = fs::read_dir(root.join("navigator-projections"))
        .expect("projection directory")
        .collect::<Result<Vec<_>, _>>()
        .expect("entries");
    assert_eq!(entries.len(), 1);
    assert!(
        !entries[0]
            .file_name()
            .to_string_lossy()
            .contains("workspace-1")
    );
}

#[test]
fn local_navigator_projection_filters_all_view_kinds_with_stable_order() {
    let root = temp_root("views");
    let workspace = workspace("workspace-1");
    let store = LocalDocumentNavigatorProjectionStore::new(root, 10).expect("store");
    store
        .replace_workspace_items(
            &workspace,
            vec![
                item(
                    "doc-b",
                    "Beta",
                    "work/beta.md",
                    true,
                    2,
                    &["work"],
                    &["rust"],
                ),
                item(
                    "doc-a",
                    "Alpha",
                    "work/alpha.md",
                    false,
                    1,
                    &["work"],
                    &["design"],
                ),
                item(
                    "doc-c",
                    "Gamma",
                    "home/gamma.md",
                    false,
                    3,
                    &["personal"],
                    &["rust"],
                ),
            ],
        )
        .expect("replace projection");

    let tree = store
        .load_navigator_page(
            &workspace,
            &query(NavigatorViewKind::Tree, None, None, 0, 10),
        )
        .expect("tree");
    let collection = store
        .load_navigator_page(
            &workspace,
            &query(NavigatorViewKind::Collection, Some("work"), None, 0, 10),
        )
        .expect("collection");
    let tag = store
        .load_navigator_page(
            &workspace,
            &query(NavigatorViewKind::Tag, Some("rust"), None, 0, 10),
        )
        .expect("tag");
    let recent = store
        .load_navigator_page(
            &workspace,
            &query(NavigatorViewKind::Recent, None, None, 0, 10),
        )
        .expect("recent");
    let favorite = store
        .load_navigator_page(
            &workspace,
            &query(NavigatorViewKind::Favorite, None, Some("beta"), 0, 10),
        )
        .expect("favorite");

    assert_eq!(ids(&tree), vec!["doc-c", "doc-a", "doc-b"]);
    assert_eq!(ids(&collection), vec!["doc-a", "doc-b"]);
    assert_eq!(ids(&tag), vec!["doc-c", "doc-b"]);
    assert_eq!(ids(&recent), vec!["doc-a", "doc-b", "doc-c"]);
    assert_eq!(ids(&favorite), vec!["doc-b"]);
}

#[test]
fn local_navigator_projection_pages_results_and_isolates_workspaces() {
    let root = temp_root("pagination-isolation");
    let store = LocalDocumentNavigatorProjectionStore::new(root, 10).expect("store");
    let workspace_a = workspace("workspace-a");
    let workspace_b = workspace("workspace-b");
    store
        .replace_workspace_items(
            &workspace_a,
            vec![
                item("doc-a1", "A1", "a/1.md", false, 1, &[], &[]),
                item("doc-a2", "A2", "a/2.md", false, 2, &[], &[]),
                item("doc-a3", "A3", "a/3.md", false, 3, &[], &[]),
            ],
        )
        .expect("workspace a");
    store
        .replace_workspace_items(
            &workspace_b,
            vec![item("doc-b1", "B1", "b/1.md", false, 1, &[], &[])],
        )
        .expect("workspace b");

    let first = store
        .load_navigator_page(
            &workspace_a,
            &query(NavigatorViewKind::Tree, None, None, 0, 2),
        )
        .expect("first page");
    let second = store
        .load_navigator_page(
            &workspace_a,
            &query(NavigatorViewKind::Tree, None, None, 2, 2),
        )
        .expect("second page");
    let other = store
        .load_navigator_page(
            &workspace_b,
            &query(NavigatorViewKind::Tree, None, None, 0, 10),
        )
        .expect("other workspace");

    assert_eq!(ids(&first), vec!["doc-a1", "doc-a2"]);
    assert_eq!(first.next_offset(), Some(2));
    assert_eq!(ids(&second), vec!["doc-a3"]);
    assert_eq!(second.next_offset(), None);
    assert_eq!(ids(&other), vec!["doc-b1"]);
}

#[test]
fn local_navigator_projection_returns_empty_missing_and_rejects_corruption() {
    let root = temp_root("empty-corrupt");
    let workspace = workspace("workspace-1");
    let store = LocalDocumentNavigatorProjectionStore::new(root.clone(), 10).expect("store");
    let missing = store
        .load_navigator_page(
            &workspace,
            &query(NavigatorViewKind::Tree, None, None, 0, 10),
        )
        .expect("missing is empty");
    assert!(missing.items().is_empty());

    store
        .replace_workspace_items(
            &workspace,
            vec![item(
                "doc-1",
                "Private title",
                "notes/private.md",
                false,
                1,
                &[],
                &[],
            )],
        )
        .expect("projection");
    let snapshot = fs::read_dir(root.join("navigator-projections"))
        .expect("directory")
        .next()
        .expect("entry")
        .expect("path")
        .path();
    let encoded = fs::read_to_string(&snapshot).expect("encoded");
    assert!(!encoded.contains("Private title"));
    assert!(!encoded.contains("notes/private.md"));
    assert!(!encoded.contains("raw document body"));
    fs::write(snapshot, "schema\t99\nitem\tbroken\n").expect("corrupt");

    let error = store
        .load_navigator_page(
            &workspace,
            &query(NavigatorViewKind::Tree, None, None, 0, 10),
        )
        .expect_err("corrupt projection");
    assert_eq!(error, DocumentNavigatorProjectionError::CorruptedProjection);
}

fn query(
    view: NavigatorViewKind,
    view_key: Option<&str>,
    filter: Option<&str>,
    offset: u32,
    limit: u16,
) -> DocumentNavigatorProjectionQuery {
    DocumentNavigatorProjectionQuery::new(view, view_key, filter, offset, limit).expect("query")
}

fn item(
    id: &str,
    title: &str,
    path: &str,
    favorite: bool,
    recent_rank: u64,
    collections: &[&str],
    tags: &[&str],
) -> DocumentNavigatorItem {
    DocumentNavigatorItem::new(
        DocumentId::new(id).expect("id"),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
        collections
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        tags.iter().map(|value| (*value).to_string()).collect(),
        favorite,
        recent_rank,
    )
    .expect("item")
}

fn ids(page: &cabinet_ports::document_navigator::DocumentNavigatorPage) -> Vec<&str> {
    page.items()
        .iter()
        .map(DocumentNavigatorItem::document_id)
        .collect()
}

fn workspace(id: &str) -> WorkspaceId {
    WorkspaceId::new(id).expect("workspace")
}

fn temp_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "sponzey-document-navigator-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ))
}
