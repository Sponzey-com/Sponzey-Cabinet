use std::fs;
use std::path::PathBuf;

use cabinet_platform::release_smoke::{CleanInstallSmokeInput, run_clean_install_smoke};

#[test]
fn clean_install_smoke_initializes_local_profile_once_without_external_services() {
    let root = unique_root("clean-install");

    let report = run_clean_install_smoke(CleanInstallSmokeInput::new(root.clone()))
        .expect("clean install smoke");

    assert!(report.completed());
    assert!(report.healthy());
    assert_eq!(report.created_directories(), 5);
    assert_eq!(report.already_present_directories(), 0);
    assert!(root.join("metadata").is_dir());
    assert!(root.join("version-store").is_dir());
    assert!(root.join("assets").is_dir());
    assert!(root.join("search-index").is_dir());
    assert!(root.join("workspaces").is_dir());

    fs::remove_dir_all(root).ok();
}

fn unique_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("sponzey-cabinet-{label}-{}", std::process::id()));
    fs::remove_dir_all(&root).ok();
    root
}
