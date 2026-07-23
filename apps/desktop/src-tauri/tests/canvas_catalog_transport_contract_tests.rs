#[test]
fn tauri_canvas_catalog_commands_are_registered_with_one_bootstrap_policy() {
    let source = include_str!("../src/main.rs");

    for command in ["get_desktop_canvas_catalog", "select_desktop_canvas"] {
        assert!(
            source.contains(&format!("fn {command}(")),
            "missing command function: {command}"
        );
        assert!(
            source.contains(&format!("            {command},")),
            "missing command registration: {command}"
        );
    }
    assert!(source.contains("DEFAULT_CANVAS_CATALOG_LIMIT"));
    assert!(source.contains("DesktopCanvasCatalogRuntime::new("));
    assert!(!source.contains("std::env::var(\"CANVAS"));
}
