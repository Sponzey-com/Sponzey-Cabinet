use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_first_run::LocalFirstRunStore;
use cabinet_adapters::local_migration::{
    LocalMigrationStore, MIGRATION_LOCK_FILE, MIGRATION_VERSIONS_FILE,
};
use cabinet_core::config::{AppConfig, ExternalEnvironmentSnapshot};
use cabinet_core::first_run::{FirstRunInitializer, FirstRunState};
use cabinet_core::migration::{
    MigrationPlan, MigrationProductEvent, MigrationRunner, MigrationState, MigrationVersion,
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
            "sponzey-cabinet-migration-{test_name}-{}-{nanos}",
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

fn run_first_run(config: &AppConfig) {
    let outcome =
        FirstRunInitializer::new(config.clone()).initialize(&mut LocalFirstRunStore::new());
    assert_eq!(outcome.final_state, FirstRunState::Completed);
}

#[test]
fn local_migration_store_records_initial_noop_version_after_first_run() {
    let profile = TempProfile::new("record");
    let config = profile.app_config();
    run_first_run(&config);
    let mut store = LocalMigrationStore::new(config.local_paths.metadata_dir.clone());
    let runner = MigrationRunner::new(MigrationPlan::initial());

    let outcome = runner.run(&mut store);

    assert_eq!(outcome.final_state, MigrationState::Completed);
    assert_eq!(outcome.applied_versions, vec![MigrationVersion::new(1)]);
    assert_eq!(
        outcome.product_event,
        MigrationProductEvent::MigrationCompleted {
            applied_versions: vec![MigrationVersion::new(1)],
        }
    );
    assert_eq!(
        fs::read_to_string(
            config
                .local_paths
                .metadata_dir
                .join(MIGRATION_VERSIONS_FILE)
        )
        .expect("versions file should exist"),
        "1\n"
    );
    assert!(
        !config
            .local_paths
            .metadata_dir
            .join(MIGRATION_LOCK_FILE)
            .exists()
    );
}

#[test]
fn local_migration_store_rerun_does_not_duplicate_initial_version() {
    let profile = TempProfile::new("rerun");
    let config = profile.app_config();
    run_first_run(&config);

    let runner = MigrationRunner::new(MigrationPlan::initial());
    let first = runner.run(&mut LocalMigrationStore::new(
        config.local_paths.metadata_dir.clone(),
    ));
    let second = runner.run(&mut LocalMigrationStore::new(
        config.local_paths.metadata_dir.clone(),
    ));

    assert_eq!(first.final_state, MigrationState::Completed);
    assert_eq!(second.final_state, MigrationState::Completed);
    assert!(second.applied_versions.is_empty());
    assert_eq!(
        fs::read_to_string(
            config
                .local_paths
                .metadata_dir
                .join(MIGRATION_VERSIONS_FILE)
        )
        .expect("versions file should exist"),
        "1\n"
    );
}
