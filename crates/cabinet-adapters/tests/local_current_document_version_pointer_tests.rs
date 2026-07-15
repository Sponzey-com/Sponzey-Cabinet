use std::fs;

use cabinet_adapters::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use cabinet_domain::document::DocumentId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};

#[test]
fn local_pointer_compare_and_set_persists_across_restart() {
    let root = temp_root("restart");
    let workspace = workspace("workspace-1");
    let document = document("doc-1");
    let mut pointer = LocalCurrentDocumentVersionPointer::new(root.clone());

    assert_eq!(
        pointer
            .load_current_version(&workspace, &document)
            .expect("missing"),
        None
    );
    pointer
        .compare_and_set_current_version(&workspace, &document, None, version("v1"))
        .expect("initial pointer");
    let mut restarted = LocalCurrentDocumentVersionPointer::new(root);
    assert_eq!(
        restarted
            .load_current_version(&workspace, &document)
            .expect("restart pointer")
            .as_ref()
            .map(VersionId::as_str),
        Some("v1")
    );
    restarted
        .compare_and_set_current_version(&workspace, &document, Some(&version("v1")), version("v2"))
        .expect("advance pointer");
    assert_eq!(
        restarted
            .load_current_version(&workspace, &document)
            .expect("advanced")
            .expect("version")
            .as_str(),
        "v2"
    );
}

#[test]
fn local_pointer_rejects_wrong_expected_without_mutation() {
    let root = temp_root("conflict");
    let workspace = workspace("workspace-1");
    let document = document("doc-1");
    let mut pointer = LocalCurrentDocumentVersionPointer::new(root);
    pointer
        .compare_and_set_current_version(&workspace, &document, None, version("v1"))
        .expect("initial");

    let duplicate_create = pointer
        .compare_and_set_current_version(&workspace, &document, None, version("v2"))
        .expect_err("duplicate create conflicts");
    let stale_update = pointer
        .compare_and_set_current_version(
            &workspace,
            &document,
            Some(&version("stale")),
            version("v2"),
        )
        .expect_err("stale update conflicts");

    assert_eq!(
        duplicate_create,
        CurrentDocumentVersionPointerError::Conflict
    );
    assert_eq!(stale_update, CurrentDocumentVersionPointerError::Conflict);
    assert_eq!(
        pointer
            .load_current_version(&workspace, &document)
            .expect("load")
            .expect("version")
            .as_str(),
        "v1"
    );
}

#[test]
fn local_pointer_isolates_workspace_and_document_and_hides_raw_ids() {
    let root = temp_root("isolation");
    let mut pointer = LocalCurrentDocumentVersionPointer::new(root.clone());
    for (workspace_id, document_id, version_id) in [
        ("workspace-a", "doc-1", "v-a1"),
        ("workspace-a", "doc-2", "v-a2"),
        ("workspace-b", "doc-1", "v-b1"),
    ] {
        pointer
            .compare_and_set_current_version(
                &workspace(workspace_id),
                &document(document_id),
                None,
                version(version_id),
            )
            .expect("set pointer");
    }

    assert_eq!(
        pointer
            .load_current_version(&workspace("workspace-a"), &document("doc-2"))
            .expect("load")
            .expect("version")
            .as_str(),
        "v-a2"
    );
    let paths = walk_paths(&root);
    assert!(paths.iter().all(|path| !path.contains("workspace-a")));
    assert!(paths.iter().all(|path| !path.contains("doc-1")));
}

#[test]
fn local_pointer_rejects_corrupt_snapshot_without_leaking_content() {
    let root = temp_root("corrupt");
    let workspace = workspace("workspace-1");
    let document = document("doc-1");
    let mut pointer = LocalCurrentDocumentVersionPointer::new(root.clone());
    pointer
        .compare_and_set_current_version(&workspace, &document, None, version("v1"))
        .expect("set");
    let snapshot = find_snapshot(&root);
    fs::write(
        &snapshot,
        "schema=99\nversion=/Users/private/raw document body\n",
    )
    .expect("corrupt");

    let error = pointer
        .load_current_version(&workspace, &document)
        .expect_err("corrupt pointer");

    assert_eq!(error, CurrentDocumentVersionPointerError::CorruptedPointer);
    assert_eq!(error.code(), "current_document_version.corrupted");
    assert!(!format!("{error:?}").contains("/Users/"));
}

fn workspace(value: &str) -> WorkspaceId {
    WorkspaceId::new(value).expect("workspace")
}

fn document(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document")
}

fn version(value: &str) -> VersionId {
    VersionId::new(value).expect("version")
}

fn find_snapshot(root: &std::path::Path) -> std::path::PathBuf {
    fs::read_dir(root)
        .expect("workspace dirs")
        .flat_map(|entry| fs::read_dir(entry.expect("workspace").path()).expect("document dirs"))
        .flat_map(|entry| fs::read_dir(entry.expect("document").path()).expect("files"))
        .map(|entry| entry.expect("file").path())
        .find(|path| path.extension().is_some_and(|value| value == "pointer"))
        .expect("pointer snapshot")
}

fn walk_paths(root: &std::path::Path) -> Vec<String> {
    let mut pending = vec![root.to_path_buf()];
    let mut paths = Vec::new();
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path).expect("read dir") {
            let entry = entry.expect("entry");
            let path = entry.path();
            paths.push(path.to_string_lossy().to_string());
            if path.is_dir() {
                pending.push(path);
            }
        }
    }
    paths
}

fn temp_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "sponzey-current-version-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ))
}
