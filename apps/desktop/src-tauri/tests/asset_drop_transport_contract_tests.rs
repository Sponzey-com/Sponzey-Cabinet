use std::fs;

#[test]
fn tauri_composition_registers_path_private_asset_drop_bridge() {
    let source = fs::read_to_string("src/main.rs").expect("main source");

    assert!(source.contains(".on_webview_event("));
    assert!(source.contains("tauri::WebviewEvent::DragDrop"));
    assert!(source.contains("tauri::DragDropEvent::Drop"));
    assert!(source.contains("register_selected_paths(paths.clone())"));
    assert!(source.contains("ASSET_DROP_SELECTION_EVENT"));
    assert!(source.contains("ASSET_DRAG_STATE_EVENT"));
    assert!(!source.contains("emit(ASSET_DROP_SELECTION_EVENT, paths"));
}

#[test]
fn asset_drop_custom_event_names_and_payload_are_stable() {
    let source = fs::read_to_string("src/main.rs").expect("main source");

    assert!(source.contains("cabinet-asset-drop-selection"));
    assert!(source.contains("cabinet-asset-drag-state"));
    assert!(source.contains("file_count"));
}
