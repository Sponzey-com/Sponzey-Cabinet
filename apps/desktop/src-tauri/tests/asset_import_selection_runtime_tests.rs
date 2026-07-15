use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_desktop_shell::DesktopAssetImportSelectionRuntime;

#[test]
fn selection_runtime_returns_opaque_path_free_descriptors_and_cancel_result() {
    let root = std::env::temp_dir().join(format!(
        "sponzey-picker-runtime-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let path = root.join("design.pdf");
    fs::write(&path, b"pdf-data").expect("fixture");
    let runtime = DesktopAssetImportSelectionRuntime::new(64).expect("runtime");

    let selected = runtime.register_selected_paths(vec![path.clone()]);
    let cancelled = runtime.register_selected_paths(Vec::new());
    let json = serde_json::to_string(&selected).expect("json");

    assert!(selected.ok);
    assert_eq!(selected.data.as_ref().expect("data").files.len(), 1);
    assert!(
        selected.data.as_ref().expect("data").files[0]
            .handle
            .starts_with("picker:")
    );
    assert_eq!(
        selected.data.as_ref().expect("data").files[0].file_name,
        "design.pdf"
    );
    assert!(!json.contains(path.to_string_lossy().as_ref()));
    assert!(cancelled.ok);
    assert!(cancelled.data.expect("cancelled data").cancelled);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn selection_runtime_maps_unsafe_source_without_path_detail() {
    let runtime = DesktopAssetImportSelectionRuntime::new(64).expect("runtime");
    let response = runtime.register_selected_paths(vec![std::env::temp_dir()]);
    let json = serde_json::to_string(&response).expect("json");

    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("asset_import.unsafe_source")
    );
    assert!(!json.contains(std::env::temp_dir().to_string_lossy().as_ref()));
}
