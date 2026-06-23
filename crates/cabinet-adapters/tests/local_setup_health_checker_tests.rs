use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_first_run::{FIRST_RUN_MARKER_FILE, LocalFirstRunStore};
use cabinet_adapters::local_setup_health::{
    LocalSetupHealthChecker, LocalSetupHealthIssue, LocalSetupHealthIssueKind,
    LocalSetupHealthRole, LocalSetupHealthStatus,
};
use cabinet_core::config::{AppConfig, ExternalEnvironmentSnapshot};
use cabinet_core::first_run::{FirstRunInitializer, FirstRunState};

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
            "sponzey-cabinet-health-{test_name}-{}-{nanos}",
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

#[test]
fn local_setup_health_checker_reports_healthy_first_run_profile() {
    let profile = TempProfile::new("healthy");
    let config = profile.app_config();
    run_first_run(&config);

    let report = LocalSetupHealthChecker::new(config.local_paths.clone()).check();

    assert_eq!(report.status(), LocalSetupHealthStatus::Healthy);
    assert!(report.issues().is_empty());
}

#[test]
fn local_setup_health_checker_reports_missing_required_directory() {
    let profile = TempProfile::new("missing-dir");
    let config = profile.app_config();
    run_first_run(&config);
    fs::remove_dir_all(&config.local_paths.asset_store_dir).expect("remove asset dir");

    let report = LocalSetupHealthChecker::new(config.local_paths.clone()).check();

    assert_eq!(report.status(), LocalSetupHealthStatus::Unhealthy);
    assert!(report.issues().contains(&LocalSetupHealthIssue::new(
        LocalSetupHealthRole::AssetStore,
        LocalSetupHealthIssueKind::MissingDirectory,
    )));
}

#[test]
fn local_setup_health_checker_reports_path_that_is_not_directory() {
    let profile = TempProfile::new("path-file");
    let config = profile.app_config();
    run_first_run(&config);
    fs::remove_dir_all(&config.local_paths.search_index_dir).expect("remove search dir");
    fs::write(&config.local_paths.search_index_dir, b"not a directory").expect("write file");

    let report = LocalSetupHealthChecker::new(config.local_paths.clone()).check();

    assert!(report.issues().contains(&LocalSetupHealthIssue::new(
        LocalSetupHealthRole::SearchIndex,
        LocalSetupHealthIssueKind::PathIsNotDirectory,
    )));
}

#[test]
fn local_setup_health_checker_reports_missing_first_run_marker() {
    let profile = TempProfile::new("missing-marker");
    let config = profile.app_config();
    run_first_run(&config);
    fs::remove_file(config.local_paths.metadata_dir.join(FIRST_RUN_MARKER_FILE))
        .expect("remove marker");

    let report = LocalSetupHealthChecker::new(config.local_paths.clone()).check();

    assert!(report.issues().contains(&LocalSetupHealthIssue::new(
        LocalSetupHealthRole::Metadata,
        LocalSetupHealthIssueKind::MissingFirstRunMarker,
    )));
}

fn run_first_run(config: &AppConfig) {
    let outcome =
        FirstRunInitializer::new(config.clone()).initialize(&mut LocalFirstRunStore::new());
    assert_eq!(outcome.final_state, FirstRunState::Completed);
}
