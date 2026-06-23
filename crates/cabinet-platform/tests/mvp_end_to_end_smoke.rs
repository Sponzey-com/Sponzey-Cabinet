use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_platform::release_smoke::{MvpEndToEndSmokeInput, run_mvp_end_to_end_smoke};

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
            "sponzey-cabinet-mvp-e2e-{test_name}-{}-{nanos}",
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
fn mvp_end_to_end_smoke_covers_create_edit_link_search_asset_and_restore() {
    let profile = TempProfile::new("happy-path");

    let report = run_mvp_end_to_end_smoke(MvpEndToEndSmokeInput::new(profile.path.clone()))
        .expect("mvp end-to-end smoke should pass");

    assert!(report.first_run_completed());
    assert!(report.migration_completed());
    assert!(report.document_created());
    assert!(report.document_edited());
    assert!(report.wikilink_parsed());
    assert!(report.asset_reference_parsed());
    assert!(report.search_result_found());
    assert!(report.backlink_found());
    assert!(report.asset_metadata_listed());
    assert!(report.restore_preview_available());
    assert!(report.restore_completed());
    assert!(report.restored_current_document_matches_initial_version());
    assert!(report.product_log_sensitive_data_absent());
    assert_eq!(report.history_entry_count(), 3);
}
