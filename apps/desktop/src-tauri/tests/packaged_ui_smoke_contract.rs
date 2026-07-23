use cabinet_desktop_shell::{
    PackagedUiSmokeAssetFixture, PackagedUiSmokeCanvasFixture, PackagedUiSmokeErrorCode,
    PackagedUiSmokeFailureStage, PackagedUiSmokeMode, PackagedUiSmokeReport,
    PackagedUiSmokeRestartFailureStage, PackagedUiSmokeRestartReport, PackagedUiSmokeStage,
    validate_packaged_ui_smoke_initial_report, validate_packaged_ui_smoke_report,
    validate_packaged_ui_smoke_restart_report,
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
            PackagedUiSmokeFailureStage::GraphTargetSave,
            "PHASE015_PACKAGED_UI_GRAPH_TARGET_SAVE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphSourceSave,
            "PHASE015_PACKAGED_UI_GRAPH_SOURCE_SAVE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphProjection,
            "PHASE015_PACKAGED_UI_GRAPH_PROJECTION_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphLocalEdge,
            "PHASE015_PACKAGED_UI_GRAPH_LOCAL_EDGE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphGlobalEdge,
            "PHASE015_PACKAGED_UI_GRAPH_GLOBAL_EDGE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphSafeLabels,
            "PHASE015_PACKAGED_UI_GRAPH_SAFE_LABELS_FAILED",
        ),
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
fn graph_attachment_failures_have_stable_non_sensitive_action_codes() {
    let cases = [
        (
            PackagedUiSmokeFailureStage::GraphAttachmentOpen,
            "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_OPEN_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphAttachmentLocalEdge,
            "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_LOCAL_EDGE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphAttachmentLocalFilter,
            "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_LOCAL_FILTER_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphAttachmentLocalNode,
            "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_LOCAL_NODE_MISSING",
        ),
        (
            PackagedUiSmokeFailureStage::GraphAttachmentLocalIdentity,
            "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_LOCAL_IDENTITY_MISMATCH",
        ),
        (
            PackagedUiSmokeFailureStage::GraphAttachmentLocalLabel,
            "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_LOCAL_LABEL_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphAttachmentGlobalEdge,
            "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_GLOBAL_EDGE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::GraphAttachmentRoute,
            "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_ROUTE_FAILED",
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
            PackagedUiSmokeFailureStage::AssetImportReadback,
            "PHASE015_PACKAGED_UI_ASSET_IMPORT_READBACK_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::AssetImportOperation,
            "PHASE015_PACKAGED_UI_ASSET_IMPORT_OPERATION_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::AssetImportScope,
            "PHASE015_PACKAGED_UI_ASSET_IMPORT_SCOPE_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::AssetImportPresentation,
            "PHASE015_PACKAGED_UI_ASSET_IMPORT_PRESENTATION_FAILED",
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
        (
            PackagedUiSmokeFailureStage::RestoreHome,
            "PHASE016_PACKAGED_UI_RESTORE_HOME_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::RestoreDocument,
            "PHASE016_PACKAGED_UI_RESTORE_DOCUMENT_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::RestoreSearch,
            "PHASE016_PACKAGED_UI_RESTORE_SEARCH_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::RestoreGraph,
            "PHASE016_PACKAGED_UI_RESTORE_GRAPH_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::RestoreCanvas,
            "PHASE016_PACKAGED_UI_RESTORE_CANVAS_FAILED",
        ),
        (
            PackagedUiSmokeFailureStage::RestoreAssets,
            "PHASE016_PACKAGED_UI_RESTORE_ASSETS_FAILED",
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
            PackagedUiSmokeFailureStage::CanvasTextEdit,
            "PHASE017_PACKAGED_UI_CANVAS_TEXT_EDIT_FAILED",
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
    assert_eq!(mode.public_response().stage, None);
}

#[test]
fn enabled_smoke_mode_exposes_only_the_immutable_process_stage() {
    let initial = PackagedUiSmokeMode::enabled(PackagedUiSmokeStage::Initial);
    let restart = PackagedUiSmokeMode::enabled(PackagedUiSmokeStage::RestartVerification);
    let upgrade = PackagedUiSmokeMode::enabled(PackagedUiSmokeStage::UpgradeVerification);

    assert_eq!(
        initial.public_response().stage,
        Some(PackagedUiSmokeStage::Initial)
    );
    assert_eq!(
        restart.public_response().stage,
        Some(PackagedUiSmokeStage::RestartVerification)
    );
    assert_ne!(
        initial.public_response().stage,
        restart.public_response().stage
    );
    assert_eq!(
        upgrade.public_response().stage,
        Some(PackagedUiSmokeStage::UpgradeVerification)
    );
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
fn canvas_corruption_fixture_is_fail_closed_and_changes_only_the_discovered_current_pointer() {
    assert_eq!(
        PackagedUiSmokeCanvasFixture::disabled().corrupt_current_pointer(),
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
        .corrupt_current_pointer()
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
    assert_eq!(
        PackagedUiSmokeFailureStage::CanvasRecoveryOpen.error_code(),
        "PHASE017_PACKAGED_UI_CANVAS_RECOVERY_OPEN_FAILED"
    );
    assert_eq!(
        PackagedUiSmokeFailureStage::CanvasRecoveryDetect.error_code(),
        "PHASE017_PACKAGED_UI_CANVAS_RECOVERY_DETECT_FAILED"
    );
    assert_eq!(
        PackagedUiSmokeFailureStage::CanvasRecoveryApply.error_code(),
        "PHASE017_PACKAGED_UI_CANVAS_RECOVERY_APPLY_FAILED"
    );
}

#[test]
fn valid_report_requires_all_real_surfaces_and_the_full_sample_count() {
    let result = validate_packaged_ui_smoke_report(PackagedUiSmokeReport {
        home_ready: true,
        graph_ready: true,
        graph_link_fixture_saved: true,
        graph_local_edge_verified: true,
        graph_global_edge_verified: true,
        graph_safe_labels_verified: true,
        canvas_ready: true,
        canvas_text_edit_readback_verified: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        attachment_import_completed: true,
        attachment_current_readback_verified: true,
        attachment_document_readback_verified: true,
        attachment_restart_readback_verified: true,
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
fn attachment_evidence_cannot_be_replaced_by_aggregate_readback_counts() {
    let result = validate_packaged_ui_smoke_report(PackagedUiSmokeReport {
        home_ready: true,
        graph_ready: true,
        graph_link_fixture_saved: true,
        graph_local_edge_verified: true,
        graph_global_edge_verified: true,
        graph_safe_labels_verified: true,
        canvas_ready: true,
        canvas_text_edit_readback_verified: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        attachment_import_completed: true,
        attachment_current_readback_verified: true,
        attachment_document_readback_verified: true,
        attachment_restart_readback_verified: false,
        keyboard_document_workflow_verified: true,
        sample_count: 200,
        p95_ms: 20,
        error_count: 0,
        failure_stage: None,
        action_count: 999,
        durable_readback_count: 999,
    });

    assert_eq!(
        result,
        Err(PackagedUiSmokeErrorCode::AttachmentRestartReadbackMissing)
    );
}

#[test]
fn initial_report_requires_current_evidence_but_defers_restart_evidence() {
    let report = PackagedUiSmokeReport {
        home_ready: true,
        graph_ready: true,
        graph_link_fixture_saved: true,
        graph_local_edge_verified: true,
        graph_global_edge_verified: true,
        graph_safe_labels_verified: true,
        canvas_ready: true,
        canvas_text_edit_readback_verified: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        attachment_import_completed: true,
        attachment_current_readback_verified: true,
        attachment_document_readback_verified: true,
        attachment_restart_readback_verified: false,
        keyboard_document_workflow_verified: true,
        sample_count: 200,
        p95_ms: 20,
        error_count: 0,
        failure_stage: None,
        action_count: 999,
        durable_readback_count: 999,
    };

    assert!(validate_packaged_ui_smoke_initial_report(report).is_ok());
    assert_eq!(
        validate_packaged_ui_smoke_report(report),
        Err(PackagedUiSmokeErrorCode::AttachmentRestartReadbackMissing)
    );
}

#[test]
fn restart_report_requires_independent_readback_and_zero_errors() {
    assert!(
        validate_packaged_ui_smoke_restart_report(PackagedUiSmokeRestartReport {
            attachment_restart_readback_verified: true,
            canvas_text_restart_readback_verified: true,
            error_count: 0,
            failure_stage: None,
        })
        .is_ok()
    );
    assert_eq!(
        validate_packaged_ui_smoke_restart_report(PackagedUiSmokeRestartReport {
            attachment_restart_readback_verified: false,
            canvas_text_restart_readback_verified: true,
            error_count: 0,
            failure_stage: Some(PackagedUiSmokeRestartFailureStage::AttachmentDetail),
        }),
        Err(PackagedUiSmokeErrorCode::AttachmentRestartReadbackMissing)
    );
    assert_eq!(
        validate_packaged_ui_smoke_restart_report(PackagedUiSmokeRestartReport {
            attachment_restart_readback_verified: true,
            canvas_text_restart_readback_verified: true,
            error_count: 1,
            failure_stage: Some(PackagedUiSmokeRestartFailureStage::Home),
        }),
        Err(PackagedUiSmokeErrorCode::UiErrorReported)
    );
    assert_eq!(
        validate_packaged_ui_smoke_restart_report(PackagedUiSmokeRestartReport {
            attachment_restart_readback_verified: true,
            canvas_text_restart_readback_verified: false,
            error_count: 0,
            failure_stage: Some(PackagedUiSmokeRestartFailureStage::CanvasTextReadback),
        }),
        Err(PackagedUiSmokeErrorCode::CanvasTextRestartReadbackMissing)
    );
}

#[test]
fn restart_failure_stages_have_stable_non_sensitive_codes() {
    let cases = [
        (
            PackagedUiSmokeRestartFailureStage::Home,
            "PHASE015_PACKAGED_UI_RESTART_HOME_FAILED",
        ),
        (
            PackagedUiSmokeRestartFailureStage::Document,
            "PHASE015_PACKAGED_UI_RESTART_DOCUMENT_FAILED",
        ),
        (
            PackagedUiSmokeRestartFailureStage::AttachmentTab,
            "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_TAB_FAILED",
        ),
        (
            PackagedUiSmokeRestartFailureStage::AttachmentList,
            "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_LIST_FAILED",
        ),
        (
            PackagedUiSmokeRestartFailureStage::AttachmentListLoading,
            "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_LIST_LOADING",
        ),
        (
            PackagedUiSmokeRestartFailureStage::AttachmentListEmpty,
            "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_LIST_EMPTY",
        ),
        (
            PackagedUiSmokeRestartFailureStage::AttachmentListFailed,
            "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_LIST_QUERY_FAILED",
        ),
        (
            PackagedUiSmokeRestartFailureStage::AttachmentListMissing,
            "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_PANEL_MISSING",
        ),
        (
            PackagedUiSmokeRestartFailureStage::AttachmentDetail,
            "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_DETAIL_FAILED",
        ),
        (
            PackagedUiSmokeRestartFailureStage::CanvasOpen,
            "PHASE017_PACKAGED_UI_RESTART_CANVAS_OPEN_FAILED",
        ),
        (
            PackagedUiSmokeRestartFailureStage::CanvasCatalogSelect,
            "PHASE017_PACKAGED_UI_RESTART_CANVAS_CATALOG_SELECT_FAILED",
        ),
        (
            PackagedUiSmokeRestartFailureStage::CanvasTextReadback,
            "PHASE017_PACKAGED_UI_RESTART_CANVAS_TEXT_READBACK_FAILED",
        ),
    ];
    for (stage, expected) in cases {
        assert_eq!(stage.error_code(), expected);
    }
}

#[test]
fn every_structured_attachment_evidence_stage_has_a_stable_error() {
    let complete = PackagedUiSmokeReport {
        home_ready: true,
        graph_ready: true,
        graph_link_fixture_saved: true,
        graph_local_edge_verified: true,
        graph_global_edge_verified: true,
        graph_safe_labels_verified: true,
        canvas_ready: true,
        canvas_text_edit_readback_verified: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        attachment_import_completed: true,
        attachment_current_readback_verified: true,
        attachment_document_readback_verified: true,
        attachment_restart_readback_verified: true,
        keyboard_document_workflow_verified: true,
        sample_count: 200,
        p95_ms: 20,
        error_count: 0,
        failure_stage: None,
        action_count: 999,
        durable_readback_count: 999,
    };
    let cases = [
        (
            PackagedUiSmokeReport {
                attachment_import_completed: false,
                ..complete
            },
            PackagedUiSmokeErrorCode::AttachmentImportEvidenceMissing,
        ),
        (
            PackagedUiSmokeReport {
                attachment_current_readback_verified: false,
                ..complete
            },
            PackagedUiSmokeErrorCode::AttachmentCurrentReadbackMissing,
        ),
        (
            PackagedUiSmokeReport {
                attachment_document_readback_verified: false,
                ..complete
            },
            PackagedUiSmokeErrorCode::AttachmentDocumentReadbackMissing,
        ),
        (
            PackagedUiSmokeReport {
                attachment_restart_readback_verified: false,
                ..complete
            },
            PackagedUiSmokeErrorCode::AttachmentRestartReadbackMissing,
        ),
    ];

    for (report, expected) in cases {
        assert_eq!(validate_packaged_ui_smoke_report(report), Err(expected));
        assert!(
            expected
                .as_str()
                .starts_with("PHASE015_PACKAGED_UI_ATTACHMENT_")
        );
    }
}

#[test]
fn every_structured_topology_evidence_stage_has_a_stable_error() {
    let complete = PackagedUiSmokeReport {
        home_ready: true,
        graph_ready: true,
        graph_link_fixture_saved: true,
        graph_local_edge_verified: true,
        graph_global_edge_verified: true,
        graph_safe_labels_verified: true,
        canvas_ready: true,
        canvas_text_edit_readback_verified: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        attachment_import_completed: true,
        attachment_current_readback_verified: true,
        attachment_document_readback_verified: true,
        attachment_restart_readback_verified: true,
        keyboard_document_workflow_verified: true,
        sample_count: 200,
        p95_ms: 20,
        error_count: 0,
        failure_stage: None,
        action_count: 999,
        durable_readback_count: 999,
    };
    let cases = [
        (
            PackagedUiSmokeReport {
                graph_link_fixture_saved: false,
                ..complete
            },
            PackagedUiSmokeErrorCode::GraphLinkFixtureEvidenceMissing,
        ),
        (
            PackagedUiSmokeReport {
                graph_local_edge_verified: false,
                ..complete
            },
            PackagedUiSmokeErrorCode::GraphLocalEdgeEvidenceMissing,
        ),
        (
            PackagedUiSmokeReport {
                graph_global_edge_verified: false,
                ..complete
            },
            PackagedUiSmokeErrorCode::GraphGlobalEdgeEvidenceMissing,
        ),
        (
            PackagedUiSmokeReport {
                graph_safe_labels_verified: false,
                ..complete
            },
            PackagedUiSmokeErrorCode::GraphSafeLabelsEvidenceMissing,
        ),
    ];

    for (report, expected) in cases {
        assert_eq!(validate_packaged_ui_smoke_report(report), Err(expected));
        assert!(expected.as_str().starts_with("PHASE015_PACKAGED_UI_GRAPH_"));
    }
}

#[test]
fn invalid_report_returns_stable_non_sensitive_error_codes() {
    let missing = validate_packaged_ui_smoke_report(PackagedUiSmokeReport {
        home_ready: true,
        graph_ready: false,
        graph_link_fixture_saved: true,
        graph_local_edge_verified: true,
        graph_global_edge_verified: true,
        graph_safe_labels_verified: true,
        canvas_ready: true,
        canvas_text_edit_readback_verified: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        attachment_import_completed: true,
        attachment_current_readback_verified: true,
        attachment_document_readback_verified: true,
        attachment_restart_readback_verified: true,
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
        graph_link_fixture_saved: true,
        graph_local_edge_verified: true,
        graph_global_edge_verified: true,
        graph_safe_labels_verified: true,
        canvas_ready: true,
        canvas_text_edit_readback_verified: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        attachment_import_completed: true,
        attachment_current_readback_verified: true,
        attachment_document_readback_verified: true,
        attachment_restart_readback_verified: true,
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
        graph_link_fixture_saved: true,
        graph_local_edge_verified: true,
        graph_global_edge_verified: true,
        graph_safe_labels_verified: true,
        canvas_ready: true,
        canvas_text_edit_readback_verified: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        attachment_import_completed: true,
        attachment_current_readback_verified: true,
        attachment_document_readback_verified: true,
        attachment_restart_readback_verified: true,
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
        graph_link_fixture_saved: true,
        graph_local_edge_verified: true,
        graph_global_edge_verified: true,
        graph_safe_labels_verified: true,
        canvas_ready: true,
        canvas_text_edit_readback_verified: true,
        assets_ready: true,
        document_version_workflow_verified: false,
        document_attachment_workflow_verified: true,
        attachment_import_completed: true,
        attachment_current_readback_verified: true,
        attachment_document_readback_verified: true,
        attachment_restart_readback_verified: true,
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
        graph_link_fixture_saved: true,
        graph_local_edge_verified: true,
        graph_global_edge_verified: true,
        graph_safe_labels_verified: true,
        canvas_ready: true,
        canvas_text_edit_readback_verified: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: false,
        attachment_import_completed: true,
        attachment_current_readback_verified: true,
        attachment_document_readback_verified: true,
        attachment_restart_readback_verified: true,
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
        graph_link_fixture_saved: true,
        graph_local_edge_verified: true,
        graph_global_edge_verified: true,
        graph_safe_labels_verified: true,
        canvas_ready: true,
        canvas_text_edit_readback_verified: true,
        assets_ready: true,
        document_version_workflow_verified: true,
        document_attachment_workflow_verified: true,
        attachment_import_completed: true,
        attachment_current_readback_verified: true,
        attachment_document_readback_verified: true,
        attachment_restart_readback_verified: true,
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
