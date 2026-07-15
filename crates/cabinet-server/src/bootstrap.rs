use std::collections::BTreeMap;

use cabinet_core::server_config::ServerConfigInput;

const BIND_ADDRESS_KEY: &str = "SPONZEY_CABINET_SERVER_BIND_ADDRESS";
const PUBLIC_URL_KEY: &str = "SPONZEY_CABINET_SERVER_PUBLIC_URL";
const METADATA_STORE_LOCATION_KEY: &str = "SPONZEY_CABINET_SERVER_METADATA_STORE_LOCATION";
const OBJECT_STORAGE_BACKEND_KEY: &str = "SPONZEY_CABINET_SERVER_OBJECT_STORAGE_BACKEND";
const OBJECT_STORAGE_LOCATION_KEY: &str = "SPONZEY_CABINET_SERVER_OBJECT_STORAGE_LOCATION";
const OBJECT_STORAGE_ENDPOINT_KEY: &str = "SPONZEY_CABINET_SERVER_OBJECT_STORAGE_ENDPOINT";
const OBJECT_STORAGE_BUCKET_KEY: &str = "SPONZEY_CABINET_SERVER_OBJECT_STORAGE_BUCKET";
const OBJECT_STORAGE_ACCESS_KEY_ID_KEY: &str =
    "SPONZEY_CABINET_SERVER_OBJECT_STORAGE_ACCESS_KEY_ID";
const OBJECT_STORAGE_SECRET_ACCESS_KEY_KEY: &str =
    "SPONZEY_CABINET_SERVER_OBJECT_STORAGE_SECRET_ACCESS_KEY";
const BACKUP_STORE_LOCATION_KEY: &str = "SPONZEY_CABINET_SERVER_BACKUP_STORE_LOCATION";
const BACKUP_RETENTION_DAYS_KEY: &str = "SPONZEY_CABINET_SERVER_BACKUP_RETENTION_DAYS";
const AUDIT_RETENTION_DAYS_KEY: &str = "SPONZEY_CABINET_SERVER_AUDIT_RETENTION_DAYS";
const FIELD_DEBUG_MAX_TTL_SECONDS_KEY: &str = "SPONZEY_CABINET_SERVER_FIELD_DEBUG_MAX_TTL_SECONDS";
const AUTH_SESSION_TTL_SECONDS_KEY: &str = "SPONZEY_CABINET_AUTH_SESSION_TTL_SECONDS";
const AUTH_TOKEN_SECRET_KEY: &str = "SPONZEY_CABINET_AUTH_TOKEN_SECRET";
const AUTH_TOKEN_BYTE_LENGTH_KEY: &str = "SPONZEY_CABINET_AUTH_TOKEN_BYTE_LENGTH";
const PRODUCT_LOG_SINK_KEY: &str = "SPONZEY_CABINET_SERVER_PRODUCT_LOG_SINK";
const DEVELOPMENT_LOG_MODE_KEY: &str = "SPONZEY_CABINET_SERVER_DEVELOPMENT_LOG_MODE";

const KNOWN_KEYS: [&str; 18] = [
    BIND_ADDRESS_KEY,
    PUBLIC_URL_KEY,
    METADATA_STORE_LOCATION_KEY,
    OBJECT_STORAGE_BACKEND_KEY,
    OBJECT_STORAGE_LOCATION_KEY,
    OBJECT_STORAGE_ENDPOINT_KEY,
    OBJECT_STORAGE_BUCKET_KEY,
    OBJECT_STORAGE_ACCESS_KEY_ID_KEY,
    OBJECT_STORAGE_SECRET_ACCESS_KEY_KEY,
    BACKUP_STORE_LOCATION_KEY,
    BACKUP_RETENTION_DAYS_KEY,
    AUDIT_RETENTION_DAYS_KEY,
    FIELD_DEBUG_MAX_TTL_SECONDS_KEY,
    AUTH_SESSION_TTL_SECONDS_KEY,
    AUTH_TOKEN_SECRET_KEY,
    AUTH_TOKEN_BYTE_LENGTH_KEY,
    PRODUCT_LOG_SINK_KEY,
    DEVELOPMENT_LOG_MODE_KEY,
];

pub trait ServerEnvironmentSource {
    fn read_environment(&self) -> ServerEnvironmentSnapshot;
}

pub struct ProcessEnvironmentSource;

impl ServerEnvironmentSource for ProcessEnvironmentSource {
    fn read_environment(&self) -> ServerEnvironmentSnapshot {
        let mut values = BTreeMap::new();
        for (key, value) in std::env::vars() {
            if KNOWN_KEYS.contains(&key.as_str()) {
                values.insert(key, value);
            }
        }
        ServerEnvironmentSnapshot { values }
    }
}

pub struct ServerBootstrapReader<'source, S: ServerEnvironmentSource> {
    source: &'source S,
}

impl<'source, S: ServerEnvironmentSource> ServerBootstrapReader<'source, S> {
    pub const fn new(source: &'source S) -> Self {
        Self { source }
    }

    pub fn read_once(&self) -> ServerEnvironmentSnapshot {
        self.source.read_environment()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerEnvironmentSnapshot {
    values: BTreeMap<String, String>,
}

impl ServerEnvironmentSnapshot {
    pub fn empty() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    pub fn from_pairs<const N: usize>(pairs: [(&str, &str); N]) -> Self {
        Self {
            values: pairs
                .into_iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        }
    }

    pub fn into_config_input(self) -> ServerConfigInput {
        let mut input = ServerConfigInput::local_dev_defaults();
        if let Some(value) = self.values.get(BIND_ADDRESS_KEY) {
            input = input.with_bind_address(value);
        }
        if let Some(value) = self.values.get(PUBLIC_URL_KEY) {
            input = input.with_public_url(value);
        }
        if let Some(value) = self.values.get(METADATA_STORE_LOCATION_KEY) {
            input = input.with_metadata_store_location(value);
        }
        if let Some(value) = self.values.get(OBJECT_STORAGE_BACKEND_KEY) {
            input = input.with_object_storage_backend(value);
        }
        if let Some(value) = self.values.get(OBJECT_STORAGE_LOCATION_KEY) {
            input = input.with_object_storage_location(value);
        }
        if let Some(value) = self.values.get(OBJECT_STORAGE_ENDPOINT_KEY) {
            input = input.with_object_storage_endpoint(value);
        }
        if let Some(value) = self.values.get(OBJECT_STORAGE_BUCKET_KEY) {
            input = input.with_object_storage_bucket(value);
        }
        if let Some(value) = self.values.get(OBJECT_STORAGE_ACCESS_KEY_ID_KEY) {
            input = input.with_object_storage_access_key_id(value);
        }
        if let Some(value) = self.values.get(OBJECT_STORAGE_SECRET_ACCESS_KEY_KEY) {
            input = input.with_object_storage_secret_access_key(value);
        }
        if let Some(value) = self.values.get(BACKUP_STORE_LOCATION_KEY) {
            input = input.with_backup_store_location(value);
        }
        if let Some(value) = self
            .values
            .get(BACKUP_RETENTION_DAYS_KEY)
            .and_then(|value| value.parse::<u32>().ok())
        {
            input = input.with_backup_retention_days(value);
        }
        if let Some(value) = self
            .values
            .get(AUDIT_RETENTION_DAYS_KEY)
            .and_then(|value| value.parse::<u32>().ok())
        {
            input = input.with_audit_retention_days(value);
        }
        if let Some(value) = self
            .values
            .get(FIELD_DEBUG_MAX_TTL_SECONDS_KEY)
            .and_then(|value| value.parse::<u32>().ok())
        {
            input = input.with_field_debug_max_ttl_seconds(value);
        }
        if let Some(value) = self
            .values
            .get(AUTH_SESSION_TTL_SECONDS_KEY)
            .and_then(|value| value.parse::<u32>().ok())
        {
            input = input.with_auth_session_ttl_seconds(value);
        }
        if let Some(value) = self.values.get(AUTH_TOKEN_SECRET_KEY) {
            input = input.with_auth_token_secret(value);
        }
        if let Some(value) = self
            .values
            .get(AUTH_TOKEN_BYTE_LENGTH_KEY)
            .and_then(|value| value.parse::<u16>().ok())
        {
            input = input.with_auth_token_byte_length(value);
        }
        if let Some(value) = self.values.get(PRODUCT_LOG_SINK_KEY) {
            input = input.with_product_log_sink(value);
        }
        if let Some(value) = self.values.get(DEVELOPMENT_LOG_MODE_KEY) {
            input = input.with_development_log_mode(value);
        }
        input
    }
}
