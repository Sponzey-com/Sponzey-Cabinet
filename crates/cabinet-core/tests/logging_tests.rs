use cabinet_core::config::LoggingConfig;
use cabinet_core::logging::{
    DevelopmentLogEvent, DevelopmentLogEventName, DevelopmentLogger, FieldDebugLogEvent,
    FieldDebugLogEventName, FieldDebugLogger, FieldDebugScope, FieldDebugTtl, LogErrorCode,
    LogField, LogFieldError, LogLevel, LogRecord, ProductLogEvent, ProductLogEventName,
    ProductLogger,
};

#[derive(Default)]
struct CaptureProductLogger {
    records: Vec<LogRecord>,
}

impl ProductLogger for CaptureProductLogger {
    fn write_product(&mut self, event: ProductLogEvent) {
        self.records.push(event.to_record());
    }
}

#[derive(Default)]
struct CaptureFieldDebugLogger {
    records: Vec<LogRecord>,
}

impl FieldDebugLogger for CaptureFieldDebugLogger {
    fn write_field_debug(&mut self, event: FieldDebugLogEvent) {
        self.records.push(event.to_record());
    }
}

#[derive(Default)]
struct CaptureDevelopmentLogger {
    records: Vec<LogRecord>,
}

impl DevelopmentLogger for CaptureDevelopmentLogger {
    fn write_development(&mut self, event: DevelopmentLogEvent) {
        self.records.push(event.to_record());
    }
}

#[test]
fn product_log_event_uses_stable_name_and_error_code_only() {
    let event = ProductLogEvent::new(
        ProductLogEventName::FirstRunFailed,
        Some(LogErrorCode::FirstRunStoreCreationFailed),
    );

    let record = event.to_record();

    assert_eq!(record.level, LogLevel::Product);
    assert_eq!(record.event_name, "first_run.failed");
    assert_eq!(record.error_code, Some("FIRST_RUN_STORE_CREATION_FAILED"));
    assert!(record.fields.is_empty());
}

#[test]
fn logger_ports_are_separated_by_log_policy_level() {
    let mut product = CaptureProductLogger::default();
    let mut field_debug = CaptureFieldDebugLogger::default();
    let mut development = CaptureDevelopmentLogger::default();

    product.write_product(ProductLogEvent::new(
        ProductLogEventName::MigrationCompleted,
        None,
    ));
    field_debug.write_field_debug(
        FieldDebugLogEvent::new(
            FieldDebugLogEventName::MigrationState,
            FieldDebugScope::new("migration", FieldDebugTtl::minutes(15)).expect("valid scope"),
            vec![LogField::new("state", "Running").expect("safe field")],
        )
        .expect("valid field debug event"),
    );
    development.write_development(
        DevelopmentLogEvent::new(
            DevelopmentLogEventName::FakePortCall,
            vec![LogField::new("state", "called").expect("safe field")],
        )
        .expect("valid development event"),
    );

    assert_eq!(product.records[0].level, LogLevel::Product);
    assert_eq!(field_debug.records[0].level, LogLevel::FieldDebug);
    assert_eq!(development.records[0].level, LogLevel::Development);
}

#[test]
fn sanitized_log_field_rejects_sensitive_keys_values_and_raw_paths() {
    assert_eq!(
        LogField::new("document_body", "hello").expect_err("document body must be rejected"),
        LogFieldError::SensitiveKey
    );
    assert_eq!(
        LogField::new("path_role", "/Users/example/secret.md").expect_err("raw path rejected"),
        LogFieldError::SensitiveValue
    );
    assert_eq!(
        LogField::new("state", "token=abc").expect_err("secret-like value rejected"),
        LogFieldError::SensitiveValue
    );
    assert!(LogField::new("path_role", "metadata_store").is_ok());
}

#[test]
fn logging_config_defaults_keep_field_debug_and_development_disabled() {
    let config = LoggingConfig::default();

    assert_eq!(config.product_log_enabled, true);
    assert_eq!(config.field_debug_enabled, false);
    assert_eq!(config.development_log_enabled, false);
}
