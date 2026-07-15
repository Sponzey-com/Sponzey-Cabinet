use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_platform::release_smoke::{
    StartupRepairEvent, StartupRepairSmokeInput, StartupRepairState, run_startup_repair_smoke,
    transition_startup_repair,
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
            "sponzey-cabinet-startup-repair-{test_name}-{}-{nanos}",
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
fn startup_repair_smoke_rebuilds_corrupted_indexes_without_losing_current_workspace_data() {
    let profile = TempProfile::new("corrupted-search-index");

    let report = run_startup_repair_smoke(StartupRepairSmokeInput::new(profile.path.clone()))
        .expect("startup repair smoke should pass");

    assert!(report.first_run_completed());
    assert!(report.initial_migration_completed());
    assert!(report.corruption_detected_before_repair());
    assert!(report.startup_repair_completed());
    assert!(report.corrupted_index_rebuilt());
    assert!(report.current_document_preserved());
    assert!(report.search_result_found());
    assert!(report.product_log_sensitive_data_absent());
}

#[test]
fn startup_repair_state_machine_rejects_invalid_transition() {
    let error = transition_startup_repair(
        StartupRepairState::NotStarted,
        StartupRepairEvent::ProjectionRebuilt,
    )
    .expect_err("invalid transition should fail");

    assert_eq!(error.code(), "startup_repair.invalid_transition");
}
