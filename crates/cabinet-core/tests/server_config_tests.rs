use cabinet_core::server_config::{
    DevelopmentLogMode, ObjectStorageBackend, ProductLogSink, ServerConfigErrorCode,
    ServerConfigInput,
};

#[test]
fn local_dev_defaults_validate_without_external_services() {
    let config = ServerConfigInput::local_dev_defaults()
        .validate()
        .expect("local dev defaults should be valid");

    assert_eq!(config.bind_address().to_string(), "127.0.0.1:7400");
    assert_eq!(config.public_url(), "http://127.0.0.1:7400");
    assert!(
        config
            .metadata_store_location()
            .ends_with("metadata.sqlite3")
    );
    assert_eq!(
        config.object_storage_backend(),
        ObjectStorageBackend::LocalDisk
    );
    assert!(config.object_storage_location().ends_with("object-store"));
    assert_eq!(
        config.object_storage_s3_compatible().endpoint(),
        "http://127.0.0.1:9000"
    );
    assert_eq!(
        config.object_storage_s3_compatible().bucket(),
        "cabinet-local"
    );
    assert!(config.backup_store_location().ends_with("backups"));
    assert_eq!(config.backup_retention_days(), 30);
    assert_eq!(config.audit_retention_days(), 90);
    assert_eq!(config.field_debug_max_ttl_seconds(), 900);
    assert_eq!(config.auth().session_ttl_seconds(), 3600);
    assert_eq!(config.auth().token_byte_length(), 32);
    assert!(config.auth().token_secret().expose_secret().len() >= 16);
    assert_eq!(config.product_log_sink(), ProductLogSink::Stdout);
    assert_eq!(config.development_log_mode(), DevelopmentLogMode::Disabled);
}

#[test]
fn s3_compatible_storage_config_is_validated_and_redacted() {
    let config = ServerConfigInput::local_dev_defaults()
        .with_object_storage_backend("s3-compatible")
        .with_object_storage_endpoint("https://objects.example.test")
        .with_object_storage_bucket("cabinet-prod")
        .with_object_storage_access_key_id("access-key-id")
        .with_object_storage_secret_access_key("secret-access-key")
        .validate()
        .expect("valid s3-compatible config");

    let s3 = config.object_storage_s3_compatible();
    let rendered = format!("{s3:?}");

    assert_eq!(
        config.object_storage_backend(),
        ObjectStorageBackend::S3Compatible
    );
    assert_eq!(s3.endpoint(), "https://objects.example.test");
    assert_eq!(s3.bucket(), "cabinet-prod");
    assert_eq!(s3.access_key_id(), "access-key-id");
    assert_eq!(s3.secret_access_key().expose_secret(), "secret-access-key");
    assert!(!rendered.contains("access-key-id"));
    assert!(!rendered.contains("secret-access-key"));
}

#[test]
fn invalid_s3_endpoint_is_rejected_with_safe_error() {
    let error = ServerConfigInput::local_dev_defaults()
        .with_object_storage_backend("s3-compatible")
        .with_object_storage_endpoint("objects.example.test")
        .validate()
        .expect_err("invalid s3 endpoint must fail validation");

    assert_eq!(
        error.code(),
        ServerConfigErrorCode::InvalidObjectStorageEndpoint
    );
    assert_eq!(error.public_message(), "invalid object storage endpoint");
    assert!(!format!("{error:?}").contains("objects.example.test"));
}

#[test]
fn invalid_s3_bucket_is_rejected_with_safe_error() {
    let error = ServerConfigInput::local_dev_defaults()
        .with_object_storage_backend("s3-compatible")
        .with_object_storage_bucket("bucket with spaces")
        .validate()
        .expect_err("invalid s3 bucket must fail validation");

    assert_eq!(
        error.code(),
        ServerConfigErrorCode::InvalidObjectStorageBucket
    );
    assert_eq!(error.public_message(), "invalid object storage bucket");
}

#[test]
fn invalid_s3_secret_is_rejected_without_debug_leak() {
    let error = ServerConfigInput::local_dev_defaults()
        .with_object_storage_backend("s3-compatible")
        .with_object_storage_secret_access_key("short")
        .validate()
        .expect_err("invalid s3 secret must fail validation");

    assert_eq!(
        error.code(),
        ServerConfigErrorCode::InvalidObjectStorageSecret
    );
    assert_eq!(error.public_message(), "invalid object storage secret");
    assert!(!format!("{error:?}").contains("short"));
}

#[test]
fn empty_backup_store_location_is_rejected() {
    let error = ServerConfigInput::local_dev_defaults()
        .with_backup_store_location(" ")
        .validate()
        .expect_err("empty backup location must fail validation");

    assert_eq!(
        error.code(),
        ServerConfigErrorCode::MissingBackupStoreLocation
    );
    assert_eq!(error.public_message(), "backup store location is required");
}

#[test]
fn invalid_backup_retention_days_is_rejected() {
    let error = ServerConfigInput::local_dev_defaults()
        .with_backup_retention_days(0)
        .validate()
        .expect_err("zero backup retention must fail validation");

    assert_eq!(error.code(), ServerConfigErrorCode::InvalidBackupRetention);
    assert_eq!(error.public_message(), "invalid backup retention policy");
}

#[test]
fn invalid_bind_address_returns_stable_safe_error() {
    let error = ServerConfigInput::local_dev_defaults()
        .with_bind_address("not a socket address")
        .validate()
        .expect_err("invalid bind address must fail validation");

    assert_eq!(error.code(), ServerConfigErrorCode::InvalidBindAddress);
    assert_eq!(error.public_message(), "invalid server bind address");
    let rendered = format!("{error:?}");
    assert!(!rendered.contains("SPONZEY_CABINET"));
    assert!(!rendered.contains("not a socket address"));
}

#[test]
fn unsupported_storage_backend_is_rejected_before_bootstrap() {
    let error = ServerConfigInput::local_dev_defaults()
        .with_object_storage_backend("external-only")
        .validate()
        .expect_err("unsupported backend must fail validation");

    assert_eq!(
        error.code(),
        ServerConfigErrorCode::UnsupportedObjectStorageBackend
    );
    assert_eq!(error.public_message(), "unsupported object storage backend");
}

#[test]
fn empty_metadata_store_location_is_rejected() {
    let error = ServerConfigInput::local_dev_defaults()
        .with_metadata_store_location(" ")
        .validate()
        .expect_err("empty metadata store location must fail validation");

    assert_eq!(
        error.code(),
        ServerConfigErrorCode::MissingMetadataStoreLocation
    );
    assert_eq!(
        error.public_message(),
        "metadata store location is required"
    );
}

#[test]
fn invalid_development_log_mode_is_rejected() {
    let error = ServerConfigInput::local_dev_defaults()
        .with_development_log_mode("production-verbose")
        .validate()
        .expect_err("invalid development log mode must fail validation");

    assert_eq!(
        error.code(),
        ServerConfigErrorCode::UnsupportedDevelopmentLogMode
    );
    assert_eq!(error.public_message(), "unsupported development log mode");
}

#[test]
fn invalid_auth_session_ttl_is_rejected() {
    let error = ServerConfigInput::local_dev_defaults()
        .with_auth_session_ttl_seconds(0)
        .validate()
        .expect_err("zero session TTL must fail validation");

    assert_eq!(error.code(), ServerConfigErrorCode::InvalidAuthSessionTtl);
    assert_eq!(error.public_message(), "invalid auth session TTL");
}

#[test]
fn invalid_auth_token_secret_is_rejected_without_debug_leak() {
    let error = ServerConfigInput::local_dev_defaults()
        .with_auth_token_secret("short-secret")
        .validate()
        .expect_err("short secret must fail validation");

    assert_eq!(error.code(), ServerConfigErrorCode::InvalidAuthTokenSecret);
    assert_eq!(error.public_message(), "invalid auth token secret");
    assert!(!format!("{error:?}").contains("short-secret"));
}

#[test]
fn invalid_auth_token_byte_length_is_rejected() {
    let error = ServerConfigInput::local_dev_defaults()
        .with_auth_token_byte_length(8)
        .validate()
        .expect_err("short token byte length must fail validation");

    assert_eq!(
        error.code(),
        ServerConfigErrorCode::InvalidAuthTokenByteLength
    );
    assert_eq!(error.public_message(), "invalid auth token byte length");
}
