#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Product,
    FieldDebug,
    Development,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    pub level: LogLevel,
    pub event_name: &'static str,
    pub error_code: Option<&'static str>,
    pub fields: Vec<LogField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogField {
    pub key: String,
    pub value: String,
}

impl LogField {
    pub fn new(key: &str, value: &str) -> Result<Self, LogFieldError> {
        let trimmed_key = key.trim();
        let trimmed_value = value.trim();
        if trimmed_key.is_empty() {
            return Err(LogFieldError::EmptyKey);
        }
        if trimmed_value.is_empty() {
            return Err(LogFieldError::EmptyValue);
        }
        if is_sensitive_key(trimmed_key) {
            return Err(LogFieldError::SensitiveKey);
        }
        if is_sensitive_value(trimmed_value) {
            return Err(LogFieldError::SensitiveValue);
        }

        Ok(Self {
            key: trimmed_key.to_string(),
            value: trimmed_value.to_string(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFieldError {
    EmptyKey,
    EmptyValue,
    SensitiveKey,
    SensitiveValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogErrorCode {
    FirstRunStoreCreationFailed,
    FirstRunMetadataWriteFailed,
    MigrationVersionRecordFailed,
    MigrationLockAcquireFailed,
    LocalSetupUnhealthy,
}

impl LogErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FirstRunStoreCreationFailed => "FIRST_RUN_STORE_CREATION_FAILED",
            Self::FirstRunMetadataWriteFailed => "FIRST_RUN_METADATA_WRITE_FAILED",
            Self::MigrationVersionRecordFailed => "MIGRATION_VERSION_RECORD_FAILED",
            Self::MigrationLockAcquireFailed => "MIGRATION_LOCK_ACQUIRE_FAILED",
            Self::LocalSetupUnhealthy => "LOCAL_SETUP_UNHEALTHY",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductLogEventName {
    FirstRunCompleted,
    FirstRunFailed,
    MigrationCompleted,
    MigrationFailed,
    UsecaseFailed,
    LocalSetupUnhealthy,
}

impl ProductLogEventName {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FirstRunCompleted => "first_run.completed",
            Self::FirstRunFailed => "first_run.failed",
            Self::MigrationCompleted => "migration.completed",
            Self::MigrationFailed => "migration.failed",
            Self::UsecaseFailed => "usecase.failed",
            Self::LocalSetupUnhealthy => "local_setup.unhealthy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductLogEvent {
    name: ProductLogEventName,
    error_code: Option<LogErrorCode>,
}

impl ProductLogEvent {
    pub const fn new(name: ProductLogEventName, error_code: Option<LogErrorCode>) -> Self {
        Self { name, error_code }
    }

    pub fn to_record(self) -> LogRecord {
        LogRecord {
            level: LogLevel::Product,
            event_name: self.name.as_str(),
            error_code: self.error_code.map(LogErrorCode::as_str),
            fields: Vec::new(),
        }
    }
}

pub trait ProductLogger {
    fn write_product(&mut self, event: ProductLogEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldDebugLogEventName {
    FirstRunStep,
    MigrationState,
    CacheDiagnostic,
    IndexDiagnostic,
}

impl FieldDebugLogEventName {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FirstRunStep => "field.first_run.step",
            Self::MigrationState => "field.migration.state",
            Self::CacheDiagnostic => "field.cache.diagnostic",
            Self::IndexDiagnostic => "field.index.diagnostic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldDebugTtl {
    pub seconds: u32,
}

impl FieldDebugTtl {
    pub const fn minutes(minutes: u32) -> Self {
        Self {
            seconds: minutes * 60,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDebugScope {
    name: String,
    ttl: FieldDebugTtl,
}

impl FieldDebugScope {
    pub fn new(name: &str, ttl: FieldDebugTtl) -> Result<Self, LogFieldError> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err(LogFieldError::EmptyKey);
        }
        if ttl.seconds == 0 {
            return Err(LogFieldError::EmptyValue);
        }
        if is_sensitive_value(trimmed_name) {
            return Err(LogFieldError::SensitiveValue);
        }

        Ok(Self {
            name: trimmed_name.to_string(),
            ttl,
        })
    }

    fn fields(&self) -> Vec<LogField> {
        vec![
            LogField {
                key: "scope".to_string(),
                value: self.name.clone(),
            },
            LogField {
                key: "ttl_seconds".to_string(),
                value: self.ttl.seconds.to_string(),
            },
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDebugLogEvent {
    name: FieldDebugLogEventName,
    scope: FieldDebugScope,
    fields: Vec<LogField>,
}

impl FieldDebugLogEvent {
    pub fn new(
        name: FieldDebugLogEventName,
        scope: FieldDebugScope,
        fields: Vec<LogField>,
    ) -> Result<Self, LogFieldError> {
        Ok(Self {
            name,
            scope,
            fields,
        })
    }

    pub fn to_record(self) -> LogRecord {
        let mut fields = self.scope.fields();
        fields.extend(self.fields);
        LogRecord {
            level: LogLevel::FieldDebug,
            event_name: self.name.as_str(),
            error_code: None,
            fields,
        }
    }
}

pub trait FieldDebugLogger {
    fn write_field_debug(&mut self, event: FieldDebugLogEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentLogEventName {
    LocalTestSetup,
    FakePortCall,
    ParserIntermediateResult,
    BenchmarkDetail,
}

impl DevelopmentLogEventName {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalTestSetup => "dev.local_test_setup",
            Self::FakePortCall => "dev.fake_port_call",
            Self::ParserIntermediateResult => "dev.parser_intermediate_result",
            Self::BenchmarkDetail => "dev.benchmark_detail",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentLogEvent {
    name: DevelopmentLogEventName,
    fields: Vec<LogField>,
}

impl DevelopmentLogEvent {
    pub fn new(
        name: DevelopmentLogEventName,
        fields: Vec<LogField>,
    ) -> Result<Self, LogFieldError> {
        Ok(Self { name, fields })
    }

    pub fn to_record(self) -> LogRecord {
        LogRecord {
            level: LogLevel::Development,
            event_name: self.name.as_str(),
            error_code: None,
            fields: self.fields,
        }
    }
}

pub trait DevelopmentLogger {
    fn write_development(&mut self, event: DevelopmentLogEvent);
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    normalized != "path_role"
        && (normalized.contains("body")
            || normalized.contains("content")
            || normalized.contains("secret")
            || normalized.contains("token")
            || normalized.contains("password")
            || normalized == "path"
            || normalized.contains("raw_path"))
}

fn is_sensitive_value(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    normalized.contains("token=")
        || normalized.contains("password")
        || normalized.contains("secret")
        || value.starts_with('/')
        || value.contains('\\')
}
