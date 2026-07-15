use std::fmt;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

#[derive(Clone, PartialEq, Eq)]
pub struct ServerConfigInput {
    bind_address: String,
    public_url: String,
    metadata_store_location: String,
    object_storage_backend: String,
    object_storage_location: String,
    object_storage_endpoint: String,
    object_storage_bucket: String,
    object_storage_access_key_id: String,
    object_storage_secret_access_key: String,
    backup_store_location: String,
    backup_retention_days: u32,
    audit_retention_days: u32,
    field_debug_max_ttl_seconds: u32,
    auth_session_ttl_seconds: u32,
    auth_token_secret: String,
    auth_token_byte_length: u16,
    product_log_sink: String,
    development_log_mode: String,
}

impl ServerConfigInput {
    pub fn local_dev_defaults() -> Self {
        Self {
            bind_address: "127.0.0.1:7400".to_string(),
            public_url: "http://127.0.0.1:7400".to_string(),
            metadata_store_location: ".sponzey-cabinet/self-host/metadata.sqlite3".to_string(),
            object_storage_backend: "local-disk".to_string(),
            object_storage_location: ".sponzey-cabinet/self-host/object-store".to_string(),
            object_storage_endpoint: "http://127.0.0.1:9000".to_string(),
            object_storage_bucket: "cabinet-local".to_string(),
            object_storage_access_key_id: "local-dev-object-access".to_string(),
            object_storage_secret_access_key: "local-dev-object-secret-000000".to_string(),
            backup_store_location: ".sponzey-cabinet/self-host/backups".to_string(),
            backup_retention_days: 30,
            audit_retention_days: 90,
            field_debug_max_ttl_seconds: 900,
            auth_session_ttl_seconds: 3600,
            auth_token_secret: "local-dev-auth-token-secret-00000000".to_string(),
            auth_token_byte_length: 32,
            product_log_sink: "stdout".to_string(),
            development_log_mode: "disabled".to_string(),
        }
    }

    pub fn validate(self) -> Result<ServerConfig, ServerConfigError> {
        let bind_address = self
            .bind_address
            .trim()
            .parse::<SocketAddr>()
            .map_err(|_| ServerConfigError::new(ServerConfigErrorCode::InvalidBindAddress))?;
        let public_url = validate_public_url(&self.public_url)?;
        let metadata_store_location = validate_required_path(
            &self.metadata_store_location,
            ServerConfigErrorCode::MissingMetadataStoreLocation,
        )?;
        let object_storage_backend = ObjectStorageBackend::parse(&self.object_storage_backend)?;
        let object_storage_location = validate_required_path(
            &self.object_storage_location,
            ServerConfigErrorCode::MissingObjectStorageLocation,
        )?;
        let object_storage_s3_compatible = S3CompatibleObjectStorageConfig::new(
            self.object_storage_endpoint,
            self.object_storage_bucket,
            self.object_storage_access_key_id,
            self.object_storage_secret_access_key,
        )?;
        let backup_store_location = validate_required_path(
            &self.backup_store_location,
            ServerConfigErrorCode::MissingBackupStoreLocation,
        )?;
        if self.backup_retention_days == 0 {
            return Err(ServerConfigError::new(
                ServerConfigErrorCode::InvalidBackupRetention,
            ));
        }
        if self.audit_retention_days == 0 {
            return Err(ServerConfigError::new(
                ServerConfigErrorCode::InvalidAuditRetention,
            ));
        }
        if self.field_debug_max_ttl_seconds == 0 {
            return Err(ServerConfigError::new(
                ServerConfigErrorCode::InvalidFieldDebugMaxTtl,
            ));
        }
        let auth = ServerAuthConfig::new(
            self.auth_session_ttl_seconds,
            self.auth_token_byte_length,
            self.auth_token_secret,
        )?;
        let product_log_sink = ProductLogSink::parse(&self.product_log_sink)?;
        let development_log_mode = DevelopmentLogMode::parse(&self.development_log_mode)?;

        Ok(ServerConfig {
            bind_address,
            public_url,
            metadata_store_location,
            object_storage_backend,
            object_storage_location,
            object_storage_s3_compatible,
            backup_store_location,
            backup_retention_days: self.backup_retention_days,
            audit_retention_days: self.audit_retention_days,
            field_debug_max_ttl_seconds: self.field_debug_max_ttl_seconds,
            auth,
            product_log_sink,
            development_log_mode,
        })
    }

    pub fn with_bind_address(mut self, bind_address: &str) -> Self {
        self.bind_address = bind_address.to_string();
        self
    }

    pub fn with_public_url(mut self, public_url: &str) -> Self {
        self.public_url = public_url.to_string();
        self
    }

    pub fn with_metadata_store_location(mut self, metadata_store_location: &str) -> Self {
        self.metadata_store_location = metadata_store_location.to_string();
        self
    }

    pub fn with_object_storage_backend(mut self, object_storage_backend: &str) -> Self {
        self.object_storage_backend = object_storage_backend.to_string();
        self
    }

    pub fn with_object_storage_location(mut self, object_storage_location: &str) -> Self {
        self.object_storage_location = object_storage_location.to_string();
        self
    }

    pub fn with_object_storage_endpoint(mut self, object_storage_endpoint: &str) -> Self {
        self.object_storage_endpoint = object_storage_endpoint.to_string();
        self
    }

    pub fn with_object_storage_bucket(mut self, object_storage_bucket: &str) -> Self {
        self.object_storage_bucket = object_storage_bucket.to_string();
        self
    }

    pub fn with_object_storage_access_key_id(mut self, object_storage_access_key_id: &str) -> Self {
        self.object_storage_access_key_id = object_storage_access_key_id.to_string();
        self
    }

    pub fn with_object_storage_secret_access_key(
        mut self,
        object_storage_secret_access_key: &str,
    ) -> Self {
        self.object_storage_secret_access_key = object_storage_secret_access_key.to_string();
        self
    }

    pub fn with_backup_store_location(mut self, backup_store_location: &str) -> Self {
        self.backup_store_location = backup_store_location.to_string();
        self
    }

    pub const fn with_backup_retention_days(mut self, backup_retention_days: u32) -> Self {
        self.backup_retention_days = backup_retention_days;
        self
    }

    pub fn with_audit_retention_days(mut self, audit_retention_days: u32) -> Self {
        self.audit_retention_days = audit_retention_days;
        self
    }

    pub fn with_field_debug_max_ttl_seconds(mut self, field_debug_max_ttl_seconds: u32) -> Self {
        self.field_debug_max_ttl_seconds = field_debug_max_ttl_seconds;
        self
    }

    pub fn with_auth_session_ttl_seconds(mut self, auth_session_ttl_seconds: u32) -> Self {
        self.auth_session_ttl_seconds = auth_session_ttl_seconds;
        self
    }

    pub fn with_auth_token_secret(mut self, auth_token_secret: &str) -> Self {
        self.auth_token_secret = auth_token_secret.to_string();
        self
    }

    pub fn with_auth_token_byte_length(mut self, auth_token_byte_length: u16) -> Self {
        self.auth_token_byte_length = auth_token_byte_length;
        self
    }

    pub fn with_product_log_sink(mut self, product_log_sink: &str) -> Self {
        self.product_log_sink = product_log_sink.to_string();
        self
    }

    pub fn with_development_log_mode(mut self, development_log_mode: &str) -> Self {
        self.development_log_mode = development_log_mode.to_string();
        self
    }
}

impl Default for ServerConfigInput {
    fn default() -> Self {
        Self::local_dev_defaults()
    }
}

impl fmt::Debug for ServerConfigInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ServerConfigInput")
            .field("bind_address", &self.bind_address)
            .field("public_url", &self.public_url)
            .field("metadata_store_location", &self.metadata_store_location)
            .field("object_storage_backend", &self.object_storage_backend)
            .field("object_storage_location", &self.object_storage_location)
            .field("object_storage_endpoint", &self.object_storage_endpoint)
            .field("object_storage_bucket", &self.object_storage_bucket)
            .field("object_storage_access_key_id", &"<redacted>")
            .field("object_storage_secret_access_key", &"<redacted>")
            .field("backup_store_location", &self.backup_store_location)
            .field("backup_retention_days", &self.backup_retention_days)
            .field("audit_retention_days", &self.audit_retention_days)
            .field(
                "field_debug_max_ttl_seconds",
                &self.field_debug_max_ttl_seconds,
            )
            .field("auth_session_ttl_seconds", &self.auth_session_ttl_seconds)
            .field("auth_token_secret", &"<redacted>")
            .field("auth_token_byte_length", &self.auth_token_byte_length)
            .field("product_log_sink", &self.product_log_sink)
            .field("development_log_mode", &self.development_log_mode)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    bind_address: SocketAddr,
    public_url: String,
    metadata_store_location: PathBuf,
    object_storage_backend: ObjectStorageBackend,
    object_storage_location: PathBuf,
    object_storage_s3_compatible: S3CompatibleObjectStorageConfig,
    backup_store_location: PathBuf,
    backup_retention_days: u32,
    audit_retention_days: u32,
    field_debug_max_ttl_seconds: u32,
    auth: ServerAuthConfig,
    product_log_sink: ProductLogSink,
    development_log_mode: DevelopmentLogMode,
}

impl ServerConfig {
    pub const fn bind_address(&self) -> SocketAddr {
        self.bind_address
    }

    pub fn public_url(&self) -> &str {
        &self.public_url
    }

    pub fn metadata_store_location(&self) -> &Path {
        &self.metadata_store_location
    }

    pub const fn object_storage_backend(&self) -> ObjectStorageBackend {
        self.object_storage_backend
    }

    pub fn object_storage_location(&self) -> &Path {
        &self.object_storage_location
    }

    pub const fn object_storage_s3_compatible(&self) -> &S3CompatibleObjectStorageConfig {
        &self.object_storage_s3_compatible
    }

    pub fn backup_store_location(&self) -> &Path {
        &self.backup_store_location
    }

    pub const fn backup_retention_days(&self) -> u32 {
        self.backup_retention_days
    }

    pub const fn audit_retention_days(&self) -> u32 {
        self.audit_retention_days
    }

    pub const fn field_debug_max_ttl_seconds(&self) -> u32 {
        self.field_debug_max_ttl_seconds
    }

    pub const fn auth(&self) -> &ServerAuthConfig {
        &self.auth
    }

    pub const fn product_log_sink(&self) -> ProductLogSink {
        self.product_log_sink
    }

    pub const fn development_log_mode(&self) -> DevelopmentLogMode {
        self.development_log_mode
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ServerSecret {
    value: String,
}

impl ServerSecret {
    fn new(value: String, error_code: ServerConfigErrorCode) -> Result<Self, ServerConfigError> {
        let trimmed = value.trim();
        if trimmed.len() < 16 || trimmed.chars().any(char::is_control) {
            return Err(ServerConfigError::new(error_code));
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn expose_secret(&self) -> &str {
        &self.value
    }
}

impl fmt::Debug for ServerSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ServerSecret(<redacted>)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerAuthConfig {
    session_ttl_seconds: u32,
    token_byte_length: u16,
    token_secret: ServerSecret,
}

impl ServerAuthConfig {
    fn new(
        session_ttl_seconds: u32,
        token_byte_length: u16,
        token_secret: String,
    ) -> Result<Self, ServerConfigError> {
        if session_ttl_seconds == 0 {
            return Err(ServerConfigError::new(
                ServerConfigErrorCode::InvalidAuthSessionTtl,
            ));
        }
        if token_byte_length < 16 {
            return Err(ServerConfigError::new(
                ServerConfigErrorCode::InvalidAuthTokenByteLength,
            ));
        }
        Ok(Self {
            session_ttl_seconds,
            token_byte_length,
            token_secret: ServerSecret::new(
                token_secret,
                ServerConfigErrorCode::InvalidAuthTokenSecret,
            )?,
        })
    }

    pub const fn session_ttl_seconds(&self) -> u32 {
        self.session_ttl_seconds
    }

    pub const fn token_byte_length(&self) -> u16 {
        self.token_byte_length
    }

    pub const fn token_secret(&self) -> &ServerSecret {
        &self.token_secret
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct S3CompatibleObjectStorageConfig {
    endpoint: String,
    bucket: String,
    access_key_id: String,
    secret_access_key: ServerSecret,
}

impl S3CompatibleObjectStorageConfig {
    fn new(
        endpoint: String,
        bucket: String,
        access_key_id: String,
        secret_access_key: String,
    ) -> Result<Self, ServerConfigError> {
        Ok(Self {
            endpoint: validate_object_storage_endpoint(&endpoint)?,
            bucket: validate_object_storage_bucket(&bucket)?,
            access_key_id: validate_object_storage_access_key_id(&access_key_id)?,
            secret_access_key: ServerSecret::new(
                secret_access_key,
                ServerConfigErrorCode::InvalidObjectStorageSecret,
            )?,
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    pub fn access_key_id(&self) -> &str {
        &self.access_key_id
    }

    pub const fn secret_access_key(&self) -> &ServerSecret {
        &self.secret_access_key
    }
}

impl fmt::Debug for S3CompatibleObjectStorageConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("S3CompatibleObjectStorageConfig")
            .field("backend_type", &ObjectStorageBackend::S3Compatible.as_str())
            .field("endpoint", &self.endpoint)
            .field("bucket", &self.bucket)
            .field("access_key_id", &"<redacted>")
            .field("secret_access_key", &self.secret_access_key)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectStorageBackend {
    LocalDisk,
    S3Compatible,
}

impl ObjectStorageBackend {
    fn parse(value: &str) -> Result<Self, ServerConfigError> {
        match value.trim() {
            "local-disk" => Ok(Self::LocalDisk),
            "s3-compatible" => Ok(Self::S3Compatible),
            _ => Err(ServerConfigError::new(
                ServerConfigErrorCode::UnsupportedObjectStorageBackend,
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalDisk => "local-disk",
            Self::S3Compatible => "s3-compatible",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductLogSink {
    Stdout,
    Stderr,
}

impl ProductLogSink {
    fn parse(value: &str) -> Result<Self, ServerConfigError> {
        match value.trim() {
            "stdout" => Ok(Self::Stdout),
            "stderr" => Ok(Self::Stderr),
            _ => Err(ServerConfigError::new(
                ServerConfigErrorCode::UnsupportedProductLogSink,
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentLogMode {
    Disabled,
    LocalOnly,
}

impl DevelopmentLogMode {
    fn parse(value: &str) -> Result<Self, ServerConfigError> {
        match value.trim() {
            "disabled" => Ok(Self::Disabled),
            "local-only" => Ok(Self::LocalOnly),
            _ => Err(ServerConfigError::new(
                ServerConfigErrorCode::UnsupportedDevelopmentLogMode,
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::LocalOnly => "local-only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerConfigErrorCode {
    InvalidBindAddress,
    InvalidPublicUrl,
    MissingMetadataStoreLocation,
    UnsupportedObjectStorageBackend,
    MissingObjectStorageLocation,
    InvalidObjectStorageEndpoint,
    InvalidObjectStorageBucket,
    InvalidObjectStorageAccessKeyId,
    InvalidObjectStorageSecret,
    MissingBackupStoreLocation,
    InvalidBackupRetention,
    InvalidAuditRetention,
    InvalidFieldDebugMaxTtl,
    InvalidAuthSessionTtl,
    InvalidAuthTokenSecret,
    InvalidAuthTokenByteLength,
    UnsupportedProductLogSink,
    UnsupportedDevelopmentLogMode,
}

impl ServerConfigErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidBindAddress => "SERVER_CONFIG_INVALID_BIND_ADDRESS",
            Self::InvalidPublicUrl => "SERVER_CONFIG_INVALID_PUBLIC_URL",
            Self::MissingMetadataStoreLocation => "SERVER_CONFIG_MISSING_METADATA_STORE_LOCATION",
            Self::UnsupportedObjectStorageBackend => {
                "SERVER_CONFIG_UNSUPPORTED_OBJECT_STORAGE_BACKEND"
            }
            Self::MissingObjectStorageLocation => "SERVER_CONFIG_MISSING_OBJECT_STORAGE_LOCATION",
            Self::InvalidObjectStorageEndpoint => "SERVER_CONFIG_INVALID_OBJECT_STORAGE_ENDPOINT",
            Self::InvalidObjectStorageBucket => "SERVER_CONFIG_INVALID_OBJECT_STORAGE_BUCKET",
            Self::InvalidObjectStorageAccessKeyId => {
                "SERVER_CONFIG_INVALID_OBJECT_STORAGE_ACCESS_KEY_ID"
            }
            Self::InvalidObjectStorageSecret => "SERVER_CONFIG_INVALID_OBJECT_STORAGE_SECRET",
            Self::MissingBackupStoreLocation => "SERVER_CONFIG_MISSING_BACKUP_STORE_LOCATION",
            Self::InvalidBackupRetention => "SERVER_CONFIG_INVALID_BACKUP_RETENTION",
            Self::InvalidAuditRetention => "SERVER_CONFIG_INVALID_AUDIT_RETENTION",
            Self::InvalidFieldDebugMaxTtl => "SERVER_CONFIG_INVALID_FIELD_DEBUG_MAX_TTL",
            Self::InvalidAuthSessionTtl => "SERVER_CONFIG_INVALID_AUTH_SESSION_TTL",
            Self::InvalidAuthTokenSecret => "SERVER_CONFIG_INVALID_AUTH_TOKEN_SECRET",
            Self::InvalidAuthTokenByteLength => "SERVER_CONFIG_INVALID_AUTH_TOKEN_BYTE_LENGTH",
            Self::UnsupportedProductLogSink => "SERVER_CONFIG_UNSUPPORTED_PRODUCT_LOG_SINK",
            Self::UnsupportedDevelopmentLogMode => "SERVER_CONFIG_UNSUPPORTED_DEVELOPMENT_LOG_MODE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerConfigError {
    code: ServerConfigErrorCode,
}

impl ServerConfigError {
    pub const fn new(code: ServerConfigErrorCode) -> Self {
        Self { code }
    }

    pub const fn code(&self) -> ServerConfigErrorCode {
        self.code
    }

    pub const fn public_message(&self) -> &'static str {
        match self.code {
            ServerConfigErrorCode::InvalidBindAddress => "invalid server bind address",
            ServerConfigErrorCode::InvalidPublicUrl => "invalid server public URL",
            ServerConfigErrorCode::MissingMetadataStoreLocation => {
                "metadata store location is required"
            }
            ServerConfigErrorCode::UnsupportedObjectStorageBackend => {
                "unsupported object storage backend"
            }
            ServerConfigErrorCode::MissingObjectStorageLocation => {
                "object storage location is required"
            }
            ServerConfigErrorCode::InvalidObjectStorageEndpoint => {
                "invalid object storage endpoint"
            }
            ServerConfigErrorCode::InvalidObjectStorageBucket => "invalid object storage bucket",
            ServerConfigErrorCode::InvalidObjectStorageAccessKeyId => {
                "invalid object storage access key id"
            }
            ServerConfigErrorCode::InvalidObjectStorageSecret => "invalid object storage secret",
            ServerConfigErrorCode::MissingBackupStoreLocation => {
                "backup store location is required"
            }
            ServerConfigErrorCode::InvalidBackupRetention => "invalid backup retention policy",
            ServerConfigErrorCode::InvalidAuditRetention => "invalid audit retention policy",
            ServerConfigErrorCode::InvalidFieldDebugMaxTtl => "invalid Field Debug max TTL",
            ServerConfigErrorCode::InvalidAuthSessionTtl => "invalid auth session TTL",
            ServerConfigErrorCode::InvalidAuthTokenSecret => "invalid auth token secret",
            ServerConfigErrorCode::InvalidAuthTokenByteLength => "invalid auth token byte length",
            ServerConfigErrorCode::UnsupportedProductLogSink => "unsupported Product Log sink",
            ServerConfigErrorCode::UnsupportedDevelopmentLogMode => {
                "unsupported development log mode"
            }
        }
    }

    pub const fn code_str(&self) -> &'static str {
        self.code.as_str()
    }
}

fn validate_public_url(value: &str) -> Result<String, ServerConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.contains(char::is_whitespace)
        || !(trimmed.starts_with("http://") || trimmed.starts_with("https://"))
    {
        return Err(ServerConfigError::new(
            ServerConfigErrorCode::InvalidPublicUrl,
        ));
    }
    Ok(trimmed.to_string())
}

fn validate_object_storage_endpoint(value: &str) -> Result<String, ServerConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.contains(char::is_whitespace)
        || !(trimmed.starts_with("http://") || trimmed.starts_with("https://"))
    {
        return Err(ServerConfigError::new(
            ServerConfigErrorCode::InvalidObjectStorageEndpoint,
        ));
    }
    Ok(trimmed.to_string())
}

fn validate_object_storage_bucket(value: &str) -> Result<String, ServerConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.contains(char::is_whitespace)
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.chars().any(char::is_control)
    {
        return Err(ServerConfigError::new(
            ServerConfigErrorCode::InvalidObjectStorageBucket,
        ));
    }
    Ok(trimmed.to_string())
}

fn validate_object_storage_access_key_id(value: &str) -> Result<String, ServerConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(ServerConfigError::new(
            ServerConfigErrorCode::InvalidObjectStorageAccessKeyId,
        ));
    }
    Ok(trimmed.to_string())
}

fn validate_required_path(
    value: &str,
    error_code: ServerConfigErrorCode,
) -> Result<PathBuf, ServerConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ServerConfigError::new(error_code));
    }
    Ok(PathBuf::from(trimmed))
}
