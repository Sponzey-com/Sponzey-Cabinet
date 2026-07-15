use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_atomic_file::atomic_temp_path;
use cabinet_adapters::local_first_run::LocalFirstRunStore;
use cabinet_adapters::local_migration::{
    LocalMigrationStore, MIGRATION_LOCK_FILE, MIGRATION_VERSIONS_FILE,
};
use cabinet_core::config::{AppConfig, ExternalEnvironmentSnapshot};
use cabinet_core::first_run::{FirstRunInitializer, FirstRunState};
use cabinet_core::migration::{
    MigrationErrorCode, MigrationPlan, MigrationProductEvent, MigrationRunner, MigrationState,
    MigrationStore, MigrationVersion,
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

#[test]
fn local_migration_store_rejects_malformed_duplicate_and_out_of_order_ledgers() {
    for (name, content) in [
        ("malformed", "1\nnot-a-version\n"),
        ("duplicate", "1\n1\n"),
        ("out-of-order", "2\n1\n"),
        ("blank-record", "1\n\n2\n"),
    ] {
        let profile = TempProfile::new(name);
        let config = profile.app_config();
        run_first_run(&config);
        let ledger = config
            .local_paths
            .metadata_dir
            .join(MIGRATION_VERSIONS_FILE);
        fs::write(&ledger, content).expect("invalid fixture ledger should be written");
        let mut store = LocalMigrationStore::new(config.local_paths.metadata_dir.clone());

        assert_eq!(
            store.applied_versions(),
            Err(MigrationErrorCode::VersionReadFailed),
            "{name} ledger must fail closed"
        );
        assert_eq!(
            fs::read_to_string(&ledger).expect("ledger should remain readable"),
            content,
            "{name} ledger must not be rewritten"
        );
    }
}

#[test]
fn local_migration_store_atomically_replaces_valid_ledger_and_discards_stale_staging() {
    let profile = TempProfile::new("atomic-record");
    let config = profile.app_config();
    run_first_run(&config);
    let ledger = config
        .local_paths
        .metadata_dir
        .join(MIGRATION_VERSIONS_FILE);
    let staging = atomic_temp_path(&ledger).expect("ledger should have an atomic staging path");
    fs::write(&ledger, "1\n").expect("initial ledger should be written");
    fs::write(&staging, "1\n999\n").expect("stale staging should be written");
    let mut store = LocalMigrationStore::new(config.local_paths.metadata_dir.clone());

    store
        .record_version(MigrationVersion::new(2))
        .expect("valid next version should be recorded");

    assert_eq!(
        fs::read_to_string(&ledger).expect("ledger should remain readable"),
        "1\n2\n"
    );
    assert!(!staging.exists(), "stale staging must not survive commit");
}

#[test]
fn local_migration_store_refuses_to_overwrite_invalid_ledger_when_recording() {
    let profile = TempProfile::new("invalid-record");
    let config = profile.app_config();
    run_first_run(&config);
    let ledger = config
        .local_paths
        .metadata_dir
        .join(MIGRATION_VERSIONS_FILE);
    fs::write(&ledger, "1\ncorrupt\n").expect("invalid ledger should be written");
    let mut store = LocalMigrationStore::new(config.local_paths.metadata_dir.clone());

    assert_eq!(
        store.record_version(MigrationVersion::new(2)),
        Err(MigrationErrorCode::VersionRecordFailed)
    );
    assert_eq!(
        fs::read_to_string(&ledger).expect("ledger should remain readable"),
        "1\ncorrupt\n"
    );
}

#[test]
fn local_migration_runner_releases_lock_after_malformed_ledger_failure() {
    let profile = TempProfile::new("malformed-runner-cleanup");
    let config = profile.app_config();
    run_first_run(&config);
    let ledger = config
        .local_paths
        .metadata_dir
        .join(MIGRATION_VERSIONS_FILE);
    let lock = config.local_paths.metadata_dir.join(MIGRATION_LOCK_FILE);
    fs::write(&ledger, "1\ninvalid\n").expect("invalid ledger should be written");

    let outcome = MigrationRunner::new(MigrationPlan::initial()).run(
        &mut LocalMigrationStore::new(config.local_paths.metadata_dir.clone()),
    );

    assert_eq!(
        outcome.final_state,
        MigrationState::Failed {
            error_code: MigrationErrorCode::VersionReadFailed,
            retryable: true,
        }
    );
    assert!(!lock.exists(), "failed migration must release its lock");
    assert_eq!(
        fs::read_to_string(ledger).expect("invalid ledger should remain readable"),
        "1\ninvalid\n"
    );
}
