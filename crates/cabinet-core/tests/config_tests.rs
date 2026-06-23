use std::path::PathBuf;

use cabinet_core::config::{
    AppConfig, BootstrapConfigInput, ConfigError, ExternalEnvironmentSnapshot,
};

#[test]
fn app_config_is_created_from_explicit_environment_snapshot() {
    let snapshot = ExternalEnvironmentSnapshot::from_pairs([
        ("SPONZEY_CABINET_APP_DATA_DIR", "/tmp/sponzey-cabinet"),
        (
            "SPONZEY_CABINET_WORKSPACE_ROOT",
            "/tmp/sponzey-cabinet/workspaces",
        ),
    ]);

    let config = AppConfig::from_environment_snapshot(snapshot).expect("config should be valid");

    assert_eq!(
        config.local_paths.app_data_dir,
        PathBuf::from("/tmp/sponzey-cabinet")
    );
    assert_eq!(
        config.local_paths.workspace_root,
        PathBuf::from("/tmp/sponzey-cabinet/workspaces")
    );
    assert_eq!(
        config.local_paths.metadata_dir,
        PathBuf::from("/tmp/sponzey-cabinet/metadata")
    );
    assert_eq!(
        config.local_paths.version_store_dir,
        PathBuf::from("/tmp/sponzey-cabinet/version-store")
    );
    assert_eq!(
        config.local_paths.asset_store_dir,
        PathBuf::from("/tmp/sponzey-cabinet/assets")
    );
    assert_eq!(
        config.local_paths.search_index_dir,
        PathBuf::from("/tmp/sponzey-cabinet/search-index")
    );
    assert_eq!(config.logging.field_debug_enabled, false);
    assert_eq!(config.logging.development_log_enabled, false);
}

#[test]
fn bootstrap_config_input_consumes_initial_environment_snapshot() {
    let snapshot = ExternalEnvironmentSnapshot::from_pairs([(
        "SPONZEY_CABINET_APP_DATA_DIR",
        "/tmp/bootstrap",
    )]);
    let input = BootstrapConfigInput::new(snapshot);

    let config = input
        .into_app_config()
        .expect("bootstrap input should create config");

    assert_eq!(
        config.local_paths.workspace_root,
        PathBuf::from("/tmp/bootstrap/workspaces")
    );
}

#[test]
fn app_config_rejects_missing_app_data_dir() {
    let snapshot = ExternalEnvironmentSnapshot::from_pairs([]);

    let error = AppConfig::from_environment_snapshot(snapshot).expect_err("missing path must fail");

    assert_eq!(
        error,
        ConfigError::MissingRequiredValue("SPONZEY_CABINET_APP_DATA_DIR")
    );
}

#[test]
fn app_config_rejects_empty_app_data_dir() {
    let snapshot =
        ExternalEnvironmentSnapshot::from_pairs([("SPONZEY_CABINET_APP_DATA_DIR", "   ")]);

    let error = AppConfig::from_environment_snapshot(snapshot).expect_err("empty path must fail");

    assert_eq!(
        error,
        ConfigError::InvalidValue("SPONZEY_CABINET_APP_DATA_DIR")
    );
}
