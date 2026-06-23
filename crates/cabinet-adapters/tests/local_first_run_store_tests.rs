use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_first_run::{FIRST_RUN_MARKER_FILE, LocalFirstRunStore};
use cabinet_core::config::{AppConfig, ExternalEnvironmentSnapshot};
use cabinet_core::first_run::{
    FirstRunInitializer, FirstRunProductEvent, FirstRunState, FirstRunStoreStatus,
};

struct TempProfile {
    path: PathBuf,
}

impl TempProfile {
    fn new(test_name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = PathBuf::from("/tmp").join(format!(
            "sponzey-cabinet-{test_name}-{}-{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        Self { path }
    }

    fn app_config(&self) -> AppConfig {
        let app_data_dir = self.path.to_string_lossy();
        let snapshot = ExternalEnvironmentSnapshot::from_pairs([(
            "SPONZEY_CABINET_APP_DATA_DIR",
            &app_data_dir,
        )]);
        AppConfig::from_environment_snapshot(snapshot).expect("temp config should be valid")
    }
}

impl Drop for TempProfile {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn assert_dir_exists(path: &Path) {
    assert!(path.is_dir(), "directory should exist: {}", path.display());
}

#[test]
fn local_first_run_store_creates_clean_temp_profile() {
    let profile = TempProfile::new("clean");
    let config = profile.app_config();
    let mut store = LocalFirstRunStore::new();
    let initializer = FirstRunInitializer::new(config.clone());

    let outcome = initializer.initialize(&mut store);

    assert_eq!(outcome.final_state, FirstRunState::Completed);
    assert_eq!(
        outcome.product_event,
        FirstRunProductEvent::FirstRunCompleted
    );
    assert_eq!(outcome.created_directories, 5);
    assert_eq!(outcome.already_present_directories, 0);
    assert_eq!(outcome.metadata_status, FirstRunStoreStatus::Created);
    assert_dir_exists(&config.local_paths.metadata_dir);
    assert_dir_exists(&config.local_paths.version_store_dir);
    assert_dir_exists(&config.local_paths.asset_store_dir);
    assert_dir_exists(&config.local_paths.search_index_dir);
    assert_dir_exists(&config.local_paths.workspace_root);
    assert!(
        config
            .local_paths
            .metadata_dir
            .join(FIRST_RUN_MARKER_FILE)
            .is_file()
    );
}

#[test]
fn local_first_run_store_is_idempotent_for_existing_temp_profile() {
    let profile = TempProfile::new("rerun");
    let config = profile.app_config();
    let initializer = FirstRunInitializer::new(config);

    let first = initializer.initialize(&mut LocalFirstRunStore::new());
    let second = initializer.initialize(&mut LocalFirstRunStore::new());

    assert_eq!(first.final_state, FirstRunState::Completed);
    assert_eq!(second.final_state, FirstRunState::Completed);
    assert_eq!(second.created_directories, 0);
    assert_eq!(second.already_present_directories, 5);
    assert_eq!(second.metadata_status, FirstRunStoreStatus::AlreadyPresent);
}
