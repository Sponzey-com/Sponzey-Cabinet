use std::cell::Cell;

use cabinet_core::server_config::{ObjectStorageBackend, ServerConfigErrorCode};
use cabinet_server::bootstrap::{
    ServerBootstrapReader, ServerEnvironmentSnapshot, ServerEnvironmentSource,
};

struct CountingEnvironmentSource {
    read_count: Cell<u32>,
    snapshot: ServerEnvironmentSnapshot,
}

impl CountingEnvironmentSource {
    fn new(snapshot: ServerEnvironmentSnapshot) -> Self {
        Self {
            read_count: Cell::new(0),
            snapshot,
        }
    }

    fn read_count(&self) -> u32 {
        self.read_count.get()
    }
}

impl ServerEnvironmentSource for CountingEnvironmentSource {
    fn read_environment(&self) -> ServerEnvironmentSnapshot {
        self.read_count.set(self.read_count.get() + 1);
        self.snapshot.clone()
    }
}

#[test]
fn bootstrap_reader_reads_external_environment_once() {
    let source = CountingEnvironmentSource::new(ServerEnvironmentSnapshot::from_pairs([
        ("SPONZEY_CABINET_SERVER_BIND_ADDRESS", "127.0.0.1:7500"),
        ("SPONZEY_CABINET_SERVER_PUBLIC_URL", "http://127.0.0.1:7500"),
        (
            "SPONZEY_CABINET_SERVER_OBJECT_STORAGE_BACKEND",
            "s3-compatible",
        ),
        (
            "SPONZEY_CABINET_SERVER_OBJECT_STORAGE_ENDPOINT",
            "https://objects.example.test",
        ),
        (
            "SPONZEY_CABINET_SERVER_OBJECT_STORAGE_BUCKET",
            "cabinet-prod",
        ),
        (
            "SPONZEY_CABINET_SERVER_OBJECT_STORAGE_ACCESS_KEY_ID",
            "access-key-id",
        ),
        (
            "SPONZEY_CABINET_SERVER_OBJECT_STORAGE_SECRET_ACCESS_KEY",
            "secret-access-key",
        ),
        (
            "SPONZEY_CABINET_SERVER_BACKUP_STORE_LOCATION",
            ".sponzey-cabinet/test/backups",
        ),
        ("SPONZEY_CABINET_SERVER_BACKUP_RETENTION_DAYS", "7"),
        ("SPONZEY_CABINET_AUTH_SESSION_TTL_SECONDS", "120"),
        (
            "SPONZEY_CABINET_AUTH_TOKEN_SECRET",
            "0123456789abcdef0123456789abcdef",
        ),
        ("SPONZEY_CABINET_AUTH_TOKEN_BYTE_LENGTH", "48"),
    ]));
    let reader = ServerBootstrapReader::new(&source);

    let input = reader.read_once().into_config_input();
    let config = input.validate().expect("valid overridden config");

    assert_eq!(source.read_count(), 1);
    assert_eq!(config.bind_address().to_string(), "127.0.0.1:7500");
    assert_eq!(config.public_url(), "http://127.0.0.1:7500");
    assert_eq!(
        config.object_storage_backend(),
        ObjectStorageBackend::S3Compatible
    );
    assert_eq!(
        config.object_storage_s3_compatible().endpoint(),
        "https://objects.example.test"
    );
    assert_eq!(
        config.object_storage_s3_compatible().bucket(),
        "cabinet-prod"
    );
    assert!(config.backup_store_location().ends_with("backups"));
    assert_eq!(config.backup_retention_days(), 7);
    assert_eq!(config.auth().session_ttl_seconds(), 120);
    assert_eq!(config.auth().token_byte_length(), 48);
    assert_eq!(
        config.auth().token_secret().expose_secret(),
        "0123456789abcdef0123456789abcdef"
    );
}

#[test]
fn empty_environment_snapshot_uses_local_dev_defaults() {
    let config = ServerEnvironmentSnapshot::empty()
        .into_config_input()
        .validate()
        .expect("empty snapshot should use local defaults");

    assert_eq!(
        config.object_storage_backend(),
        ObjectStorageBackend::LocalDisk
    );
    assert_eq!(config.public_url(), "http://127.0.0.1:7400");
}

#[test]
fn bootstrap_config_validation_error_is_stable_and_safe() {
    let error = ServerEnvironmentSnapshot::from_pairs([(
        "SPONZEY_CABINET_SERVER_BIND_ADDRESS",
        "not-a-socket",
    )])
    .into_config_input()
    .validate()
    .expect_err("invalid bind address must fail");

    assert_eq!(error.code(), ServerConfigErrorCode::InvalidBindAddress);
    assert_eq!(error.public_message(), "invalid server bind address");
    assert!(!format!("{error:?}").contains("not-a-socket"));
}
