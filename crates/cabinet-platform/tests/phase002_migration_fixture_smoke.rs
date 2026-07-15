use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_platform::release_smoke::{
    Phase002MigrationFixtureSmokeInput, run_phase002_migration_fixture_smoke,
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
            "sponzey-cabinet-phase002-migration-fixture-{test_name}-{}-{nanos}",
            std::process::id()
        ));
        fs::remove_dir_all(&path).ok();
        Self { path }
    }
}

impl Drop for TempProfile {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).ok();
    }
}

#[test]
fn phase002_migration_fixture_smoke_preserves_self_host_runtime_records() {
    let profile = TempProfile::new("happy-path");

    let report = run_phase002_migration_fixture_smoke(Phase002MigrationFixtureSmokeInput::new(
        profile.path.clone(),
    ))
    .expect("Phase 002 migration fixture smoke should pass");

    assert!(report.first_run_completed());
    assert!(report.initial_migration_completed());
    assert!(report.migration_idempotent());
    assert_eq!(report.fixture_record_count(), 17);
    assert!(report.required_fixture_records_preserved());
    assert!(report.migration_failure_preserved_current_fixture());
    assert!(report.product_log_sensitive_data_absent());
}
