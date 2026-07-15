use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_phase002_migration_fixture::{
    LocalPhase002FixtureFailure, LocalPhase002FixtureStoreError,
    LocalPhase002MigrationFixtureStore, PHASE002_FIXTURE_FILE,
};
use cabinet_core::migration::{
    Phase002FixtureRecord, Phase002FixtureRecordKind, Phase002MigrationFixture,
};

struct TempMetadata {
    path: PathBuf,
}

impl TempMetadata {
    fn new(test_name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = PathBuf::from("/tmp").join(format!(
            "sponzey-cabinet-phase002-fixture-{test_name}-{}-{nanos}",
            std::process::id()
        ));
        fs::remove_dir_all(&path).ok();
        fs::create_dir_all(&path).expect("metadata dir");
        Self { path }
    }
}

impl Drop for TempMetadata {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).ok();
    }
}

#[test]
fn local_phase002_fixture_store_saves_and_loads_required_records() {
    let temp = TempMetadata::new("save-load");
    let store = LocalPhase002MigrationFixtureStore::new(temp.path.clone());
    let fixture = Phase002MigrationFixture::self_host_sample();

    store.save_fixture(&fixture).expect("save fixture");
    let loaded = store.load_fixture().expect("load fixture");

    assert_eq!(loaded, fixture);
    assert!(temp.path.join(PHASE002_FIXTURE_FILE).exists());
    assert!(loaded.contains_kind(Phase002FixtureRecordKind::DocumentCurrentSnapshot));
    assert!(loaded.contains_kind(Phase002FixtureRecordKind::FieldDebugSession));
    assert!(loaded.contains_kind(Phase002FixtureRecordKind::BackupJobRecord));
}

#[test]
fn local_phase002_fixture_store_rerun_is_idempotent() {
    let temp = TempMetadata::new("rerun");
    let store = LocalPhase002MigrationFixtureStore::new(temp.path.clone());
    let fixture = Phase002MigrationFixture::self_host_sample();

    store.save_fixture(&fixture).expect("first save");
    store.save_fixture(&fixture).expect("second save");
    let loaded = store.load_fixture().expect("load fixture");

    assert_eq!(loaded.record_count(), fixture.record_count());
    assert_eq!(loaded, fixture);
}

#[test]
fn local_phase002_fixture_store_failed_commit_preserves_existing_fixture() {
    let temp = TempMetadata::new("failure-preservation");
    let store = LocalPhase002MigrationFixtureStore::new(temp.path.clone());
    let original = Phase002MigrationFixture::self_host_sample();
    let changed = changed_fixture();

    store.save_fixture(&original).expect("initial save");
    let error = store
        .save_fixture_with_failure_for_test(&changed, LocalPhase002FixtureFailure::BeforeCommit)
        .expect_err("failed commit");
    let loaded = store.load_fixture().expect("load fixture after failure");

    assert_eq!(error, LocalPhase002FixtureStoreError::WriteFailed);
    assert_eq!(loaded, original);
    assert_ne!(loaded, changed);
}

#[test]
fn local_phase002_fixture_store_reports_corrupted_fixture_file() {
    let temp = TempMetadata::new("corrupted");
    fs::write(temp.path.join(PHASE002_FIXTURE_FILE), "bad\tfixture\n").expect("write corruption");
    let store = LocalPhase002MigrationFixtureStore::new(temp.path.clone());

    let error = store.load_fixture().expect_err("corrupted fixture");

    assert!(matches!(
        error,
        LocalPhase002FixtureStoreError::CorruptedFixture(_)
    ));
}

fn changed_fixture() -> Phase002MigrationFixture {
    let mut records = Phase002MigrationFixture::self_host_sample()
        .records()
        .to_vec();
    records.push(
        Phase002FixtureRecord::new(
            Phase002FixtureRecordKind::AuditEvent,
            "audit-extra",
            vec![("event", "changed")],
            None,
        )
        .expect("extra record"),
    );
    Phase002MigrationFixture::new(records).expect("changed fixture")
}
