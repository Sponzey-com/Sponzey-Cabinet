use cabinet_adapters::local_asset_external_opener::{
    ExternalPathLauncher, LocalAssetExternalOpener,
};
use cabinet_domain::asset::{AssetFileName, AssetId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_external_open::{AssetExternalOpenError, AssetExternalOpener};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[test]
fn local_opener_materializes_a_read_only_named_copy_before_launch() {
    let root = unique_root("success");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let bytes = b"safe asset";
    let digest = format!("{:x}", Sha256::digest(bytes));
    let asset = AssetId::from_sha256_hex(&digest).expect("asset");
    let source = object_path(&root, &workspace, &asset);
    fs::create_dir_all(source.parent().expect("parent")).expect("object parent");
    fs::write(&source, bytes).expect("object");
    let launcher = RecordingLauncher::default();
    let observed = launcher.0.clone();
    let opener = LocalAssetExternalOpener::with_launcher(root.clone(), launcher);

    opener
        .open(
            &workspace,
            &asset,
            &AssetFileName::new("design notes.txt").expect("name"),
        )
        .expect("open");
    opener
        .open(
            &workspace,
            &asset,
            &AssetFileName::new("design notes.txt").expect("name"),
        )
        .expect("repeat open replaces the read-only cache copy");

    let launched = opener_path(&root, &workspace, &asset, "design notes.txt");
    assert_eq!(fs::read(&launched).expect("copy"), bytes);
    assert!(fs::metadata(&launched)
        .expect("metadata")
        .permissions()
        .readonly());
    assert_eq!(
        observed.lock().expect("path").clone().expect("launched"),
        launched
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn local_opener_rejects_missing_and_non_file_objects_before_launch() {
    let root = unique_root("invalid");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let asset = AssetId::from_sha256_hex(&"a".repeat(64)).expect("asset");
    let opener =
        LocalAssetExternalOpener::with_launcher(root.clone(), RecordingLauncher::default());
    assert_eq!(
        opener.open(
            &workspace,
            &asset,
            &AssetFileName::new("file.txt").expect("name")
        ),
        Err(AssetExternalOpenError::NotFound)
    );

    let source = object_path(&root, &workspace, &asset);
    fs::create_dir_all(&source).expect("directory object");
    assert_eq!(
        opener.open(
            &workspace,
            &asset,
            &AssetFileName::new("file.txt").expect("name")
        ),
        Err(AssetExternalOpenError::Corrupted)
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[derive(Clone, Default)]
struct RecordingLauncher(Arc<Mutex<Option<PathBuf>>>);

impl ExternalPathLauncher for RecordingLauncher {
    fn launch(&self, path: &Path) -> Result<(), ()> {
        *self.0.lock().expect("path") = Some(path.to_path_buf());
        Ok(())
    }
}

fn unique_root(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("cabinet-asset-open-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

fn object_path(root: &Path, workspace: &WorkspaceId, asset: &AssetId) -> PathBuf {
    root.join("assets/objects")
        .join(hex(workspace.as_str()))
        .join(&asset.as_str()[..2])
        .join(format!("{}.bin", asset.as_str()))
}

fn opener_path(root: &Path, workspace: &WorkspaceId, asset: &AssetId, name: &str) -> PathBuf {
    root.join("asset-open-cache")
        .join(hex(workspace.as_str()))
        .join(asset.as_str())
        .join(name)
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
