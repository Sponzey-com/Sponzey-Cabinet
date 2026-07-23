use cabinet_adapters::local_asset_external_opener::{
    ExternalPathLauncher, LocalAssetExternalOpener,
};
use cabinet_adapters::local_document_store_migration::LocalDocumentStoreMigration;
use cabinet_desktop_shell::{
    DesktopAssetDetailRequestDto, DesktopAssetDetailResponse, DesktopAssetExternalOpenResponse,
    DesktopAssetImportOperationRequestDto, DesktopAssetImportResponse,
    DesktopAssetImportSelectionResponse, DesktopAssetImportSelectionRuntime,
    DesktopAssetPreviewResponse, DesktopAssetSearchRequestDto, DesktopAssetSearchResponse,
    DesktopAssetSearchRuntime, DesktopBackupCatalogRequestDto, DesktopBackupCatalogResponse,
    DesktopBackupOperationRequestDto, DesktopBackupOperationResponse,
    DesktopBackupPackageRequestDto, DesktopBackupRecoveryRequestDto, DesktopBackupRecoveryResponse,
    DesktopBackupRecoveryRuntime, DesktopCanvasCatalogQueryRequestDto,
    DesktopCanvasCatalogResponse, DesktopCanvasCatalogRuntime,
    DesktopCanvasCatalogSelectRequestDto, DesktopCanvasRequestDto, DesktopCanvasResponse,
    DesktopCanvasRuntime, DesktopDocumentAssetsCommandResponse, DesktopDocumentAssetsRuntime,
    DesktopDocumentAttachmentMutationRequestDto, DesktopDocumentAttachmentMutationResponse,
    DesktopDocumentAttachmentMutationRuntime, DesktopDocumentAuthoringCommandResponse,
    DesktopDocumentAuthoringRequestDto, DesktopDocumentAuthoringRuntime,
    DesktopDocumentDiffOperationRequestDto, DesktopDocumentDiffOperationResponse,
    DesktopDocumentDiffOperationRuntime, DesktopDocumentDiffOperationTokenRequestDto,
    DesktopDocumentDiffRequestDto, DesktopDocumentDiffResponse, DesktopDocumentDiffRuntime,
    DesktopDocumentMutationRequestDto, DesktopDocumentMutationRuntime,
    DesktopDocumentNavigatorCommandResponse, DesktopDocumentNavigatorRequestDto,
    DesktopDocumentNavigatorRuntime, DesktopDocumentQueryRequestDto, DesktopDocumentQueryResponse,
    DesktopDocumentQueryRuntime, DesktopDocumentSearchCommandResponse,
    DesktopDocumentSearchRequestDto, DesktopDocumentSearchRuntime,
    DesktopGlobalKnowledgeGraphCommandResponse, DesktopGlobalKnowledgeGraphRequestDto,
    DesktopGlobalKnowledgeGraphRuntime, DesktopGraphPreferenceLoadRequestDto,
    DesktopGraphPreferenceLoadResponse, DesktopGraphPreferenceRuntime,
    DesktopGraphPreferenceSaveRequestDto, DesktopGraphPreferenceSaveResponse,
    DesktopKnowledgeGraphCommandResponse, DesktopKnowledgeGraphRuntime,
    DesktopLocalCommandRequestDto, DesktopLocalCommandRuntimeResponse,
    DesktopProjectionFreshnessResponse, DesktopProjectionIdentityRequestDto,
    DesktopProjectionReindexResponse, DesktopProjectionRepairOperationRequestDto,
    DesktopProjectionRepairOperationResponse, DesktopProjectionRepairOperationRuntime,
    DesktopProjectionRepairStartRequestDto, DesktopProjectionRunResponse, DesktopProjectionRuntime,
    DesktopRestoreCancelRequestDto, DesktopRestoreConfirmRequestDto,
    DesktopRevisionGuardedAssetImportRequestDto, DesktopShellRequest,
    DesktopWorkspaceAssetsRequestDto, DesktopWorkspaceAssetsResponse,
    DesktopWorkspaceHomeCommandResponse, DesktopWorkspaceHomeRuntime, PackagedUiSmokeAssetFixture,
    PackagedUiSmokeCanvasFixture, PackagedUiSmokeMode, PackagedUiSmokeModeResponse,
    PackagedUiSmokeReport, PackagedUiSmokeRestartReport, PackagedUiSmokeStage,
    create_desktop_package_smoke_report, route_desktop_command,
    route_local_desktop_command_request, route_tauri_command,
    validate_packaged_ui_smoke_initial_report, validate_packaged_ui_smoke_restart_report,
};
use cabinet_domain::document::DocumentBodyPolicy;
use cabinet_ports::asset_external_open::AssetExternalOpener;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, LogicalSize, Manager};
use tauri_plugin_dialog::DialogExt;

mod bounded_child_process;

const ASSET_DRAG_STATE_EVENT: &str = "cabinet-asset-drag-state";
const ASSET_DROP_SELECTION_EVENT: &str = "cabinet-asset-drop-selection";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopAssetDragStateEvent {
    state: &'static str,
    file_count: usize,
}

#[derive(Debug)]
struct PackagedUiSmokeVisualFixture {
    root: Option<PathBuf>,
    summary: Mutex<Option<PackagedUiSmokeVisualSummary>>,
    failure_code: Mutex<Option<String>>,
    failure_stage: Mutex<Option<String>>,
}

impl PackagedUiSmokeVisualFixture {
    fn new(root: Option<PathBuf>) -> Self {
        Self {
            root,
            summary: Mutex::new(None),
            failure_code: Mutex::new(None),
            failure_stage: Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackagedUiSmokeViewportRequest {
    width: u32,
    height: u32,
    zoom_percent: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackagedUiSmokeCaptureRequest {
    artifact_key: String,
    sample_bounds: Option<PackagedUiSmokeVisualRect>,
    viewport_width: u32,
    viewport_height: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackagedUiSmokeVisualRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackagedUiSmokeCaptureResponse {
    digest: String,
    byte_count: u64,
    sampled_pixel_count: u64,
    non_background_pixel_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackagedUiSmokeVisualSummary {
    route_viewport_count: u32,
    renderer_viewport_count: u32,
    artifact_count: u32,
    screenshot_count: u32,
    screenshot_aggregate_digest: String,
    accessibility_route_focus_count: u32,
    accessibility_keyboard_journey_count: u32,
    accessibility_focus_restoration_count: u32,
    accessibility_visible_control_count: u32,
    accessibility_named_control_count: u32,
    accessibility_text_zoom_percent: u32,
    accessibility_keyboard_error_count: u32,
    accessibility_focus_error_count: u32,
    accessibility_internal_exposure_count: u32,
}

const DEFAULT_DOCUMENT_BODY_MAX_BYTES: usize = 10 * 1024 * 1024;
const DEFAULT_DOCUMENT_DIFF_OPERATION_CAPACITY: usize = 32;
const DEFAULT_PROJECTION_BATCH_LIMIT: usize = 64;
const DEFAULT_PROJECTION_MAX_ATTEMPTS: u32 = 3;
const DEFAULT_PROJECTION_RECONCILE_DOCUMENT_LIMIT: usize = 100_000;
const DEFAULT_ASSET_IMPORT_CHUNK_BYTES: usize = 256 * 1024;
const DEFAULT_ASSET_PREVIEW_MAX_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_BACKUP_MAX_FILE_COUNT: u64 = 100_000;
const DEFAULT_BACKUP_MAX_TOTAL_BYTES: u64 = 20 * 1024 * 1024 * 1024;
const DEFAULT_CANVAS_CATALOG_LIMIT: usize = 100;
const PACKAGED_UPGRADE_DOCUMENT_ID: &str = "packaged-upgrade-document";
const PACKAGED_UPGRADE_DOCUMENT_TITLE: &str = "이전 버전 문서";
const PACKAGED_UPGRADE_DOCUMENT_BODY: &str = "# 이전 버전 문서\n\n업그레이드 보존 검증\n";

#[derive(Debug, Clone, Copy)]
struct PackagedSmokeExternalPathLauncher;

impl ExternalPathLauncher for PackagedSmokeExternalPathLauncher {
    fn launch(&self, path: &Path) -> Result<(), ()> {
        path.is_file().then_some(()).ok_or(())
    }
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();

    if args.first().is_some_and(|arg| arg == "--shell-smoke") {
        run_shell_smoke(args.get(1).cloned());
        return;
    }

    if args.first().is_some_and(|arg| arg == "--packaged-smoke") {
        run_packaged_smoke();
        return;
    }

    if args.first().is_some_and(|arg| arg == "--packaged-ui-smoke") {
        let Some(root) = args.get(1).map(PathBuf::from) else {
            println!("phase012_packaged_ui_smoke=failed");
            println!("error_code=PHASE012_PACKAGED_UI_PROFILE_MISSING");
            std::process::exit(1);
        };
        run_tauri_app(Some(root), Some(PackagedUiSmokeStage::Initial));
        return;
    }

    if args
        .first()
        .is_some_and(|arg| arg == "--packaged-ui-smoke-upgrade")
    {
        let Some(root) = args.get(1).map(PathBuf::from) else {
            println!("phase015_packaged_ui_smoke_initial=failed");
            println!("upgrade_existing_document_readback_verified=false");
            println!("error_code=PHASE016_PACKAGED_UI_UPGRADE_PROFILE_MISSING");
            std::process::exit(1);
        };
        if let Err(error_code) = seed_packaged_upgrade_profile(&root) {
            println!("phase015_packaged_ui_smoke_initial=failed");
            println!("upgrade_existing_document_readback_verified=false");
            println!("error_code={error_code}");
            std::process::exit(1);
        }
        run_tauri_app(Some(root), Some(PackagedUiSmokeStage::UpgradeVerification));
        return;
    }

    if args
        .first()
        .is_some_and(|arg| arg == "--packaged-ui-smoke-restart")
    {
        let Some(root) = args.get(1).map(PathBuf::from) else {
            println!("phase015_packaged_ui_smoke_restart=failed");
            println!("error_code=PHASE015_PACKAGED_UI_PROFILE_MISSING");
            std::process::exit(1);
        };
        run_tauri_app(Some(root), Some(PackagedUiSmokeStage::RestartVerification));
        return;
    }

    if args
        .first()
        .is_some_and(|arg| arg == "--packaged-ui-visual-smoke")
    {
        let Some(root) = args.get(1).map(PathBuf::from) else {
            println!("phase016_packaged_ui_visual_smoke=failed");
            println!("error_code=PHASE016_PACKAGED_UI_PROFILE_MISSING");
            std::process::exit(1);
        };
        run_tauri_app(Some(root), Some(PackagedUiSmokeStage::VisualEvidence));
        return;
    }

    run_tauri_app(None, None);
}

fn run_shell_smoke(command: Option<String>) {
    let command = command.unwrap_or_else(|| "open_workspace".to_string());
    let response = route_desktop_command(DesktopShellRequest { command });

    println!("Sponzey Cabinet desktop shell");
    println!("boundary={}", response.boundary);
    println!("command={}", response.command);
}

fn run_packaged_smoke() {
    let report = create_desktop_package_smoke_report();

    println!("Sponzey Cabinet packaged desktop smoke");
    println!("boundary={}", report.boundary);
    println!("dist_dir={}", report.dist_dir.display());
    println!("index_html_exists={}", report.index_html_exists);
    println!("app_bundle_exists={}", report.app_bundle_exists);
    println!("styles_css_exists={}", report.styles_css_exists);
    println!("node_runtime_required={}", report.node_runtime_required);

    if !report.index_html_exists || !report.app_bundle_exists || !report.styles_css_exists {
        std::process::exit(1);
    }
}

#[tauri::command]
fn route_desktop_shell_command(
    command: String,
) -> cabinet_desktop_shell::DesktopShellCommandResponse {
    route_tauri_command(command)
}

#[tauri::command]
fn route_desktop_local_command(
    request: DesktopLocalCommandRequestDto,
) -> DesktopLocalCommandRuntimeResponse {
    route_local_desktop_command_request(request)
}

#[tauri::command]
fn get_desktop_workspace_home(
    request: DesktopLocalCommandRequestDto,
    runtime: tauri::State<'_, DesktopWorkspaceHomeRuntime>,
) -> DesktopWorkspaceHomeCommandResponse {
    runtime.execute(request)
}

#[tauri::command]
fn get_desktop_document_navigator(
    request: DesktopDocumentNavigatorRequestDto,
    runtime: tauri::State<'_, DesktopDocumentNavigatorRuntime>,
) -> DesktopDocumentNavigatorCommandResponse {
    runtime.execute(request)
}

#[tauri::command]
fn search_desktop_documents(
    request: DesktopDocumentSearchRequestDto,
    runtime: tauri::State<'_, DesktopDocumentSearchRuntime>,
) -> DesktopDocumentSearchCommandResponse {
    runtime.execute(request)
}

#[tauri::command]
fn search_desktop_assets(
    request: DesktopAssetSearchRequestDto,
    runtime: tauri::State<'_, DesktopAssetSearchRuntime>,
) -> DesktopAssetSearchResponse {
    runtime.execute(request)
}

#[tauri::command]
fn execute_desktop_document_authoring(
    request: DesktopDocumentAuthoringRequestDto,
    runtime: tauri::State<'_, DesktopDocumentAuthoringRuntime>,
) -> DesktopDocumentAuthoringCommandResponse {
    runtime.execute(request)
}

#[tauri::command]
fn execute_desktop_document_mutation(
    request: DesktopDocumentMutationRequestDto,
    runtime: tauri::State<'_, DesktopDocumentMutationRuntime>,
) -> DesktopDocumentAuthoringCommandResponse {
    runtime.execute(request)
}

#[tauri::command]
fn execute_desktop_document_query(
    request: DesktopDocumentQueryRequestDto,
    runtime: tauri::State<'_, DesktopDocumentQueryRuntime>,
) -> DesktopDocumentQueryResponse {
    runtime.execute(request)
}

#[tauri::command]
fn execute_desktop_document_diff(
    request: DesktopDocumentDiffRequestDto,
    runtime: tauri::State<'_, DesktopDocumentDiffRuntime>,
) -> DesktopDocumentDiffResponse {
    runtime.execute(request)
}

#[tauri::command]
fn start_desktop_document_diff_operation(
    request: DesktopDocumentDiffOperationRequestDto,
    runtime: tauri::State<'_, DesktopDocumentDiffOperationRuntime>,
) -> DesktopDocumentDiffOperationResponse {
    runtime.start(request)
}

#[tauri::command]
fn get_desktop_document_diff_operation_status(
    request: DesktopDocumentDiffOperationTokenRequestDto,
    runtime: tauri::State<'_, DesktopDocumentDiffOperationRuntime>,
) -> DesktopDocumentDiffOperationResponse {
    runtime.status(request)
}

#[tauri::command]
fn cancel_desktop_document_diff_operation(
    request: DesktopDocumentDiffOperationTokenRequestDto,
    runtime: tauri::State<'_, DesktopDocumentDiffOperationRuntime>,
) -> DesktopDocumentDiffOperationResponse {
    runtime.cancel(request)
}

#[tauri::command]
fn execute_desktop_canvas(
    request: DesktopCanvasRequestDto,
    runtime: tauri::State<'_, DesktopCanvasRuntime>,
) -> DesktopCanvasResponse {
    runtime.execute(request)
}

#[tauri::command]
fn get_desktop_canvas_catalog(
    request: DesktopCanvasCatalogQueryRequestDto,
    runtime: tauri::State<'_, DesktopCanvasCatalogRuntime>,
) -> DesktopCanvasCatalogResponse {
    runtime.query(request)
}

#[tauri::command]
fn select_desktop_canvas(
    request: DesktopCanvasCatalogSelectRequestDto,
    runtime: tauri::State<'_, DesktopCanvasCatalogRuntime>,
) -> DesktopCanvasCatalogResponse {
    runtime.select(request)
}

#[tauri::command]
fn get_desktop_knowledge_graph(
    request: DesktopLocalCommandRequestDto,
    runtime: tauri::State<'_, DesktopKnowledgeGraphRuntime>,
) -> DesktopKnowledgeGraphCommandResponse {
    runtime.execute(request)
}

#[tauri::command]
fn get_desktop_global_knowledge_graph(
    request: DesktopGlobalKnowledgeGraphRequestDto,
    runtime: tauri::State<'_, DesktopGlobalKnowledgeGraphRuntime>,
) -> DesktopGlobalKnowledgeGraphCommandResponse {
    runtime.execute(request)
}

#[tauri::command]
fn get_desktop_graph_preference(
    request: DesktopGraphPreferenceLoadRequestDto,
    runtime: tauri::State<'_, DesktopGraphPreferenceRuntime>,
) -> DesktopGraphPreferenceLoadResponse {
    runtime.load(request)
}

#[tauri::command]
fn save_desktop_graph_preference(
    request: DesktopGraphPreferenceSaveRequestDto,
    runtime: tauri::State<'_, DesktopGraphPreferenceRuntime>,
) -> DesktopGraphPreferenceSaveResponse {
    runtime.save(request)
}

#[tauri::command]
fn get_desktop_document_assets(
    request: DesktopLocalCommandRequestDto,
    runtime: tauri::State<'_, DesktopDocumentAssetsRuntime>,
) -> DesktopDocumentAssetsCommandResponse {
    runtime.execute(request)
}

#[tauri::command]
fn get_desktop_asset_detail(
    request: DesktopAssetDetailRequestDto,
    runtime: tauri::State<'_, DesktopDocumentAssetsRuntime>,
) -> DesktopAssetDetailResponse {
    runtime.detail(request)
}

#[tauri::command]
fn get_desktop_asset_preview(
    request: DesktopAssetDetailRequestDto,
    runtime: tauri::State<'_, DesktopDocumentAssetsRuntime>,
) -> DesktopAssetPreviewResponse {
    runtime.preview(request)
}

#[tauri::command]
fn open_desktop_asset_externally(
    request: DesktopAssetDetailRequestDto,
    runtime: tauri::State<'_, DesktopDocumentAssetsRuntime>,
) -> DesktopAssetExternalOpenResponse {
    runtime.open_external(request)
}

#[tauri::command]
fn get_desktop_workspace_assets(
    request: DesktopWorkspaceAssetsRequestDto,
    runtime: tauri::State<'_, DesktopDocumentAssetsRuntime>,
) -> DesktopWorkspaceAssetsResponse {
    runtime.list_workspace(request)
}

#[tauri::command]
fn link_desktop_asset(
    request: DesktopDocumentAttachmentMutationRequestDto,
    runtime: tauri::State<'_, DesktopDocumentAttachmentMutationRuntime>,
) -> DesktopDocumentAttachmentMutationResponse {
    runtime.execute(request)
}

#[tauri::command]
fn unlink_desktop_asset(
    request: DesktopDocumentAttachmentMutationRequestDto,
    runtime: tauri::State<'_, DesktopDocumentAttachmentMutationRuntime>,
) -> DesktopDocumentAttachmentMutationResponse {
    runtime.execute(request)
}

#[tauri::command]
async fn select_desktop_asset_import_files(
    app: tauri::AppHandle,
    mode: tauri::State<'_, PackagedUiSmokeMode>,
    fixture: tauri::State<'_, PackagedUiSmokeAssetFixture>,
) -> Result<DesktopAssetImportSelectionResponse, ()> {
    let paths = if mode.is_enabled() {
        Ok(fixture.selected_paths().unwrap_or_default())
    } else {
        app.dialog()
            .file()
            .blocking_pick_files()
            .unwrap_or_default()
            .into_iter()
            .map(|file| file.into_path())
            .collect::<Result<Vec<_>, _>>()
    };
    let response = match paths {
        Ok(paths) => app
            .state::<DesktopAssetImportSelectionRuntime>()
            .register_selected_paths(paths),
        Err(_) => DesktopAssetImportSelectionResponse::failure("asset_import.unsafe_source"),
    };
    Ok(response)
}

#[tauri::command]
async fn import_desktop_asset(
    request: DesktopRevisionGuardedAssetImportRequestDto,
    runtime: tauri::State<'_, DesktopAssetImportSelectionRuntime>,
) -> Result<DesktopAssetImportResponse, ()> {
    let runtime = runtime.inner().clone();
    let started = runtime.start_revision_guarded(&request);
    if let Some(operation_id) = started.operation_id.clone() {
        tauri::async_runtime::spawn(async move {
            let _ = tauri::async_runtime::spawn_blocking(move || {
                runtime.run_started_revision_guarded(request, &operation_id)
            })
            .await;
        });
    }
    Ok(started)
}

#[tauri::command]
fn get_desktop_asset_import_status(
    request: DesktopAssetImportOperationRequestDto,
    runtime: tauri::State<'_, DesktopAssetImportSelectionRuntime>,
) -> DesktopAssetImportResponse {
    runtime.status(&request.workspace_id, &request.operation_id)
}

#[tauri::command]
fn cancel_desktop_asset_import(
    request: DesktopAssetImportOperationRequestDto,
    runtime: tauri::State<'_, DesktopAssetImportSelectionRuntime>,
) -> DesktopAssetImportResponse {
    runtime.cancel(&request.workspace_id, &request.operation_id)
}

#[tauri::command]
fn run_desktop_projection_worker(
    runtime: tauri::State<'_, DesktopProjectionRuntime>,
) -> DesktopProjectionRunResponse {
    runtime.run_once()
}

#[tauri::command]
fn get_desktop_projection_freshness(
    request: DesktopProjectionIdentityRequestDto,
    runtime: tauri::State<'_, DesktopProjectionRuntime>,
) -> DesktopProjectionFreshnessResponse {
    runtime.get_freshness(&request.workspace_id, &request.document_id)
}

#[tauri::command]
fn request_desktop_projection_reindex(
    request: DesktopProjectionIdentityRequestDto,
    runtime: tauri::State<'_, DesktopProjectionRuntime>,
) -> DesktopProjectionReindexResponse {
    runtime.request_reindex(&request.workspace_id, &request.document_id)
}

#[tauri::command]
fn start_desktop_projection_repair(
    request: DesktopProjectionRepairStartRequestDto,
    runtime: tauri::State<'_, DesktopProjectionRepairOperationRuntime>,
) -> DesktopProjectionRepairOperationResponse {
    runtime.start(&request.workspace_id, &request.document_id)
}

#[tauri::command]
fn get_desktop_projection_repair_status(
    request: DesktopProjectionRepairOperationRequestDto,
    runtime: tauri::State<'_, DesktopProjectionRepairOperationRuntime>,
) -> DesktopProjectionRepairOperationResponse {
    runtime.status(&request.workspace_id, &request.operation_id)
}

#[tauri::command]
fn cancel_desktop_projection_repair(
    request: DesktopProjectionRepairOperationRequestDto,
    runtime: tauri::State<'_, DesktopProjectionRepairOperationRuntime>,
) -> DesktopProjectionRepairOperationResponse {
    runtime.cancel(&request.workspace_id, &request.operation_id)
}

#[tauri::command]
fn create_desktop_backup_package(
    request: DesktopBackupPackageRequestDto,
    runtime: tauri::State<'_, DesktopBackupRecoveryRuntime>,
) -> DesktopBackupRecoveryResponse {
    runtime.create(request)
}

#[tauri::command]
fn list_desktop_backup_catalog(
    request: DesktopBackupCatalogRequestDto,
    runtime: tauri::State<'_, DesktopBackupRecoveryRuntime>,
) -> DesktopBackupCatalogResponse {
    runtime.list_catalog(request)
}

#[tauri::command]
fn preview_desktop_backup_restore(
    request: DesktopBackupPackageRequestDto,
    runtime: tauri::State<'_, DesktopBackupRecoveryRuntime>,
) -> DesktopBackupRecoveryResponse {
    runtime.preview(request)
}

#[tauri::command]
fn confirm_desktop_backup_restore(
    request: DesktopRestoreConfirmRequestDto,
    runtime: tauri::State<'_, DesktopBackupRecoveryRuntime>,
) -> DesktopBackupRecoveryResponse {
    runtime.confirm(request)
}

#[tauri::command]
fn cancel_desktop_backup_restore(
    request: DesktopRestoreCancelRequestDto,
    runtime: tauri::State<'_, DesktopBackupRecoveryRuntime>,
) -> DesktopBackupRecoveryResponse {
    runtime.cancel(request)
}

#[tauri::command]
fn recover_desktop_backup_startup(
    request: DesktopBackupRecoveryRequestDto,
    runtime: tauri::State<'_, DesktopBackupRecoveryRuntime>,
) -> DesktopBackupRecoveryResponse {
    runtime.recover_startup(request)
}

#[tauri::command]
async fn start_desktop_backup_operation(
    request: DesktopBackupOperationRequestDto,
    runtime: tauri::State<'_, DesktopBackupRecoveryRuntime>,
) -> Result<DesktopBackupOperationResponse, ()> {
    let runtime = runtime.inner().clone();
    let started = runtime.start_operation(request.clone());
    if started.ok {
        tauri::async_runtime::spawn(async move {
            let _ =
                tauri::async_runtime::spawn_blocking(move || runtime.run_operation(request)).await;
        });
    }
    Ok(started)
}

#[tauri::command]
fn get_desktop_backup_operation_status(
    request: DesktopBackupOperationRequestDto,
    runtime: tauri::State<'_, DesktopBackupRecoveryRuntime>,
) -> DesktopBackupOperationResponse {
    runtime.operation_status(request)
}

#[tauri::command]
fn cancel_desktop_backup_operation(
    request: DesktopBackupOperationRequestDto,
    runtime: tauri::State<'_, DesktopBackupRecoveryRuntime>,
) -> DesktopBackupOperationResponse {
    runtime.cancel_operation(request)
}

#[tauri::command]
async fn start_desktop_restore_operation(
    request: DesktopRestoreConfirmRequestDto,
    runtime: tauri::State<'_, DesktopBackupRecoveryRuntime>,
) -> Result<DesktopBackupRecoveryResponse, ()> {
    let runtime = runtime.inner().clone();
    let started = runtime.start_restore_operation(request.clone());
    if started.ok {
        tauri::async_runtime::spawn(async move {
            let _ = tauri::async_runtime::spawn_blocking(move || {
                runtime.run_restore_operation(request)
            })
            .await;
        });
    }
    Ok(started)
}

#[tauri::command]
fn get_desktop_restore_operation_status(
    request: DesktopRestoreCancelRequestDto,
    runtime: tauri::State<'_, DesktopBackupRecoveryRuntime>,
) -> DesktopBackupRecoveryResponse {
    runtime.restore_operation_status(request)
}

#[tauri::command]
fn retry_desktop_projection_repair(
    request: DesktopProjectionRepairOperationRequestDto,
    runtime: tauri::State<'_, DesktopProjectionRepairOperationRuntime>,
) -> DesktopProjectionRepairOperationResponse {
    runtime.retry(&request.workspace_id, &request.operation_id)
}

#[tauri::command]
fn run_desktop_projection_repair_operation(
    request: DesktopProjectionRepairOperationRequestDto,
    runtime: tauri::State<'_, DesktopProjectionRepairOperationRuntime>,
    projection: tauri::State<'_, DesktopProjectionRuntime>,
) -> DesktopProjectionRepairOperationResponse {
    runtime.run(&request.workspace_id, &request.operation_id, &projection)
}

#[tauri::command]
fn get_packaged_ui_smoke_mode(
    mode: tauri::State<'_, PackagedUiSmokeMode>,
) -> PackagedUiSmokeModeResponse {
    mode.public_response()
}

#[tauri::command]
fn corrupt_packaged_ui_smoke_canvas(
    mode: tauri::State<'_, PackagedUiSmokeMode>,
    fixture: tauri::State<'_, PackagedUiSmokeCanvasFixture>,
) -> Result<(), String> {
    if !mode.is_enabled() {
        return Err("PACKAGED_UI_FIXTURE_DISABLED".to_owned());
    }
    fixture.corrupt_current_pointer().map_err(str::to_owned)
}

#[tauri::command]
fn configure_packaged_ui_smoke_viewport(
    window: tauri::WebviewWindow,
    mode: tauri::State<'_, PackagedUiSmokeMode>,
    request: PackagedUiSmokeViewportRequest,
) -> Result<(), String> {
    if !mode.is_enabled() {
        return Err("PACKAGED_UI_FIXTURE_DISABLED".to_owned());
    }
    let supported = matches!(
        (request.width, request.height, request.zoom_percent),
        (1440, 900, 100) | (1180, 800, 100) | (960, 720, 100) | (760, 640, 100) | (760, 640, 200)
    );
    if !supported {
        return Err("PACKAGED_UI_VISUAL_VIEWPORT_UNSUPPORTED".to_owned());
    }
    window
        .set_size(LogicalSize::new(request.width, request.height))
        .map_err(|_| "PACKAGED_UI_VISUAL_VIEWPORT_FAILED".to_owned())
}

#[tauri::command]
async fn capture_packaged_ui_smoke_window(
    window: tauri::WebviewWindow,
    mode: tauri::State<'_, PackagedUiSmokeMode>,
    fixture: tauri::State<'_, PackagedUiSmokeVisualFixture>,
    request: PackagedUiSmokeCaptureRequest,
) -> Result<PackagedUiSmokeCaptureResponse, String> {
    if !mode.is_enabled() {
        return Err("PACKAGED_UI_FIXTURE_DISABLED".to_owned());
    }
    if request.artifact_key.is_empty()
        || request.artifact_key.len() > 80
        || !request
            .artifact_key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err("PACKAGED_UI_VISUAL_ARTIFACT_KEY_INVALID".to_owned());
    }
    let root = fixture
        .root
        .as_ref()
        .ok_or_else(|| "PACKAGED_UI_FIXTURE_DISABLED".to_owned())?
        .join("smoke-visual");
    fs::create_dir_all(&root)
        .map_err(|_| "PACKAGED_UI_VISUAL_ARTIFACT_DIRECTORY_FAILED".to_owned())?;
    let path = root.join(format!("{}.png", request.artifact_key));

    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWindow;
        let window_number = unsafe {
            let pointer = window
                .ns_window()
                .map_err(|_| "PACKAGED_UI_VISUAL_WINDOW_HANDLE_FAILED".to_owned())?;
            (&*pointer.cast::<NSWindow>()).windowNumber()
        };
        let status = bounded_child_process::run_with_timeout(
            Command::new("/usr/sbin/screencapture")
                .args(["-x", "-l", &window_number.to_string()])
                .arg(&path),
            Duration::from_secs(5),
        )
        .map_err(|error| match error {
            "CHILD_PROCESS_TIMEOUT" => "PACKAGED_UI_VISUAL_CAPTURE_TIMEOUT".to_owned(),
            _ => "PACKAGED_UI_VISUAL_CAPTURE_FAILED".to_owned(),
        })?;
        if !status.success() {
            return Err("PACKAGED_UI_VISUAL_CAPTURE_FAILED".to_owned());
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        return Err("PACKAGED_UI_VISUAL_PLATFORM_UNSUPPORTED".to_owned());
    }

    let bytes = fs::read(&path).map_err(|_| "PACKAGED_UI_VISUAL_CAPTURE_READ_FAILED".to_owned())?;
    if bytes.is_empty() {
        return Err("PACKAGED_UI_VISUAL_CAPTURE_EMPTY".to_owned());
    }
    let (sampled_pixel_count, non_background_pixel_count) = sample_visual_capture(
        &bytes,
        request.sample_bounds.as_ref(),
        request.viewport_width,
        request.viewport_height,
    )?;
    Ok(PackagedUiSmokeCaptureResponse {
        digest: format!("{:x}", Sha256::digest(&bytes)),
        byte_count: bytes.len() as u64,
        sampled_pixel_count,
        non_background_pixel_count,
    })
}

fn sample_visual_capture(
    bytes: &[u8],
    bounds: Option<&PackagedUiSmokeVisualRect>,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<(u64, u64), String> {
    let Some(bounds) = bounds else {
        return Ok((0, 0));
    };
    if viewport_width == 0
        || viewport_height == 0
        || ![bounds.x, bounds.y, bounds.width, bounds.height]
            .into_iter()
            .all(f64::is_finite)
        || bounds.width <= 0.0
        || bounds.height <= 0.0
    {
        return Err("PACKAGED_UI_VISUAL_SAMPLE_BOUNDS_INVALID".to_owned());
    }
    let image = image::load_from_memory(bytes)
        .map_err(|_| "PACKAGED_UI_VISUAL_CAPTURE_DECODE_FAILED".to_owned())?
        .to_rgba8();
    let scale = image.width() as f64 / viewport_width as f64;
    let webview_height = viewport_height as f64 * scale;
    let webview_offset_y = (image.height() as f64 - webview_height).max(0.0);
    let left = (bounds.x * scale).floor().clamp(0.0, image.width() as f64) as u32;
    let top = (webview_offset_y + bounds.y * scale)
        .floor()
        .clamp(0.0, image.height() as f64) as u32;
    let right = ((bounds.x + bounds.width) * scale)
        .ceil()
        .clamp(0.0, image.width() as f64) as u32;
    let bottom = (webview_offset_y + (bounds.y + bounds.height) * scale)
        .ceil()
        .clamp(0.0, image.height() as f64) as u32;
    if right <= left || bottom <= top {
        return Err("PACKAGED_UI_VISUAL_SAMPLE_BOUNDS_INVALID".to_owned());
    }
    let mut colors = std::collections::HashMap::<[u8; 4], u64>::new();
    let mut sampled = 0_u64;
    for y in (top..bottom).step_by(8) {
        for x in (left..right).step_by(8) {
            let pixel = image.get_pixel(x, y).0;
            *colors.entry(pixel).or_default() += 1;
            sampled += 1;
        }
    }
    let background = colors.values().copied().max().unwrap_or(0);
    Ok((sampled, sampled.saturating_sub(background)))
}

#[tauri::command]
fn report_packaged_ui_smoke_visual_evidence(
    mode: tauri::State<'_, PackagedUiSmokeMode>,
    fixture: tauri::State<'_, PackagedUiSmokeVisualFixture>,
    summary: PackagedUiSmokeVisualSummary,
) -> Result<(), String> {
    if !mode.is_enabled() {
        return Err("PACKAGED_UI_FIXTURE_DISABLED".to_owned());
    }
    validate_packaged_ui_smoke_visual_summary(&summary)?;
    *fixture
        .summary
        .lock()
        .map_err(|_| "PACKAGED_UI_VISUAL_EVIDENCE_STATE_FAILED".to_owned())? = Some(summary);
    Ok(())
}

fn validate_packaged_ui_smoke_visual_summary(
    summary: &PackagedUiSmokeVisualSummary,
) -> Result<(), String> {
    if summary.route_viewport_count != 30
        || summary.renderer_viewport_count != 10
        || summary.artifact_count != 40
        || summary.screenshot_count != 30
        || summary.screenshot_aggregate_digest.len() != 64
        || !summary
            .screenshot_aggregate_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || summary.accessibility_route_focus_count != 6
        || summary.accessibility_keyboard_journey_count != 6
        || summary.accessibility_focus_restoration_count < 6
        || summary.accessibility_visible_control_count == 0
        || summary.accessibility_named_control_count != summary.accessibility_visible_control_count
        || summary.accessibility_text_zoom_percent != 200
        || summary.accessibility_keyboard_error_count != 0
        || summary.accessibility_focus_error_count != 0
        || summary.accessibility_internal_exposure_count != 0
    {
        return Err("PACKAGED_UI_VISUAL_EVIDENCE_INVALID".to_owned());
    }
    Ok(())
}

#[tauri::command]
fn report_packaged_ui_smoke_visual_failure(
    mode: tauri::State<'_, PackagedUiSmokeMode>,
    fixture: tauri::State<'_, PackagedUiSmokeVisualFixture>,
    error_code: String,
    failure_stage: String,
) -> Result<(), String> {
    if !mode.is_enabled() {
        return Err("PACKAGED_UI_FIXTURE_DISABLED".to_owned());
    }
    if !(error_code.starts_with("PACKAGED_VISUAL_") || error_code.starts_with("VISUAL_"))
        || error_code.len() > 80
        || !error_code
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err("PACKAGED_UI_VISUAL_FAILURE_CODE_INVALID".to_owned());
    }
    if failure_stage.is_empty()
        || failure_stage.len() > 160
        || !failure_stage.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'x' | b'@' | b'-' | b'_')
        })
    {
        return Err("PACKAGED_UI_VISUAL_FAILURE_STAGE_INVALID".to_owned());
    }
    *fixture
        .failure_code
        .lock()
        .map_err(|_| "PACKAGED_UI_VISUAL_EVIDENCE_STATE_FAILED".to_owned())? = Some(error_code);
    *fixture
        .failure_stage
        .lock()
        .map_err(|_| "PACKAGED_UI_VISUAL_EVIDENCE_STATE_FAILED".to_owned())? = Some(failure_stage);
    Ok(())
}

#[tauri::command]
fn report_packaged_ui_smoke_progress(
    mode: tauri::State<'_, PackagedUiSmokeMode>,
    fixture: tauri::State<'_, PackagedUiSmokeVisualFixture>,
    stage: String,
) -> Result<(), String> {
    if !mode.is_enabled() {
        return Err("PACKAGED_UI_FIXTURE_DISABLED".to_owned());
    }
    if stage.is_empty()
        || stage.len() > 64
        || !stage
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("PACKAGED_UI_PROGRESS_STAGE_INVALID".to_owned());
    }
    let root = fixture
        .root
        .as_ref()
        .ok_or_else(|| "PACKAGED_UI_FIXTURE_DISABLED".to_owned())?;
    fs::write(root.join("smoke-progress.stage"), stage)
        .map_err(|_| "PACKAGED_UI_PROGRESS_WRITE_FAILED".to_owned())
}

#[tauri::command]
fn complete_packaged_ui_visual_smoke(
    app: tauri::AppHandle,
    mode: tauri::State<'_, PackagedUiSmokeMode>,
    fixture: tauri::State<'_, PackagedUiSmokeVisualFixture>,
) {
    if mode.stage() != Some(PackagedUiSmokeStage::VisualEvidence) {
        println!("phase016_packaged_ui_visual_smoke=failed");
        println!("error_code=PHASE016_PACKAGED_UI_STAGE_MISMATCH");
        let _ = std::io::stdout().flush();
        app.exit(1);
        std::process::exit(1);
    }
    let summary = fixture.summary.lock().ok().and_then(|value| value.clone());
    let failure_code = fixture
        .failure_code
        .lock()
        .ok()
        .and_then(|value| value.clone());
    let failure_stage = fixture
        .failure_stage
        .lock()
        .ok()
        .and_then(|value| value.clone());
    let passed = summary.is_some() && failure_code.is_none();
    println!(
        "phase016_packaged_ui_visual_smoke={}",
        if passed { "passed" } else { "failed" }
    );
    if let Some(summary) = summary {
        println!(
            "visual_route_viewport_count={}",
            summary.route_viewport_count
        );
        println!(
            "visual_renderer_viewport_count={}",
            summary.renderer_viewport_count
        );
        println!("visual_artifact_count={}", summary.artifact_count);
        println!("visual_screenshot_count={}", summary.screenshot_count);
        println!(
            "visual_screenshot_aggregate_digest={}",
            summary.screenshot_aggregate_digest
        );
        print_packaged_accessibility_summary(&summary);
    }
    if let Some(code) = failure_code {
        println!("visual_failure_code={code}");
    }
    if let Some(stage) = failure_stage {
        println!("visual_failure_stage={stage}");
    }
    if !passed {
        println!("error_code=PHASE016_PACKAGED_UI_VISUAL_EVIDENCE_FAILED");
    }
    let _ = std::io::stdout().flush();
    let exit_code = if passed { 0 } else { 1 };
    app.exit(exit_code);
    std::process::exit(exit_code);
}

fn print_packaged_accessibility_summary(summary: &PackagedUiSmokeVisualSummary) {
    println!(
        "accessibility_route_focus_count={}",
        summary.accessibility_route_focus_count
    );
    println!(
        "accessibility_keyboard_journey_count={}",
        summary.accessibility_keyboard_journey_count
    );
    println!(
        "accessibility_focus_restoration_count={}",
        summary.accessibility_focus_restoration_count
    );
    println!(
        "accessibility_visible_control_count={}",
        summary.accessibility_visible_control_count
    );
    println!(
        "accessibility_named_control_count={}",
        summary.accessibility_named_control_count
    );
    println!(
        "accessibility_text_zoom_percent={}",
        summary.accessibility_text_zoom_percent
    );
    println!(
        "accessibility_keyboard_error_count={}",
        summary.accessibility_keyboard_error_count
    );
    println!(
        "accessibility_focus_error_count={}",
        summary.accessibility_focus_error_count
    );
    println!(
        "accessibility_internal_exposure_count={}",
        summary.accessibility_internal_exposure_count
    );
}

#[tauri::command]
fn complete_packaged_ui_smoke(
    app: tauri::AppHandle,
    mode: tauri::State<'_, PackagedUiSmokeMode>,
    visual_fixture: tauri::State<'_, PackagedUiSmokeVisualFixture>,
    document_query: tauri::State<'_, DesktopDocumentQueryRuntime>,
    report: PackagedUiSmokeReport,
) {
    if !mode.is_enabled() {
        return;
    }
    if !matches!(
        mode.stage(),
        Some(PackagedUiSmokeStage::Initial | PackagedUiSmokeStage::UpgradeVerification)
    ) {
        println!("phase015_packaged_ui_smoke_initial=failed");
        println!("error_code=PHASE015_PACKAGED_UI_STAGE_MISMATCH");
        let _ = std::io::stdout().flush();
        app.exit(1);
        std::process::exit(1);
    }
    let visual_summary = visual_fixture
        .summary
        .lock()
        .ok()
        .and_then(|summary| summary.clone());
    let validation = validate_packaged_ui_smoke_initial_report(report).and_then(|()| {
        visual_summary
            .as_ref()
            .map(|_| ())
            .ok_or(cabinet_desktop_shell::PackagedUiSmokeErrorCode::VisualEvidenceMissing)
    });
    let upgrade_readback_verified = mode.stage() != Some(PackagedUiSmokeStage::UpgradeVerification)
        || upgraded_profile_readback_verified(&document_query);
    let passed = validation.is_ok() && upgrade_readback_verified;
    println!(
        "phase015_packaged_ui_smoke_initial={}",
        if passed { "passed" } else { "failed" }
    );
    println!("sample_count={}", report.sample_count);
    println!("p95_ms={}", report.p95_ms);
    println!("error_count={}", report.error_count);
    println!("action_count={}", report.action_count);
    println!("durable_readback_count={}", report.durable_readback_count);
    if mode.stage() == Some(PackagedUiSmokeStage::UpgradeVerification) {
        println!("upgrade_existing_document_readback_verified={upgrade_readback_verified}");
    }
    if let Some(summary) = visual_summary {
        println!(
            "visual_route_viewport_count={}",
            summary.route_viewport_count
        );
        println!(
            "visual_renderer_viewport_count={}",
            summary.renderer_viewport_count
        );
        println!("visual_artifact_count={}", summary.artifact_count);
        println!("visual_screenshot_count={}", summary.screenshot_count);
        println!(
            "visual_screenshot_aggregate_digest={}",
            summary.screenshot_aggregate_digest
        );
        print_packaged_accessibility_summary(&summary);
    }
    if let Ok(failure_code) = visual_fixture.failure_code.lock()
        && let Some(failure_code) = failure_code.as_ref()
    {
        println!("visual_failure_code={failure_code}");
    }
    if let Ok(failure_stage) = visual_fixture.failure_stage.lock()
        && let Some(failure_stage) = failure_stage.as_ref()
    {
        println!("visual_failure_stage={failure_stage}");
    }
    println!(
        "document_version_workflow_verified={}",
        report.document_version_workflow_verified
    );
    println!(
        "document_attachment_workflow_verified={}",
        report.document_attachment_workflow_verified
    );
    println!(
        "attachment_import_completed={}",
        report.attachment_import_completed
    );
    println!(
        "attachment_current_readback_verified={}",
        report.attachment_current_readback_verified
    );
    println!(
        "attachment_document_readback_verified={}",
        report.attachment_document_readback_verified
    );
    println!(
        "attachment_restart_readback_verified={}",
        report.attachment_restart_readback_verified
    );
    println!(
        "canvas_text_edit_readback_verified={}",
        report.canvas_text_edit_readback_verified
    );
    println!(
        "keyboard_document_workflow_verified={}",
        report.keyboard_document_workflow_verified
    );
    println!(
        "graph_link_fixture_saved={}",
        report.graph_link_fixture_saved
    );
    println!(
        "graph_local_edge_verified={}",
        report.graph_local_edge_verified
    );
    println!(
        "graph_global_edge_verified={}",
        report.graph_global_edge_verified
    );
    println!(
        "graph_safe_labels_verified={}",
        report.graph_safe_labels_verified
    );
    if !upgrade_readback_verified {
        println!("error_code=PHASE016_PACKAGED_UI_UPGRADE_READBACK_MISSING");
    } else if let Err(error) = validation {
        println!(
            "error_code={}",
            report
                .failure_stage
                .map_or_else(|| error.as_str(), |stage| stage.error_code())
        );
    }
    let _ = std::io::stdout().flush();
    let exit_code = if passed { 0 } else { 1 };
    app.exit(exit_code);
    std::process::exit(exit_code);
}

#[tauri::command]
fn complete_packaged_ui_smoke_restart(
    app: tauri::AppHandle,
    mode: tauri::State<'_, PackagedUiSmokeMode>,
    report: PackagedUiSmokeRestartReport,
) {
    if !mode.is_enabled() {
        return;
    }
    if mode.stage() != Some(PackagedUiSmokeStage::RestartVerification) {
        println!("phase015_packaged_ui_smoke_restart=failed");
        println!("error_code=PHASE015_PACKAGED_UI_STAGE_MISMATCH");
        let _ = std::io::stdout().flush();
        app.exit(1);
        std::process::exit(1);
    }
    let validation = validate_packaged_ui_smoke_restart_report(report);
    println!(
        "phase015_packaged_ui_smoke_restart={}",
        if validation.is_ok() {
            "passed"
        } else {
            "failed"
        }
    );
    println!(
        "attachment_restart_readback_verified={}",
        report.attachment_restart_readback_verified
    );
    println!(
        "canvas_text_restart_readback_verified={}",
        report.canvas_text_restart_readback_verified
    );
    println!("error_count={}", report.error_count);
    if let Err(error) = validation {
        println!(
            "error_code={}",
            report
                .failure_stage
                .map_or_else(|| error.as_str(), |stage| stage.error_code())
        );
    }
    let _ = std::io::stdout().flush();
    let exit_code = if validation.is_ok() { 0 } else { 1 };
    app.exit(exit_code);
    std::process::exit(exit_code);
}

fn run_tauri_app(
    packaged_ui_smoke_root: Option<PathBuf>,
    packaged_ui_smoke_stage: Option<PackagedUiSmokeStage>,
) {
    let smoke_mode = packaged_ui_smoke_stage
        .map_or_else(PackagedUiSmokeMode::disabled, PackagedUiSmokeMode::enabled);
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let app_data_dir = match packaged_ui_smoke_root.clone() {
                Some(root) => root,
                None => app.path().app_data_dir()?,
            };
            let migration_policy = DocumentBodyPolicy::new(DEFAULT_DOCUMENT_BODY_MAX_BYTES)
                .map_err(|_| std::io::Error::other("DOCUMENT_STORE_MIGRATION_INVALID_POLICY"))?;
            LocalDocumentStoreMigration::new(app_data_dir.clone(), migration_policy)
                .execute()
                .map_err(|error| std::io::Error::other(error.code()))?;
            let asset_fixture = if packaged_ui_smoke_root.is_some() {
                let fixture_path = app_data_dir
                    .join("smoke-fixtures")
                    .join("sanitized-smoke-asset.txt");
                let parent = fixture_path
                    .parent()
                    .ok_or_else(|| std::io::Error::other("invalid smoke fixture path"))?;
                fs::create_dir_all(parent)?;
                fs::write(&fixture_path, b"Sponzey Cabinet packaged asset fixture\n")?;
                PackagedUiSmokeAssetFixture::enabled(fixture_path)
            } else {
                PackagedUiSmokeAssetFixture::disabled()
            };
            app.manage(smoke_mode);
            app.manage(asset_fixture);
            app.manage(PackagedUiSmokeVisualFixture::new(
                packaged_ui_smoke_root.clone(),
            ));
            app.manage(match packaged_ui_smoke_root.as_ref() {
                Some(_) => PackagedUiSmokeCanvasFixture::enabled(app_data_dir.clone()),
                None => PackagedUiSmokeCanvasFixture::disabled(),
            });
            app.manage(DesktopWorkspaceHomeRuntime::new(app_data_dir.clone()));
            app.manage(DesktopGraphPreferenceRuntime::new(app_data_dir.clone()));
            app.manage(
                DesktopBackupRecoveryRuntime::new_with_projection_policy(
                    app_data_dir.clone(),
                    DEFAULT_BACKUP_MAX_FILE_COUNT,
                    DEFAULT_BACKUP_MAX_TOTAL_BYTES,
                    DEFAULT_DOCUMENT_BODY_MAX_BYTES,
                    DEFAULT_PROJECTION_BATCH_LIMIT,
                    DEFAULT_PROJECTION_MAX_ATTEMPTS,
                )
                .map_err(std::io::Error::other)?,
            );
            app.manage(DesktopKnowledgeGraphRuntime::new(app_data_dir.clone()));
            app.manage(DesktopGlobalKnowledgeGraphRuntime::new(
                app_data_dir.clone(),
            ));
            app.manage(
                DesktopCanvasRuntime::new(app_data_dir.clone()).map_err(std::io::Error::other)?,
            );
            app.manage(
                DesktopCanvasCatalogRuntime::new(
                    app_data_dir.clone(),
                    DEFAULT_CANVAS_CATALOG_LIMIT,
                )
                .map_err(std::io::Error::other)?,
            );
            let projection = DesktopProjectionRuntime::new(
                app_data_dir.clone(),
                DEFAULT_DOCUMENT_BODY_MAX_BYTES,
                DEFAULT_PROJECTION_BATCH_LIMIT,
                DEFAULT_PROJECTION_MAX_ATTEMPTS,
            )
            .map_err(std::io::Error::other)?;
            let reconciled = projection
                .reconcile_current("workspace-1", DEFAULT_PROJECTION_RECONCILE_DOCUMENT_LIMIT);
            if !reconciled.ok {
                return Err(std::io::Error::other(
                    reconciled
                        .error_code
                        .as_deref()
                        .unwrap_or("projection_reconcile.unknown_failure"),
                )
                .into());
            }
            if reconciled.enqueued_count > 0
                || reconciled.reset_count > 0
                || reconciled.already_active_count > 0
            {
                let processed = projection.run_once();
                if !processed.ok {
                    return Err(std::io::Error::other(
                        processed
                            .error_code
                            .as_deref()
                            .unwrap_or("PROJECTION_STARTUP_PROCESSING_FAILED"),
                    )
                    .into());
                }
            }
            app.manage(projection);
            app.manage(DesktopProjectionRepairOperationRuntime::new(
                app_data_dir.clone(),
            ));
            let external_opener: Arc<dyn AssetExternalOpener> = if packaged_ui_smoke_root.is_some()
            {
                Arc::new(LocalAssetExternalOpener::with_launcher(
                    app_data_dir.clone(),
                    PackagedSmokeExternalPathLauncher,
                ))
            } else {
                Arc::new(LocalAssetExternalOpener::new(app_data_dir.clone()))
            };
            let assets = DesktopDocumentAssetsRuntime::with_preview_limit_and_opener(
                app_data_dir.clone(),
                DEFAULT_DOCUMENT_BODY_MAX_BYTES,
                DEFAULT_ASSET_PREVIEW_MAX_BYTES,
                external_opener,
            )
            .map_err(std::io::Error::other)?;
            app.manage(assets);
            app.manage(
                DesktopDocumentAttachmentMutationRuntime::new(
                    app_data_dir.clone(),
                    DEFAULT_DOCUMENT_BODY_MAX_BYTES,
                )
                .map_err(std::io::Error::other)?,
            );
            app.manage(
                DesktopAssetImportSelectionRuntime::with_app_data_root_and_body_policy(
                    app_data_dir.clone(),
                    "workspace-1",
                    DEFAULT_ASSET_IMPORT_CHUNK_BYTES,
                    DocumentBodyPolicy::new(DEFAULT_DOCUMENT_BODY_MAX_BYTES)
                        .map_err(|_| std::io::Error::other("invalid document body policy"))?,
                )
                .map_err(std::io::Error::other)?,
            );
            let navigator = DesktopDocumentNavigatorRuntime::new(app_data_dir.clone(), 10_000)
                .map_err(std::io::Error::other)?;
            app.manage(navigator);
            app.manage(DesktopDocumentSearchRuntime::new(
                app_data_dir.clone(),
                DocumentBodyPolicy::new(DEFAULT_DOCUMENT_BODY_MAX_BYTES)
                    .map_err(|_| std::io::Error::other("invalid document body policy"))?,
            ));
            app.manage(DesktopAssetSearchRuntime::new(app_data_dir.clone()));
            let authoring = DesktopDocumentAuthoringRuntime::new(
                app_data_dir.clone(),
                DEFAULT_DOCUMENT_BODY_MAX_BYTES,
            )
            .map_err(std::io::Error::other)?;
            app.manage(authoring);
            app.manage(
                DesktopDocumentMutationRuntime::new(
                    app_data_dir.clone(),
                    DEFAULT_DOCUMENT_BODY_MAX_BYTES,
                )
                .map_err(std::io::Error::other)?,
            );
            app.manage(
                DesktopDocumentQueryRuntime::new(
                    app_data_dir.clone(),
                    DEFAULT_DOCUMENT_BODY_MAX_BYTES,
                )
                .map_err(std::io::Error::other)?,
            );
            app.manage(
                DesktopDocumentDiffRuntime::new(
                    app_data_dir.clone(),
                    DEFAULT_DOCUMENT_BODY_MAX_BYTES,
                )
                .map_err(std::io::Error::other)?,
            );
            app.manage(
                DesktopDocumentDiffOperationRuntime::new(
                    app_data_dir,
                    DEFAULT_DOCUMENT_BODY_MAX_BYTES,
                    DEFAULT_DOCUMENT_DIFF_OPERATION_CAPACITY,
                )
                .map_err(std::io::Error::other)?,
            );
            if should_focus_packaged_smoke_window(smoke_mode) {
                let window = app
                    .get_webview_window("main")
                    .ok_or_else(|| std::io::Error::other("PACKAGED_UI_SMOKE_WINDOW_MISSING"))?;
                window
                    .show()
                    .map_err(|_| std::io::Error::other("PACKAGED_UI_SMOKE_WINDOW_SHOW_FAILED"))?;
                window
                    .set_focus()
                    .map_err(|_| std::io::Error::other("PACKAGED_UI_SMOKE_WINDOW_FOCUS_FAILED"))?;
            }
            Ok(())
        })
        .on_webview_event(|webview, event| {
            let tauri::WebviewEvent::DragDrop(event) = event else {
                return;
            };
            match event {
                tauri::DragDropEvent::Enter { paths, .. } => {
                    let _ = webview.emit(
                        ASSET_DRAG_STATE_EVENT,
                        DesktopAssetDragStateEvent {
                            state: "entered",
                            file_count: paths.len(),
                        },
                    );
                }
                tauri::DragDropEvent::Leave => {
                    let _ = webview.emit(
                        ASSET_DRAG_STATE_EVENT,
                        DesktopAssetDragStateEvent {
                            state: "left",
                            file_count: 0,
                        },
                    );
                }
                tauri::DragDropEvent::Drop { paths, .. } => {
                    let response = webview
                        .state::<DesktopAssetImportSelectionRuntime>()
                        .register_selected_paths(paths.clone());
                    let _ = webview.emit(ASSET_DROP_SELECTION_EVENT, response);
                    let _ = webview.emit(
                        ASSET_DRAG_STATE_EVENT,
                        DesktopAssetDragStateEvent {
                            state: "dropped",
                            file_count: paths.len(),
                        },
                    );
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            route_desktop_shell_command,
            route_desktop_local_command,
            get_desktop_workspace_home,
            get_desktop_document_navigator,
            search_desktop_documents,
            search_desktop_assets,
            execute_desktop_document_authoring,
            execute_desktop_document_mutation,
            execute_desktop_document_query,
            execute_desktop_document_diff,
            start_desktop_document_diff_operation,
            get_desktop_document_diff_operation_status,
            cancel_desktop_document_diff_operation,
            execute_desktop_canvas,
            get_desktop_canvas_catalog,
            select_desktop_canvas,
            get_desktop_knowledge_graph,
            get_desktop_global_knowledge_graph,
            get_desktop_graph_preference,
            save_desktop_graph_preference,
            get_desktop_document_assets,
            get_desktop_workspace_assets,
            get_desktop_asset_detail,
            get_desktop_asset_preview,
            open_desktop_asset_externally,
            link_desktop_asset,
            unlink_desktop_asset,
            select_desktop_asset_import_files,
            import_desktop_asset,
            get_desktop_asset_import_status,
            cancel_desktop_asset_import,
            run_desktop_projection_worker,
            get_desktop_projection_freshness,
            request_desktop_projection_reindex,
            start_desktop_projection_repair,
            get_desktop_projection_repair_status,
            cancel_desktop_projection_repair,
            retry_desktop_projection_repair,
            run_desktop_projection_repair_operation,
            create_desktop_backup_package,
            list_desktop_backup_catalog,
            preview_desktop_backup_restore,
            confirm_desktop_backup_restore,
            cancel_desktop_backup_restore,
            recover_desktop_backup_startup,
            start_desktop_backup_operation,
            get_desktop_backup_operation_status,
            cancel_desktop_backup_operation,
            start_desktop_restore_operation,
            get_desktop_restore_operation_status,
            get_packaged_ui_smoke_mode,
            corrupt_packaged_ui_smoke_canvas,
            configure_packaged_ui_smoke_viewport,
            capture_packaged_ui_smoke_window,
            report_packaged_ui_smoke_visual_evidence,
            report_packaged_ui_smoke_visual_failure,
            report_packaged_ui_smoke_progress,
            complete_packaged_ui_visual_smoke,
            complete_packaged_ui_smoke,
            complete_packaged_ui_smoke_restart
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sponzey Cabinet desktop app");
}

const fn should_focus_packaged_smoke_window(mode: PackagedUiSmokeMode) -> bool {
    mode.is_enabled()
}

fn seed_packaged_upgrade_profile(root: &Path) -> Result<(), &'static str> {
    fs::create_dir_all(root).map_err(|_| "PHASE016_PACKAGED_UI_UPGRADE_PROFILE_UNAVAILABLE")?;
    let runtime =
        DesktopDocumentAuthoringRuntime::new(root.to_path_buf(), DEFAULT_DOCUMENT_BODY_MAX_BYTES)
            .map_err(|_| "PHASE016_PACKAGED_UI_UPGRADE_SEED_FAILED")?;
    let response = runtime.execute(DesktopDocumentAuthoringRequestDto::Create {
        workspace_id: "workspace-1".into(),
        document_id: PACKAGED_UPGRADE_DOCUMENT_ID.into(),
        path: "notes/upgrade.md".into(),
        body: PACKAGED_UPGRADE_DOCUMENT_BODY.into(),
        version_id: "packaged-upgrade-v1".into(),
        snapshot_ref: "snapshot:packaged-upgrade-v1".into(),
        author: "local-user".into(),
        summary: "Upgrade fixture".into(),
    });
    response
        .ok
        .then_some(())
        .ok_or("PHASE016_PACKAGED_UI_UPGRADE_SEED_FAILED")
}

fn upgraded_profile_readback_verified(runtime: &DesktopDocumentQueryRuntime) -> bool {
    let response = runtime.execute(DesktopDocumentQueryRequestDto::Current {
        workspace_id: "workspace-1".into(),
        document_id: PACKAGED_UPGRADE_DOCUMENT_ID.into(),
    });
    response.ok
        && response.data.is_some_and(|data| {
            data.title.as_deref() == Some(PACKAGED_UPGRADE_DOCUMENT_TITLE)
                && data.body.as_deref() == Some(PACKAGED_UPGRADE_DOCUMENT_BODY)
                && data.revision_number == Some(1)
        })
}

#[cfg(test)]
mod tests {
    use super::{
        PackagedUiSmokeMode, PackagedUiSmokeStage, PackagedUiSmokeVisualSummary,
        seed_packaged_upgrade_profile, should_focus_packaged_smoke_window,
        upgraded_profile_readback_verified, validate_packaged_ui_smoke_visual_summary,
    };
    use cabinet_adapters::local_document_store_migration::LocalDocumentStoreMigration;
    use cabinet_domain::document::DocumentBodyPolicy;
    use std::{fs, time::SystemTime};

    fn valid_visual_summary() -> PackagedUiSmokeVisualSummary {
        PackagedUiSmokeVisualSummary {
            route_viewport_count: 30,
            renderer_viewport_count: 10,
            artifact_count: 40,
            screenshot_count: 30,
            screenshot_aggregate_digest: "a".repeat(64),
            accessibility_route_focus_count: 6,
            accessibility_keyboard_journey_count: 6,
            accessibility_focus_restoration_count: 6,
            accessibility_visible_control_count: 84,
            accessibility_named_control_count: 84,
            accessibility_text_zoom_percent: 200,
            accessibility_keyboard_error_count: 0,
            accessibility_focus_error_count: 0,
            accessibility_internal_exposure_count: 0,
        }
    }

    #[test]
    fn packaged_visual_summary_accepts_complete_accessibility_evidence() {
        assert!(validate_packaged_ui_smoke_visual_summary(&valid_visual_summary()).is_ok());
    }

    #[test]
    fn packaged_visual_summary_rejects_incomplete_accessibility_evidence() {
        let invalid_summaries = [
            PackagedUiSmokeVisualSummary {
                accessibility_route_focus_count: 5,
                ..valid_visual_summary()
            },
            PackagedUiSmokeVisualSummary {
                accessibility_keyboard_journey_count: 5,
                ..valid_visual_summary()
            },
            PackagedUiSmokeVisualSummary {
                accessibility_named_control_count: 83,
                ..valid_visual_summary()
            },
            PackagedUiSmokeVisualSummary {
                accessibility_text_zoom_percent: 100,
                ..valid_visual_summary()
            },
            PackagedUiSmokeVisualSummary {
                accessibility_internal_exposure_count: 1,
                ..valid_visual_summary()
            },
        ];

        for summary in invalid_summaries {
            assert_eq!(
                validate_packaged_ui_smoke_visual_summary(&summary),
                Err("PACKAGED_UI_VISUAL_EVIDENCE_INVALID".to_owned())
            );
        }
    }

    #[test]
    fn only_packaged_smoke_mode_requests_native_window_focus() {
        assert!(!should_focus_packaged_smoke_window(
            PackagedUiSmokeMode::disabled()
        ));
        assert!(should_focus_packaged_smoke_window(
            PackagedUiSmokeMode::enabled(PackagedUiSmokeStage::Initial)
        ));
        assert!(should_focus_packaged_smoke_window(
            PackagedUiSmokeMode::enabled(PackagedUiSmokeStage::RestartVerification)
        ));
    }

    #[test]
    fn packaged_upgrade_fixture_migrates_and_preserves_current_document() {
        let root = std::env::temp_dir().join(format!(
            "sponzey-packaged-upgrade-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        seed_packaged_upgrade_profile(&root).unwrap();
        assert!(root.join("authoring-versions").is_dir());
        assert!(!root.join("document-store-migration-v1.complete").exists());

        LocalDocumentStoreMigration::new(
            root.clone(),
            DocumentBodyPolicy::new(4 * 1024 * 1024).unwrap(),
        )
        .execute()
        .unwrap();
        let query = super::DesktopDocumentQueryRuntime::new(root.clone(), 4 * 1024 * 1024).unwrap();
        assert!(upgraded_profile_readback_verified(&query));
        fs::remove_dir_all(root).unwrap();
    }
}
