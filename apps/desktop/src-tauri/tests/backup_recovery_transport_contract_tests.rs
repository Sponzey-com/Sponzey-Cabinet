use std::fs;
use std::path::PathBuf;

#[test]
fn tauri_composition_registers_backup_recovery_runtime_and_commands() {
    let source = fs::read_to_string(source_root().join("src").join("main.rs")).unwrap();
    for command in [
        "create_desktop_backup_package",
        "list_desktop_backup_catalog",
        "preview_desktop_backup_restore",
        "confirm_desktop_backup_restore",
        "cancel_desktop_backup_restore",
        "recover_desktop_backup_startup",
        "start_desktop_backup_operation",
        "get_desktop_backup_operation_status",
        "cancel_desktop_backup_operation",
        "start_desktop_restore_operation",
        "get_desktop_restore_operation_status",
    ] {
        assert!(source.contains(command), "missing command {command}");
    }
    let compact: String = source.split_whitespace().collect();
    assert!(compact.contains("DesktopBackupRecoveryRuntime::new_with_projection_policy("));
    assert!(compact.contains("DEFAULT_BACKUP_MAX_FILE_COUNT"));
    assert!(compact.contains("DEFAULT_BACKUP_MAX_TOTAL_BYTES"));
    assert!(compact.contains("tauri::async_runtime::spawn_blocking"));
}

fn source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
