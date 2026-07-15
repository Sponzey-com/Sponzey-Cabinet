use std::fs;
use std::path::PathBuf;

#[test]
fn tauri_composition_registers_projection_repair_operation_commands() {
    let source = fs::read_to_string(source_root().join("src").join("main.rs")).unwrap();
    for command in [
        "start_desktop_projection_repair",
        "get_desktop_projection_repair_status",
        "cancel_desktop_projection_repair",
        "retry_desktop_projection_repair",
        "run_desktop_projection_repair_operation",
    ] {
        assert!(source.contains(command), "missing command {command}");
    }
    let compact: String = source.split_whitespace().collect();
    assert!(compact.contains("DesktopProjectionRepairOperationRuntime::new("));
    assert!(compact.contains("app_data_dir.clone()"));
}

#[test]
fn repair_operation_response_serializes_only_stable_status_fields() {
    let root = std::env::temp_dir().join(format!("cabinet-repair-contract-{}", std::process::id()));
    let runtime = cabinet_desktop_shell::DesktopProjectionRepairOperationRuntime::new(root.clone());
    let response = runtime.start("workspace-1", "doc-1");
    let json = serde_json::to_value(response).unwrap();
    assert_eq!(json["ok"], true);
    assert!(json["operationId"].as_str().unwrap().starts_with("repair-"));
    assert_eq!(json["state"], "queued");
    assert!(json.get("documentBody").is_none());
    assert!(json.get("path").is_none());
    let _ = fs::remove_dir_all(root);
}

fn source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
