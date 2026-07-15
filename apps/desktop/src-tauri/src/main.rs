use cabinet_desktop_shell::{
    DesktopAssetDetailRequestDto, DesktopAssetDetailResponse, DesktopAssetPreviewResponse,
    DesktopAssetImportOperationRequestDto, DesktopAssetImportRequestDto,
    DesktopAssetImportResponse, DesktopAssetImportSelectionResponse,
    DesktopAssetImportSelectionRuntime, DesktopAssetLinkRequestDto, DesktopAssetLinkResponse,
    DesktopAssetUnlinkRequestDto, DesktopAssetUnlinkResponse, DesktopBackupOperationRequestDto,
    DesktopBackupOperationResponse, DesktopBackupPackageRequestDto,
    DesktopBackupRecoveryRequestDto, DesktopBackupRecoveryResponse, DesktopBackupRecoveryRuntime,
    DesktopCanvasRequestDto, DesktopCanvasResponse, DesktopCanvasRuntime,
    DesktopDocumentAssetsCommandResponse, DesktopDocumentAssetsRuntime,
    DesktopDocumentAuthoringCommandResponse, DesktopDocumentAuthoringRequestDto,
    DesktopDocumentAuthoringRuntime, DesktopDocumentNavigatorCommandResponse,
    DesktopDocumentNavigatorRequestDto, DesktopDocumentNavigatorRuntime,
    DesktopGlobalKnowledgeGraphCommandResponse, DesktopGlobalKnowledgeGraphRequestDto,
    DesktopGlobalKnowledgeGraphRuntime, DesktopKnowledgeGraphCommandResponse,
    DesktopKnowledgeGraphRuntime, DesktopLocalCommandRequestDto,
    DesktopLocalCommandRuntimeResponse, DesktopProjectionFreshnessResponse,
    DesktopProjectionIdentityRequestDto, DesktopProjectionReindexResponse,
    DesktopProjectionRepairOperationRequestDto, DesktopProjectionRepairOperationResponse,
    DesktopProjectionRepairOperationRuntime, DesktopProjectionRepairStartRequestDto,
    DesktopProjectionRunResponse, DesktopProjectionRuntime, DesktopRestoreCancelRequestDto,
    DesktopRestoreConfirmRequestDto, DesktopShellRequest, DesktopWorkspaceAssetsRequestDto,
    DesktopWorkspaceAssetsResponse, DesktopWorkspaceHomeCommandResponse,
    DesktopWorkspaceHomeRuntime, PackagedUiSmokeAssetFixture, PackagedUiSmokeCanvasFixture, PackagedUiSmokeMode,
    PackagedUiSmokeModeResponse, PackagedUiSmokeReport, create_desktop_package_smoke_report,
    route_desktop_command, route_local_desktop_command_request, route_tauri_command,
    validate_packaged_ui_smoke_report,
};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

const DEFAULT_DOCUMENT_BODY_MAX_BYTES: usize = 10 * 1024 * 1024;
const DEFAULT_PROJECTION_BATCH_LIMIT: usize = 64;
const DEFAULT_PROJECTION_MAX_ATTEMPTS: u32 = 3;
const DEFAULT_ASSET_IMPORT_CHUNK_BYTES: usize = 256 * 1024;
const DEFAULT_ASSET_PREVIEW_MAX_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_BACKUP_MAX_FILE_COUNT: u64 = 100_000;
const DEFAULT_BACKUP_MAX_TOTAL_BYTES: u64 = 20 * 1024 * 1024 * 1024;

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
        run_tauri_app(Some(root));
        return;
    }

    run_tauri_app(None);
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
fn execute_desktop_document_authoring(
    request: DesktopDocumentAuthoringRequestDto,
    runtime: tauri::State<'_, DesktopDocumentAuthoringRuntime>,
) -> DesktopDocumentAuthoringCommandResponse {
    runtime.execute(request)
}

#[tauri::command]
fn execute_desktop_canvas(
    request: DesktopCanvasRequestDto,
    runtime: tauri::State<'_, DesktopCanvasRuntime>,
) -> DesktopCanvasResponse {
    runtime.execute(request)
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
fn get_desktop_workspace_assets(
    request: DesktopWorkspaceAssetsRequestDto,
    runtime: tauri::State<'_, DesktopDocumentAssetsRuntime>,
) -> DesktopWorkspaceAssetsResponse {
    runtime.list_workspace(request)
}

#[tauri::command]
fn link_desktop_asset(
    request: DesktopAssetLinkRequestDto,
    runtime: tauri::State<'_, DesktopDocumentAssetsRuntime>,
) -> DesktopAssetLinkResponse {
    runtime.link(request)
}

#[tauri::command]
fn unlink_desktop_asset(
    request: DesktopAssetUnlinkRequestDto,
    runtime: tauri::State<'_, DesktopDocumentAssetsRuntime>,
) -> DesktopAssetUnlinkResponse {
    runtime.unlink(request)
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
    request: DesktopAssetImportRequestDto,
    runtime: tauri::State<'_, DesktopAssetImportSelectionRuntime>,
) -> Result<DesktopAssetImportResponse, ()> {
    let runtime = runtime.inner().clone();
    let started = runtime.start(request.clone());
    if let Some(operation_id) = started.operation_id.clone() {
        tauri::async_runtime::spawn(async move {
            let _ = tauri::async_runtime::spawn_blocking(move || {
                runtime.run_started(request, &operation_id)
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
    fixture
        .corrupt_default_current_pointer()
        .map_err(str::to_owned)
}

#[tauri::command]
fn complete_packaged_ui_smoke(
    app: tauri::AppHandle,
    mode: tauri::State<'_, PackagedUiSmokeMode>,
    report: PackagedUiSmokeReport,
) {
    if !mode.is_enabled() {
        return;
    }
    let validation = validate_packaged_ui_smoke_report(report);
    println!(
        "phase012_packaged_ui_smoke={}",
        if validation.is_ok() {
            "passed"
        } else {
            "failed"
        }
    );
    println!("sample_count={}", report.sample_count);
    println!("p95_ms={}", report.p95_ms);
    println!("error_count={}", report.error_count);
    println!("action_count={}", report.action_count);
    println!("durable_readback_count={}", report.durable_readback_count);
    if let Err(error) = validation {
        println!(
            "error_code={}",
            report
                .failure_stage
                .map_or_else(|| error.as_str(), |stage| stage.error_code())
        );
    }
    let _ = std::io::stdout().flush();
    app.exit(if validation.is_ok() { 0 } else { 1 });
}

fn run_tauri_app(packaged_ui_smoke_root: Option<PathBuf>) {
    let smoke_mode = if packaged_ui_smoke_root.is_some() {
        PackagedUiSmokeMode::enabled()
    } else {
        PackagedUiSmokeMode::disabled()
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let app_data_dir = match packaged_ui_smoke_root.clone() {
                Some(root) => root,
                None => app.path().app_data_dir()?,
            };
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
            app.manage(match packaged_ui_smoke_root.as_ref() {
                Some(_) => PackagedUiSmokeCanvasFixture::enabled(app_data_dir.clone()),
                None => PackagedUiSmokeCanvasFixture::disabled(),
            });
            app.manage(DesktopWorkspaceHomeRuntime::new(app_data_dir.clone()));
            app.manage(
                DesktopBackupRecoveryRuntime::new(
                    app_data_dir.clone(),
                    DEFAULT_BACKUP_MAX_FILE_COUNT,
                    DEFAULT_BACKUP_MAX_TOTAL_BYTES,
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
            let projection = DesktopProjectionRuntime::new(
                app_data_dir.clone(),
                DEFAULT_DOCUMENT_BODY_MAX_BYTES,
                DEFAULT_PROJECTION_BATCH_LIMIT,
                DEFAULT_PROJECTION_MAX_ATTEMPTS,
            )
            .map_err(std::io::Error::other)?;
            app.manage(projection);
            app.manage(DesktopProjectionRepairOperationRuntime::new(
                app_data_dir.clone(),
            ));
            let assets = DesktopDocumentAssetsRuntime::with_preview_limit(
                app_data_dir.clone(),
                DEFAULT_DOCUMENT_BODY_MAX_BYTES,
                DEFAULT_ASSET_PREVIEW_MAX_BYTES,
            )
            .map_err(std::io::Error::other)?;
            app.manage(assets);
            app.manage(
                DesktopAssetImportSelectionRuntime::with_app_data_root(
                    app_data_dir.clone(),
                    "workspace-1",
                    DEFAULT_ASSET_IMPORT_CHUNK_BYTES,
                )
                .map_err(std::io::Error::other)?,
            );
            let navigator = DesktopDocumentNavigatorRuntime::new(app_data_dir.clone(), 10_000)
                .map_err(std::io::Error::other)?;
            app.manage(navigator);
            let authoring =
                DesktopDocumentAuthoringRuntime::new(app_data_dir, DEFAULT_DOCUMENT_BODY_MAX_BYTES)
                    .map_err(std::io::Error::other)?;
            app.manage(authoring);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            route_desktop_shell_command,
            route_desktop_local_command,
            get_desktop_workspace_home,
            get_desktop_document_navigator,
            execute_desktop_document_authoring,
            execute_desktop_canvas,
            get_desktop_knowledge_graph,
            get_desktop_global_knowledge_graph,
            get_desktop_document_assets,
            get_desktop_workspace_assets,
            get_desktop_asset_detail,
            get_desktop_asset_preview,
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
            complete_packaged_ui_smoke
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sponzey Cabinet desktop app");
}
