use cabinet_desktop_shell::{
    DesktopGraphCameraPreferenceDto, DesktopGraphPreferenceDto,
    DesktopGraphPreferenceLoadRequestDto, DesktopGraphPreferenceRuntime,
    DesktopGraphPreferenceSaveRequestDto,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "cabinet-graph-preference-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn preference() -> DesktopGraphPreferenceDto {
    DesktopGraphPreferenceDto {
        schema_version: 2,
        depth: 2,
        direction: "incoming".into(),
        include_unresolved: false,
        include_assets: true,
        include_external: false,
        camera: DesktopGraphCameraPreferenceDto {
            center_x: 25.0,
            center_y: -50.0,
            zoom_percent: 150.0,
        },
    }
}

#[test]
fn clean_install_defaults_and_saved_preference_round_trips() {
    let root = temporary_root("roundtrip");
    let runtime = DesktopGraphPreferenceRuntime::new(root.clone());
    let request = DesktopGraphPreferenceLoadRequestDto {
        workspace_id: "workspace-1".into(),
    };
    let initial = runtime.load(request.clone());
    assert!(initial.ok);
    assert_eq!(initial.data.unwrap().depth, 1);

    let saved = runtime.save(DesktopGraphPreferenceSaveRequestDto {
        workspace_id: "workspace-1".into(),
        preference: preference(),
    });
    assert!(saved.ok);
    assert_eq!(runtime.load(request).data, Some(preference()));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn corrupt_or_invalid_preference_falls_back_without_exposing_a_path() {
    let root = temporary_root("corrupt");
    let runtime = DesktopGraphPreferenceRuntime::new(root.clone());
    assert!(
        runtime
            .save(DesktopGraphPreferenceSaveRequestDto {
                workspace_id: "workspace-1".into(),
                preference: preference(),
            })
            .ok
    );
    let settings_root = root.join("ui-settings/graph");
    let file = std::fs::read_dir(&settings_root)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::write(file, b"not-json").unwrap();
    let response = runtime.load(DesktopGraphPreferenceLoadRequestDto {
        workspace_id: "workspace-1".into(),
    });
    assert!(response.ok);
    assert_eq!(response.data.as_ref().unwrap().depth, 1);
    assert!(!format!("{response:?}").contains(root.to_string_lossy().as_ref()));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn invalid_workspace_and_out_of_range_preference_fail_closed() {
    let root = temporary_root("invalid");
    let runtime = DesktopGraphPreferenceRuntime::new(root.clone());
    let invalid_workspace = runtime.load(DesktopGraphPreferenceLoadRequestDto {
        workspace_id: " ".into(),
    });
    assert!(!invalid_workspace.ok);
    assert_eq!(
        invalid_workspace.error_code.as_deref(),
        Some("GRAPH_PREFERENCE_WORKSPACE_INVALID")
    );
    let mut invalid = preference();
    invalid.camera.zoom_percent = 900.0;
    let response = runtime.save(DesktopGraphPreferenceSaveRequestDto {
        workspace_id: "workspace-1".into(),
        preference: invalid,
    });
    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("GRAPH_PREFERENCE_INVALID")
    );
    let _ = std::fs::remove_dir_all(root);
}
