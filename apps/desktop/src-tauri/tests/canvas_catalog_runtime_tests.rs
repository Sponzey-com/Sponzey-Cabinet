use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_desktop_shell::{
    DesktopCanvasCatalogQueryRequestDto, DesktopCanvasCatalogRuntime,
    DesktopCanvasCatalogSelectRequestDto, DesktopCanvasRequestDto, DesktopCanvasRuntime,
};

#[test]
fn native_canvas_catalog_returns_safe_initial_selection_and_persists_choice() {
    let root = temp_root();
    let canvas = DesktopCanvasRuntime::new(root.clone()).unwrap();
    for (id, title) in [("canvas-a", "첫 Canvas"), ("canvas-b", "최근 Canvas")] {
        assert!(
            canvas
                .execute(DesktopCanvasRequestDto::Create {
                    workspace_id: "workspace-1".into(),
                    canvas_id: id.into(),
                    title: title.into(),
                })
                .ok
        );
    }
    let runtime = DesktopCanvasCatalogRuntime::new(root.clone(), 100).unwrap();
    let first = runtime.query(DesktopCanvasCatalogQueryRequestDto {
        workspace_id: "workspace-1".into(),
        limit: 20,
        include_archived: true,
    });
    assert!(first.ok);
    let data = first.data.unwrap();
    assert_eq!(data.entries.len(), 2);
    assert_eq!(data.selection_source, "fallback");
    assert_eq!(
        data.selected_canvas_id.as_deref(),
        data.entries.first().map(|entry| entry.canvas_id.as_str())
    );
    let json = serde_json::to_string(&data).unwrap();
    assert!(!json.contains("path"));
    assert!(!json.contains("checksum"));
    assert!(!json.contains("content"));

    let selected = runtime.select(DesktopCanvasCatalogSelectRequestDto {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-b".into(),
    });
    assert!(selected.ok);
    assert_eq!(selected.selected_canvas_id.as_deref(), Some("canvas-b"));
    let restarted = DesktopCanvasCatalogRuntime::new(root.clone(), 100)
        .unwrap()
        .query(DesktopCanvasCatalogQueryRequestDto {
            workspace_id: "workspace-1".into(),
            limit: 20,
            include_archived: false,
        });
    assert_eq!(
        restarted.data.unwrap().selected_canvas_id.as_deref(),
        Some("canvas-b")
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_canvas_catalog_rejects_invalid_limit_and_missing_selection() {
    let root = temp_root();
    let runtime = DesktopCanvasCatalogRuntime::new(root.clone(), 10).unwrap();
    let invalid = runtime.query(DesktopCanvasCatalogQueryRequestDto {
        workspace_id: "workspace-1".into(),
        limit: 11,
        include_archived: false,
    });
    assert!(!invalid.ok);
    assert_eq!(
        invalid.error_code.as_deref(),
        Some("CANVAS_CATALOG_INVALID_LIMIT")
    );
    let missing = runtime.select(DesktopCanvasCatalogSelectRequestDto {
        workspace_id: "workspace-1".into(),
        canvas_id: "missing".into(),
    });
    assert!(!missing.ok);
    assert_eq!(
        missing.error_code.as_deref(),
        Some("canvas_selection.not_found")
    );
    let _ = fs::remove_dir_all(root);
}

fn temp_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cabinet-canvas-catalog-runtime-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}
