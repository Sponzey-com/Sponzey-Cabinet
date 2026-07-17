use cabinet_desktop_shell::{
    PackagedUiSmokeAssetFixture, PackagedUiSmokeCanvasFixture, PackagedUiSmokeErrorCode,
    PackagedUiSmokeFailureStage, PackagedUiSmokeMode, PackagedUiSmokeReport,
    validate_packaged_ui_smoke_report,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn document_workflow_failures_have_stable_non_sensitive_substage_codes() {
    assert_eq!(
        PackagedUiSmokeFailureStage::DocumentCreate.error_code(),
        "PHASE012_PACKAGED_UI_DOCUMENT_CREATE_FAILED"
    );
    assert_eq!(
        PackagedUiSmokeFailureStage::DocumentEdit.error_code(),
        "PHASE012_PACKAGED_UI_DOCUMENT_EDIT_FAILED"
    );
    assert_eq!(
        PackagedUiSmokeFailureStage::DocumentSave.error_code(),
        "PHASE012_PACKAGED_UI_DOCUMENT_SAVE_FAILED"
    );
    assert_eq!(
        PackagedUiSmokeFailureStage::DocumentReopen.error_code(),
        "PHASE012_PACKAGED_UI_DOCUMENT_REOPEN_FAILED"
    );
}

#[test]
fn graph_workflow_failures_have_stable_non_sensitive_action_codes() {
    let cases = [
        (
            PackagedUiSmokeFailureStage::GraphOpen,
            "PHASE012_PACKAGED_UI_GRAPH_OPEN_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphScopeGlobal,
            "PHASE012_PACKAGED_UI_GRAPH_SCOPE_GLOBAL_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphScopeLocal,
            "PHASE012_PACKAGED_UI_GRAPH_SCOPE_LOCAL_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphDepth,
            "PHASE012_PACKAGED_UI_GRAPH_DEPTH_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphDirection,
            "PHASE012_PACKAGED_UI_GRAPH_DIRECTION_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphUnresolved,
            "PHASE012_PACKAGED_UI_GRAPH_UNRESOLVED_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphAssets,
            "PHASE012_PACKAGED_UI_GRAPH_ASSETS_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphZoomIn,
            "PHASE012_PACKAGED_UI_GRAPH_ZOOM_IN_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphFitView,
            "PHASE012_PACKAGED_UI_GRAPH_FIT_VIEW_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphNode,
            "PHASE012_PACKAGED_UI_GRAPH_NODE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphDocumentRoute,
            "PHASE012_PACKAGED_UI_GRAPH_DOCUMENT_ROUTE_FAILED",
        ),
    ];
    for (stage, code) in cases {
        assert_eq!(stage.error_code(), code);
    }
}

#[test]
fn asset_workflow_failures_have_stable_non_sensitive_action_codes() {
    let cases = [
        (
            PackagedUiSmokeFailureStage::AssetOpen,
            "PHASE012_PACKAGED_UI_ASSET_OPEN_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::AssetImport,
            "PHASE012_PACKAGED_UI_ASSET_IMPORT_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::AssetDetail,
            "PHASE012_PACKAGED_UI_ASSET_DETAIL_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::AssetPreview,
            "PHASE012_PACKAGED_UI_ASSET_PREVIEW_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::AssetUnlink,
            "PHASE012_PACKAGED_UI_ASSET_UNLINK_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::AssetLibrary,
            "PHASE012_PACKAGED_UI_ASSET_LIBRARY_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::AssetDetachedDetail,
            "PHASE012_PACKAGED_UI_ASSET_DETACHED_DETAIL_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::AssetRelink,
            "PHASE012_PACKAGED_UI_ASSET_RELINK_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::AssetFilters,
            "PHASE012_PACKAGED_UI_ASSET_FILTERS_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasAsset,
            "PHASE012_PACKAGED_UI_CANVAS_ASSET_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasAssetRoute,
            "PHASE012_PACKAGED_UI_CANVAS_ASSET_ROUTE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::AssetDocumentRoute,
            "PHASE012_PACKAGED_UI_ASSET_DOCUMENT_ROUTE_FAILED",
        ),
    ];
    for (stage, code) in cases {
        assert_eq!(stage.error_code(), code);
    }
}

#[test]
fn backup_restore_failures_have_stable_non_sensitive_action_codes() {
    let cases = [
        (
            PackagedUiSmokeFailureStage::BackupOpen,
            "PHASE012_PACKAGED_UI_BACKUP_OPEN_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::BackupCreate,
            "PHASE012_PACKAGED_UI_BACKUP_CREATE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::RestorePreview,
            "PHASE012_PACKAGED_UI_RESTORE_PREVIEW_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::RestoreConfirm,
            "PHASE012_PACKAGED_UI_RESTORE_CONFIRM_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::RestoreReopen,
            "PHASE012_PACKAGED_UI_RESTORE_REOPEN_FAILED",
        ),
    ];
    for (stage, code) in cases {
        assert_eq!(stage.error_code(), code);
    }
}

#[test]
fn canvas_workflow_failures_have_stable_non_sensitive_action_codes() {
    let cases = [
        (
            PackagedUiSmokeFailureStage::CanvasOpen,
            "PHASE012_PACKAGED_UI_CANVAS_OPEN_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasCreate,
            "PHASE012_PACKAGED_UI_CANVAS_CREATE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasNote,
            "PHASE012_PACKAGED_UI_CANVAS_NOTE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasPan,
            "PHASE012_PACKAGED_UI_CANVAS_PAN_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasZoom,
            "PHASE012_PACKAGED_UI_CANVAS_ZOOM_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasArrange,
            "PHASE012_PACKAGED_UI_CANVAS_ARRANGE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasDocument,
            "PHASE012_PACKAGED_UI_CANVAS_DOCUMENT_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasEdge,
            "PHASE012_PACKAGED_UI_CANVAS_EDGE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasDrag,
            "PHASE012_PACKAGED_UI_CANVAS_DRAG_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasResize,
            "PHASE012_PACKAGED_UI_CANVAS_RESIZE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasReopen,
            "PHASE012_PACKAGED_UI_CANVAS_REOPEN_FAILED",
        ),
    ];
    for (stage, code) in cases {
        assert_eq!(stage.error_code(), code);
    }
}

#[test]
fn canvas_lifecycle_failures_have_stable_non_sensitive_action_codes() {
    let cases = [
        (
            PackagedUiSmokeFailureStage::CanvasRename,
            "PHASE012_PACKAGED_UI_CANVAS_RENAME_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasArchive,
            "PHASE012_PACKAGED_UI_CANVAS_ARCHIVE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::CanvasArchiveReopen,
            "PHASE012_PACKAGED_UI_CANVAS_ARCHIVE_REOPEN_FAILED",
        ),
    ];
    for (stage, code) in cases {
        assert_eq!(stage.error_code(), code);
    }
}

#[test]
fn disabled_mode_is_the_default_and_does_not_expose_a_profile_path() {
    let mode = PackagedUiSmokeMode::disabled();

    assert!(!mode.is_enabled());
    assert_eq!(mode.public_response().enabled, false);
}

#[test]
fn asset_fixture_is_available_only_when_explicitly_enabled() {
    assert_eq!(
        PackagedUiSmokeAssetFixture::disabled().selected_paths(),
        None
    );

    let path = PathBuf::from("sanitized-smoke-asset.txt");
    assert_eq!(
        PackagedUiSmokeAssetFixture::enabled(path.clone()).selected_paths(),
        Some(vec![path])
    );
}

#[test]
fn canvas_corruption_fixture_is_fail_closed_and_changes_only_the_default_current_pointer() {
    assert_eq!(
        PackagedUiSmokeCanvasFixture::disabled().corrupt_default_current_pointer(),
        Err("PACKAGED_UI_FIXTURE_DISABLED")
    );

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cabinet-packaged-canvas-fixture-{}-{nonce}",
        std::process::id()
    ));
    let current = root
        .join("canvases")
        .join("776f726b73706163652d31")
        .join("64656661756c742d63616e766173")
        .join("current.canvas");
    fs::create_dir_all(current.parent().expect("current parent")).expect("canvas fixture dir");
    fs::write(&current, "valid pointer placeholder").expect("seed current pointer");
    let unrelated = root.join("unrelated");
    fs::write(&unrelated, "preserve").expect("seed unrelated file");

    PackagedUiSmokeCanvasFixture::enabled(root.clone())
        .corrupt_default_current_pointer()
        .expect("corrupt pointer");
    assert_eq!(
        fs::read_to_string(&current).expect("corrupt pointer read"),
        "corrupt packaged smoke pointer\n"
    );
    assert_eq!(
        fs::read_to_string(unrelated).expect("unrelated read"),
        "preserve"
    );
    fs::remove_dir_all(root).expect("fixture cleanup");
}

#[test]
fn canvas_recovery_failure_has_stable_non_sensitive_action_code() {
    assert_eq!(
        PackagedUiSmokeFailureStage::CanvasRecovery,
        PackagedUiSmokeFailureStage::CanvasRecovery
    );
    assert_eq!(
        PackagedUiSmokeFailureStage::CanvasRecovery.error_code(),
        "PHASE012_PACKAGED_UI_CANVAS_RECOVERY_FAILED"
    );
}

#[test]
fn valid_report_requires_all_real_surfaces_and_the_full_sample_count() {
    let result = validate_packaged_ui_smoke_report(PackagedUiSmokeReport {
        home_ready: true,
        graph_ready: true,
        canvas_ready: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        keyboard_document_workflow_verified: true,
        sample_count: 200,
        p95_ms: 299,
        error_count: 0,
        failure_stage: None,
        action_count: 91,
        durable_readback_count: 33,
    });

    assert!(result.is_ok());
}

#[test]
fn invalid_report_returns_stable_non_sensitive_error_codes() {
    let missing = validate_packaged_ui_smoke_report(PackagedUiSmokeReport {
        home_ready: true,
        graph_ready: false,
        canvas_ready: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        keyboard_document_workflow_verified: true,
        sample_count: 200,
        p95_ms: 20,
        error_count: 0,
        failure_stage: None,
        action_count: 15,
        durable_readback_count: 4,
    });
    assert_eq!(missing, Err(PackagedUiSmokeErrorCode::SurfaceMissing));

    let slow = validate_packaged_ui_smoke_report(PackagedUiSmokeReport {
        home_ready: true,
        graph_ready: true,
        canvas_ready: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        keyboard_document_workflow_verified: true,
        sample_count: 200,
        p95_ms: 301,
        error_count: 0,
        failure_stage: None,
        action_count: 91,
        durable_readback_count: 33,
    });
    assert_eq!(
        slow,
        Err(PackagedUiSmokeErrorCode::PerformanceBudgetExceeded)
    );
}

#[test]
fn report_rejects_route_only_coverage_without_mutation_readbacks() {
    let result = validate_packaged_ui_smoke_report(PackagedUiSmokeReport {
        home_ready: true,
        graph_ready: true,
        canvas_ready: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        keyboard_document_workflow_verified: true,
        sample_count: 200,
        p95_ms: 20,
        error_count: 0,
        failure_stage: None,
        action_count: 4,
        durable_readback_count: 0,
    });

    assert_eq!(
        result,
        Err(PackagedUiSmokeErrorCode::ActionCoverageIncomplete)
    );
}

#[test]
fn report_requires_document_version_workflow_evidence() {
    let result = validate_packaged_ui_smoke_report(PackagedUiSmokeReport {
        home_ready: true,
        graph_ready: true,
        canvas_ready: true,
        assets_ready: true,
        document_version_workflow_verified: false,
        document_attachment_workflow_verified: true,
        keyboard_document_workflow_verified: true,
        sample_count: 200,
        p95_ms: 20,
        error_count: 0,
        failure_stage: None,
        action_count: 91,
        durable_readback_count: 33,
    });

    assert_eq!(
        result,
        Err(PackagedUiSmokeErrorCode::DocumentVersionWorkflowMissing)
    );
}

#[test]
fn report_requires_document_attachment_workflow_evidence() {
    let result = validate_packaged_ui_smoke_report(PackagedUiSmokeReport {
        home_ready: true,
        graph_ready: true,
        canvas_ready: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: false,
        keyboard_document_workflow_verified: true,
        sample_count: 200,
        p95_ms: 20,
        error_count: 0,
        failure_stage: None,
        action_count: 97,
        durable_readback_count: 35,
    });

    assert_eq!(
        result,
        Err(PackagedUiSmokeErrorCode::DocumentAttachmentWorkflowMissing)
    );
}

#[test]
fn report_requires_keyboard_document_workflow_evidence() {
    let result = validate_packaged_ui_smoke_report(PackagedUiSmokeReport {
        home_ready: true,
        graph_ready: true,
        canvas_ready: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        keyboard_document_workflow_verified: false,
        sample_count: 200,
        p95_ms: 20,
        error_count: 0,
        failure_stage: None,
        action_count: 97,
        durable_readback_count: 35,
    });

    assert_eq!(
        result,
        Err(PackagedUiSmokeErrorCode::KeyboardDocumentWorkflowMissing)
    );
}

#[test]
fn document_attachment_failures_have_stable_non_sensitive_codes() {
    let cases = [
        (
            PackagedUiSmokeFailureStage::DocumentAttachmentTab,
            "PHASE012_PACKAGED_UI_DOCUMENT_ATTACHMENT_TAB_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::DocumentAttachmentOpen,
            "PHASE012_PACKAGED_UI_DOCUMENT_ATTACHMENT_OPEN_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::DocumentAttachmentUnlinkRequest,
            "PHASE012_PACKAGED_UI_DOCUMENT_ATTACHMENT_UNLINK_REQUEST_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::DocumentAttachmentUnlinkCancel,
            "PHASE012_PACKAGED_UI_DOCUMENT_ATTACHMENT_UNLINK_CANCEL_FAILED",
        ),
    ];
    for (stage, code) in cases {
        assert_eq!(stage.error_code(), code);
    }
}
