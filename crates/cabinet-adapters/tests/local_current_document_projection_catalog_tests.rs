use std::fs;

use cabinet_adapters::local_current_document_projection_catalog::LocalCurrentDocumentProjectionCatalog;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_projection_catalog::CurrentDocumentProjectionCatalog;

#[test]
fn catalog_lists_stable_document_and_current_version_identities() {
    let root =
        std::env::temp_dir().join(format!("cabinet-projection-catalog-{}", std::process::id()));
    let workspace = hex("workspace-1");
    for (document, version) in [("doc-2", "v2"), ("doc-1", "v1")] {
        fs::create_dir_all(
            root.join("authoring-current")
                .join("workspace-1")
                .join("documents/by-id")
                .join(document),
        )
        .unwrap();
        let pointer = root
            .join("authoring-current-version")
            .join(&workspace)
            .join(hex(document))
            .join("current.pointer");
        fs::create_dir_all(pointer.parent().unwrap()).unwrap();
        fs::write(pointer, format!("schema=1\nversion={}\n", hex(version))).unwrap();
    }
    let output = LocalCurrentDocumentProjectionCatalog::new(root.clone())
        .list_current_projection_identities(&WorkspaceId::new("workspace-1").unwrap(), 10)
        .expect("list");
    assert_eq!(
        output
            .iter()
            .map(|item| (item.document_id().as_str(), item.version_id().as_str()))
            .collect::<Vec<_>>(),
        [("doc-1", "v1"), ("doc-2", "v2")]
    );
    let _ = fs::remove_dir_all(root);
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
