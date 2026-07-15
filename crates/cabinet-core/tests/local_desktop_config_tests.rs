use std::path::PathBuf;

use cabinet_core::config::{
    ConfigError, ExternalEnvironmentReader, ExternalEnvironmentSnapshot, LocalDesktopConfig,
    bootstrap_local_desktop_config_from_reader,
};

#[test]
fn local_desktop_config_is_the_standard_phase008_config_contract() {
    let snapshot = ExternalEnvironmentSnapshot::from_pairs([
        ("SPONZEY_CABINET_APP_DATA_DIR", "/tmp/sponzey-phase008"),
        (
            "SPONZEY_CABINET_WORKSPACE_ROOT",
            "/tmp/sponzey-phase008/custom-workspaces",
        ),
    ]);

    let config = LocalDesktopConfig::from_environment_snapshot(snapshot)
        .expect("local desktop config should be valid");

    assert_eq!(
        config.local_paths.app_data_dir,
        PathBuf::from("/tmp/sponzey-phase008")
    );
    assert_eq!(
        config.local_paths.workspace_root,
        PathBuf::from("/tmp/sponzey-phase008/custom-workspaces")
    );
    assert_eq!(
        config.local_paths.metadata_dir,
        PathBuf::from("/tmp/sponzey-phase008/metadata")
    );
    assert_eq!(
        config.local_paths.version_store_dir,
        PathBuf::from("/tmp/sponzey-phase008/version-store")
    );
    assert_eq!(
        config.local_paths.asset_store_dir,
        PathBuf::from("/tmp/sponzey-phase008/assets")
    );
    assert_eq!(
        config.local_paths.search_index_dir,
        PathBuf::from("/tmp/sponzey-phase008/search-index")
    );
}

#[test]
fn bootstrap_reads_external_environment_snapshot_once() {
    let mut reader = CountingEnvironmentReader::new(ExternalEnvironmentSnapshot::from_pairs([(
        "SPONZEY_CABINET_APP_DATA_DIR",
        "/tmp/read-once",
    )]));

    let config = bootstrap_local_desktop_config_from_reader(&mut reader)
        .expect("bootstrap should produce local config");

    assert_eq!(reader.read_count(), 1);
    assert_eq!(
        config.local_paths.workspace_root,
        PathBuf::from("/tmp/read-once/workspaces")
    );
}

#[test]
fn bootstrap_returns_config_error_without_retrying_environment_reader() {
    let mut reader = CountingEnvironmentReader::new(ExternalEnvironmentSnapshot::from_pairs([]));

    let error = bootstrap_local_desktop_config_from_reader(&mut reader)
        .expect_err("missing app data dir should fail");

    assert_eq!(reader.read_count(), 1);
    assert_eq!(
        error,
        ConfigError::MissingRequiredValue("SPONZEY_CABINET_APP_DATA_DIR")
    );
}

struct CountingEnvironmentReader {
    snapshot: ExternalEnvironmentSnapshot,
    read_count: usize,
}

impl CountingEnvironmentReader {
    fn new(snapshot: ExternalEnvironmentSnapshot) -> Self {
        Self {
            snapshot,
            read_count: 0,
        }
    }

    fn read_count(&self) -> usize {
        self.read_count
    }
}

impl ExternalEnvironmentReader for CountingEnvironmentReader {
    fn read_environment_snapshot(&mut self) -> ExternalEnvironmentSnapshot {
        self.read_count += 1;
        self.snapshot.clone()
    }
}
