use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_association_catalog::DurableAssetAssociationCatalog;
use cabinet_domain::asset::{AssetAssociation, AssetId};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::{
    AssetAssociationCatalog, AssetAssociationLinkOutcome, AssetAssociationUnlinkOutcome,
};

#[test]
fn association_catalog_links_same_asset_to_two_documents_and_survives_restart() {
    let root = temp_root();
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let asset = AssetId::from_sha256_hex(&"a".repeat(64)).expect("asset");
    let mut catalog = DurableAssetAssociationCatalog::new(root.clone());
    assert_eq!(
        catalog
            .link(&workspace, association(&asset, "doc-1"))
            .expect("link one"),
        AssetAssociationLinkOutcome::Linked
    );
    assert_eq!(
        catalog
            .link(&workspace, association(&asset, "doc-2"))
            .expect("link two"),
        AssetAssociationLinkOutcome::Linked
    );
    assert_eq!(
        catalog
            .link(&workspace, association(&asset, "doc-1"))
            .expect("duplicate"),
        AssetAssociationLinkOutcome::AlreadyLinked
    );
    let mut restarted = DurableAssetAssociationCatalog::new(root.clone());
    assert_eq!(
        restarted
            .reference_count(&workspace, &asset)
            .expect("count"),
        2
    );
    assert_eq!(
        restarted
            .list_documents(&workspace, &asset, 10)
            .expect("documents")
            .len(),
        2
    );
    assert_eq!(
        restarted
            .list_assets(&workspace, &DocumentId::new("doc-1").expect("doc"), 10)
            .expect("assets")
            .len(),
        1
    );
    assert_eq!(
        restarted
            .unlink(&workspace, &asset, &DocumentId::new("doc-1").expect("doc"))
            .expect("unlink"),
        AssetAssociationUnlinkOutcome::Unlinked
    );
    assert_eq!(
        restarted
            .unlink(&workspace, &asset, &DocumentId::new("doc-1").expect("doc"))
            .expect("idempotent"),
        AssetAssociationUnlinkOutcome::NotLinked
    );
    assert_eq!(
        restarted
            .reference_count(&workspace, &asset)
            .expect("remaining"),
        1
    );
    let _ = fs::remove_dir_all(root);
}

fn association(asset: &AssetId, document: &str) -> AssetAssociation {
    AssetAssociation::new(
        asset.clone(),
        DocumentId::new(document).expect("document"),
        "Reference",
    )
    .expect("association")
}
fn temp_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "sponzey-associations-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    root
}
