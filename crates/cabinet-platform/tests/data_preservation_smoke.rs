use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_platform::release_smoke::{DataPreservationSmokeInput, run_data_preservation_smoke};

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
            "sponzey-cabinet-data-preservation-{test_name}-{}-{nanos}",
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
fn local_data_preservation_smoke_keeps_documents_versions_and_assets_after_reinit() {
    let profile = TempProfile::new("happy-path");

    let report = run_data_preservation_smoke(DataPreservationSmokeInput::new(profile.path.clone()))
        .expect("data preservation smoke should pass");

    assert!(report.first_run_completed());
    assert!(report.initial_migration_completed());
    assert!(report.migration_idempotent());
    assert!(report.current_document_preserved());
    assert!(report.version_history_preserved());
    assert!(report.specific_version_preserved());
    assert!(report.asset_metadata_preserved());
    assert!(report.asset_object_preserved());
    assert_eq!(report.history_entry_count(), 2);
    assert_eq!(report.asset_count(), 1);
}
