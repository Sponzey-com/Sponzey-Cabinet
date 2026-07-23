//! Desktop shell boundary.
//!
//! This crate is intentionally thin. Future Tauri commands should map request
//! DTOs into platform boundary calls without embedding business rules.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use cabinet_adapters::composite_graph_projection::CompositeGraphProjectionStore;
use cabinet_adapters::durable_asset_association_catalog::DurableAssetAssociationCatalog;
use cabinet_adapters::durable_asset_import_operation_repository::DurableAssetImportOperationRepository;
use cabinet_adapters::durable_asset_metadata_catalog::DurableAssetMetadataCatalog;
use cabinet_adapters::durable_backup_package_store::{
    LocalBackupPackagePolicy, LocalBackupPackageStore,
};
use cabinet_adapters::durable_canvas_graph_projection::DurableCanvasGraphRelationProjectionStore;
use cabinet_adapters::durable_canvas_repository::DurableCanvasRepository;
use cabinet_adapters::durable_document_link_catalog::DurableDocumentLinkCatalog;
use cabinet_adapters::durable_last_canvas_selection::DurableLastCanvasSelection;
use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_adapters::durable_local_link_index::DurableLocalLinkIndex;
use cabinet_adapters::durable_local_search_index::DurableLocalSearchIndex;
use cabinet_adapters::durable_projection_repair_repository::DurableProjectionRepairRepository;
use cabinet_adapters::durable_projection_work_repository::DurableProjectionWorkRepository;
use cabinet_adapters::local_asset_availability_resolver::LocalAssetAvailabilityResolver;
use cabinet_adapters::local_asset_external_opener::LocalAssetExternalOpener;
use cabinet_adapters::local_asset_import_source::{
    LocalAssetImportSource, LocalAssetImportSourceConfig,
};
use cabinet_adapters::local_asset_preview_reader::LocalAssetPreviewReader;
use cabinet_adapters::local_asset_search_index::LocalAssetSearchIndex;
use cabinet_adapters::local_asset_staging_writer::LocalAssetStagingWriter;
use cabinet_adapters::local_backup_restore_store::LocalBackupRestoreStore;
use cabinet_adapters::local_backup_store::LocalBackupStore;
use cabinet_adapters::local_content_addressed_asset_publisher::LocalContentAddressedAssetPublisher;
use cabinet_adapters::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT, LocalCreateDocumentRevisionRuntime,
};
use cabinet_adapters::local_current_document_projection_catalog::LocalCurrentDocumentProjectionCatalog;
use cabinet_adapters::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use cabinet_adapters::local_document_navigator_projection::LocalDocumentNavigatorProjectionStore;
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_adapters::local_imported_asset_document_revision_linker::LocalImportedAssetDocumentRevisionLinker;
use cabinet_adapters::local_markdown_parser::LocalMarkdownParser;
use cabinet_adapters::local_mutate_document_attachments_runtime::LocalMutateDocumentAttachmentsRuntime;
use cabinet_adapters::local_restore_document_revision_runtime::LocalRestoreDocumentRevisionRuntime;
use cabinet_adapters::local_restore_projection_recovery_runtime::LocalRestoreProjectionRecoveryRuntime;
use cabinet_adapters::local_update_document_revision_runtime::LocalUpdateDocumentRevisionRuntime;
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_adapters::local_workspace_home_projection::LocalWorkspaceHomeProjectionStore;
use cabinet_adapters::local_workspace_home_query::LocalWorkspaceHomeQueryStore;
use cabinet_adapters::local_workspace_reopener::LocalWorkspaceReopener;
use cabinet_adapters::process_local_document_diff_operation_registry::ProcessLocalDocumentDiffOperationRegistry;
use cabinet_domain::asset::{AssetId, AssetImportHandle};
use cabinet_domain::asset_import_operation::{
    AssetImportOperation, AssetImportOperationId, AssetImportState,
};
use cabinet_domain::attachment_snapshot_mutation::AttachmentSnapshotDelta;
use cabinet_domain::backup::{BackupDataClass, BackupJobId, BackupJobState, RestoreState};
use cabinet_domain::canvas::{CanvasGeometryPolicy, CanvasNodeTarget};
use cabinet_domain::document::{DocumentBodyPolicy, DocumentId, DocumentTitle};
use cabinet_domain::document_diff_operation::DocumentDiffOperationState;
use cabinet_domain::document_revision::DocumentOperationId;
use cabinet_domain::graph::{GraphEdgeKind, GraphNode, GraphNodeKind, GraphProjectionStatus};
use cabinet_domain::projection_repair::{ProjectionRepairEvent, ProjectionRepairOperationId};
use cabinet_domain::projection_work::ProjectionChangeKind;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_platform::asset_search_command::{
    AssetSearchCommandFailure, AssetSearchCommandRequest, execute_asset_search_command,
};
use cabinet_platform::document_authoring_command::{
    DocumentAuthoringCommandExecutor, DocumentAuthoringCommandFailure,
    DocumentAuthoringCommandRequest, DocumentAuthoringCommandResult,
};
use cabinet_platform::document_navigator_command::{
    DocumentNavigatorCommandFailure, DocumentNavigatorCommandLoadState,
    DocumentNavigatorCommandRequest, DocumentNavigatorCommandResult, DocumentNavigatorCommandView,
    execute_document_navigator_command,
};
use cabinet_platform::local_desktop_runtime::{
    LocalDesktopCommandErrorCode, LocalDesktopCommandPayload, LocalDesktopRuntimeCommandRequest,
    LocalDesktopUsecaseInput, map_core_local_desktop_command_request,
    summarize_local_desktop_command_for_product_log,
};
use cabinet_platform::workspace_home_command::{
    WorkspaceHomeCommandFailure, WorkspaceHomeCommandLoadState, WorkspaceHomeCommandResult,
    execute_workspace_home_command,
};
use cabinet_ports::asset_external_open::AssetExternalOpener;
use cabinet_ports::asset_import_operation_repository::AssetImportOperationRepository;
use cabinet_ports::asset_import_source::AssetImportSource;
use cabinet_ports::asset_metadata_catalog::AssetMetadataCatalog;
use cabinet_ports::backup_restore::BackupRestoreStore;
use cabinet_ports::canvas_graph_projection::CanvasGraphRelationProjectionError;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::document_repository::DocumentRepository;
use cabinet_ports::document_title_reader::DocumentTitleReader;
use cabinet_ports::imported_asset_document_link::ImportedAssetDocumentLinkPort;
use cabinet_ports::projection_repair::ProjectionRepairRepository;
use cabinet_ports::projection_work::ProjectionWorkRepository;
use cabinet_ports::version_store::{HistoryPage, VersionStore, VersionStoreError};
use cabinet_usecases::asset_external_open::{
    AssetExternalOpenProductEvent, AssetExternalOpenProductLogger, OpenAssetExternallyError,
    OpenAssetExternallyInput, OpenAssetExternallyUsecase,
};
use cabinet_usecases::asset_import::{
    CancelAssetImportInput, CancelAssetImportUsecase, ImportAssetError, ImportAssetInput,
    ImportAssetProductEvent, ImportAssetProductLogger, ImportAssetUsecase,
    ListCatalogDocumentAssetsError, ListCatalogDocumentAssetsInput,
    ListCatalogDocumentAssetsUsecase, RecoverAssetImportsInput, RecoverAssetImportsUsecase,
};
use cabinet_usecases::asset_lifecycle::{
    AssetLifecycleError, AssetLifecycleProductEvent, AssetLifecycleProductLogger,
    GetAssetDetailInput, GetAssetDetailUsecase, LinkAssetInput, LinkAssetUsecase,
    ListWorkspaceAssetsInput, ListWorkspaceAssetsUsecase, UnlinkAssetInput, UnlinkAssetUsecase,
};
use cabinet_usecases::asset_preview::{
    AssetPreviewError, AssetPreviewResult, GetAssetPreviewInput, GetAssetPreviewUsecase,
};
use cabinet_usecases::authoritative_document_diff::{
    CompareAuthoritativeDocumentRevisionsError, CompareAuthoritativeDocumentRevisionsInput,
    CompareAuthoritativeDocumentRevisionsUsecase,
};
use cabinet_usecases::authoritative_document_query::{
    GetAuthoritativeDocumentRevisionError, GetAuthoritativeDocumentRevisionInput,
    GetAuthoritativeDocumentRevisionUsecase,
};
use cabinet_usecases::authoritative_restore_preview::{
    PreviewAuthoritativeDocumentRestoreError, PreviewAuthoritativeDocumentRestoreInput,
    PreviewAuthoritativeDocumentRestoreUsecase,
};
use cabinet_usecases::backup_catalog::{
    ListBackupCatalogError, ListBackupCatalogInput, ListBackupCatalogUsecase,
};
use cabinet_usecases::backup_package::{
    BackupPackageProductEvent, BackupPackageSummary, BackupPackageUsecaseLogger,
    CreateBackupPackageInput, CreateBackupPackageUsecase, PreviewBackupRestoreInput,
    PreviewBackupRestoreUsecase,
};
use cabinet_usecases::backup_package_operation::{
    BackupPackageOperationEvent, BackupPackageOperationLogger, BackupPackageOperationOutput,
    CancelBackupPackageOperationInput, CancelBackupPackageOperationUsecase,
    GetBackupPackageOperationInput, GetBackupPackageOperationUsecase,
    RunBackupPackageOperationInput, RunBackupPackageOperationUsecase,
    StartBackupPackageOperationInput, StartBackupPackageOperationUsecase,
};
use cabinet_usecases::backup_recovery::{
    BackupRecoveryProductEvent, BackupRecoveryUsecaseLogger, RecoverBackupStartupInput,
    RecoverBackupStartupUsecase,
};
use cabinet_usecases::backup_restore::{
    BackupRestoreProductEvent, BackupRestoreUsecaseLogger, CancelBackupRestoreInput,
    CancelBackupRestoreUsecase, ConfirmBackupRestoreInput, ConfirmBackupRestoreUsecase,
    GetBackupRestoreOperationInput, GetBackupRestoreOperationUsecase,
    StartBackupRestoreOperationInput, StartBackupRestoreOperationUsecase,
};
use cabinet_usecases::canvas_catalog::{
    ResolveInitialCanvasInput, ResolveInitialCanvasUsecase, ResolvedCanvasSelectionSource,
    SelectCanvasError, SelectCanvasInput, SelectCanvasUsecase,
};
use cabinet_usecases::canvas_graph_projection::{
    CanvasGraphProjectionPolicy, ProjectCanvasGraphRelationsInput,
    ProjectCanvasGraphRelationsUsecase,
};
use cabinet_usecases::canvas_lifecycle::{
    ArchiveCanvasInput, ArchiveCanvasUsecase, CanvasLifecycleProductEvent,
    CanvasLifecycleProductLogger, CanvasLifecycleUsecaseError, CreateCanvasRecordInput,
    CreateCanvasRecordUsecase, GetCanvasRecordInput, GetCanvasRecordUsecase, RenameCanvasInput,
    RenameCanvasUsecase,
};
use cabinet_usecases::canvas_mutation::{
    AddCanvasNodeMutationInput, AddCanvasNodeMutationUsecase, AddValidatedCanvasNodeUsecase,
    AutoArrangeCanvasInput, AutoArrangeCanvasUsecase, CanvasAutoArrangePolicy, CanvasMutationError,
    CanvasMutationPolicy, CanvasMutationProductEvent, CanvasMutationProductLogger,
    CanvasNodeTargetInput, ConnectCanvasEdgeInput, ConnectCanvasEdgeUsecase,
    PreviewAutoArrangeCanvasUsecase, RemoveCanvasEdgeInput, RemoveCanvasEdgeUsecase,
    RemoveCanvasNodeInput, RemoveCanvasNodeUsecase, UpdateCanvasNodeGeometryInput,
    UpdateCanvasNodeGeometryUsecase, UpdateCanvasTextCardInput, UpdateCanvasTextCardUsecase,
    UpdateCanvasViewportInput, UpdateCanvasViewportUsecase,
};
use cabinet_usecases::canvas_recovery::{
    CanvasRecoveryError, CanvasRecoveryEvent, CanvasRecoveryLogger, RecoverCanvasInput,
    RecoverCanvasUsecase,
};
use cabinet_usecases::canvas_target_presentation::{
    CanvasTargetPresentation, ResolveCanvasTargetPresentationsError,
    ResolveCanvasTargetPresentationsInput, ResolveCanvasTargetPresentationsUsecase,
};
use cabinet_usecases::canvas_viewport::{
    GetCanvasViewportError, GetCanvasViewportInput, GetCanvasViewportUsecase,
};
use cabinet_usecases::create_document_revision::{
    CreateDocumentRevisionError, CreateDocumentRevisionInput,
};
use cabinet_usecases::document::{
    CreateDocumentProductEvent, DocumentChangeEvent, DocumentChangeEventPublisher,
    DocumentProductLogger, GetDocumentHistoryInput, GetDocumentHistoryUsecase,
    GetDocumentVersionInput, GetDocumentVersionUsecase, LineDiff, LineDiffKind,
    RenameDocumentInput, RenameDocumentUsecase,
};
use cabinet_usecases::document_diff::{
    DiffComputation as AuthoritativeDiffComputation,
    DiffLimitReason as AuthoritativeDiffLimitReason, DiffPolicy as AuthoritativeDiffPolicy,
    DocumentTitleDelta as AuthoritativeDocumentTitleDelta,
    LineDiffKind as AuthoritativeLineDiffKind,
};
use cabinet_usecases::document_diff_operation::{
    CancelDocumentDiffOperationError, CancelDocumentDiffOperationInput,
    CancelDocumentDiffOperationUsecase, DocumentDiffOperationIdGenerator,
    GetDocumentDiffOperationStatusError, GetDocumentDiffOperationStatusInput,
    GetDocumentDiffOperationStatusUsecase, RunDocumentDiffOperationInput,
    RunDocumentDiffOperationUsecase, StartDocumentDiffOperationError,
    StartDocumentDiffOperationInput, StartDocumentDiffOperationUsecase,
};
use cabinet_usecases::document_link_catalog_projection::ApplyDocumentLinkCatalogChangeUsecase;
use cabinet_usecases::global_graph::{
    GetGlobalKnowledgeGraphError, GetGlobalKnowledgeGraphInput, GetGlobalKnowledgeGraphUsecase,
};
use cabinet_usecases::graph::{
    GetLocalKnowledgeGraphError, GetLocalKnowledgeGraphInput, GetLocalKnowledgeGraphUsecase,
    LocalGraphDirection,
};
use cabinet_usecases::mutate_document_attachments::{
    MutateDocumentAttachmentsError, MutateDocumentAttachmentsInput,
    MutateDocumentAttachmentsOutcomeKind,
};
use cabinet_usecases::projection_freshness::{
    GetCurrentProjectionFreshnessInput, GetCurrentProjectionFreshnessUsecase,
    ProjectionFreshnessState,
};
use cabinet_usecases::projection_kind_writer_router::ProjectionKindWriterRouter;
use cabinet_usecases::projection_repair_operation::{
    CancelProjectionRepairInput, CancelProjectionRepairUsecase, GetProjectionRepairStatusInput,
    GetProjectionRepairStatusUsecase, ProjectionRepairOperationIdGenerator,
    ProjectionRepairUsecaseError, RetryProjectionRepairInput, RetryProjectionRepairUsecase,
    StartProjectionRepairInput, StartProjectionRepairUsecase,
};
use cabinet_usecases::projection_work::EnqueueProjectionWorkUsecase;
use cabinet_usecases::projection_worker::{
    ProjectionWorkerError, ProjectionWorkerPolicy, RunProjectionWorkerUsecase,
};
use cabinet_usecases::reconcile_current_projections::{
    ReconcileCurrentProjectionsInput, ReconcileCurrentProjectionsUsecase,
};
use cabinet_usecases::reference_projection_fanout::ReindexReferenceDependentsUsecase;
use cabinet_usecases::reindex_asset_graph_projection::{
    ReindexAssetGraphProjectionError, ReindexAssetGraphProjectionInput,
    ReindexAssetGraphProjectionUsecase,
};
use cabinet_usecases::reindex_projection::{
    ReindexCurrentProjectionInput, ReindexCurrentProjectionUsecase,
};
use cabinet_usecases::resolve_attachment_diff_availability::{
    ResolveAttachmentDiffAvailabilityError, ResolveAttachmentDiffAvailabilityInput,
    ResolveAttachmentDiffAvailabilityUsecase, ResolvedAttachmentDiff,
};
use cabinet_usecases::resolved_link_graph_writer::{
    AssetGraphProjectionPolicy, ResolvedLinkGraphProjectionWriter,
};
use cabinet_usecases::restore_document_revision::{
    RestoreDocumentRevisionError, RestoreDocumentRevisionInput,
};
use cabinet_usecases::restore_product_log::{RestoreProductEvent, RestoreProductLogger};
use cabinet_usecases::restore_projection_rebuild::{
    RebuildRestoreProjectionsInput, RebuildRestoreProjectionsUsecase,
};
use cabinet_usecases::restore_target_asset_preflight::{
    RestoreTargetAssetPreflightError, RestoreTargetAssetPreflightInput,
    RestoreTargetAssetPreflightOutcome, RestoreTargetAssetPreflightUsecase,
};
use cabinet_usecases::search::{
    SearchDocumentsError, SearchDocumentsInput, SearchDocumentsUsecase,
};
use cabinet_usecases::search_projection_writer::SearchProjectionWriter;
use cabinet_usecases::update_document_revision::{
    UpdateDocumentRevisionError, UpdateDocumentRevisionInput,
};
use cabinet_usecases::versioned_projection_processor::VersionedMarkdownProjectionProcessor;
use cabinet_usecases::workspace_home_update::UpdateWorkspaceHomeProjectionUsecase;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackagedUiSmokeModeResponse {
    pub enabled: bool,
    pub stage: Option<PackagedUiSmokeStage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PackagedUiSmokeStage {
    Initial,
    UpgradeVerification,
    RestartVerification,
    VisualEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackagedUiSmokeMode {
    stage: Option<PackagedUiSmokeStage>,
}

impl PackagedUiSmokeMode {
    pub const fn disabled() -> Self {
        Self { stage: None }
    }

    pub const fn enabled(stage: PackagedUiSmokeStage) -> Self {
        Self { stage: Some(stage) }
    }

    pub const fn is_enabled(self) -> bool {
        self.stage.is_some()
    }

    pub const fn stage(self) -> Option<PackagedUiSmokeStage> {
        self.stage
    }

    pub const fn public_response(self) -> PackagedUiSmokeModeResponse {
        PackagedUiSmokeModeResponse {
            enabled: self.is_enabled(),
            stage: self.stage,
        }
    }
}

impl Default for PackagedUiSmokeMode {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Debug, Clone)]
pub struct PackagedUiSmokeAssetFixture {
    selected_path: Option<PathBuf>,
}

impl PackagedUiSmokeAssetFixture {
    pub fn disabled() -> Self {
        Self {
            selected_path: None,
        }
    }

    pub fn enabled(selected_path: PathBuf) -> Self {
        Self {
            selected_path: Some(selected_path),
        }
    }

    pub fn selected_paths(&self) -> Option<Vec<PathBuf>> {
        self.selected_path.clone().map(|path| vec![path])
    }
}

#[derive(Debug, Clone)]
pub struct PackagedUiSmokeCanvasFixture {
    root: Option<PathBuf>,
}

impl PackagedUiSmokeCanvasFixture {
    pub fn disabled() -> Self {
        Self { root: None }
    }

    pub fn enabled(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }

    pub fn corrupt_current_pointer(&self) -> Result<(), &'static str> {
        let root = self.root.as_ref().ok_or("PACKAGED_UI_FIXTURE_DISABLED")?;
        let workspace = root
            .join("canvases")
            .join(hex_fixture_identity("workspace-1"));
        let mut canvases = fs::read_dir(workspace)
            .map_err(|_| "PACKAGED_UI_CANVAS_FIXTURE_MISSING")?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        canvases.sort();
        if canvases.len() != 1 {
            return Err("PACKAGED_UI_CANVAS_FIXTURE_AMBIGUOUS");
        }
        let path = canvases[0].join("current.canvas");
        if !path.is_file() {
            return Err("PACKAGED_UI_CANVAS_FIXTURE_MISSING");
        }
        fs::write(path, b"corrupt packaged smoke pointer\n")
            .map_err(|_| "PACKAGED_UI_CANVAS_FIXTURE_WRITE_FAILED")
    }
}

fn hex_fixture_identity(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackagedUiSmokeReport {
    pub home_ready: bool,
    pub graph_ready: bool,
    pub graph_link_fixture_saved: bool,
    pub graph_local_edge_verified: bool,
    pub graph_global_edge_verified: bool,
    pub graph_safe_labels_verified: bool,
    pub canvas_ready: bool,
    pub canvas_text_edit_readback_verified: bool,
    pub assets_ready: bool,
    pub document_version_workflow_verified: bool,
    pub document_attachment_workflow_verified: bool,
    pub attachment_import_completed: bool,
    pub attachment_current_readback_verified: bool,
    pub attachment_document_readback_verified: bool,
    pub attachment_restart_readback_verified: bool,
    pub keyboard_document_workflow_verified: bool,
    pub sample_count: u32,
    pub p95_ms: u64,
    pub error_count: u32,
    pub failure_stage: Option<PackagedUiSmokeFailureStage>,
    pub action_count: u32,
    pub durable_readback_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackagedUiSmokeRestartReport {
    pub attachment_restart_readback_verified: bool,
    pub canvas_text_restart_readback_verified: bool,
    pub error_count: u32,
    pub failure_stage: Option<PackagedUiSmokeRestartFailureStage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PackagedUiSmokeRestartFailureStage {
    Home,
    Document,
    AttachmentTab,
    AttachmentList,
    AttachmentListLoading,
    AttachmentListEmpty,
    AttachmentListFailed,
    AttachmentListMissing,
    AttachmentDetail,
    CanvasOpen,
    CanvasCatalogSelect,
    CanvasTextReadback,
}

impl PackagedUiSmokeRestartFailureStage {
    pub const fn error_code(self) -> &'static str {
        match self {
            Self::Home => "PHASE015_PACKAGED_UI_RESTART_HOME_FAILED",
            Self::Document => "PHASE015_PACKAGED_UI_RESTART_DOCUMENT_FAILED",
            Self::AttachmentTab => "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_TAB_FAILED",
            Self::AttachmentList => "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_LIST_FAILED",
            Self::AttachmentListLoading => "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_LIST_LOADING",
            Self::AttachmentListEmpty => "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_LIST_EMPTY",
            Self::AttachmentListFailed => {
                "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_LIST_QUERY_FAILED"
            }
            Self::AttachmentListMissing => "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_PANEL_MISSING",
            Self::AttachmentDetail => "PHASE015_PACKAGED_UI_RESTART_ATTACHMENT_DETAIL_FAILED",
            Self::CanvasOpen => "PHASE017_PACKAGED_UI_RESTART_CANVAS_OPEN_FAILED",
            Self::CanvasCatalogSelect => {
                "PHASE017_PACKAGED_UI_RESTART_CANVAS_CATALOG_SELECT_FAILED"
            }
            Self::CanvasTextReadback => "PHASE017_PACKAGED_UI_RESTART_CANVAS_TEXT_READBACK_FAILED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PackagedUiSmokeFailureStage {
    Home,
    DocumentCreate,
    DocumentEdit,
    DocumentSave,
    DocumentReopen,
    GraphTargetSave,
    GraphSourceSave,
    GraphProjection,
    GraphLocalEdge,
    GraphGlobalEdge,
    GraphSafeLabels,
    DocumentHistoryTab,
    DocumentHistoryLoad,
    DocumentHistoryReadback,
    DocumentDiff,
    DocumentRestorePreviewAction,
    DocumentRestorePreviewReadback,
    DocumentRestoreReview,
    DocumentRestoreCancel,
    DocumentRestoreConfirm,
    DocumentRestoreReadback,
    DocumentAttachmentTab,
    DocumentAttachmentOpen,
    DocumentAttachmentUnlinkRequest,
    DocumentAttachmentUnlinkCancel,
    GraphOpen,
    GraphScopeGlobal,
    GraphScopeLocal,
    GraphDepth,
    GraphDirection,
    GraphUnresolved,
    GraphAssets,
    GraphZoomIn,
    GraphZoomOut,
    GraphFitView,
    GraphNode,
    GraphDocumentRoute,
    GraphAttachmentOpen,
    GraphAttachmentLocalEdge,
    GraphAttachmentLocalFilter,
    GraphAttachmentLocalNode,
    GraphAttachmentLocalIdentity,
    GraphAttachmentLocalLabel,
    GraphAttachmentGlobalEdge,
    GraphAttachmentRoute,
    Canvas,
    CanvasOpen,
    CanvasCreate,
    CanvasNote,
    CanvasTextEdit,
    CanvasPan,
    CanvasZoom,
    CanvasArrange,
    CanvasDocument,
    CanvasEdge,
    CanvasDrag,
    CanvasResize,
    CanvasReopen,
    CanvasRename,
    CanvasArchive,
    CanvasArchiveReopen,
    CanvasRecovery,
    CanvasRecoveryOpen,
    CanvasRecoveryDetect,
    CanvasRecoveryApply,
    Assets,
    AssetOpen,
    AssetImport,
    AssetImportReadback,
    AssetImportOperation,
    AssetImportScope,
    AssetImportPresentation,
    AssetDetail,
    AssetPreview,
    AssetUnlink,
    AssetLibrary,
    AssetDetachedDetail,
    AssetRelink,
    AssetFilters,
    AssetFilterAll,
    AssetFilterImage,
    AssetFilterPdf,
    AssetFilterDocument,
    AssetFilterOther,
    CanvasAsset,
    CanvasAssetRoute,
    AssetDocumentRoute,
    BackupOpen,
    BackupCreate,
    RestorePreview,
    RestoreConfirm,
    RestoreReopen,
    RestoreHome,
    RestoreDocument,
    RestoreSearch,
    RestoreGraph,
    RestoreCanvas,
    RestoreAssets,
    VisualEvidence,
    Measurement,
}

impl PackagedUiSmokeFailureStage {
    pub const fn error_code(self) -> &'static str {
        match self {
            Self::Home => "PHASE012_PACKAGED_UI_HOME_FAILED",
            Self::DocumentCreate => "PHASE012_PACKAGED_UI_DOCUMENT_CREATE_FAILED",
            Self::DocumentEdit => "PHASE012_PACKAGED_UI_DOCUMENT_EDIT_FAILED",
            Self::DocumentSave => "PHASE012_PACKAGED_UI_DOCUMENT_SAVE_FAILED",
            Self::DocumentReopen => "PHASE012_PACKAGED_UI_DOCUMENT_REOPEN_FAILED",
            Self::GraphTargetSave => "PHASE015_PACKAGED_UI_GRAPH_TARGET_SAVE_FAILED",
            Self::GraphSourceSave => "PHASE015_PACKAGED_UI_GRAPH_SOURCE_SAVE_FAILED",
            Self::GraphProjection => "PHASE015_PACKAGED_UI_GRAPH_PROJECTION_FAILED",
            Self::GraphLocalEdge => "PHASE015_PACKAGED_UI_GRAPH_LOCAL_EDGE_FAILED",
            Self::GraphGlobalEdge => "PHASE015_PACKAGED_UI_GRAPH_GLOBAL_EDGE_FAILED",
            Self::GraphSafeLabels => "PHASE015_PACKAGED_UI_GRAPH_SAFE_LABELS_FAILED",
            Self::DocumentHistoryTab => "PHASE012_PACKAGED_UI_DOCUMENT_HISTORY_TAB_FAILED",
            Self::DocumentHistoryLoad => "PHASE012_PACKAGED_UI_DOCUMENT_HISTORY_LOAD_FAILED",
            Self::DocumentHistoryReadback => {
                "PHASE012_PACKAGED_UI_DOCUMENT_HISTORY_READBACK_FAILED"
            }
            Self::DocumentDiff => "PHASE012_PACKAGED_UI_DOCUMENT_DIFF_FAILED",
            Self::DocumentRestorePreviewAction => {
                "PHASE012_PACKAGED_UI_DOCUMENT_RESTORE_PREVIEW_ACTION_FAILED"
            }
            Self::DocumentRestorePreviewReadback => {
                "PHASE012_PACKAGED_UI_DOCUMENT_RESTORE_PREVIEW_READBACK_FAILED"
            }
            Self::DocumentRestoreReview => "PHASE012_PACKAGED_UI_DOCUMENT_RESTORE_REVIEW_FAILED",
            Self::DocumentRestoreCancel => "PHASE012_PACKAGED_UI_DOCUMENT_RESTORE_CANCEL_FAILED",
            Self::DocumentRestoreConfirm => "PHASE012_PACKAGED_UI_DOCUMENT_RESTORE_CONFIRM_FAILED",
            Self::DocumentRestoreReadback => {
                "PHASE012_PACKAGED_UI_DOCUMENT_RESTORE_READBACK_FAILED"
            }
            Self::DocumentAttachmentTab => "PHASE012_PACKAGED_UI_DOCUMENT_ATTACHMENT_TAB_FAILED",
            Self::DocumentAttachmentOpen => "PHASE012_PACKAGED_UI_DOCUMENT_ATTACHMENT_OPEN_FAILED",
            Self::DocumentAttachmentUnlinkRequest => {
                "PHASE012_PACKAGED_UI_DOCUMENT_ATTACHMENT_UNLINK_REQUEST_FAILED"
            }
            Self::DocumentAttachmentUnlinkCancel => {
                "PHASE012_PACKAGED_UI_DOCUMENT_ATTACHMENT_UNLINK_CANCEL_FAILED"
            }
            Self::GraphOpen => "PHASE012_PACKAGED_UI_GRAPH_OPEN_FAILED",
            Self::GraphScopeGlobal => "PHASE012_PACKAGED_UI_GRAPH_SCOPE_GLOBAL_FAILED",
            Self::GraphScopeLocal => "PHASE012_PACKAGED_UI_GRAPH_SCOPE_LOCAL_FAILED",
            Self::GraphDepth => "PHASE012_PACKAGED_UI_GRAPH_DEPTH_FAILED",
            Self::GraphDirection => "PHASE012_PACKAGED_UI_GRAPH_DIRECTION_FAILED",
            Self::GraphUnresolved => "PHASE012_PACKAGED_UI_GRAPH_UNRESOLVED_FAILED",
            Self::GraphAssets => "PHASE012_PACKAGED_UI_GRAPH_ASSETS_FAILED",
            Self::GraphZoomIn => "PHASE012_PACKAGED_UI_GRAPH_ZOOM_IN_FAILED",
            Self::GraphZoomOut => "PHASE012_PACKAGED_UI_GRAPH_ZOOM_OUT_FAILED",
            Self::GraphFitView => "PHASE012_PACKAGED_UI_GRAPH_FIT_VIEW_FAILED",
            Self::GraphNode => "PHASE012_PACKAGED_UI_GRAPH_NODE_FAILED",
            Self::GraphDocumentRoute => "PHASE012_PACKAGED_UI_GRAPH_DOCUMENT_ROUTE_FAILED",
            Self::GraphAttachmentOpen => "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_OPEN_FAILED",
            Self::GraphAttachmentLocalEdge => {
                "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_LOCAL_EDGE_FAILED"
            }
            Self::GraphAttachmentLocalFilter => {
                "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_LOCAL_FILTER_FAILED"
            }
            Self::GraphAttachmentLocalNode => {
                "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_LOCAL_NODE_MISSING"
            }
            Self::GraphAttachmentLocalIdentity => {
                "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_LOCAL_IDENTITY_MISMATCH"
            }
            Self::GraphAttachmentLocalLabel => {
                "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_LOCAL_LABEL_FAILED"
            }
            Self::GraphAttachmentGlobalEdge => {
                "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_GLOBAL_EDGE_FAILED"
            }
            Self::GraphAttachmentRoute => "PHASE016_PACKAGED_UI_GRAPH_ATTACHMENT_ROUTE_FAILED",
            Self::Canvas => "PHASE012_PACKAGED_UI_CANVAS_FAILED",
            Self::CanvasOpen => "PHASE012_PACKAGED_UI_CANVAS_OPEN_FAILED",
            Self::CanvasCreate => "PHASE012_PACKAGED_UI_CANVAS_CREATE_FAILED",
            Self::CanvasNote => "PHASE012_PACKAGED_UI_CANVAS_NOTE_FAILED",
            Self::CanvasTextEdit => "PHASE017_PACKAGED_UI_CANVAS_TEXT_EDIT_FAILED",
            Self::CanvasPan => "PHASE012_PACKAGED_UI_CANVAS_PAN_FAILED",
            Self::CanvasZoom => "PHASE012_PACKAGED_UI_CANVAS_ZOOM_FAILED",
            Self::CanvasArrange => "PHASE012_PACKAGED_UI_CANVAS_ARRANGE_FAILED",
            Self::CanvasDocument => "PHASE012_PACKAGED_UI_CANVAS_DOCUMENT_FAILED",
            Self::CanvasEdge => "PHASE012_PACKAGED_UI_CANVAS_EDGE_FAILED",
            Self::CanvasDrag => "PHASE012_PACKAGED_UI_CANVAS_DRAG_FAILED",
            Self::CanvasResize => "PHASE012_PACKAGED_UI_CANVAS_RESIZE_FAILED",
            Self::CanvasReopen => "PHASE012_PACKAGED_UI_CANVAS_REOPEN_FAILED",
            Self::CanvasRename => "PHASE012_PACKAGED_UI_CANVAS_RENAME_FAILED",
            Self::CanvasArchive => "PHASE012_PACKAGED_UI_CANVAS_ARCHIVE_FAILED",
            Self::CanvasArchiveReopen => "PHASE012_PACKAGED_UI_CANVAS_ARCHIVE_REOPEN_FAILED",
            Self::CanvasRecovery => "PHASE012_PACKAGED_UI_CANVAS_RECOVERY_FAILED",
            Self::CanvasRecoveryOpen => "PHASE017_PACKAGED_UI_CANVAS_RECOVERY_OPEN_FAILED",
            Self::CanvasRecoveryDetect => "PHASE017_PACKAGED_UI_CANVAS_RECOVERY_DETECT_FAILED",
            Self::CanvasRecoveryApply => "PHASE017_PACKAGED_UI_CANVAS_RECOVERY_APPLY_FAILED",
            Self::Assets => "PHASE012_PACKAGED_UI_ASSETS_FAILED",
            Self::AssetOpen => "PHASE012_PACKAGED_UI_ASSET_OPEN_FAILED",
            Self::AssetImport => "PHASE012_PACKAGED_UI_ASSET_IMPORT_FAILED",
            Self::AssetImportReadback => "PHASE015_PACKAGED_UI_ASSET_IMPORT_READBACK_FAILED",
            Self::AssetImportOperation => "PHASE015_PACKAGED_UI_ASSET_IMPORT_OPERATION_FAILED",
            Self::AssetImportScope => "PHASE015_PACKAGED_UI_ASSET_IMPORT_SCOPE_FAILED",
            Self::AssetImportPresentation => {
                "PHASE015_PACKAGED_UI_ASSET_IMPORT_PRESENTATION_FAILED"
            }
            Self::AssetDetail => "PHASE012_PACKAGED_UI_ASSET_DETAIL_FAILED",
            Self::AssetPreview => "PHASE012_PACKAGED_UI_ASSET_PREVIEW_FAILED",
            Self::AssetUnlink => "PHASE012_PACKAGED_UI_ASSET_UNLINK_FAILED",
            Self::AssetLibrary => "PHASE012_PACKAGED_UI_ASSET_LIBRARY_FAILED",
            Self::AssetDetachedDetail => "PHASE012_PACKAGED_UI_ASSET_DETACHED_DETAIL_FAILED",
            Self::AssetRelink => "PHASE012_PACKAGED_UI_ASSET_RELINK_FAILED",
            Self::AssetFilters => "PHASE012_PACKAGED_UI_ASSET_FILTERS_FAILED",
            Self::AssetFilterAll => "PHASE012_PACKAGED_UI_ASSET_FILTER_ALL_FAILED",
            Self::AssetFilterImage => "PHASE012_PACKAGED_UI_ASSET_FILTER_IMAGE_FAILED",
            Self::AssetFilterPdf => "PHASE012_PACKAGED_UI_ASSET_FILTER_PDF_FAILED",
            Self::AssetFilterDocument => "PHASE012_PACKAGED_UI_ASSET_FILTER_DOCUMENT_FAILED",
            Self::AssetFilterOther => "PHASE012_PACKAGED_UI_ASSET_FILTER_OTHER_FAILED",
            Self::CanvasAsset => "PHASE012_PACKAGED_UI_CANVAS_ASSET_FAILED",
            Self::CanvasAssetRoute => "PHASE012_PACKAGED_UI_CANVAS_ASSET_ROUTE_FAILED",
            Self::AssetDocumentRoute => "PHASE012_PACKAGED_UI_ASSET_DOCUMENT_ROUTE_FAILED",
            Self::BackupOpen => "PHASE012_PACKAGED_UI_BACKUP_OPEN_FAILED",
            Self::BackupCreate => "PHASE012_PACKAGED_UI_BACKUP_CREATE_FAILED",
            Self::RestorePreview => "PHASE012_PACKAGED_UI_RESTORE_PREVIEW_FAILED",
            Self::RestoreConfirm => "PHASE012_PACKAGED_UI_RESTORE_CONFIRM_FAILED",
            Self::RestoreReopen => "PHASE012_PACKAGED_UI_RESTORE_REOPEN_FAILED",
            Self::RestoreHome => "PHASE016_PACKAGED_UI_RESTORE_HOME_FAILED",
            Self::RestoreDocument => "PHASE016_PACKAGED_UI_RESTORE_DOCUMENT_FAILED",
            Self::RestoreSearch => "PHASE016_PACKAGED_UI_RESTORE_SEARCH_FAILED",
            Self::RestoreGraph => "PHASE016_PACKAGED_UI_RESTORE_GRAPH_FAILED",
            Self::RestoreCanvas => "PHASE016_PACKAGED_UI_RESTORE_CANVAS_FAILED",
            Self::RestoreAssets => "PHASE016_PACKAGED_UI_RESTORE_ASSETS_FAILED",
            Self::VisualEvidence => "PHASE016_PACKAGED_UI_VISUAL_EVIDENCE_FAILED",
            Self::Measurement => "PHASE012_PACKAGED_UI_MEASUREMENT_FAILED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackagedUiSmokeErrorCode {
    SurfaceMissing,
    SampleCountInvalid,
    PerformanceBudgetExceeded,
    UiErrorReported,
    ActionCoverageIncomplete,
    DocumentVersionWorkflowMissing,
    DocumentAttachmentWorkflowMissing,
    AttachmentImportEvidenceMissing,
    AttachmentCurrentReadbackMissing,
    AttachmentDocumentReadbackMissing,
    AttachmentRestartReadbackMissing,
    CanvasTextEditReadbackMissing,
    CanvasTextRestartReadbackMissing,
    GraphLinkFixtureEvidenceMissing,
    GraphLocalEdgeEvidenceMissing,
    GraphGlobalEdgeEvidenceMissing,
    GraphSafeLabelsEvidenceMissing,
    KeyboardDocumentWorkflowMissing,
    VisualEvidenceMissing,
}

impl PackagedUiSmokeErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SurfaceMissing => "PHASE012_PACKAGED_UI_SURFACE_MISSING",
            Self::SampleCountInvalid => "PHASE012_PACKAGED_UI_SAMPLE_COUNT_INVALID",
            Self::PerformanceBudgetExceeded => "PHASE012_PACKAGED_UI_PERFORMANCE_BUDGET_EXCEEDED",
            Self::UiErrorReported => "PHASE012_PACKAGED_UI_ERROR_REPORTED",
            Self::ActionCoverageIncomplete => "PHASE012_PACKAGED_UI_ACTION_COVERAGE_INCOMPLETE",
            Self::DocumentVersionWorkflowMissing => {
                "PHASE012_PACKAGED_UI_DOCUMENT_VERSION_WORKFLOW_MISSING"
            }
            Self::DocumentAttachmentWorkflowMissing => {
                "PHASE012_PACKAGED_UI_DOCUMENT_ATTACHMENT_WORKFLOW_MISSING"
            }
            Self::AttachmentImportEvidenceMissing => {
                "PHASE015_PACKAGED_UI_ATTACHMENT_IMPORT_EVIDENCE_MISSING"
            }
            Self::AttachmentCurrentReadbackMissing => {
                "PHASE015_PACKAGED_UI_ATTACHMENT_CURRENT_READBACK_MISSING"
            }
            Self::AttachmentDocumentReadbackMissing => {
                "PHASE015_PACKAGED_UI_ATTACHMENT_DOCUMENT_READBACK_MISSING"
            }
            Self::AttachmentRestartReadbackMissing => {
                "PHASE015_PACKAGED_UI_ATTACHMENT_RESTART_READBACK_MISSING"
            }
            Self::CanvasTextEditReadbackMissing => {
                "PHASE017_PACKAGED_UI_CANVAS_TEXT_EDIT_READBACK_MISSING"
            }
            Self::CanvasTextRestartReadbackMissing => {
                "PHASE017_PACKAGED_UI_CANVAS_TEXT_RESTART_READBACK_MISSING"
            }
            Self::GraphLinkFixtureEvidenceMissing => {
                "PHASE015_PACKAGED_UI_GRAPH_LINK_FIXTURE_EVIDENCE_MISSING"
            }
            Self::GraphLocalEdgeEvidenceMissing => {
                "PHASE015_PACKAGED_UI_GRAPH_LOCAL_EDGE_EVIDENCE_MISSING"
            }
            Self::GraphGlobalEdgeEvidenceMissing => {
                "PHASE015_PACKAGED_UI_GRAPH_GLOBAL_EDGE_EVIDENCE_MISSING"
            }
            Self::GraphSafeLabelsEvidenceMissing => {
                "PHASE015_PACKAGED_UI_GRAPH_SAFE_LABELS_EVIDENCE_MISSING"
            }
            Self::KeyboardDocumentWorkflowMissing => {
                "PHASE012_PACKAGED_UI_KEYBOARD_DOCUMENT_WORKFLOW_MISSING"
            }
            Self::VisualEvidenceMissing => "PHASE016_PACKAGED_UI_VISUAL_EVIDENCE_MISSING",
        }
    }
}

pub fn validate_packaged_ui_smoke_report(
    report: PackagedUiSmokeReport,
) -> Result<(), PackagedUiSmokeErrorCode> {
    validate_packaged_ui_smoke_initial_report(report)?;
    if !report.attachment_restart_readback_verified {
        return Err(PackagedUiSmokeErrorCode::AttachmentRestartReadbackMissing);
    }
    Ok(())
}

pub fn validate_packaged_ui_smoke_initial_report(
    report: PackagedUiSmokeReport,
) -> Result<(), PackagedUiSmokeErrorCode> {
    if !report.home_ready || !report.graph_ready || !report.canvas_ready || !report.assets_ready {
        return Err(PackagedUiSmokeErrorCode::SurfaceMissing);
    }
    if !report.document_version_workflow_verified {
        return Err(PackagedUiSmokeErrorCode::DocumentVersionWorkflowMissing);
    }
    if !report.document_attachment_workflow_verified {
        return Err(PackagedUiSmokeErrorCode::DocumentAttachmentWorkflowMissing);
    }
    if !report.attachment_import_completed {
        return Err(PackagedUiSmokeErrorCode::AttachmentImportEvidenceMissing);
    }
    if !report.attachment_current_readback_verified {
        return Err(PackagedUiSmokeErrorCode::AttachmentCurrentReadbackMissing);
    }
    if !report.attachment_document_readback_verified {
        return Err(PackagedUiSmokeErrorCode::AttachmentDocumentReadbackMissing);
    }
    if !report.keyboard_document_workflow_verified {
        return Err(PackagedUiSmokeErrorCode::KeyboardDocumentWorkflowMissing);
    }
    if !report.graph_link_fixture_saved {
        return Err(PackagedUiSmokeErrorCode::GraphLinkFixtureEvidenceMissing);
    }
    if !report.graph_local_edge_verified {
        return Err(PackagedUiSmokeErrorCode::GraphLocalEdgeEvidenceMissing);
    }
    if !report.graph_global_edge_verified {
        return Err(PackagedUiSmokeErrorCode::GraphGlobalEdgeEvidenceMissing);
    }
    if !report.graph_safe_labels_verified {
        return Err(PackagedUiSmokeErrorCode::GraphSafeLabelsEvidenceMissing);
    }
    if !report.canvas_text_edit_readback_verified {
        return Err(PackagedUiSmokeErrorCode::CanvasTextEditReadbackMissing);
    }
    if report.sample_count != 200 {
        return Err(PackagedUiSmokeErrorCode::SampleCountInvalid);
    }
    if report.error_count != 0 {
        return Err(PackagedUiSmokeErrorCode::UiErrorReported);
    }
    if report.action_count < 90 || report.durable_readback_count < 33 {
        return Err(PackagedUiSmokeErrorCode::ActionCoverageIncomplete);
    }
    if report.p95_ms > 300 {
        return Err(PackagedUiSmokeErrorCode::PerformanceBudgetExceeded);
    }
    Ok(())
}

pub const fn validate_packaged_ui_smoke_restart_report(
    report: PackagedUiSmokeRestartReport,
) -> Result<(), PackagedUiSmokeErrorCode> {
    if !report.attachment_restart_readback_verified {
        return Err(PackagedUiSmokeErrorCode::AttachmentRestartReadbackMissing);
    }
    if !report.canvas_text_restart_readback_verified {
        return Err(PackagedUiSmokeErrorCode::CanvasTextRestartReadbackMissing);
    }
    if report.error_count != 0 {
        return Err(PackagedUiSmokeErrorCode::UiErrorReported);
    }
    Ok(())
}

pub struct DesktopProjectionRepairOperationRuntime {
    repository: Mutex<DurableProjectionRepairRepository>,
    ids: Mutex<DesktopRepairIdSource>,
}

const DESKTOP_CANVAS_GRAPH_STARTUP_RECOVERY_LIMIT: usize = 1_000;

#[derive(Debug, Clone)]
pub struct DesktopCanvasCatalogRuntime {
    root: PathBuf,
    max_limit: usize,
}

impl DesktopCanvasCatalogRuntime {
    pub fn new(root: PathBuf, max_limit: usize) -> Result<Self, &'static str> {
        if max_limit == 0 {
            return Err("CANVAS_CATALOG_INVALID_LIMIT");
        }
        Ok(Self { root, max_limit })
    }

    pub fn query(
        &self,
        request: DesktopCanvasCatalogQueryRequestDto,
    ) -> DesktopCanvasCatalogResponse {
        if request.limit == 0 || request.limit > self.max_limit {
            return DesktopCanvasCatalogResponse::failure("CANVAS_CATALOG_INVALID_LIMIT", false);
        }
        let catalog = DurableCanvasRepository::new(self.root.clone());
        let selection = DurableLastCanvasSelection::new(self.root.clone());
        match ResolveInitialCanvasUsecase::new().execute(
            ResolveInitialCanvasInput::new(
                &request.workspace_id,
                request.limit,
                request.include_archived,
            ),
            &catalog,
            &selection,
        ) {
            Ok(output) => DesktopCanvasCatalogResponse {
                ok: true,
                data: Some(DesktopCanvasCatalogDataDto {
                    entries: output
                        .entries()
                        .iter()
                        .map(|entry| DesktopCanvasCatalogEntryDto {
                            canvas_id: entry.canvas_id().as_str().to_string(),
                            title: entry.title().as_str().to_string(),
                            lifecycle: format!("{:?}", entry.lifecycle()).to_lowercase(),
                            revision: entry.revision().value(),
                        })
                        .collect(),
                    selected_canvas_id: output.selected_canvas_id().map(str::to_string),
                    selection_source: match output.selection_source() {
                        ResolvedCanvasSelectionSource::LastUsed => "last_used",
                        ResolvedCanvasSelectionSource::Fallback => "fallback",
                        ResolvedCanvasSelectionSource::Empty => "empty",
                    }
                    .to_string(),
                }),
                selected_canvas_id: output.selected_canvas_id().map(str::to_string),
                error_code: None,
                retryable: false,
            },
            Err(error) => DesktopCanvasCatalogResponse::failure(error.code(), error.retryable()),
        }
    }

    pub fn select(
        &self,
        request: DesktopCanvasCatalogSelectRequestDto,
    ) -> DesktopCanvasCatalogResponse {
        let catalog = DurableCanvasRepository::new(self.root.clone());
        let mut selection = DurableLastCanvasSelection::new(self.root.clone());
        match SelectCanvasUsecase::new().execute(
            SelectCanvasInput::new(&request.workspace_id, &request.canvas_id, self.max_limit),
            &catalog,
            &mut selection,
        ) {
            Ok(output) => DesktopCanvasCatalogResponse {
                ok: true,
                data: None,
                selected_canvas_id: Some(output.selected_canvas_id().to_string()),
                error_code: None,
                retryable: false,
            },
            Err(error) => DesktopCanvasCatalogResponse::failure(
                error.code(),
                matches!(
                    error,
                    SelectCanvasError::CatalogUnavailable | SelectCanvasError::SelectionUnavailable
                ),
            ),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCanvasCatalogQueryRequestDto {
    pub workspace_id: String,
    pub limit: usize,
    pub include_archived: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCanvasCatalogSelectRequestDto {
    pub workspace_id: String,
    pub canvas_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCanvasCatalogEntryDto {
    pub canvas_id: String,
    pub title: String,
    pub lifecycle: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCanvasCatalogDataDto {
    pub entries: Vec<DesktopCanvasCatalogEntryDto>,
    pub selected_canvas_id: Option<String>,
    pub selection_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCanvasCatalogResponse {
    pub ok: bool,
    pub data: Option<DesktopCanvasCatalogDataDto>,
    pub selected_canvas_id: Option<String>,
    pub error_code: Option<String>,
    pub retryable: bool,
}

impl DesktopCanvasCatalogResponse {
    fn failure(error_code: &str, retryable: bool) -> Self {
        Self {
            ok: false,
            data: None,
            selected_canvas_id: None,
            error_code: Some(error_code.to_string()),
            retryable,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DesktopCanvasRuntime {
    root: PathBuf,
    policy: CanvasMutationPolicy,
    arrange_policy: CanvasAutoArrangePolicy,
    document_body_policy: DocumentBodyPolicy,
    graph_policy: CanvasGraphProjectionPolicy,
    product_events: Arc<Mutex<Vec<DesktopCanvasProductEvent>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopCanvasProductEvent {
    Created {
        canvas_id: String,
        revision: u64,
    },
    Archived {
        canvas_id: String,
        revision: u64,
    },
    SaveFailed {
        canvas_id: String,
        error_code: String,
    },
    Recovered {
        canvas_id: String,
        revision: u64,
    },
    RecoveryFailed {
        canvas_id: String,
        error_code: String,
    },
}

impl DesktopCanvasRuntime {
    pub fn new(root: PathBuf) -> Result<Self, &'static str> {
        let geometry = CanvasGeometryPolicy::new(80, 1200, 60, 900, 25, 400)
            .map_err(|_| "CANVAS_INVALID_POLICY")?;
        let policy =
            CanvasMutationPolicy::new(500, 1000, geometry).map_err(|_| "CANVAS_INVALID_POLICY")?;
        let arrange_policy = CanvasAutoArrangePolicy::new(4, 80, 80, 360, 240)
            .map_err(|_| "CANVAS_INVALID_POLICY")?;
        let document_body_policy =
            DocumentBodyPolicy::new(4 * 1024 * 1024).map_err(|_| "CANVAS_INVALID_POLICY")?;
        let graph_policy =
            CanvasGraphProjectionPolicy::new(1000).map_err(|_| "CANVAS_INVALID_POLICY")?;
        let runtime = Self {
            root,
            policy,
            arrange_policy,
            document_body_policy,
            graph_policy,
            product_events: Arc::new(Mutex::new(Vec::new())),
        };
        runtime.recover_graph_relations()?;
        Ok(runtime)
    }

    fn recover_graph_relations(&self) -> Result<(), &'static str> {
        let repository = DurableCanvasRepository::new(self.root.clone());
        let records = repository
            .list_current_canvas_records(DESKTOP_CANVAS_GRAPH_STARTUP_RECOVERY_LIMIT)
            .map_err(|_| "CANVAS_GRAPH_STARTUP_RECOVERY_FAILED")?;
        for discovered in records {
            self.project_graph_relations(
                discovered.workspace_id().as_str(),
                discovered.record().clone(),
            )
            .map_err(|_| "CANVAS_GRAPH_STARTUP_RECOVERY_FAILED")?;
        }
        Ok(())
    }
    pub fn product_events(&self) -> Vec<DesktopCanvasProductEvent> {
        self.product_events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
    pub fn execute(&self, request: DesktopCanvasRequestDto) -> DesktopCanvasResponse {
        let mut repository = DurableCanvasRepository::new(self.root.clone());
        let mut logger = DesktopCanvasLogSink {
            events: Arc::clone(&self.product_events),
        };
        let write_canvas_id = request.write_canvas_id().map(str::to_owned);
        let operation_id = request.operation_id().map(str::to_owned);
        let workspace_id = request.workspace_id().to_owned();
        if operation_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            let error = CanvasRuntimeError::Mutation(CanvasMutationError::InvalidInput);
            if let Some(canvas_id) = write_canvas_id.as_deref() {
                logger.write_failure(canvas_id, error.code());
            }
            return DesktopCanvasResponse::failure(error);
        }
        let result = match request {
            DesktopCanvasRequestDto::GetViewport {
                workspace_id,
                canvas_id,
                center_x,
                center_y,
                zoom_percent,
                surface_width,
                surface_height,
                overscan,
                node_limit,
                edge_limit,
            } => {
                if let Err(error) = GetCanvasRecordUsecase::new().execute(
                    GetCanvasRecordInput::new(&workspace_id, &canvas_id),
                    &repository,
                ) {
                    return DesktopCanvasResponse::failure(CanvasRuntimeError::Lifecycle(error));
                }
                return match GetCanvasViewportUsecase::new().execute(
                    GetCanvasViewportInput::new(
                        &workspace_id,
                        &canvas_id,
                        center_x,
                        center_y,
                        zoom_percent,
                        surface_width,
                        surface_height,
                        overscan,
                        node_limit,
                        edge_limit,
                    ),
                    &repository,
                ) {
                    Ok(page) => match self.resolve_target_presentations(&workspace_id, &page.nodes)
                    {
                        Ok(presentations) => {
                            DesktopCanvasResponse::success_viewport(page, &presentations)
                        }
                        Err(error) => DesktopCanvasResponse::failure(
                            CanvasRuntimeError::TargetPresentation(error),
                        ),
                    },
                    Err(error) => DesktopCanvasResponse::failure(CanvasRuntimeError::Query(error)),
                };
            }
            DesktopCanvasRequestDto::PreviewAutoArrange {
                workspace_id,
                canvas_id,
                expected_revision,
            } => PreviewAutoArrangeCanvasUsecase::new()
                .execute(
                    AutoArrangeCanvasInput::new(&workspace_id, &canvas_id, expected_revision),
                    &self.arrange_policy,
                    &repository,
                )
                .map(|value| value.record().clone())
                .map_err(CanvasRuntimeError::Mutation),
            DesktopCanvasRequestDto::Create {
                workspace_id,
                canvas_id,
                title,
            } => CreateCanvasRecordUsecase::new()
                .execute(
                    CreateCanvasRecordInput::new(&workspace_id, &canvas_id, &title),
                    &mut repository,
                    &mut logger,
                )
                .map(|v| v.record().clone())
                .map_err(CanvasRuntimeError::Lifecycle),
            DesktopCanvasRequestDto::Get {
                workspace_id,
                canvas_id,
            } => GetCanvasRecordUsecase::new()
                .execute(
                    GetCanvasRecordInput::new(&workspace_id, &canvas_id),
                    &repository,
                )
                .map(|v| v.record().clone())
                .map_err(CanvasRuntimeError::Lifecycle),
            DesktopCanvasRequestDto::Recover {
                workspace_id,
                canvas_id,
                ..
            } => RecoverCanvasUsecase::new(128)
                .expect("fixed Canvas recovery candidate policy")
                .execute(
                    RecoverCanvasInput::new(&workspace_id, &canvas_id),
                    &mut repository,
                    &mut logger,
                )
                .map_err(CanvasRuntimeError::Recovery)
                .and_then(|_| {
                    GetCanvasRecordUsecase::new()
                        .execute(
                            GetCanvasRecordInput::new(&workspace_id, &canvas_id),
                            &repository,
                        )
                        .map(|output| output.record().clone())
                        .map_err(CanvasRuntimeError::Lifecycle)
                }),
            DesktopCanvasRequestDto::Rename {
                workspace_id,
                canvas_id,
                expected_revision,
                title,
                ..
            } => RenameCanvasUsecase::new()
                .execute(
                    RenameCanvasInput::new(&workspace_id, &canvas_id, expected_revision, &title),
                    &mut repository,
                    &mut logger,
                )
                .map(|v| v.record().clone())
                .map_err(CanvasRuntimeError::Lifecycle),
            DesktopCanvasRequestDto::Archive {
                workspace_id,
                canvas_id,
                expected_revision,
                ..
            } => ArchiveCanvasUsecase::new()
                .execute(
                    ArchiveCanvasInput::new(&workspace_id, &canvas_id, expected_revision),
                    &mut repository,
                    &mut logger,
                )
                .map(|v| v.record().clone())
                .map_err(CanvasRuntimeError::Lifecycle),
            DesktopCanvasRequestDto::AddDocumentNode {
                workspace_id,
                canvas_id,
                expected_revision,
                node_id,
                document_id,
                x,
                y,
                width,
                height,
                ..
            } => AddValidatedCanvasNodeUsecase::new()
                .execute(
                    AddCanvasNodeMutationInput::new(
                        &workspace_id,
                        &canvas_id,
                        expected_revision,
                        &node_id,
                        CanvasNodeTargetInput::Document(document_id),
                        x,
                        y,
                        width,
                        height,
                    ),
                    &self.policy,
                    &LocalDocumentRepository::with_body_policy(
                        self.root.join("authoring-current"),
                        self.document_body_policy,
                    ),
                    &DurableAssetMetadataCatalog::new(self.root.clone()),
                    &mut repository,
                    &mut logger,
                )
                .map(|v| v.record().clone())
                .map_err(CanvasRuntimeError::Mutation),
            DesktopCanvasRequestDto::AddAssetNode {
                workspace_id,
                canvas_id,
                expected_revision,
                node_id,
                asset_id,
                x,
                y,
                width,
                height,
                ..
            } => AddValidatedCanvasNodeUsecase::new()
                .execute(
                    AddCanvasNodeMutationInput::new(
                        &workspace_id,
                        &canvas_id,
                        expected_revision,
                        &node_id,
                        CanvasNodeTargetInput::Attachment(asset_id),
                        x,
                        y,
                        width,
                        height,
                    ),
                    &self.policy,
                    &LocalDocumentRepository::with_body_policy(
                        self.root.join("authoring-current"),
                        self.document_body_policy,
                    ),
                    &DurableAssetMetadataCatalog::new(self.root.clone()),
                    &mut repository,
                    &mut logger,
                )
                .map(|v| v.record().clone())
                .map_err(CanvasRuntimeError::Mutation),
            DesktopCanvasRequestDto::AddTextNode {
                workspace_id,
                canvas_id,
                expected_revision,
                node_id,
                text,
                x,
                y,
                width,
                height,
                ..
            } => AddCanvasNodeMutationUsecase::new()
                .execute(
                    AddCanvasNodeMutationInput::new(
                        &workspace_id,
                        &canvas_id,
                        expected_revision,
                        &node_id,
                        CanvasNodeTargetInput::Text(text),
                        x,
                        y,
                        width,
                        height,
                    ),
                    &self.policy,
                    &mut repository,
                    &mut logger,
                )
                .map(|v| v.record().clone())
                .map_err(CanvasRuntimeError::Mutation),
            DesktopCanvasRequestDto::ConnectEdge {
                workspace_id,
                canvas_id,
                expected_revision,
                edge_id,
                source_node_id,
                target_node_id,
                ..
            } => ConnectCanvasEdgeUsecase::new()
                .execute(
                    ConnectCanvasEdgeInput::new(
                        &workspace_id,
                        &canvas_id,
                        expected_revision,
                        &edge_id,
                        &source_node_id,
                        &target_node_id,
                    ),
                    &self.policy,
                    &mut repository,
                    &mut logger,
                )
                .map(|v| v.record().clone())
                .map_err(CanvasRuntimeError::Mutation),
            DesktopCanvasRequestDto::RemoveNode {
                workspace_id,
                canvas_id,
                expected_revision,
                node_id,
                ..
            } => RemoveCanvasNodeUsecase::new()
                .execute(
                    RemoveCanvasNodeInput::new(
                        &workspace_id,
                        &canvas_id,
                        expected_revision,
                        &node_id,
                    ),
                    &mut repository,
                    &mut logger,
                )
                .map(|v| v.record().clone())
                .map_err(CanvasRuntimeError::Mutation),
            DesktopCanvasRequestDto::RemoveEdge {
                workspace_id,
                canvas_id,
                expected_revision,
                edge_id,
                ..
            } => RemoveCanvasEdgeUsecase::new()
                .execute(
                    RemoveCanvasEdgeInput::new(
                        &workspace_id,
                        &canvas_id,
                        expected_revision,
                        &edge_id,
                    ),
                    &mut repository,
                    &mut logger,
                )
                .map(|value| value.record().clone())
                .map_err(CanvasRuntimeError::Mutation),
            DesktopCanvasRequestDto::UpdateNodeGeometry {
                workspace_id,
                canvas_id,
                expected_revision,
                node_id,
                x,
                y,
                width,
                height,
                ..
            } => UpdateCanvasNodeGeometryUsecase::new()
                .execute(
                    UpdateCanvasNodeGeometryInput::new(
                        &workspace_id,
                        &canvas_id,
                        expected_revision,
                        &node_id,
                        x,
                        y,
                        width,
                        height,
                    ),
                    &self.policy,
                    &mut repository,
                    &mut logger,
                )
                .map(|v| v.record().clone())
                .map_err(CanvasRuntimeError::Mutation),
            DesktopCanvasRequestDto::UpdateTextCard {
                workspace_id,
                canvas_id,
                expected_revision,
                node_id,
                text,
                ..
            } => UpdateCanvasTextCardUsecase::new()
                .execute(
                    UpdateCanvasTextCardInput::new(
                        &workspace_id,
                        &canvas_id,
                        expected_revision,
                        &node_id,
                        &text,
                    ),
                    &mut repository,
                    &mut logger,
                )
                .map(|value| value.record().clone())
                .map_err(CanvasRuntimeError::Mutation),
            DesktopCanvasRequestDto::UpdateViewport {
                workspace_id,
                canvas_id,
                expected_revision,
                center_x,
                center_y,
                zoom_percent,
                ..
            } => UpdateCanvasViewportUsecase::new()
                .execute(
                    UpdateCanvasViewportInput::new(
                        &workspace_id,
                        &canvas_id,
                        expected_revision,
                        center_x,
                        center_y,
                        zoom_percent,
                    ),
                    &self.policy,
                    &mut repository,
                    &mut logger,
                )
                .map(|v| v.record().clone())
                .map_err(CanvasRuntimeError::Mutation),
            DesktopCanvasRequestDto::AutoArrange {
                workspace_id,
                canvas_id,
                expected_revision,
                ..
            } => AutoArrangeCanvasUsecase::new()
                .execute(
                    AutoArrangeCanvasInput::new(&workspace_id, &canvas_id, expected_revision),
                    &self.arrange_policy,
                    &mut repository,
                    &mut logger,
                )
                .map(|v| v.record().clone())
                .map_err(CanvasRuntimeError::Mutation),
        };
        match result {
            Ok(record) => {
                if write_canvas_id.is_some()
                    && let Err(error) = self.project_graph_relations(&workspace_id, record.clone())
                {
                    let error = CanvasRuntimeError::GraphProjection(error);
                    if let Some(canvas_id) = write_canvas_id.as_deref() {
                        logger.write_failure(canvas_id, error.code());
                    }
                    return DesktopCanvasResponse::failure(error);
                }
                match self.resolve_target_presentations(&workspace_id, record.canvas().nodes()) {
                    Ok(presentations) => {
                        DesktopCanvasResponse::success(record, operation_id, &presentations)
                    }
                    Err(error) => DesktopCanvasResponse::failure(
                        CanvasRuntimeError::TargetPresentation(error),
                    ),
                }
            }
            Err(error) => {
                if let Some(canvas_id) = write_canvas_id.as_deref() {
                    logger.write_failure(canvas_id, error.code());
                }
                DesktopCanvasResponse::failure(error)
            }
        }
    }

    fn project_graph_relations(
        &self,
        workspace_id: &str,
        record: cabinet_ports::canvas_repository::CanvasRecord,
    ) -> Result<(), CanvasGraphRelationProjectionError> {
        ProjectCanvasGraphRelationsUsecase::new(self.graph_policy)
            .execute(
                ProjectCanvasGraphRelationsInput::new(workspace_id, record),
                &mut DurableCanvasGraphRelationProjectionStore::new(self.root.clone()),
            )
            .map(|_| ())
    }

    fn resolve_target_presentations(
        &self,
        workspace_id: &str,
        nodes: &[cabinet_domain::canvas::CanvasNode],
    ) -> Result<Vec<CanvasTargetPresentation>, ResolveCanvasTargetPresentationsError> {
        ResolveCanvasTargetPresentationsUsecase::new()
            .execute(
                ResolveCanvasTargetPresentationsInput::new(workspace_id, nodes),
                &LocalDocumentRepository::with_body_policy(
                    self.root.join("authoring-current"),
                    self.document_body_policy,
                ),
                &DurableAssetMetadataCatalog::new(self.root.clone()),
            )
            .map(|output| output.presentations().to_vec())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum DesktopCanvasRequestDto {
    Create {
        workspace_id: String,
        canvas_id: String,
        title: String,
    },
    Get {
        workspace_id: String,
        canvas_id: String,
    },
    Recover {
        workspace_id: String,
        canvas_id: String,
        operation_id: String,
    },
    GetViewport {
        workspace_id: String,
        canvas_id: String,
        center_x: Option<i32>,
        center_y: Option<i32>,
        zoom_percent: Option<u16>,
        surface_width: u32,
        surface_height: u32,
        overscan: u32,
        node_limit: usize,
        edge_limit: usize,
    },
    PreviewAutoArrange {
        workspace_id: String,
        canvas_id: String,
        expected_revision: u64,
    },
    Rename {
        workspace_id: String,
        canvas_id: String,
        expected_revision: u64,
        title: String,
        operation_id: String,
    },
    Archive {
        workspace_id: String,
        canvas_id: String,
        expected_revision: u64,
        operation_id: String,
    },
    AddDocumentNode {
        workspace_id: String,
        canvas_id: String,
        expected_revision: u64,
        node_id: String,
        document_id: String,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        operation_id: String,
    },
    AddAssetNode {
        workspace_id: String,
        canvas_id: String,
        expected_revision: u64,
        node_id: String,
        asset_id: String,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        operation_id: String,
    },
    AddTextNode {
        workspace_id: String,
        canvas_id: String,
        expected_revision: u64,
        node_id: String,
        text: String,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        operation_id: String,
    },
    ConnectEdge {
        workspace_id: String,
        canvas_id: String,
        expected_revision: u64,
        edge_id: String,
        source_node_id: String,
        target_node_id: String,
        operation_id: String,
    },
    RemoveNode {
        workspace_id: String,
        canvas_id: String,
        expected_revision: u64,
        node_id: String,
        operation_id: String,
    },
    RemoveEdge {
        workspace_id: String,
        canvas_id: String,
        expected_revision: u64,
        edge_id: String,
        operation_id: String,
    },
    UpdateNodeGeometry {
        workspace_id: String,
        canvas_id: String,
        expected_revision: u64,
        node_id: String,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        operation_id: String,
    },
    UpdateTextCard {
        workspace_id: String,
        canvas_id: String,
        expected_revision: u64,
        node_id: String,
        text: String,
        operation_id: String,
    },
    UpdateViewport {
        workspace_id: String,
        canvas_id: String,
        expected_revision: u64,
        center_x: i32,
        center_y: i32,
        zoom_percent: u16,
        operation_id: String,
    },
    AutoArrange {
        workspace_id: String,
        canvas_id: String,
        expected_revision: u64,
        operation_id: String,
    },
}
impl DesktopCanvasRequestDto {
    fn workspace_id(&self) -> &str {
        match self {
            Self::Create { workspace_id, .. }
            | Self::Get { workspace_id, .. }
            | Self::Recover { workspace_id, .. }
            | Self::GetViewport { workspace_id, .. }
            | Self::PreviewAutoArrange { workspace_id, .. }
            | Self::Rename { workspace_id, .. }
            | Self::Archive { workspace_id, .. }
            | Self::AddDocumentNode { workspace_id, .. }
            | Self::AddAssetNode { workspace_id, .. }
            | Self::AddTextNode { workspace_id, .. }
            | Self::ConnectEdge { workspace_id, .. }
            | Self::RemoveNode { workspace_id, .. }
            | Self::RemoveEdge { workspace_id, .. }
            | Self::UpdateNodeGeometry { workspace_id, .. }
            | Self::UpdateTextCard { workspace_id, .. }
            | Self::UpdateViewport { workspace_id, .. }
            | Self::AutoArrange { workspace_id, .. } => workspace_id,
        }
    }
    fn operation_id(&self) -> Option<&str> {
        match self {
            Self::Rename { operation_id, .. }
            | Self::Recover { operation_id, .. }
            | Self::Archive { operation_id, .. }
            | Self::AddDocumentNode { operation_id, .. }
            | Self::AddAssetNode { operation_id, .. }
            | Self::AddTextNode { operation_id, .. }
            | Self::ConnectEdge { operation_id, .. }
            | Self::RemoveNode { operation_id, .. }
            | Self::RemoveEdge { operation_id, .. }
            | Self::UpdateNodeGeometry { operation_id, .. }
            | Self::UpdateTextCard { operation_id, .. }
            | Self::UpdateViewport { operation_id, .. }
            | Self::AutoArrange { operation_id, .. } => Some(operation_id),
            Self::Create { .. }
            | Self::Get { .. }
            | Self::GetViewport { .. }
            | Self::PreviewAutoArrange { .. } => None,
        }
    }

    fn write_canvas_id(&self) -> Option<&str> {
        match self {
            Self::Create { canvas_id, .. }
            | Self::Recover { canvas_id, .. }
            | Self::Rename { canvas_id, .. }
            | Self::Archive { canvas_id, .. }
            | Self::AddDocumentNode { canvas_id, .. }
            | Self::AddAssetNode { canvas_id, .. }
            | Self::AddTextNode { canvas_id, .. }
            | Self::ConnectEdge { canvas_id, .. }
            | Self::RemoveNode { canvas_id, .. }
            | Self::RemoveEdge { canvas_id, .. }
            | Self::UpdateNodeGeometry { canvas_id, .. }
            | Self::UpdateTextCard { canvas_id, .. }
            | Self::UpdateViewport { canvas_id, .. }
            | Self::AutoArrange { canvas_id, .. } => Some(canvas_id),
            Self::Get { .. } | Self::GetViewport { .. } | Self::PreviewAutoArrange { .. } => None,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCanvasResponse {
    pub ok: bool,
    pub data: Option<DesktopCanvasDataDto>,
    pub error_code: Option<String>,
    pub retryable: bool,
    pub recovery_required: bool,
    pub operation_id: Option<String>,
}
impl DesktopCanvasResponse {
    fn success(
        record: cabinet_ports::canvas_repository::CanvasRecord,
        operation_id: Option<String>,
        presentations: &[CanvasTargetPresentation],
    ) -> Self {
        Self {
            ok: true,
            data: Some(DesktopCanvasDataDto::from_record(record, presentations)),
            error_code: None,
            retryable: false,
            recovery_required: false,
            operation_id,
        }
    }
    fn success_viewport(
        page: cabinet_ports::canvas_viewport_query::CanvasViewportPage,
        presentations: &[CanvasTargetPresentation],
    ) -> Self {
        Self {
            ok: true,
            data: Some(DesktopCanvasDataDto::from_viewport(page, presentations)),
            error_code: None,
            retryable: false,
            recovery_required: false,
            operation_id: None,
        }
    }
    fn failure(error: CanvasRuntimeError) -> Self {
        let code = error.code();
        Self {
            ok: false,
            data: None,
            error_code: Some(code.into()),
            retryable: matches!(
                error,
                CanvasRuntimeError::Lifecycle(CanvasLifecycleUsecaseError::StorageUnavailable)
                    | CanvasRuntimeError::Mutation(CanvasMutationError::StorageUnavailable)
                    | CanvasRuntimeError::Recovery(CanvasRecoveryError::StorageUnavailable)
                    | CanvasRuntimeError::Query(GetCanvasViewportError::StorageUnavailable)
                    | CanvasRuntimeError::TargetPresentation(
                        ResolveCanvasTargetPresentationsError::StorageUnavailable
                    )
                    | CanvasRuntimeError::GraphProjection(
                        CanvasGraphRelationProjectionError::StorageUnavailable
                    )
            ),
            recovery_required: matches!(error, CanvasRuntimeError::Recovery(_))
                || matches!(error, CanvasRuntimeError::GraphProjection(_))
                || matches!(
                    code,
                    "CANVAS_RECOVERY_REQUIRED" | "CANVAS_TARGET_RECOVERY_REQUIRED"
                ),
            operation_id: None,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCanvasDataDto {
    pub canvas_id: String,
    pub title: String,
    pub revision: u64,
    pub lifecycle: String,
    pub viewport: DesktopCanvasViewportDto,
    pub nodes: Vec<DesktopCanvasNodeDto>,
    pub edges: Vec<DesktopCanvasEdgeDto>,
    pub total_node_count: usize,
    pub total_edge_count: usize,
    pub matching_node_count: usize,
    pub matching_edge_count: usize,
    pub truncated: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCanvasViewportDto {
    pub center_x: i32,
    pub center_y: i32,
    pub zoom_percent: u16,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCanvasNodeDto {
    pub node_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub display_label: String,
    pub target_status: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCanvasEdgeDto {
    pub edge_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
}
impl DesktopCanvasDataDto {
    fn from_record(
        record: cabinet_ports::canvas_repository::CanvasRecord,
        presentations: &[CanvasTargetPresentation],
    ) -> Self {
        let v = record.viewport();
        let total_node_count = record.canvas().nodes().len();
        let total_edge_count = record.canvas().edges().len();
        Self {
            canvas_id: record.canvas().id().as_str().into(),
            title: record.title().as_str().into(),
            revision: record.revision().value(),
            lifecycle: format!("{:?}", record.canvas().state()).to_lowercase(),
            viewport: DesktopCanvasViewportDto {
                center_x: v.center_x(),
                center_y: v.center_y(),
                zoom_percent: v.zoom_percent(),
            },
            nodes: record
                .canvas()
                .nodes()
                .iter()
                .map(|n| {
                    let (kind, target) = match n.target() {
                        CanvasNodeTarget::Document(v) => ("document", v.as_str()),
                        CanvasNodeTarget::Attachment(v) => ("attachment", v.as_str()),
                        CanvasNodeTarget::ExternalLink(v) => ("external", v.as_str()),
                        CanvasNodeTarget::TextCard(v) => ("text", v.as_str()),
                    };
                    let g = n.geometry();
                    let presentation = presentation_for(presentations, n.id().as_str());
                    DesktopCanvasNodeDto {
                        node_id: n.id().as_str().into(),
                        target_kind: kind.into(),
                        target_id: target.into(),
                        display_label: presentation.display_label().into(),
                        target_status: presentation.status().as_str().into(),
                        x: g.position().x(),
                        y: g.position().y(),
                        width: g.size().width(),
                        height: g.size().height(),
                    }
                })
                .collect(),
            edges: record
                .canvas()
                .edges()
                .iter()
                .map(|e| DesktopCanvasEdgeDto {
                    edge_id: e.id().as_str().into(),
                    source_node_id: e.source_node_id().as_str().into(),
                    target_node_id: e.target_node_id().as_str().into(),
                })
                .collect(),
            total_node_count,
            total_edge_count,
            matching_node_count: total_node_count,
            matching_edge_count: total_edge_count,
            truncated: false,
        }
    }
    fn from_viewport(
        page: cabinet_ports::canvas_viewport_query::CanvasViewportPage,
        presentations: &[CanvasTargetPresentation],
    ) -> Self {
        let cabinet_ports::canvas_viewport_query::CanvasViewportPage {
            canvas_id,
            title,
            revision,
            lifecycle,
            viewport,
            nodes,
            edges,
            total_node_count,
            total_edge_count,
            matching_node_count,
            matching_edge_count,
            truncated,
        } = page;
        Self {
            canvas_id: canvas_id.as_str().into(),
            title: title.as_str().into(),
            revision: revision.value(),
            lifecycle: format!("{lifecycle:?}").to_lowercase(),
            viewport: DesktopCanvasViewportDto {
                center_x: viewport.center_x(),
                center_y: viewport.center_y(),
                zoom_percent: viewport.zoom_percent(),
            },
            nodes: nodes
                .iter()
                .map(|node| DesktopCanvasNodeDto::from_node(node, presentations))
                .collect(),
            edges: edges
                .iter()
                .map(|edge| DesktopCanvasEdgeDto {
                    edge_id: edge.id().as_str().into(),
                    source_node_id: edge.source_node_id().as_str().into(),
                    target_node_id: edge.target_node_id().as_str().into(),
                })
                .collect(),
            total_node_count,
            total_edge_count,
            matching_node_count,
            matching_edge_count,
            truncated,
        }
    }
}
impl DesktopCanvasNodeDto {
    fn from_node(
        node: &cabinet_domain::canvas::CanvasNode,
        presentations: &[CanvasTargetPresentation],
    ) -> Self {
        let (target_kind, target_id) = match node.target() {
            CanvasNodeTarget::Document(value) => ("document", value.as_str()),
            CanvasNodeTarget::Attachment(value) => ("attachment", value.as_str()),
            CanvasNodeTarget::ExternalLink(value) => ("external", value.as_str()),
            CanvasNodeTarget::TextCard(value) => ("text", value.as_str()),
        };
        let geometry = node.geometry();
        let presentation = presentation_for(presentations, node.id().as_str());
        Self {
            node_id: node.id().as_str().into(),
            target_kind: target_kind.into(),
            target_id: target_id.into(),
            display_label: presentation.display_label().into(),
            target_status: presentation.status().as_str().into(),
            x: geometry.position().x(),
            y: geometry.position().y(),
            width: geometry.size().width(),
            height: geometry.size().height(),
        }
    }
}

fn presentation_for<'a>(
    presentations: &'a [CanvasTargetPresentation],
    node_id: &str,
) -> &'a CanvasTargetPresentation {
    presentations
        .iter()
        .find(|presentation| presentation.node_id() == node_id)
        .expect("presentation usecase must return one value per node")
}
struct DesktopCanvasLogSink {
    events: Arc<Mutex<Vec<DesktopCanvasProductEvent>>>,
}
impl DesktopCanvasLogSink {
    fn push(&self, event: DesktopCanvasProductEvent) {
        self.events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(event);
    }

    fn write_failure(&self, canvas_id: &str, error_code: &str) {
        self.push(DesktopCanvasProductEvent::SaveFailed {
            canvas_id: canvas_id.into(),
            error_code: error_code.into(),
        });
    }
}
impl CanvasLifecycleProductLogger for DesktopCanvasLogSink {
    fn write_product(&mut self, event: CanvasLifecycleProductEvent) {
        match event {
            CanvasLifecycleProductEvent::Created {
                canvas_id,
                revision,
            } => {
                self.push(DesktopCanvasProductEvent::Created {
                    canvas_id,
                    revision,
                });
            }
            CanvasLifecycleProductEvent::Archived {
                canvas_id,
                revision,
            } => {
                self.push(DesktopCanvasProductEvent::Archived {
                    canvas_id,
                    revision,
                });
            }
            CanvasLifecycleProductEvent::Renamed { .. } => {}
        }
    }
}
impl CanvasMutationProductLogger for DesktopCanvasLogSink {
    fn write_product(&mut self, _: CanvasMutationProductEvent) {}
}
impl CanvasRecoveryLogger for DesktopCanvasLogSink {
    fn write_product(&mut self, event: CanvasRecoveryEvent) {
        if let Some(revision) = event.revision() {
            self.push(DesktopCanvasProductEvent::Recovered {
                canvas_id: event.canvas_id().into(),
                revision,
            });
        } else if let Some(error_code) = event.error_code() {
            self.push(DesktopCanvasProductEvent::RecoveryFailed {
                canvas_id: event.canvas_id().into(),
                error_code: error_code.into(),
            });
        }
    }
}
#[derive(Debug, Clone, Copy)]
enum CanvasRuntimeError {
    Lifecycle(CanvasLifecycleUsecaseError),
    Mutation(CanvasMutationError),
    Recovery(CanvasRecoveryError),
    Query(GetCanvasViewportError),
    TargetPresentation(ResolveCanvasTargetPresentationsError),
    GraphProjection(CanvasGraphRelationProjectionError),
}
impl CanvasRuntimeError {
    fn code(self) -> &'static str {
        match self {
            Self::Lifecycle(v) => v.code(),
            Self::Mutation(v) => v.code(),
            Self::Recovery(v) => v.code(),
            Self::Query(v) => v.code(),
            Self::TargetPresentation(v) => v.code(),
            Self::GraphProjection(error) => match error {
                CanvasGraphRelationProjectionError::InvalidInput => {
                    "CANVAS_GRAPH_PROJECTION_INVALID"
                }
                CanvasGraphRelationProjectionError::RelationLimitExceeded => {
                    "CANVAS_GRAPH_PROJECTION_LIMIT_EXCEEDED"
                }
                CanvasGraphRelationProjectionError::StorageUnavailable => {
                    "CANVAS_GRAPH_PROJECTION_RECOVERY_REQUIRED"
                }
                CanvasGraphRelationProjectionError::CorruptedProjection => {
                    "CANVAS_GRAPH_PROJECTION_CORRUPTED"
                }
            },
        }
    }
}

impl DesktopProjectionRepairOperationRuntime {
    pub fn new(app_data_root: PathBuf) -> Self {
        let prefix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self {
            repository: Mutex::new(DurableProjectionRepairRepository::new(app_data_root)),
            ids: Mutex::new(DesktopRepairIdSource { prefix, next: 0 }),
        }
    }

    pub fn start(
        &self,
        workspace_id: &str,
        document_id: &str,
    ) -> DesktopProjectionRepairOperationResponse {
        let (Ok(mut repository), Ok(mut ids)) = (self.repository.lock(), self.ids.lock()) else {
            return DesktopProjectionRepairOperationResponse::failure(
                "projection_repair.runtime_unavailable",
                true,
            );
        };
        match StartProjectionRepairUsecase::new().execute(
            StartProjectionRepairInput::new(workspace_id, document_id),
            &mut *ids,
            &mut *repository,
        ) {
            Ok(output) => DesktopProjectionRepairOperationResponse::operation(output.operation()),
            Err(error) => operation_failure(error),
        }
    }

    pub fn status(
        &self,
        workspace_id: &str,
        operation_id: &str,
    ) -> DesktopProjectionRepairOperationResponse {
        let Ok(repository) = self.repository.lock() else {
            return DesktopProjectionRepairOperationResponse::failure(
                "projection_repair.runtime_unavailable",
                true,
            );
        };
        match GetProjectionRepairStatusUsecase::new().execute(
            GetProjectionRepairStatusInput::new(workspace_id, operation_id),
            &*repository,
        ) {
            Ok(output) => DesktopProjectionRepairOperationResponse {
                ok: true,
                operation_id: Some(operation_id.into()),
                state: Some(repair_state_name(output.state()).into()),
                attempt: output.attempt(),
                completed_units: output.completed_units(),
                total_units: output.total_units(),
                error_code: None,
                retryable: false,
            },
            Err(error) => operation_failure(error),
        }
    }

    pub fn cancel(
        &self,
        workspace_id: &str,
        operation_id: &str,
    ) -> DesktopProjectionRepairOperationResponse {
        let Ok(mut repository) = self.repository.lock() else {
            return DesktopProjectionRepairOperationResponse::failure(
                "projection_repair.runtime_unavailable",
                true,
            );
        };
        match CancelProjectionRepairUsecase::new().execute(
            CancelProjectionRepairInput::new(workspace_id, operation_id),
            &mut *repository,
        ) {
            Ok(output) => DesktopProjectionRepairOperationResponse::operation(output.operation()),
            Err(error) => operation_failure(error),
        }
    }

    pub fn retry(
        &self,
        workspace_id: &str,
        operation_id: &str,
    ) -> DesktopProjectionRepairOperationResponse {
        let Ok(mut repository) = self.repository.lock() else {
            return DesktopProjectionRepairOperationResponse::failure(
                "projection_repair.runtime_unavailable",
                true,
            );
        };
        match RetryProjectionRepairUsecase::new().execute(
            RetryProjectionRepairInput::new(workspace_id, operation_id),
            &mut *repository,
        ) {
            Ok(output) => DesktopProjectionRepairOperationResponse::operation(output.operation()),
            Err(error) => operation_failure(error),
        }
    }

    pub fn run(
        &self,
        workspace_id: &str,
        operation_id: &str,
        projection: &DesktopProjectionRuntime,
    ) -> DesktopProjectionRepairOperationResponse {
        let running =
            match self.transition(workspace_id, operation_id, ProjectionRepairEvent::Start) {
                Ok(value) => value,
                Err(response) => return response,
            };
        let pending_result = projection.run_once();
        let pending_freshness =
            projection.get_freshness(workspace_id, running.document_id().as_str());
        if pending_result.ok
            && pending_freshness.ok
            && pending_freshness.state.as_deref() == Some("ready")
        {
            return self
                .transition(workspace_id, operation_id, ProjectionRepairEvent::Succeeded)
                .map(|operation| DesktopProjectionRepairOperationResponse::operation(&operation))
                .unwrap_or_else(|response| response);
        }
        let mut reindex = projection.request_reindex(workspace_id, running.document_id().as_str());
        for _ in 1..3 {
            if reindex.ok || !reindex.retryable {
                break;
            }
            thread::sleep(Duration::from_millis(10));
            reindex = projection.request_reindex(workspace_id, running.document_id().as_str());
        }
        if !reindex.ok {
            let freshness = projection.get_freshness(workspace_id, running.document_id().as_str());
            if reindex.retryable && freshness.ok && freshness.state.as_deref() == Some("ready") {
                return self
                    .transition(workspace_id, operation_id, ProjectionRepairEvent::Succeeded)
                    .map(|operation| {
                        DesktopProjectionRepairOperationResponse::operation(&operation)
                    })
                    .unwrap_or_else(|response| response);
            }
            #[cfg(debug_assertions)]
            eprintln!(
                "DEV projection.repair.reindex_failed error_code={} retryable={} freshness_state={}",
                reindex.error_code.as_deref().unwrap_or("unknown"),
                reindex.retryable,
                freshness.state.as_deref().unwrap_or("unavailable")
            );
            return self.finish_failure(workspace_id, operation_id, reindex.retryable);
        }
        if let Err(response) = self.transition(
            workspace_id,
            operation_id,
            ProjectionRepairEvent::PublishStarted,
        ) {
            return response;
        }
        let result = projection.run_once();
        let target_freshness =
            projection.get_freshness(workspace_id, running.document_id().as_str());
        if result.ok && target_freshness.ok && target_freshness.state.as_deref() == Some("ready") {
            self.transition(workspace_id, operation_id, ProjectionRepairEvent::Succeeded)
                .map(|operation| DesktopProjectionRepairOperationResponse::operation(&operation))
                .unwrap_or_else(|response| response)
        } else {
            self.finish_failure(
                workspace_id,
                operation_id,
                result.retryable || target_freshness.retryable,
            )
        }
    }

    fn finish_failure(
        &self,
        workspace_id: &str,
        operation_id: &str,
        retryable: bool,
    ) -> DesktopProjectionRepairOperationResponse {
        let event = if retryable {
            ProjectionRepairEvent::FailedRetryable
        } else {
            ProjectionRepairEvent::FailedFatal
        };
        self.transition(workspace_id, operation_id, event)
            .map(|operation| DesktopProjectionRepairOperationResponse::operation(&operation))
            .unwrap_or_else(|response| response)
    }

    fn transition(
        &self,
        workspace_id: &str,
        operation_id: &str,
        event: ProjectionRepairEvent,
    ) -> Result<
        cabinet_domain::projection_repair::ProjectionRepairOperation,
        DesktopProjectionRepairOperationResponse,
    > {
        let workspace = WorkspaceId::new(workspace_id).map_err(|_| {
            DesktopProjectionRepairOperationResponse::failure(
                "projection_repair.invalid_input",
                false,
            )
        })?;
        let id = ProjectionRepairOperationId::new(operation_id).map_err(|_| {
            DesktopProjectionRepairOperationResponse::failure(
                "projection_repair.invalid_input",
                false,
            )
        })?;
        let mut repository = self.repository.lock().map_err(|_| {
            DesktopProjectionRepairOperationResponse::failure(
                "projection_repair.runtime_unavailable",
                true,
            )
        })?;
        let current = repository
            .get(&id)
            .map_err(|error| DesktopProjectionRepairOperationResponse::failure(error.code(), true))?
            .ok_or_else(|| {
                DesktopProjectionRepairOperationResponse::failure(
                    "projection_repair.not_found",
                    false,
                )
            })?;
        if current.workspace_id() != &workspace {
            return Err(DesktopProjectionRepairOperationResponse::failure(
                "projection_repair.not_found",
                false,
            ));
        }
        let expected = current.state();
        let next = current
            .transition(event)
            .map_err(|error| {
                DesktopProjectionRepairOperationResponse::failure(error.code(), false)
            })?
            .into_operation();
        repository.replace(next.clone(), expected).map_err(|error| DesktopProjectionRepairOperationResponse::failure(error.code(), matches!(error, cabinet_ports::projection_repair::ProjectionRepairRepositoryError::Conflict | cabinet_ports::projection_repair::ProjectionRepairRepositoryError::StorageUnavailable)))?;
        Ok(next)
    }
}

struct DesktopRepairIdSource {
    prefix: u128,
    next: u64,
}
impl ProjectionRepairOperationIdGenerator for DesktopRepairIdSource {
    fn next_id(&mut self) -> Result<String, ()> {
        self.next = self.next.checked_add(1).ok_or(())?;
        Ok(format!("repair-{}-{}", self.prefix, self.next))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopProjectionRepairOperationResponse {
    pub ok: bool,
    pub operation_id: Option<String>,
    pub state: Option<String>,
    pub attempt: u32,
    pub completed_units: u8,
    pub total_units: u8,
    pub error_code: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopProjectionRepairStartRequestDto {
    pub workspace_id: String,
    pub document_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopProjectionRepairOperationRequestDto {
    pub workspace_id: String,
    pub operation_id: String,
}
impl DesktopProjectionRepairOperationResponse {
    fn operation(operation: &cabinet_domain::projection_repair::ProjectionRepairOperation) -> Self {
        let progress = operation.progress();
        Self {
            ok: true,
            operation_id: Some(operation.operation_id().as_str().into()),
            state: Some(repair_state_name(operation.state()).into()),
            attempt: operation.attempt(),
            completed_units: progress.completed_units(),
            total_units: progress.total_units(),
            error_code: None,
            retryable: false,
        }
    }
    fn failure(code: &str, retryable: bool) -> Self {
        Self {
            ok: false,
            operation_id: None,
            state: None,
            attempt: 0,
            completed_units: 0,
            total_units: 0,
            error_code: Some(code.into()),
            retryable,
        }
    }
}
fn operation_failure(
    error: ProjectionRepairUsecaseError,
) -> DesktopProjectionRepairOperationResponse {
    DesktopProjectionRepairOperationResponse::failure(error.code(), error.retryable())
}
const fn repair_state_name(
    state: cabinet_domain::projection_repair::ProjectionRepairState,
) -> &'static str {
    use cabinet_domain::projection_repair::ProjectionRepairState::*;
    match state {
        Queued => "queued",
        Running => "running",
        Publishing => "publishing",
        CancelPending => "cancel_pending",
        Succeeded => "succeeded",
        FailedRetryable => "failed_retryable",
        FailedFatal => "failed_fatal",
        Cancelled => "cancelled",
    }
}

pub struct DesktopProjectionRuntime {
    policy: ProjectionWorkerPolicy,
    state: Mutex<DesktopProjectionState>,
}

struct DesktopProjectionState {
    work: DurableProjectionWorkRepository,
    versions: LocalVersionStore,
    documents: LocalDocumentRepository,
    pointer: LocalCurrentDocumentVersionPointer,
    current_documents: LocalCurrentDocumentProjectionCatalog,
    catalog: DurableDocumentLinkCatalog,
    associations: DurableAssetAssociationCatalog,
    search: DurableLocalSearchIndex,
    links: DurableLocalLinkIndex,
    graphs: DurableLocalGraphProjectionStore,
    parser: LocalMarkdownParser,
}

impl DesktopProjectionRuntime {
    pub fn new(
        app_data_root: PathBuf,
        max_body_bytes: usize,
        batch_limit: usize,
        max_attempts: u32,
    ) -> Result<Self, &'static str> {
        let body_policy = DocumentBodyPolicy::new(max_body_bytes)
            .map_err(|_| "PROJECTION_INVALID_BODY_POLICY")?;
        let policy = ProjectionWorkerPolicy::new(batch_limit, max_attempts)
            .map_err(|_| "PROJECTION_INVALID_WORKER_POLICY")?;
        let uses_authoritative_revision_store = app_data_root
            .join(LOCAL_DOCUMENT_VERSION_ROOT)
            .try_exists()
            .map_err(|_| "PROJECTION_REVISION_STORE_UNAVAILABLE")?;
        let version_root = if uses_authoritative_revision_store {
            LOCAL_DOCUMENT_VERSION_ROOT
        } else {
            "authoring-versions"
        };
        let pointer_root = if uses_authoritative_revision_store {
            LOCAL_DOCUMENT_POINTER_ROOT
        } else {
            "authoring-current-version"
        };
        let versions =
            LocalVersionStore::with_body_policy(app_data_root.join(version_root), body_policy);
        versions
            .migrate_revision_numbers()
            .map_err(|_| "PROJECTION_REVISION_MIGRATION_FAILED")?;
        Ok(Self {
            policy,
            state: Mutex::new(DesktopProjectionState {
                work: DurableProjectionWorkRepository::new(app_data_root.clone()),
                versions,
                documents: LocalDocumentRepository::with_body_policy(
                    app_data_root.join("authoring-current"),
                    body_policy,
                ),
                pointer: LocalCurrentDocumentVersionPointer::new(app_data_root.join(pointer_root)),
                current_documents: LocalCurrentDocumentProjectionCatalog::new(
                    app_data_root.clone(),
                ),
                catalog: DurableDocumentLinkCatalog::new(app_data_root.clone()),
                associations: DurableAssetAssociationCatalog::new(app_data_root.clone()),
                search: DurableLocalSearchIndex::new(app_data_root.clone(), body_policy),
                links: DurableLocalLinkIndex::new(app_data_root.clone()),
                graphs: DurableLocalGraphProjectionStore::new(app_data_root),
                parser: LocalMarkdownParser::new(),
            }),
        })
    }

    pub fn run_once(&self) -> DesktopProjectionRunResponse {
        let Ok(mut state) = self.state.lock() else {
            return DesktopProjectionRunResponse::failure("PROJECTION_RUNTIME_UNAVAILABLE", true);
        };
        let DesktopProjectionState {
            work,
            versions,
            documents,
            pointer,
            current_documents: _,
            catalog,
            associations,
            search,
            links,
            graphs,
            parser,
        } = &mut *state;
        let mut search_writer = SearchProjectionWriter::new(pointer, documents, search);
        let mut relation_writer = ResolvedLinkGraphProjectionWriter::new(
            pointer,
            catalog,
            associations,
            AssetGraphProjectionPolicy::new(500).expect("validated desktop asset graph policy"),
            links,
            graphs,
        );
        let mut router = ProjectionKindWriterRouter::new(&mut search_writer, &mut relation_writer);
        let mut processor =
            VersionedMarkdownProjectionProcessor::new(versions, parser, &mut router);
        match RunProjectionWorkerUsecase::new(self.policy).execute(work, &mut processor) {
            Ok(output) => DesktopProjectionRunResponse::success(
                output.ready_count(),
                output.retry_scheduled_count(),
                output.failed_count(),
            ),
            Err(error) => DesktopProjectionRunResponse::failure(
                projection_worker_error_code(error),
                matches!(error, ProjectionWorkerError::RepositoryFailure),
            ),
        }
    }

    pub fn get_freshness(
        &self,
        workspace_id: &str,
        document_id: &str,
    ) -> DesktopProjectionFreshnessResponse {
        let Ok(state) = self.state.lock() else {
            return DesktopProjectionFreshnessResponse::failure(
                "projection_freshness.runtime_unavailable",
                true,
            );
        };
        match GetCurrentProjectionFreshnessUsecase::new().execute(
            GetCurrentProjectionFreshnessInput::new(workspace_id, document_id),
            &state.pointer,
            &state.work,
        ) {
            Ok(output) => DesktopProjectionFreshnessResponse {
                ok: true,
                state: Some(freshness_name(output.aggregate_state()).to_string()),
                current_version_id: Some(output.current_version_id().as_str().to_string()),
                projections: output
                    .projections()
                    .iter()
                    .map(|item| DesktopProjectionKindFreshnessDto {
                        kind: item.kind().as_str().to_string(),
                        state: freshness_name(item.state()).to_string(),
                    })
                    .collect(),
                error_code: None,
                retryable: false,
            },
            Err(error) => {
                DesktopProjectionFreshnessResponse::failure(error.code(), error.retryable())
            }
        }
    }

    pub fn request_reindex(
        &self,
        workspace_id: &str,
        document_id: &str,
    ) -> DesktopProjectionReindexResponse {
        let Ok(mut state) = self.state.lock() else {
            return DesktopProjectionReindexResponse::failure(
                "projection_reindex.runtime_unavailable",
                true,
            );
        };
        let DesktopProjectionState { pointer, work, .. } = &mut *state;
        match ReindexCurrentProjectionUsecase::new().execute(
            ReindexCurrentProjectionInput::new(workspace_id, document_id),
            pointer,
            work,
        ) {
            Ok(output) => DesktopProjectionReindexResponse {
                ok: true,
                enqueued_count: output.enqueued_count(),
                reset_count: output.reset_count(),
                already_active_count: output.already_active_count(),
                error_code: None,
                retryable: false,
            },
            Err(error) => {
                DesktopProjectionReindexResponse::failure(error.code(), error.retryable())
            }
        }
    }

    pub fn reconcile_current(
        &self,
        workspace_id: &str,
        document_limit: usize,
    ) -> DesktopProjectionReconcileResponse {
        let Ok(mut state) = self.state.lock() else {
            return DesktopProjectionReconcileResponse::failure(
                "projection_reconcile.runtime_unavailable",
                true,
            );
        };
        let DesktopProjectionState {
            current_documents,
            pointer,
            work,
            ..
        } = &mut *state;
        match ReconcileCurrentProjectionsUsecase::new().execute(
            ReconcileCurrentProjectionsInput::new(workspace_id, document_limit),
            current_documents,
            pointer,
            work,
        ) {
            Ok(output) => DesktopProjectionReconcileResponse {
                ok: true,
                document_count: output.document_count(),
                ready_document_count: output.ready_document_count(),
                enqueued_count: output.enqueued_count(),
                reset_count: output.reset_count(),
                already_active_count: output.already_active_count(),
                error_code: None,
                retryable: false,
            },
            Err(error) => {
                DesktopProjectionReconcileResponse::failure(error.code(), error.retryable())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopProjectionKindFreshnessDto {
    pub kind: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopProjectionIdentityRequestDto {
    pub workspace_id: String,
    pub document_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopProjectionFreshnessResponse {
    pub ok: bool,
    pub state: Option<String>,
    pub current_version_id: Option<String>,
    pub projections: Vec<DesktopProjectionKindFreshnessDto>,
    pub error_code: Option<String>,
    pub retryable: bool,
}
impl DesktopProjectionFreshnessResponse {
    fn failure(code: &'static str, retryable: bool) -> Self {
        Self {
            ok: false,
            state: None,
            current_version_id: None,
            projections: Vec::new(),
            error_code: Some(code.to_string()),
            retryable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopProjectionReindexResponse {
    pub ok: bool,
    pub enqueued_count: usize,
    pub reset_count: usize,
    pub already_active_count: usize,
    pub error_code: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopProjectionReconcileResponse {
    pub ok: bool,
    pub document_count: usize,
    pub ready_document_count: usize,
    pub enqueued_count: usize,
    pub reset_count: usize,
    pub already_active_count: usize,
    pub error_code: Option<String>,
    pub retryable: bool,
}

impl DesktopProjectionReconcileResponse {
    fn failure(code: &'static str, retryable: bool) -> Self {
        Self {
            ok: false,
            document_count: 0,
            ready_document_count: 0,
            enqueued_count: 0,
            reset_count: 0,
            already_active_count: 0,
            error_code: Some(code.to_string()),
            retryable,
        }
    }
}
impl DesktopProjectionReindexResponse {
    fn failure(code: &'static str, retryable: bool) -> Self {
        Self {
            ok: false,
            enqueued_count: 0,
            reset_count: 0,
            already_active_count: 0,
            error_code: Some(code.to_string()),
            retryable,
        }
    }
}

const fn freshness_name(state: ProjectionFreshnessState) -> &'static str {
    match state {
        ProjectionFreshnessState::Ready => "ready",
        ProjectionFreshnessState::Stale => "stale",
        ProjectionFreshnessState::Repairing => "repairing",
        ProjectionFreshnessState::Failed => "failed",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopProjectionRunResponse {
    pub ok: bool,
    pub ready_count: usize,
    pub retry_scheduled_count: usize,
    pub failed_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
}

impl DesktopProjectionRunResponse {
    fn success(ready_count: usize, retry_scheduled_count: usize, failed_count: usize) -> Self {
        Self {
            ok: true,
            ready_count,
            retry_scheduled_count,
            failed_count,
            error_code: None,
            retryable: false,
        }
    }

    fn failure(error_code: &'static str, retryable: bool) -> Self {
        Self {
            ok: false,
            ready_count: 0,
            retry_scheduled_count: 0,
            failed_count: 0,
            error_code: Some(error_code.to_string()),
            retryable,
        }
    }
}

const fn projection_worker_error_code(error: ProjectionWorkerError) -> &'static str {
    match error {
        ProjectionWorkerError::InvalidPolicy => "PROJECTION_INVALID_WORKER_POLICY",
        ProjectionWorkerError::RepositoryFailure => "PROJECTION_REPOSITORY_UNAVAILABLE",
        ProjectionWorkerError::InvalidTransition => "PROJECTION_INVALID_TRANSITION",
    }
}

pub struct DesktopDocumentMutationRuntime {
    state: Mutex<DesktopDocumentMutationState>,
}

struct DesktopDocumentMutationState {
    create: LocalCreateDocumentRevisionRuntime,
    update: LocalUpdateDocumentRevisionRuntime,
    events: DesktopDocumentChangeSink,
}

impl DesktopDocumentMutationRuntime {
    pub fn new(app_data_root: PathBuf, max_body_bytes: usize) -> Result<Self, &'static str> {
        let body_policy = DocumentBodyPolicy::new(max_body_bytes)
            .map_err(|_| "DOCUMENT_REVISION_INVALID_BODY_POLICY")?;
        Ok(Self {
            state: Mutex::new(DesktopDocumentMutationState {
                create: LocalCreateDocumentRevisionRuntime::new(app_data_root.clone(), body_policy),
                update: LocalUpdateDocumentRevisionRuntime::new(app_data_root.clone(), body_policy),
                events: DesktopDocumentChangeSink::with_authoritative_body_policy(
                    app_data_root,
                    body_policy,
                ),
            }),
        })
    }

    pub fn execute(
        &self,
        request: DesktopDocumentMutationRequestDto,
    ) -> DesktopDocumentAuthoringCommandResponse {
        let Ok(mut state) = self.state.lock() else {
            return DesktopDocumentAuthoringCommandResponse::runtime_unavailable();
        };
        match request {
            DesktopDocumentMutationRequestDto::Create {
                operation_id,
                workspace_id,
                document_id,
                body,
                author,
                summary,
            } => match state.create.execute(CreateDocumentRevisionInput::new(
                &operation_id,
                &workspace_id,
                &document_id,
                &body,
                &author,
                &summary,
            )) {
                Ok(output) => {
                    if state
                        .events
                        .publish_authoritative_created(
                            &workspace_id,
                            &document_id,
                            output.version_id().as_str(),
                        )
                        .is_err()
                    {
                        return DesktopDocumentAuthoringCommandResponse::revision_failure(
                            "DOCUMENT_REVISION_RECOVERY_REQUIRED",
                            true,
                            true,
                        );
                    }
                    DesktopDocumentAuthoringCommandResponse::success_data(
                        DesktopDocumentAuthoringDataDto::without_content(
                            "created",
                            document_id,
                            output.version_id().as_str().to_string(),
                        ),
                    )
                }
                Err(error) => map_create_revision_error(error),
            },
            DesktopDocumentMutationRequestDto::Update {
                operation_id,
                workspace_id,
                document_id,
                expected_current_version_id,
                body,
                author,
                summary,
            } => match state.update.execute(UpdateDocumentRevisionInput::new(
                &operation_id,
                &workspace_id,
                &document_id,
                &expected_current_version_id,
                &body,
                &author,
                &summary,
            )) {
                Ok(output) => {
                    if state
                        .events
                        .publish_authoritative_updated(
                            &workspace_id,
                            &document_id,
                            output.version_id().as_str(),
                        )
                        .is_err()
                    {
                        return DesktopDocumentAuthoringCommandResponse::revision_failure(
                            "DOCUMENT_REVISION_RECOVERY_REQUIRED",
                            true,
                            true,
                        );
                    }
                    DesktopDocumentAuthoringCommandResponse::success_data(
                        DesktopDocumentAuthoringDataDto::without_content(
                            "updated",
                            document_id,
                            output.version_id().as_str().to_string(),
                        ),
                    )
                }
                Err(error) => map_update_revision_error(error),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum DesktopDocumentMutationRequestDto {
    Create {
        operation_id: String,
        workspace_id: String,
        document_id: String,
        body: String,
        author: String,
        summary: String,
    },
    Update {
        operation_id: String,
        workspace_id: String,
        document_id: String,
        expected_current_version_id: String,
        body: String,
        author: String,
        summary: String,
    },
}

fn map_create_revision_error(
    error: CreateDocumentRevisionError,
) -> DesktopDocumentAuthoringCommandResponse {
    match error {
        CreateDocumentRevisionError::InvalidInput => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_REVISION_INVALID_INPUT",
                false,
                false,
            )
        }
        CreateDocumentRevisionError::OperationConflict
        | CreateDocumentRevisionError::CommitConflict => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_REVISION_CONFLICT",
                false,
                false,
            )
        }
        CreateDocumentRevisionError::RecoveryRequired => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_REVISION_RECOVERY_REQUIRED",
                true,
                true,
            )
        }
        CreateDocumentRevisionError::FingerprintUnavailable
        | CreateDocumentRevisionError::MetadataUnavailable
        | CreateDocumentRevisionError::JournalUnavailable
        | CreateDocumentRevisionError::CommitUnavailable => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_REVISION_STORAGE_UNAVAILABLE",
                true,
                false,
            )
        }
    }
}

fn map_update_revision_error(
    error: UpdateDocumentRevisionError,
) -> DesktopDocumentAuthoringCommandResponse {
    match error {
        UpdateDocumentRevisionError::InvalidInput => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_REVISION_INVALID_INPUT",
                false,
                false,
            )
        }
        UpdateDocumentRevisionError::NotFound => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_REVISION_NOT_FOUND",
                false,
                false,
            )
        }
        UpdateDocumentRevisionError::OperationConflict
        | UpdateDocumentRevisionError::CommitConflict => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_REVISION_CONFLICT",
                false,
                false,
            )
        }
        UpdateDocumentRevisionError::RecoveryRequired => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_REVISION_RECOVERY_REQUIRED",
                true,
                true,
            )
        }
        UpdateDocumentRevisionError::StorageUnavailable
        | UpdateDocumentRevisionError::FingerprintUnavailable
        | UpdateDocumentRevisionError::MetadataUnavailable
        | UpdateDocumentRevisionError::JournalUnavailable
        | UpdateDocumentRevisionError::CommitUnavailable => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_REVISION_STORAGE_UNAVAILABLE",
                true,
                false,
            )
        }
    }
}

pub struct DesktopDocumentQueryRuntime {
    versions: LocalVersionStore,
    pointer: LocalCurrentDocumentVersionPointer,
}

impl DesktopDocumentQueryRuntime {
    pub fn new(app_data_root: PathBuf, max_body_bytes: usize) -> Result<Self, &'static str> {
        let body_policy = DocumentBodyPolicy::new(max_body_bytes)
            .map_err(|_| "DOCUMENT_QUERY_INVALID_BODY_POLICY")?;
        Ok(Self {
            versions: LocalVersionStore::with_body_policy(
                app_data_root.join(LOCAL_DOCUMENT_VERSION_ROOT),
                body_policy,
            ),
            pointer: LocalCurrentDocumentVersionPointer::new(
                app_data_root.join(LOCAL_DOCUMENT_POINTER_ROOT),
            ),
        })
    }

    pub fn execute(&self, request: DesktopDocumentQueryRequestDto) -> DesktopDocumentQueryResponse {
        match request {
            DesktopDocumentQueryRequestDto::Current {
                workspace_id,
                document_id,
            } => self.read_revision(
                "current",
                GetAuthoritativeDocumentRevisionInput::current(&workspace_id, &document_id),
            ),
            DesktopDocumentQueryRequestDto::Version {
                workspace_id,
                document_id,
                version_token,
            } => self.read_revision(
                "version",
                GetAuthoritativeDocumentRevisionInput::version(
                    &workspace_id,
                    &document_id,
                    &version_token,
                ),
            ),
            DesktopDocumentQueryRequestDto::History {
                workspace_id,
                document_id,
                cursor,
                limit,
            } => match GetDocumentHistoryUsecase::new().execute(
                GetDocumentHistoryInput::new(
                    &workspace_id,
                    &document_id,
                    cursor.as_deref(),
                    usize::from(limit),
                ),
                &self.versions,
            ) {
                Ok(output) => match DesktopDocumentQueryDataDto::history(output.page()) {
                    Ok(data) => DesktopDocumentQueryResponse::success(data),
                    Err(()) => DesktopDocumentQueryResponse::failure(
                        "DOCUMENT_QUERY_CORRUPTED_DATA",
                        false,
                        true,
                    ),
                },
                Err(error) => match error {
                    cabinet_usecases::document::GetDocumentHistoryError::InvalidInput => {
                        DesktopDocumentQueryResponse::failure(
                            "DOCUMENT_QUERY_INVALID_INPUT",
                            false,
                            false,
                        )
                    }
                    cabinet_usecases::document::GetDocumentHistoryError::StorageUnavailable => {
                        DesktopDocumentQueryResponse::failure(
                            "DOCUMENT_QUERY_STORAGE_UNAVAILABLE",
                            true,
                            false,
                        )
                    }
                },
            },
        }
    }

    fn read_revision(
        &self,
        kind: &'static str,
        input: GetAuthoritativeDocumentRevisionInput,
    ) -> DesktopDocumentQueryResponse {
        match GetAuthoritativeDocumentRevisionUsecase::new().execute(
            input,
            &self.pointer,
            &self.versions,
        ) {
            Ok(output) => match DesktopDocumentQueryDataDto::revision(kind, output.record()) {
                Ok(data) => DesktopDocumentQueryResponse::success(data),
                Err(()) => DesktopDocumentQueryResponse::failure(
                    "DOCUMENT_QUERY_CORRUPTED_DATA",
                    false,
                    true,
                ),
            },
            Err(error) => map_authoritative_query_error(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum DesktopDocumentQueryRequestDto {
    Current {
        workspace_id: String,
        document_id: String,
    },
    History {
        workspace_id: String,
        document_id: String,
        cursor: Option<String>,
        limit: u16,
    },
    Version {
        workspace_id: String,
        document_id: String,
        version_token: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentQueryResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DesktopDocumentQueryDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
    pub repair_required: bool,
}

impl DesktopDocumentQueryResponse {
    fn success(data: DesktopDocumentQueryDataDto) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error_code: None,
            retryable: false,
            repair_required: false,
        }
    }

    fn failure(error_code: &str, retryable: bool, repair_required: bool) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(error_code.to_string()),
            retryable,
            repair_required,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentQueryDataDto {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_version_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<DesktopDocumentQueryHistoryEntryDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

impl DesktopDocumentQueryDataDto {
    fn revision(
        kind: &'static str,
        record: &cabinet_ports::version_store::VersionRecord,
    ) -> Result<Self, ()> {
        let revision_number = record.entry().revision_number().ok_or(())?.value();
        let version_token = record.version_id().as_str().to_string();
        let body = record.snapshot().body();
        let title = DocumentTitle::from_markdown_body(body);
        Ok(Self {
            kind: kind.to_string(),
            current_version_token: (kind == "current").then(|| version_token.clone()),
            version_token: (kind == "version").then_some(version_token),
            revision_number: Some(revision_number),
            title: Some(title.as_str().to_string()),
            body: Some(body.as_str().to_string()),
            entries: Vec::new(),
            next_cursor: None,
            has_more: false,
        })
    }

    fn history(page: &HistoryPage) -> Result<Self, ()> {
        let entries = page
            .entries()
            .iter()
            .map(|entry| {
                Ok(DesktopDocumentQueryHistoryEntryDto {
                    revision_number: entry.revision_number().ok_or(())?.value(),
                    version_token: entry.version_id().as_str().to_string(),
                    summary: entry.summary().as_str().to_string(),
                    author: entry.author().as_str().to_string(),
                    created_at_epoch_ms: entry.created_at_epoch_ms(),
                })
            })
            .collect::<Result<Vec<_>, ()>>()?;
        let next_cursor = page.next_cursor().map(|cursor| cursor.as_str().to_string());
        Ok(Self {
            kind: "history".to_string(),
            current_version_token: None,
            version_token: None,
            revision_number: None,
            title: None,
            body: None,
            entries,
            has_more: next_cursor.is_some(),
            next_cursor,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentQueryHistoryEntryDto {
    pub revision_number: u64,
    pub version_token: String,
    pub summary: String,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at_epoch_ms: Option<u64>,
}

fn map_authoritative_query_error(
    error: GetAuthoritativeDocumentRevisionError,
) -> DesktopDocumentQueryResponse {
    match error {
        GetAuthoritativeDocumentRevisionError::InvalidInput => {
            DesktopDocumentQueryResponse::failure("DOCUMENT_QUERY_INVALID_INPUT", false, false)
        }
        GetAuthoritativeDocumentRevisionError::NotFound => {
            DesktopDocumentQueryResponse::failure("DOCUMENT_QUERY_NOT_FOUND", false, false)
        }
        GetAuthoritativeDocumentRevisionError::StorageUnavailable => {
            DesktopDocumentQueryResponse::failure("DOCUMENT_QUERY_STORAGE_UNAVAILABLE", true, false)
        }
        GetAuthoritativeDocumentRevisionError::CorruptedData => {
            DesktopDocumentQueryResponse::failure("DOCUMENT_QUERY_CORRUPTED_DATA", false, true)
        }
    }
}

pub struct DesktopDocumentDiffRuntime {
    versions: LocalVersionStore,
    pointer: LocalCurrentDocumentVersionPointer,
    usecase: CompareAuthoritativeDocumentRevisionsUsecase,
    availability: LocalAssetAvailabilityResolver,
    resolve_availability: ResolveAttachmentDiffAvailabilityUsecase,
}

impl DesktopDocumentDiffRuntime {
    pub fn new(app_data_root: PathBuf, max_body_bytes: usize) -> Result<Self, &'static str> {
        Self::with_policy(
            app_data_root,
            max_body_bytes,
            AuthoritativeDiffPolicy::default(),
        )
    }

    pub fn with_policy(
        app_data_root: PathBuf,
        max_body_bytes: usize,
        policy: AuthoritativeDiffPolicy,
    ) -> Result<Self, &'static str> {
        let body_policy = DocumentBodyPolicy::new(max_body_bytes)
            .map_err(|_| "DOCUMENT_DIFF_INVALID_BODY_POLICY")?;
        Ok(Self {
            versions: LocalVersionStore::with_body_policy(
                app_data_root.join(LOCAL_DOCUMENT_VERSION_ROOT),
                body_policy,
            ),
            pointer: LocalCurrentDocumentVersionPointer::new(
                app_data_root.join(LOCAL_DOCUMENT_POINTER_ROOT),
            ),
            usecase: CompareAuthoritativeDocumentRevisionsUsecase::with_policy(policy),
            availability: LocalAssetAvailabilityResolver::new(app_data_root),
            resolve_availability: ResolveAttachmentDiffAvailabilityUsecase::new(),
        })
    }

    pub fn execute(&self, request: DesktopDocumentDiffRequestDto) -> DesktopDocumentDiffResponse {
        let (workspace_id, input) = match request {
            DesktopDocumentDiffRequestDto::CurrentToVersion {
                workspace_id,
                document_id,
                version_token,
            } => {
                let input = CompareAuthoritativeDocumentRevisionsInput::current_to_version(
                    &workspace_id,
                    &document_id,
                    &version_token,
                );
                (workspace_id, input)
            }
            DesktopDocumentDiffRequestDto::Versions {
                workspace_id,
                document_id,
                left_version_token,
                right_version_token,
            } => {
                let input = CompareAuthoritativeDocumentRevisionsInput::versions(
                    &workspace_id,
                    &document_id,
                    &left_version_token,
                    &right_version_token,
                );
                (workspace_id, input)
            }
        };
        match self.usecase.execute(input, &self.pointer, &self.versions) {
            Ok(output) => {
                let attachments = match self.resolve_availability.execute(
                    ResolveAttachmentDiffAvailabilityInput::new(
                        &workspace_id,
                        output.attachment_diff().clone(),
                    ),
                    &self.availability,
                ) {
                    Ok(attachments) => attachments,
                    Err(error) => return map_attachment_availability_error(error),
                };
                DesktopDocumentDiffResponse::success(DesktopDocumentDiffDataDto::from(
                    output.left_version_id().as_str(),
                    output.right_version_id().as_str(),
                    output.computation(),
                    &attachments,
                ))
            }
            Err(error) => map_authoritative_diff_error(error),
        }
    }
}

const DEFAULT_BACKGROUND_DIFF_MAX_BYTES: usize = 32 * 1024 * 1024;
const DEFAULT_BACKGROUND_DIFF_MAX_LINES: usize = 500_000;
const DEFAULT_BACKGROUND_DIFF_MAX_HUNKS: usize = 50_000;

pub struct DesktopDocumentDiffOperationRuntime {
    registry: ProcessLocalDocumentDiffOperationRegistry,
    ids: Mutex<DesktopDocumentDiffOperationIdSource>,
    pointer: LocalCurrentDocumentVersionPointer,
    versions: LocalVersionStore,
    worker: RunDocumentDiffOperationUsecase,
    availability: LocalAssetAvailabilityResolver,
    resolve_availability: ResolveAttachmentDiffAvailabilityUsecase,
    product_events: Arc<Mutex<Vec<&'static str>>>,
}

impl DesktopDocumentDiffOperationRuntime {
    pub fn new(
        app_data_root: PathBuf,
        max_body_bytes: usize,
        capacity: usize,
    ) -> Result<Self, &'static str> {
        let policy = AuthoritativeDiffPolicy::new(
            3,
            DEFAULT_BACKGROUND_DIFF_MAX_BYTES,
            DEFAULT_BACKGROUND_DIFF_MAX_LINES,
            DEFAULT_BACKGROUND_DIFF_MAX_HUNKS,
        )
        .map_err(|_| "DOCUMENT_DIFF_OPERATION_INVALID_POLICY")?;
        Self::with_policy(app_data_root, max_body_bytes, capacity, policy)
    }

    pub fn with_policy(
        app_data_root: PathBuf,
        max_body_bytes: usize,
        capacity: usize,
        policy: AuthoritativeDiffPolicy,
    ) -> Result<Self, &'static str> {
        let body_policy = DocumentBodyPolicy::new(max_body_bytes)
            .map_err(|_| "DOCUMENT_DIFF_OPERATION_INVALID_BODY_POLICY")?;
        let registry = ProcessLocalDocumentDiffOperationRegistry::new(capacity)
            .map_err(|_| "DOCUMENT_DIFF_OPERATION_INVALID_CAPACITY")?;
        let prefix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Ok(Self {
            registry,
            ids: Mutex::new(DesktopDocumentDiffOperationIdSource { prefix, next: 0 }),
            pointer: LocalCurrentDocumentVersionPointer::new(
                app_data_root.join(LOCAL_DOCUMENT_POINTER_ROOT),
            ),
            versions: LocalVersionStore::with_body_policy(
                app_data_root.join(LOCAL_DOCUMENT_VERSION_ROOT),
                body_policy,
            ),
            worker: RunDocumentDiffOperationUsecase::with_diff_service(
                cabinet_usecases::document_diff::DocumentLineDiffService::with_policy(policy),
            ),
            availability: LocalAssetAvailabilityResolver::new(app_data_root),
            resolve_availability: ResolveAttachmentDiffAvailabilityUsecase::new(),
            product_events: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn start(
        &self,
        request: DesktopDocumentDiffOperationRequestDto,
    ) -> DesktopDocumentDiffOperationResponse {
        let input = match request {
            DesktopDocumentDiffOperationRequestDto::CurrentToVersion {
                workspace_id,
                document_id,
                version_token,
            } => StartDocumentDiffOperationInput::current_to_version(
                &workspace_id,
                &document_id,
                &version_token,
            ),
            DesktopDocumentDiffOperationRequestDto::Versions {
                workspace_id,
                document_id,
                left_version_token,
                right_version_token,
            } => StartDocumentDiffOperationInput::versions(
                &workspace_id,
                &document_id,
                &left_version_token,
                &right_version_token,
            ),
        };
        let Ok(mut ids) = self.ids.lock() else {
            return DesktopDocumentDiffOperationResponse::failure(
                "DOCUMENT_DIFF_OPERATION_RUNTIME_UNAVAILABLE",
                true,
            );
        };
        let mut registry = self.registry.clone();
        let output =
            match StartDocumentDiffOperationUsecase::new().execute(input, &mut *ids, &mut registry)
            {
                Ok(output) => output,
                Err(error) => return map_document_diff_operation_start_error(error),
            };
        drop(ids);
        self.record_product_event(output.product_log_event());

        let operation_token = output.operation_id().as_str().to_string();
        let worker_token = operation_token.clone();
        let worker = self.worker;
        let pointer = self.pointer.clone();
        let versions = self.versions.clone();
        let product_events = Arc::clone(&self.product_events);
        thread::spawn(move || {
            if let Ok(output) = worker.execute(
                RunDocumentDiffOperationInput::new(&worker_token),
                &mut registry,
                &pointer,
                &versions,
            ) {
                if let Some(event) = output.product_log_event() {
                    push_document_diff_product_event(&product_events, event);
                }
            }
        });

        DesktopDocumentDiffOperationResponse::success(DesktopDocumentDiffOperationDataDto {
            operation_token,
            state: document_diff_operation_state_name(output.state()).to_string(),
            diff: None,
            failure_code: None,
        })
    }

    pub fn status(
        &self,
        request: DesktopDocumentDiffOperationTokenRequestDto,
    ) -> DesktopDocumentDiffOperationResponse {
        let registry = self.registry.clone();
        let output = match GetDocumentDiffOperationStatusUsecase::new().execute(
            GetDocumentDiffOperationStatusInput::new(&request.operation_token),
            &registry,
        ) {
            Ok(output) => output,
            Err(error) => return map_document_diff_operation_status_error(error),
        };
        let diff = match output.result() {
            Some(result) => {
                let Some(target) = output.target() else {
                    return DesktopDocumentDiffOperationResponse::failure(
                        "DOCUMENT_DIFF_OPERATION_RESULT_CORRUPTED",
                        false,
                    );
                };
                let attachments = match self.resolve_availability.execute(
                    ResolveAttachmentDiffAvailabilityInput::new(
                        target.workspace_id().as_str(),
                        result.attachment_diff().clone(),
                    ),
                    &self.availability,
                ) {
                    Ok(value) => value,
                    Err(error) => return map_document_diff_operation_attachment_error(error),
                };
                Some(DesktopDocumentDiffDataDto::from(
                    result.left_version_id().as_str(),
                    result.right_version_id().as_str(),
                    result.computation(),
                    &attachments,
                ))
            }
            None => None,
        };
        DesktopDocumentDiffOperationResponse::success(DesktopDocumentDiffOperationDataDto {
            operation_token: output.operation_id().as_str().to_string(),
            state: document_diff_operation_state_name(output.state()).to_string(),
            diff,
            failure_code: output.failure_code().map(str::to_string),
        })
    }

    pub fn cancel(
        &self,
        request: DesktopDocumentDiffOperationTokenRequestDto,
    ) -> DesktopDocumentDiffOperationResponse {
        let mut registry = self.registry.clone();
        match CancelDocumentDiffOperationUsecase::new().execute(
            CancelDocumentDiffOperationInput::new(&request.operation_token),
            &mut registry,
        ) {
            Ok(output) => {
                if let Some(event) = output.product_log_event() {
                    self.record_product_event(event);
                }
                DesktopDocumentDiffOperationResponse::success(DesktopDocumentDiffOperationDataDto {
                    operation_token: output.operation_id().as_str().to_string(),
                    state: document_diff_operation_state_name(output.state()).to_string(),
                    diff: None,
                    failure_code: None,
                })
            }
            Err(error) => map_document_diff_operation_cancel_error(error),
        }
    }

    fn record_product_event(&self, event: &'static str) {
        push_document_diff_product_event(&self.product_events, event);
    }
}

struct DesktopDocumentDiffOperationIdSource {
    prefix: u128,
    next: u64,
}

impl DocumentDiffOperationIdGenerator for DesktopDocumentDiffOperationIdSource {
    fn next_id(&mut self) -> Result<String, ()> {
        self.next = self.next.checked_add(1).ok_or(())?;
        Ok(format!("diff-{:x}-{}", self.prefix, self.next))
    }
}

fn push_document_diff_product_event(events: &Arc<Mutex<Vec<&'static str>>>, event: &'static str) {
    if let Ok(mut events) = events.lock() {
        events.push(event);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum DesktopDocumentDiffOperationRequestDto {
    CurrentToVersion {
        workspace_id: String,
        document_id: String,
        version_token: String,
    },
    Versions {
        workspace_id: String,
        document_id: String,
        left_version_token: String,
        right_version_token: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentDiffOperationTokenRequestDto {
    pub operation_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentDiffOperationResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DesktopDocumentDiffOperationDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
    pub repair_required: bool,
}

impl DesktopDocumentDiffOperationResponse {
    fn success(data: DesktopDocumentDiffOperationDataDto) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error_code: None,
            retryable: false,
            repair_required: false,
        }
    }

    fn failure(error_code: &str, retryable: bool) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(error_code.to_string()),
            retryable,
            repair_required: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentDiffOperationDataDto {
    pub operation_token: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<DesktopDocumentDiffDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,
}

const fn document_diff_operation_state_name(state: DocumentDiffOperationState) -> &'static str {
    match state {
        DocumentDiffOperationState::Accepted => "accepted",
        DocumentDiffOperationState::Running => "running",
        DocumentDiffOperationState::Completed => "completed",
        DocumentDiffOperationState::Cancelled => "cancelled",
        DocumentDiffOperationState::Expired => "expired",
        DocumentDiffOperationState::Failed => "failed",
    }
}

fn map_document_diff_operation_start_error(
    error: StartDocumentDiffOperationError,
) -> DesktopDocumentDiffOperationResponse {
    let code = match error {
        StartDocumentDiffOperationError::InvalidInput => "DOCUMENT_DIFF_OPERATION_INVALID_INPUT",
        StartDocumentDiffOperationError::OperationIdUnavailable => {
            "DOCUMENT_DIFF_OPERATION_ID_UNAVAILABLE"
        }
        StartDocumentDiffOperationError::AlreadyExists => "DOCUMENT_DIFF_OPERATION_ALREADY_EXISTS",
        StartDocumentDiffOperationError::CapacityExceeded => {
            "DOCUMENT_DIFF_OPERATION_CAPACITY_EXCEEDED"
        }
        StartDocumentDiffOperationError::RegistryUnavailable => {
            "DOCUMENT_DIFF_OPERATION_RUNTIME_UNAVAILABLE"
        }
    };
    DesktopDocumentDiffOperationResponse::failure(code, error.retryable())
}

fn map_document_diff_operation_status_error(
    error: GetDocumentDiffOperationStatusError,
) -> DesktopDocumentDiffOperationResponse {
    let code = match error {
        GetDocumentDiffOperationStatusError::InvalidInput => {
            "DOCUMENT_DIFF_OPERATION_INVALID_INPUT"
        }
        GetDocumentDiffOperationStatusError::RegistryUnavailable => {
            "DOCUMENT_DIFF_OPERATION_RUNTIME_UNAVAILABLE"
        }
    };
    DesktopDocumentDiffOperationResponse::failure(code, error.retryable())
}

fn map_document_diff_operation_cancel_error(
    error: CancelDocumentDiffOperationError,
) -> DesktopDocumentDiffOperationResponse {
    let code = match error {
        CancelDocumentDiffOperationError::InvalidInput => "DOCUMENT_DIFF_OPERATION_INVALID_INPUT",
        CancelDocumentDiffOperationError::InvalidTransition => {
            "DOCUMENT_DIFF_OPERATION_INVALID_TRANSITION"
        }
        CancelDocumentDiffOperationError::CancellationTooLate => {
            "DOCUMENT_DIFF_OPERATION_CANCELLATION_TOO_LATE"
        }
        CancelDocumentDiffOperationError::Conflict => "DOCUMENT_DIFF_OPERATION_CONFLICT",
        CancelDocumentDiffOperationError::RegistryUnavailable => {
            "DOCUMENT_DIFF_OPERATION_RUNTIME_UNAVAILABLE"
        }
    };
    DesktopDocumentDiffOperationResponse::failure(code, error.retryable())
}

fn map_document_diff_operation_attachment_error(
    error: ResolveAttachmentDiffAvailabilityError,
) -> DesktopDocumentDiffOperationResponse {
    match error {
        ResolveAttachmentDiffAvailabilityError::InvalidInput => {
            DesktopDocumentDiffOperationResponse::failure(
                "DOCUMENT_DIFF_OPERATION_INVALID_INPUT",
                false,
            )
        }
        ResolveAttachmentDiffAvailabilityError::StorageUnavailable => {
            DesktopDocumentDiffOperationResponse::failure(
                "DOCUMENT_DIFF_OPERATION_STORAGE_UNAVAILABLE",
                true,
            )
        }
        ResolveAttachmentDiffAvailabilityError::CorruptedData => {
            DesktopDocumentDiffOperationResponse::failure(
                "DOCUMENT_DIFF_OPERATION_CORRUPTED_DATA",
                false,
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum DesktopDocumentDiffRequestDto {
    CurrentToVersion {
        workspace_id: String,
        document_id: String,
        version_token: String,
    },
    Versions {
        workspace_id: String,
        document_id: String,
        left_version_token: String,
        right_version_token: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentDiffResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DesktopDocumentDiffDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
    pub repair_required: bool,
}

impl DesktopDocumentDiffResponse {
    fn success(data: DesktopDocumentDiffDataDto) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error_code: None,
            retryable: false,
            repair_required: false,
        }
    }

    fn failure(error_code: &str, retryable: bool, repair_required: bool) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(error_code.to_string()),
            retryable,
            repair_required,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentDiffDataDto {
    pub kind: String,
    pub left_version_token: String,
    pub right_version_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_reason: Option<String>,
    pub added_count: usize,
    pub removed_count: usize,
    pub attachment_diff: DesktopDocumentAttachmentDiffDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_delta: Option<DesktopDocumentTitleDeltaDto>,
    pub hunks: Vec<DesktopDocumentDiffHunkDto>,
}

impl DesktopDocumentDiffDataDto {
    fn from(
        left_version_token: &str,
        right_version_token: &str,
        computation: &AuthoritativeDiffComputation,
        attachment_diff: &ResolvedAttachmentDiff,
    ) -> Self {
        match computation {
            AuthoritativeDiffComputation::TooLarge(reason) => Self {
                kind: "too_large".to_string(),
                left_version_token: left_version_token.to_string(),
                right_version_token: right_version_token.to_string(),
                limit_reason: Some(diff_limit_reason(*reason).to_string()),
                added_count: 0,
                removed_count: 0,
                attachment_diff: DesktopDocumentAttachmentDiffDto::from_resolved(attachment_diff),
                title_delta: None,
                hunks: Vec::new(),
            },
            AuthoritativeDiffComputation::Complete(result) => Self {
                kind: "complete".to_string(),
                left_version_token: left_version_token.to_string(),
                right_version_token: right_version_token.to_string(),
                limit_reason: None,
                added_count: result.added_count(),
                removed_count: result.removed_count(),
                attachment_diff: DesktopDocumentAttachmentDiffDto::from_resolved(attachment_diff),
                title_delta: Some(DesktopDocumentTitleDeltaDto::from(result.title_delta())),
                hunks: result
                    .hunks()
                    .iter()
                    .map(DesktopDocumentDiffHunkDto::from)
                    .collect(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentAttachmentDiffDto {
    pub kind: String,
    pub added: Vec<DesktopDocumentAttachmentLabelDto>,
    pub removed: Vec<DesktopDocumentAttachmentLabelDto>,
    pub relabeled: Vec<DesktopDocumentAttachmentRelabelDto>,
    pub unchanged_count: usize,
}

impl DesktopDocumentAttachmentDiffDto {
    pub fn from_resolved(diff: &ResolvedAttachmentDiff) -> Self {
        match diff {
            ResolvedAttachmentDiff::Known(known) => Self {
                kind: "known".to_string(),
                added: known
                    .added()
                    .iter()
                    .map(|reference| DesktopDocumentAttachmentLabelDto {
                        label: reference.reference().label().to_string(),
                        availability: attachment_availability(reference.availability()).to_string(),
                    })
                    .collect(),
                removed: known
                    .removed()
                    .iter()
                    .map(|reference| DesktopDocumentAttachmentLabelDto {
                        label: reference.reference().label().to_string(),
                        availability: attachment_availability(reference.availability()).to_string(),
                    })
                    .collect(),
                relabeled: known
                    .relabeled()
                    .iter()
                    .map(|change| DesktopDocumentAttachmentRelabelDto {
                        before_label: change.before_label().to_string(),
                        after_label: change.after_label().to_string(),
                        availability: attachment_availability(change.availability()).to_string(),
                    })
                    .collect(),
                unchanged_count: known.unchanged_count(),
            },
            ResolvedAttachmentDiff::LegacyUnknown => Self {
                kind: "legacy_unknown".to_string(),
                added: Vec::new(),
                removed: Vec::new(),
                relabeled: Vec::new(),
                unchanged_count: 0,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentAttachmentLabelDto {
    pub label: String,
    pub availability: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentAttachmentRelabelDto {
    pub before_label: String,
    pub after_label: String,
    pub availability: String,
}

const fn attachment_availability(
    value: cabinet_ports::asset_availability::AssetAvailability,
) -> &'static str {
    match value {
        cabinet_ports::asset_availability::AssetAvailability::Available => "available",
        cabinet_ports::asset_availability::AssetAvailability::Missing => "missing",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentTitleDeltaDto {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

impl DesktopDocumentTitleDeltaDto {
    fn from(delta: &AuthoritativeDocumentTitleDelta) -> Self {
        match delta {
            AuthoritativeDocumentTitleDelta::Unchanged => Self {
                kind: "unchanged".to_string(),
                before: None,
                after: None,
            },
            AuthoritativeDocumentTitleDelta::Changed { before, after } => Self {
                kind: "changed".to_string(),
                before: Some(before.clone()),
                after: Some(after.clone()),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentDiffHunkDto {
    pub old_start_line: usize,
    pub new_start_line: usize,
    pub added_count: usize,
    pub removed_count: usize,
    pub lines: Vec<DesktopDocumentDiffLineDto>,
}

impl DesktopDocumentDiffHunkDto {
    fn from(hunk: &cabinet_usecases::document_diff::DiffHunk) -> Self {
        Self {
            old_start_line: hunk.old_start_line(),
            new_start_line: hunk.new_start_line(),
            added_count: hunk.added_count(),
            removed_count: hunk.removed_count(),
            lines: hunk
                .lines()
                .iter()
                .map(DesktopDocumentDiffLineDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentDiffLineDto {
    pub kind: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_line_number: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_line_number: Option<usize>,
}

impl DesktopDocumentDiffLineDto {
    fn from(line: &cabinet_usecases::document_diff::LineDiff) -> Self {
        Self {
            kind: match line.kind() {
                AuthoritativeLineDiffKind::Equal => "unchanged",
                AuthoritativeLineDiffKind::Added => "added",
                AuthoritativeLineDiffKind::Removed => "removed",
            }
            .to_string(),
            text: line.text().to_string(),
            old_line_number: line.old_line_number(),
            new_line_number: line.new_line_number(),
        }
    }
}

const fn diff_limit_reason(reason: AuthoritativeDiffLimitReason) -> &'static str {
    match reason {
        AuthoritativeDiffLimitReason::Bytes => "bytes",
        AuthoritativeDiffLimitReason::Lines => "lines",
        AuthoritativeDiffLimitReason::Hunks => "hunks",
    }
}

fn map_authoritative_diff_error(
    error: CompareAuthoritativeDocumentRevisionsError,
) -> DesktopDocumentDiffResponse {
    match error {
        CompareAuthoritativeDocumentRevisionsError::InvalidInput => {
            DesktopDocumentDiffResponse::failure("DOCUMENT_DIFF_INVALID_INPUT", false, false)
        }
        CompareAuthoritativeDocumentRevisionsError::NotFound => {
            DesktopDocumentDiffResponse::failure("DOCUMENT_DIFF_NOT_FOUND", false, false)
        }
        CompareAuthoritativeDocumentRevisionsError::StorageUnavailable => {
            DesktopDocumentDiffResponse::failure("DOCUMENT_DIFF_STORAGE_UNAVAILABLE", true, false)
        }
        CompareAuthoritativeDocumentRevisionsError::CorruptedData => {
            DesktopDocumentDiffResponse::failure("DOCUMENT_DIFF_CORRUPTED_DATA", false, true)
        }
    }
}

fn map_attachment_availability_error(
    error: ResolveAttachmentDiffAvailabilityError,
) -> DesktopDocumentDiffResponse {
    match error {
        ResolveAttachmentDiffAvailabilityError::InvalidInput => {
            DesktopDocumentDiffResponse::failure("DOCUMENT_DIFF_INVALID_INPUT", false, false)
        }
        ResolveAttachmentDiffAvailabilityError::StorageUnavailable => {
            DesktopDocumentDiffResponse::failure("DOCUMENT_DIFF_STORAGE_UNAVAILABLE", true, false)
        }
        ResolveAttachmentDiffAvailabilityError::CorruptedData => {
            DesktopDocumentDiffResponse::failure("DOCUMENT_DIFF_CORRUPTED_DATA", false, true)
        }
    }
}

fn map_authoritative_restore_preview_error(
    error: PreviewAuthoritativeDocumentRestoreError,
) -> DesktopDocumentAuthoringCommandResponse {
    match error {
        PreviewAuthoritativeDocumentRestoreError::InvalidInput => {
            DesktopDocumentAuthoringCommandResponse::restore_failure(
                "DOCUMENT_RESTORE_INVALID_INPUT",
                false,
            )
        }
        PreviewAuthoritativeDocumentRestoreError::NotFound => {
            DesktopDocumentAuthoringCommandResponse::restore_failure(
                "DOCUMENT_RESTORE_NOT_FOUND",
                false,
            )
        }
        PreviewAuthoritativeDocumentRestoreError::StorageUnavailable => {
            DesktopDocumentAuthoringCommandResponse::restore_failure(
                "DOCUMENT_RESTORE_STORAGE_UNAVAILABLE",
                true,
            )
        }
        PreviewAuthoritativeDocumentRestoreError::CorruptedData => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_RESTORE_CORRUPTED_DATA",
                false,
                true,
            )
        }
    }
}

fn map_restore_preview_availability_error(
    error: ResolveAttachmentDiffAvailabilityError,
) -> DesktopDocumentAuthoringCommandResponse {
    match error {
        ResolveAttachmentDiffAvailabilityError::InvalidInput => {
            DesktopDocumentAuthoringCommandResponse::restore_failure(
                "DOCUMENT_RESTORE_INVALID_INPUT",
                false,
            )
        }
        ResolveAttachmentDiffAvailabilityError::StorageUnavailable => {
            DesktopDocumentAuthoringCommandResponse::restore_failure(
                "DOCUMENT_RESTORE_STORAGE_UNAVAILABLE",
                true,
            )
        }
        ResolveAttachmentDiffAvailabilityError::CorruptedData => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_RESTORE_CORRUPTED_DATA",
                false,
                true,
            )
        }
    }
}

fn map_restore_target_preflight_error(
    error: RestoreTargetAssetPreflightError,
) -> DesktopDocumentAuthoringCommandResponse {
    match error {
        RestoreTargetAssetPreflightError::InvalidInput => {
            DesktopDocumentAuthoringCommandResponse::restore_failure(
                "DOCUMENT_RESTORE_INVALID_INPUT",
                false,
            )
        }
        RestoreTargetAssetPreflightError::StorageUnavailable => {
            DesktopDocumentAuthoringCommandResponse::restore_failure(
                "DOCUMENT_RESTORE_STORAGE_UNAVAILABLE",
                true,
            )
        }
        RestoreTargetAssetPreflightError::CorruptedData => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_RESTORE_CORRUPTED_DATA",
                false,
                true,
            )
        }
    }
}

fn map_restore_target_snapshot_error(
    error: VersionStoreError,
) -> DesktopDocumentAuthoringCommandResponse {
    match error {
        VersionStoreError::CorruptedHistory | VersionStoreError::MismatchedVersionSnapshot => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_RESTORE_CORRUPTED_DATA",
                false,
                true,
            )
        }
        _ => DesktopDocumentAuthoringCommandResponse::restore_failure(
            "DOCUMENT_RESTORE_STORAGE_UNAVAILABLE",
            true,
        ),
    }
}

fn map_restore_document_revision_error(
    error: RestoreDocumentRevisionError,
) -> DesktopDocumentAuthoringCommandResponse {
    match error {
        RestoreDocumentRevisionError::InvalidInput => {
            DesktopDocumentAuthoringCommandResponse::restore_failure(
                "DOCUMENT_RESTORE_INVALID_INPUT",
                false,
            )
        }
        RestoreDocumentRevisionError::NotFound => {
            DesktopDocumentAuthoringCommandResponse::restore_failure(
                "DOCUMENT_RESTORE_NOT_FOUND",
                false,
            )
        }
        RestoreDocumentRevisionError::MissingDependency => {
            DesktopDocumentAuthoringCommandResponse::restore_failure(
                "DOCUMENT_RESTORE_MISSING_DEPENDENCY",
                false,
            )
        }
        RestoreDocumentRevisionError::CommitConflict
        | RestoreDocumentRevisionError::OperationConflict => {
            DesktopDocumentAuthoringCommandResponse::restore_failure(
                "DOCUMENT_RESTORE_VERSION_CONFLICT",
                false,
            )
        }
        RestoreDocumentRevisionError::StorageUnavailable
        | RestoreDocumentRevisionError::FingerprintUnavailable
        | RestoreDocumentRevisionError::MetadataUnavailable
        | RestoreDocumentRevisionError::JournalUnavailable
        | RestoreDocumentRevisionError::CommitUnavailable => {
            DesktopDocumentAuthoringCommandResponse::restore_failure(
                "DOCUMENT_RESTORE_STORAGE_UNAVAILABLE",
                true,
            )
        }
        RestoreDocumentRevisionError::RecoveryRequired => {
            DesktopDocumentAuthoringCommandResponse::revision_failure(
                "DOCUMENT_RESTORE_RECOVERY_REQUIRED",
                true,
                true,
            )
        }
    }
}

pub struct DesktopDocumentAuthoringRuntime {
    executor: DocumentAuthoringCommandExecutor,
    state: Mutex<DesktopDocumentAuthoringState>,
}

struct DesktopDocumentAuthoringState {
    documents: LocalDocumentRepository,
    versions: LocalVersionStore,
    pointer: LocalCurrentDocumentVersionPointer,
    authoritative_versions: LocalVersionStore,
    authoritative_pointer: LocalCurrentDocumentVersionPointer,
    availability: LocalAssetAvailabilityResolver,
    resolve_availability: ResolveAttachmentDiffAvailabilityUsecase,
    restore_preflight: RestoreTargetAssetPreflightUsecase,
    restore: LocalRestoreDocumentRevisionRuntime,
    events: DesktopDocumentChangeSink,
    authoritative_events: DesktopDocumentChangeSink,
    product_log: DesktopDocumentProductLogSink,
}

impl DesktopDocumentAuthoringRuntime {
    pub fn new(app_data_root: PathBuf, max_body_bytes: usize) -> Result<Self, &'static str> {
        let body_policy = DocumentBodyPolicy::new(max_body_bytes)
            .map_err(|_| "DOCUMENT_AUTHORING_INVALID_BODY_POLICY")?;
        let versions = LocalVersionStore::with_body_policy(
            app_data_root.join("authoring-versions"),
            body_policy,
        );
        versions
            .migrate_revision_numbers()
            .map_err(|_| "DOCUMENT_AUTHORING_REVISION_MIGRATION_FAILED")?;
        let mut authoritative_events = DesktopDocumentChangeSink::with_authoritative_body_policy(
            app_data_root.clone(),
            body_policy,
        );
        let recovered =
            LocalRestoreProjectionRecoveryRuntime::new(app_data_root.clone(), body_policy)
                .recover(1000)
                .map_err(|_| "DOCUMENT_RESTORE_STARTUP_RECOVERY_FAILED")?;
        for candidate in recovered.recovered() {
            authoritative_events
                .publish_authoritative_updated(
                    candidate.workspace_id().as_str(),
                    candidate.document_id().as_str(),
                    candidate.version_id().as_str(),
                )
                .map_err(|_| "DOCUMENT_RESTORE_STARTUP_RECOVERY_FAILED")?;
        }
        Ok(Self {
            executor: DocumentAuthoringCommandExecutor::new(body_policy),
            state: Mutex::new(DesktopDocumentAuthoringState {
                documents: LocalDocumentRepository::with_body_policy(
                    app_data_root.join("authoring-current"),
                    body_policy,
                ),
                versions,
                pointer: LocalCurrentDocumentVersionPointer::new(
                    app_data_root.join("authoring-current-version"),
                ),
                authoritative_versions: LocalVersionStore::with_body_policy(
                    app_data_root.join(LOCAL_DOCUMENT_VERSION_ROOT),
                    body_policy,
                ),
                authoritative_pointer: LocalCurrentDocumentVersionPointer::new(
                    app_data_root.join(LOCAL_DOCUMENT_POINTER_ROOT),
                ),
                availability: LocalAssetAvailabilityResolver::new(app_data_root.clone()),
                resolve_availability: ResolveAttachmentDiffAvailabilityUsecase::new(),
                restore_preflight: RestoreTargetAssetPreflightUsecase::new(),
                restore: LocalRestoreDocumentRevisionRuntime::new(
                    app_data_root.clone(),
                    body_policy,
                ),
                events: DesktopDocumentChangeSink::with_body_policy(
                    app_data_root.clone(),
                    body_policy,
                ),
                authoritative_events,
                product_log: DesktopDocumentProductLogSink::default(),
            }),
        })
    }

    pub fn execute(
        &self,
        request: DesktopDocumentAuthoringRequestDto,
    ) -> DesktopDocumentAuthoringCommandResponse {
        let Ok(mut state) = self.state.lock() else {
            return DesktopDocumentAuthoringCommandResponse::runtime_unavailable();
        };
        let DesktopDocumentAuthoringState {
            documents,
            versions,
            pointer,
            authoritative_versions,
            authoritative_pointer,
            availability,
            resolve_availability,
            restore_preflight,
            restore,
            events,
            authoritative_events,
            product_log,
        } = &mut *state;

        match request {
            DesktopDocumentAuthoringRequestDto::Create { .. }
            | DesktopDocumentAuthoringRequestDto::Update { .. }
            | DesktopDocumentAuthoringRequestDto::GetCurrent { .. } => {
                match self.executor.execute(
                    request.into(),
                    documents,
                    versions,
                    pointer,
                    events,
                    product_log,
                ) {
                    Ok(result) => DesktopDocumentAuthoringCommandResponse::success(result),
                    Err(failure) => DesktopDocumentAuthoringCommandResponse::failure(failure),
                }
            }
            DesktopDocumentAuthoringRequestDto::Rename {
                workspace_id,
                document_id,
                current_version_id,
                title,
                path,
            } => {
                let Ok(workspace) = WorkspaceId::new(&workspace_id) else {
                    return DesktopDocumentAuthoringCommandResponse::restore_failure(
                        "DOCUMENT_AUTHORING_INVALID_INPUT",
                        false,
                    );
                };
                let Ok(document) = DocumentId::new(&document_id) else {
                    return DesktopDocumentAuthoringCommandResponse::restore_failure(
                        "DOCUMENT_AUTHORING_INVALID_INPUT",
                        false,
                    );
                };
                let Ok(expected_version) = VersionId::new(&current_version_id) else {
                    return DesktopDocumentAuthoringCommandResponse::restore_failure(
                        "DOCUMENT_AUTHORING_INVALID_INPUT",
                        false,
                    );
                };
                match pointer.load_current_version(&workspace, &document) {
                    Ok(Some(current)) if current == expected_version => {}
                    Ok(_) => {
                        return DesktopDocumentAuthoringCommandResponse::restore_failure(
                            "DOCUMENT_AUTHORING_VERSION_CONFLICT",
                            false,
                        );
                    }
                    Err(_) => {
                        return DesktopDocumentAuthoringCommandResponse::restore_failure(
                            "DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE",
                            true,
                        );
                    }
                }
                match RenameDocumentUsecase::new().execute(
                    RenameDocumentInput::new(
                        &workspace_id,
                        &document_id,
                        &current_version_id,
                        &title,
                        &path,
                    ),
                    documents,
                    events,
                    product_log,
                ) {
                    Ok(output) => DesktopDocumentAuthoringCommandResponse::success_data(
                        DesktopDocumentAuthoringDataDto::renamed(
                            output.document_id().as_str(),
                            &current_version_id,
                            output.title().as_str(),
                            output.path().as_str(),
                        ),
                    ),
                    Err(error) => DesktopDocumentAuthoringCommandResponse::restore_failure(
                        match error.code() {
                            "document.invalid_input" => "DOCUMENT_AUTHORING_INVALID_INPUT",
                            "document.not_found" => "DOCUMENT_AUTHORING_NOT_FOUND",
                            _ => "DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE",
                        },
                        error.code() == "document.storage_unavailable",
                    ),
                }
            }
            DesktopDocumentAuthoringRequestDto::GetHistory {
                workspace_id,
                document_id,
                limit,
            } => {
                let result = GetDocumentHistoryUsecase::new().execute(
                    GetDocumentHistoryInput::new(
                        &workspace_id,
                        &document_id,
                        None,
                        usize::from(limit),
                    ),
                    versions,
                );
                match result {
                    Ok(output) => {
                        match DesktopDocumentAuthoringDataDto::history(&document_id, output.page())
                        {
                            Ok(data) => DesktopDocumentAuthoringCommandResponse::success_data(data),
                            Err(error_code) => {
                                DesktopDocumentAuthoringCommandResponse::restore_failure(
                                    error_code, false,
                                )
                            }
                        }
                    }
                    Err(_) => DesktopDocumentAuthoringCommandResponse::restore_failure(
                        "DOCUMENT_RESTORE_STORAGE_UNAVAILABLE",
                        true,
                    ),
                }
            }
            DesktopDocumentAuthoringRequestDto::GetVersion {
                workspace_id,
                document_id,
                version_id,
            } => {
                let result = GetDocumentVersionUsecase::new().execute(
                    GetDocumentVersionInput::new(&workspace_id, &document_id, &version_id),
                    versions,
                );
                match result {
                    Ok(output) => DesktopDocumentAuthoringCommandResponse::success_data(
                        DesktopDocumentAuthoringDataDto::version(
                            &document_id,
                            &version_id,
                            output.snapshot().body().as_str(),
                        ),
                    ),
                    Err(_) => DesktopDocumentAuthoringCommandResponse::restore_failure(
                        "DOCUMENT_RESTORE_NOT_FOUND",
                        false,
                    ),
                }
            }
            DesktopDocumentAuthoringRequestDto::PreviewRestore {
                workspace_id,
                document_id,
                target_version_id,
            } => {
                let result = PreviewAuthoritativeDocumentRestoreUsecase::new().execute(
                    PreviewAuthoritativeDocumentRestoreInput::new(
                        &workspace_id,
                        &document_id,
                        &target_version_id,
                    ),
                    authoritative_pointer,
                    authoritative_versions,
                );
                match result {
                    Ok(output) => {
                        let workspace = match WorkspaceId::new(&workspace_id) {
                            Ok(value) => value,
                            Err(_) => {
                                return DesktopDocumentAuthoringCommandResponse::restore_failure(
                                    "DOCUMENT_RESTORE_INVALID_INPUT",
                                    false,
                                );
                            }
                        };
                        let document = match DocumentId::new(&document_id) {
                            Ok(value) => value,
                            Err(_) => {
                                return DesktopDocumentAuthoringCommandResponse::restore_failure(
                                    "DOCUMENT_RESTORE_INVALID_INPUT",
                                    false,
                                );
                            }
                        };
                        let target_version = match VersionId::new(&target_version_id) {
                            Ok(value) => value,
                            Err(_) => {
                                return DesktopDocumentAuthoringCommandResponse::restore_failure(
                                    "DOCUMENT_RESTORE_INVALID_INPUT",
                                    false,
                                );
                            }
                        };
                        let target_snapshot = match authoritative_versions.get_version_snapshot(
                            &workspace,
                            &document,
                            &target_version,
                        ) {
                            Ok(Some(snapshot)) => snapshot,
                            Ok(None) => {
                                return DesktopDocumentAuthoringCommandResponse::restore_failure(
                                    "DOCUMENT_RESTORE_NOT_FOUND",
                                    false,
                                );
                            }
                            Err(error) => return map_restore_target_snapshot_error(error),
                        };
                        let preflight = match restore_preflight.execute(
                            RestoreTargetAssetPreflightInput::new(
                                &workspace_id,
                                target_snapshot.attachment_state().clone(),
                            ),
                            availability,
                        ) {
                            Ok(value) => value,
                            Err(error) => return map_restore_target_preflight_error(error),
                        };
                        let missing_asset_labels = match preflight {
                            RestoreTargetAssetPreflightOutcome::BlockedMissingAssets(missing) => {
                                missing
                                    .into_iter()
                                    .map(|reference| reference.label().to_string())
                                    .collect::<Vec<_>>()
                            }
                            RestoreTargetAssetPreflightOutcome::Available
                            | RestoreTargetAssetPreflightOutcome::LegacyPreserved => Vec::new(),
                        };
                        let attachments = match resolve_availability.execute(
                            ResolveAttachmentDiffAvailabilityInput::new(
                                &workspace_id,
                                output.attachment_diff().clone(),
                            ),
                            availability,
                        ) {
                            Ok(attachments) => attachments,
                            Err(error) => return map_restore_preview_availability_error(error),
                        };
                        let lines = match output.computation() {
                            AuthoritativeDiffComputation::Complete(result) => result.lines(),
                            AuthoritativeDiffComputation::TooLarge(_) => &[],
                        };
                        let diff = DesktopDocumentDiffDataDto::from(
                            output.expected_current_version_id().as_str(),
                            output.target_version_id().as_str(),
                            output.computation(),
                            &attachments,
                        );
                        DesktopDocumentAuthoringCommandResponse::success_data(
                            DesktopDocumentAuthoringDataDto::restore_preview(
                                &document_id,
                                output.target_version_id().as_str(),
                                output.expected_current_version_id().as_str(),
                                output.can_restore() && missing_asset_labels.is_empty(),
                                missing_asset_labels,
                                lines,
                                diff,
                            ),
                        )
                    }
                    Err(error) => map_authoritative_restore_preview_error(error),
                }
            }
            DesktopDocumentAuthoringRequestDto::Restore {
                operation_id,
                workspace_id,
                document_id,
                target_version_id,
                expected_current_version_id,
                author,
                summary,
            } => match restore.execute_with_logger(
                RestoreDocumentRevisionInput::new(
                    &operation_id,
                    &workspace_id,
                    &document_id,
                    &target_version_id,
                    &expected_current_version_id,
                    &author,
                    &summary,
                ),
                product_log,
            ) {
                Ok(output) => {
                    if authoritative_events
                        .publish_authoritative_updated(
                            &workspace_id,
                            &document_id,
                            output.version_id().as_str(),
                        )
                        .is_err()
                    {
                        product_log.write_restore_product(RestoreProductEvent::RecoveryRequired);
                        return DesktopDocumentAuthoringCommandResponse::revision_failure(
                            "DOCUMENT_RESTORE_RECOVERY_REQUIRED",
                            true,
                            true,
                        );
                    }
                    product_log.write_restore_product(RestoreProductEvent::Completed);
                    DesktopDocumentAuthoringCommandResponse::success_data(
                        DesktopDocumentAuthoringDataDto::restored(
                            &document_id,
                            output.version_id().as_str(),
                            output.revision_number().value(),
                        ),
                    )
                }
                Err(error) => map_restore_document_revision_error(error),
            },
        }
    }

    pub fn product_event_count(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.product_log.event_count)
            .unwrap_or(0)
    }

    pub fn restore_product_event_names(&self) -> Vec<&'static str> {
        self.state
            .lock()
            .map(|state| state.product_log.restore_event_names.clone())
            .unwrap_or_default()
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum DesktopDocumentAuthoringRequestDto {
    Create {
        workspace_id: String,
        document_id: String,
        path: String,
        body: String,
        version_id: String,
        snapshot_ref: String,
        author: String,
        summary: String,
    },
    Update {
        workspace_id: String,
        document_id: String,
        body: String,
        expected_version_id: String,
        version_id: String,
        snapshot_ref: String,
        author: String,
        summary: String,
    },
    Rename {
        workspace_id: String,
        document_id: String,
        current_version_id: String,
        title: String,
        path: String,
    },
    GetCurrent {
        workspace_id: String,
        document_id: String,
    },
    GetHistory {
        workspace_id: String,
        document_id: String,
        limit: u16,
    },
    GetVersion {
        workspace_id: String,
        document_id: String,
        version_id: String,
    },
    PreviewRestore {
        workspace_id: String,
        document_id: String,
        target_version_id: String,
    },
    Restore {
        operation_id: String,
        workspace_id: String,
        document_id: String,
        target_version_id: String,
        expected_current_version_id: String,
        author: String,
        summary: String,
    },
}

impl From<DesktopDocumentAuthoringRequestDto> for DocumentAuthoringCommandRequest {
    fn from(request: DesktopDocumentAuthoringRequestDto) -> Self {
        match request {
            DesktopDocumentAuthoringRequestDto::Create {
                workspace_id,
                document_id,
                path,
                body,
                version_id,
                snapshot_ref,
                author,
                summary,
            } => Self::Create {
                workspace_id,
                document_id,
                path,
                body,
                version_id,
                snapshot_ref,
                author,
                summary,
            },
            DesktopDocumentAuthoringRequestDto::Update {
                workspace_id,
                document_id,
                body,
                expected_version_id,
                version_id,
                snapshot_ref,
                author,
                summary,
            } => Self::Update {
                workspace_id,
                document_id,
                body,
                expected_version_id,
                version_id,
                snapshot_ref,
                author,
                summary,
            },
            DesktopDocumentAuthoringRequestDto::GetCurrent {
                workspace_id,
                document_id,
            } => Self::GetCurrent {
                workspace_id,
                document_id,
            },
            DesktopDocumentAuthoringRequestDto::Rename { .. } => {
                unreachable!("rename requests are handled before authoring executor")
            }
            _ => unreachable!("history and restore requests are handled before authoring executor"),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentAuthoringCommandResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DesktopDocumentAuthoringDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
    pub repair_required: bool,
}

impl std::fmt::Debug for DesktopDocumentAuthoringCommandResponse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DesktopDocumentAuthoringCommandResponse")
            .field("ok", &self.ok)
            .field(
                "data_kind",
                &self.data.as_ref().map(|data| data.kind.as_str()),
            )
            .field(
                "document_id",
                &self.data.as_ref().map(|data| data.document_id.as_str()),
            )
            .field(
                "current_version_id",
                &self
                    .data
                    .as_ref()
                    .map(|data| data.current_version_id.as_str()),
            )
            .field("error_code", &self.error_code)
            .field("retryable", &self.retryable)
            .field("repair_required", &self.repair_required)
            .finish()
    }
}

impl DesktopDocumentAuthoringCommandResponse {
    fn success(result: DocumentAuthoringCommandResult) -> Self {
        Self {
            ok: true,
            data: Some(result.into()),
            error_code: None,
            retryable: false,
            repair_required: false,
        }
    }

    fn failure(failure: DocumentAuthoringCommandFailure) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(failure.error_code.to_string()),
            retryable: failure.retryable,
            repair_required: failure.repair_required,
        }
    }

    fn runtime_unavailable() -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some("DOCUMENT_AUTHORING_RUNTIME_UNAVAILABLE".to_string()),
            retryable: true,
            repair_required: false,
        }
    }

    fn success_data(data: DesktopDocumentAuthoringDataDto) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error_code: None,
            retryable: false,
            repair_required: false,
        }
    }

    fn restore_failure(error_code: &str, retryable: bool) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(error_code.to_string()),
            retryable,
            repair_required: false,
        }
    }

    fn revision_failure(error_code: &str, retryable: bool, repair_required: bool) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(error_code.to_string()),
            retryable,
            repair_required,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentAuthoringDataDto {
    pub kind: String,
    pub document_id: String,
    pub current_version_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<DesktopDocumentHistoryEntryDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_current_version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_restore: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restore_diff: Option<DesktopDocumentDiffDataDto>,
    pub missing_asset_labels: Vec<String>,
    pub lines: Vec<DesktopRestoreDiffLineDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restored_version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_state: Option<String>,
}

impl From<DocumentAuthoringCommandResult> for DesktopDocumentAuthoringDataDto {
    fn from(result: DocumentAuthoringCommandResult) -> Self {
        match result {
            DocumentAuthoringCommandResult::Created {
                document_id,
                current_version_id,
            } => Self::without_content("created", document_id, current_version_id),
            DocumentAuthoringCommandResult::Updated {
                document_id,
                current_version_id,
            } => Self::without_content("updated", document_id, current_version_id),
            DocumentAuthoringCommandResult::Current {
                document_id,
                title,
                path,
                body,
                current_version_id,
            } => Self {
                kind: "current".to_string(),
                document_id,
                current_version_id,
                title: Some(title),
                path: Some(path),
                body: Some(body),
                entries: Vec::new(),
                version_id: None,
                target_version_id: None,
                expected_current_version_id: None,
                can_restore: None,
                restore_diff: None,
                missing_asset_labels: Vec::new(),
                lines: Vec::new(),
                restored_version_id: None,
                revision_number: None,
                final_state: None,
            },
        }
    }
}

impl DesktopDocumentAuthoringDataDto {
    fn renamed(document_id: &str, current_version_id: &str, title: &str, path: &str) -> Self {
        Self {
            kind: "renamed".to_string(),
            document_id: document_id.to_string(),
            current_version_id: current_version_id.to_string(),
            title: Some(title.to_string()),
            path: Some(path.to_string()),
            body: None,
            entries: Vec::new(),
            version_id: None,
            target_version_id: None,
            expected_current_version_id: None,
            can_restore: None,
            restore_diff: None,
            missing_asset_labels: Vec::new(),
            lines: Vec::new(),
            restored_version_id: None,
            revision_number: None,
            final_state: None,
        }
    }

    fn without_content(kind: &str, document_id: String, current_version_id: String) -> Self {
        Self {
            kind: kind.to_string(),
            document_id,
            current_version_id,
            title: None,
            path: None,
            body: None,
            entries: Vec::new(),
            version_id: None,
            target_version_id: None,
            expected_current_version_id: None,
            can_restore: None,
            restore_diff: None,
            missing_asset_labels: Vec::new(),
            lines: Vec::new(),
            restored_version_id: None,
            revision_number: None,
            final_state: None,
        }
    }

    fn history(document_id: &str, page: &HistoryPage) -> Result<Self, &'static str> {
        let entries = page
            .entries()
            .iter()
            .map(|entry| {
                let revision_number = entry
                    .revision_number()
                    .ok_or("DOCUMENT_HISTORY_REVISION_UNAVAILABLE")?;
                Ok(DesktopDocumentHistoryEntryDto {
                    revision_number: revision_number.value(),
                    version_id: entry.version_id().as_str().to_string(),
                    summary: entry.summary().as_str().to_string(),
                    author: entry.author().as_str().to_string(),
                    created_at: entry
                        .created_at_epoch_ms()
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                })
            })
            .collect::<Result<Vec<_>, &'static str>>()?;
        Ok(Self {
            kind: "history".to_string(),
            document_id: document_id.to_string(),
            current_version_id: String::new(),
            title: None,
            path: None,
            body: None,
            entries,
            version_id: None,
            target_version_id: None,
            expected_current_version_id: None,
            can_restore: None,
            restore_diff: None,
            missing_asset_labels: Vec::new(),
            lines: Vec::new(),
            restored_version_id: None,
            revision_number: None,
            final_state: None,
        })
    }

    fn version(document_id: &str, version_id: &str, body: &str) -> Self {
        Self {
            kind: "version".to_string(),
            document_id: document_id.to_string(),
            current_version_id: String::new(),
            title: None,
            path: None,
            body: Some(body.to_string()),
            entries: Vec::new(),
            version_id: Some(version_id.to_string()),
            target_version_id: None,
            expected_current_version_id: None,
            can_restore: None,
            restore_diff: None,
            missing_asset_labels: Vec::new(),
            lines: Vec::new(),
            restored_version_id: None,
            revision_number: None,
            final_state: None,
        }
    }

    fn restore_preview(
        document_id: &str,
        target_version_id: &str,
        expected_current_version_id: &str,
        can_restore: bool,
        missing_asset_labels: Vec<String>,
        lines: &[LineDiff],
        restore_diff: DesktopDocumentDiffDataDto,
    ) -> Self {
        Self {
            kind: "restore_preview".to_string(),
            document_id: document_id.to_string(),
            current_version_id: String::new(),
            title: None,
            path: None,
            body: None,
            entries: Vec::new(),
            version_id: None,
            target_version_id: Some(target_version_id.to_string()),
            expected_current_version_id: Some(expected_current_version_id.to_string()),
            can_restore: Some(can_restore),
            restore_diff: Some(restore_diff),
            missing_asset_labels,
            lines: lines
                .iter()
                .map(|line| DesktopRestoreDiffLineDto {
                    kind: match line.kind() {
                        LineDiffKind::Equal => "unchanged".to_string(),
                        LineDiffKind::Removed => "removed".to_string(),
                        LineDiffKind::Added => "added".to_string(),
                    },
                    text: line.text().to_string(),
                })
                .collect(),
            restored_version_id: None,
            revision_number: None,
            final_state: None,
        }
    }

    fn restored(document_id: &str, restored_version_id: &str, revision_number: u64) -> Self {
        Self {
            kind: "restored".to_string(),
            document_id: document_id.to_string(),
            current_version_id: restored_version_id.to_string(),
            title: None,
            path: None,
            body: None,
            entries: Vec::new(),
            version_id: None,
            target_version_id: None,
            expected_current_version_id: None,
            can_restore: None,
            restore_diff: None,
            missing_asset_labels: Vec::new(),
            lines: Vec::new(),
            restored_version_id: Some(restored_version_id.to_string()),
            revision_number: Some(revision_number),
            final_state: Some("Completed".to_string()),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentHistoryEntryDto {
    pub revision_number: u64,
    pub version_id: String,
    pub summary: String,
    pub author: String,
    pub created_at: String,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopRestoreDiffLineDto {
    pub kind: String,
    pub text: String,
}

struct DesktopDocumentChangeSink {
    event_count: usize,
    repository: DurableProjectionWorkRepository,
    catalog: DurableDocumentLinkCatalog,
    links: DurableLocalLinkIndex,
    pointer: LocalCurrentDocumentVersionPointer,
    documents: LocalDocumentRepository,
    home: LocalWorkspaceHomeProjectionStore,
    last_error_code: Option<&'static str>,
}

impl DesktopDocumentChangeSink {
    #[cfg(test)]
    fn new(app_data_root: PathBuf) -> Self {
        Self::with_body_policy(
            app_data_root,
            DocumentBodyPolicy::new(10 * 1024 * 1024).expect("valid desktop body policy"),
        )
    }

    fn with_body_policy(app_data_root: PathBuf, body_policy: DocumentBodyPolicy) -> Self {
        Self::with_pointer_root(app_data_root, body_policy, "authoring-current-version")
    }

    fn with_authoritative_body_policy(
        app_data_root: PathBuf,
        body_policy: DocumentBodyPolicy,
    ) -> Self {
        Self::with_pointer_root(app_data_root, body_policy, LOCAL_DOCUMENT_POINTER_ROOT)
    }

    fn with_pointer_root(
        app_data_root: PathBuf,
        body_policy: DocumentBodyPolicy,
        pointer_root: &str,
    ) -> Self {
        Self {
            event_count: 0,
            repository: DurableProjectionWorkRepository::new(app_data_root.clone()),
            catalog: DurableDocumentLinkCatalog::new(app_data_root.clone()),
            links: DurableLocalLinkIndex::new(app_data_root.clone()),
            pointer: LocalCurrentDocumentVersionPointer::new(app_data_root.join(pointer_root)),
            documents: LocalDocumentRepository::with_body_policy(
                app_data_root.join("authoring-current"),
                body_policy,
            ),
            home: LocalWorkspaceHomeProjectionStore::new(app_data_root),
            last_error_code: None,
        }
    }

    fn publish_authoritative_created(
        &mut self,
        workspace_id: &str,
        document_id: &str,
        version_id: &str,
    ) -> Result<(), &'static str> {
        let workspace =
            WorkspaceId::new(workspace_id).map_err(|_| "DOCUMENT_REVISION_INVALID_INPUT")?;
        let document =
            DocumentId::new(document_id).map_err(|_| "DOCUMENT_REVISION_INVALID_INPUT")?;
        let current = self
            .documents
            .get_current_by_id(&workspace, &document)
            .map_err(|_| "DOCUMENT_REVISION_RECOVERY_REQUIRED")?
            .ok_or("DOCUMENT_REVISION_RECOVERY_REQUIRED")?;
        let event = DocumentChangeEvent::DocumentCreated {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            version_id: version_id.to_string(),
            title: current.metadata().title().as_str().to_string(),
            path: current.path().as_str().to_string(),
        };
        self.publish_checked(event)
    }

    fn publish_authoritative_updated(
        &mut self,
        workspace_id: &str,
        document_id: &str,
        version_id: &str,
    ) -> Result<(), &'static str> {
        let workspace =
            WorkspaceId::new(workspace_id).map_err(|_| "DOCUMENT_REVISION_INVALID_INPUT")?;
        let document =
            DocumentId::new(document_id).map_err(|_| "DOCUMENT_REVISION_INVALID_INPUT")?;
        let current = self
            .documents
            .get_current_by_id(&workspace, &document)
            .map_err(|_| "DOCUMENT_REVISION_RECOVERY_REQUIRED")?
            .ok_or("DOCUMENT_REVISION_RECOVERY_REQUIRED")?;
        self.publish_checked(DocumentChangeEvent::DocumentUpdated {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            version_id: version_id.to_string(),
            title: current.metadata().title().as_str().to_string(),
            path: current.path().as_str().to_string(),
        })
    }

    fn publish_checked(&mut self, event: DocumentChangeEvent) -> Result<(), &'static str> {
        self.publish(event);
        match self.last_error_code {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl DocumentChangeEventPublisher for DesktopDocumentChangeSink {
    fn publish(&mut self, event: DocumentChangeEvent) {
        self.event_count += 1;
        let home_error = UpdateWorkspaceHomeProjectionUsecase::new(100)
            .expect("valid desktop home projection capacity")
            .execute(event.clone(), &self.documents, &mut self.home)
            .err()
            .map(|error| error.code());
        let catalog_error = ApplyDocumentLinkCatalogChangeUsecase::new()
            .execute(&event, &mut self.catalog)
            .err()
            .map(|error| error.code());
        let fanout_error = ReindexReferenceDependentsUsecase::new()
            .execute(&event, &self.links, &self.pointer, &mut self.repository)
            .err()
            .map(|error| error.code());
        let projection_error = EnqueueProjectionWorkUsecase::new()
            .execute(event, &mut self.repository)
            .err()
            .map(|error| error.code());
        self.last_error_code = home_error
            .or(catalog_error)
            .or(fanout_error)
            .or(projection_error);
    }
}

#[derive(Default)]
struct DesktopDocumentProductLogSink {
    event_count: usize,
    last_error_code: Option<&'static str>,
    restore_event_names: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct DesktopAssetImportSelectionRuntime {
    app_data_root: PathBuf,
    local_workspace_id: String,
    chunk_bytes: usize,
    document_body_policy: DocumentBodyPolicy,
    source: Arc<Mutex<LocalAssetImportSource>>,
    next_handle: Arc<Mutex<u64>>,
    next_operation: Arc<Mutex<u64>>,
    operation_prefix: u128,
}

impl DesktopAssetImportSelectionRuntime {
    pub fn new(max_chunk_bytes: usize) -> Result<Self, &'static str> {
        Self::with_app_data_root(PathBuf::new(), "workspace-1", max_chunk_bytes)
    }

    pub fn with_app_data_root(
        app_data_root: PathBuf,
        local_workspace_id: &str,
        max_chunk_bytes: usize,
    ) -> Result<Self, &'static str> {
        Self::with_app_data_root_and_body_policy(
            app_data_root,
            local_workspace_id,
            max_chunk_bytes,
            DocumentBodyPolicy::new(10 * 1024 * 1024)
                .map_err(|_| "asset_import.invalid_body_policy")?,
        )
    }

    pub fn with_app_data_root_and_body_policy(
        app_data_root: PathBuf,
        local_workspace_id: &str,
        max_chunk_bytes: usize,
        document_body_policy: DocumentBodyPolicy,
    ) -> Result<Self, &'static str> {
        let config =
            LocalAssetImportSourceConfig::new(max_chunk_bytes).map_err(|error| error.code())?;
        WorkspaceId::new(local_workspace_id).map_err(|_| "asset_import.invalid_workspace")?;
        if !app_data_root.as_os_str().is_empty() {
            let mut operations = DurableAssetImportOperationRepository::new(app_data_root.clone());
            let mut staging = LocalAssetStagingWriter::new(app_data_root.clone());
            let mut logger = DesktopAssetImportProductLogSink::default();
            RecoverAssetImportsUsecase::new()
                .execute(
                    RecoverAssetImportsInput::new(local_workspace_id, 500)
                        .map_err(|error| error.code())?,
                    &mut operations,
                    &mut staging,
                    &mut logger,
                )
                .map_err(|error| error.code())?;
        }
        let operation_prefix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Ok(Self {
            app_data_root,
            local_workspace_id: local_workspace_id.to_string(),
            chunk_bytes: max_chunk_bytes,
            document_body_policy,
            source: Arc::new(Mutex::new(LocalAssetImportSource::new(config))),
            next_handle: Arc::new(Mutex::new(0)),
            next_operation: Arc::new(Mutex::new(0)),
            operation_prefix,
        })
    }

    pub fn register_selected_paths(
        &self,
        paths: Vec<PathBuf>,
    ) -> DesktopAssetImportSelectionResponse {
        if paths.is_empty() {
            return DesktopAssetImportSelectionResponse::success(true, Vec::new());
        }
        let mut source = match self.source.lock() {
            Ok(source) => source,
            Err(_) => {
                return DesktopAssetImportSelectionResponse::failure(
                    "asset_import.read_unavailable",
                );
            }
        };
        let mut sequence = match self.next_handle.lock() {
            Ok(sequence) => sequence,
            Err(_) => {
                return DesktopAssetImportSelectionResponse::failure(
                    "asset_import.read_unavailable",
                );
            }
        };
        let mut files = Vec::with_capacity(paths.len());
        for path in paths {
            *sequence = sequence.saturating_add(1);
            let handle = match AssetImportHandle::new(&format!("picker:{}", *sequence)) {
                Ok(handle) => handle,
                Err(_) => {
                    return DesktopAssetImportSelectionResponse::failure(
                        "asset_import.invalid_handle",
                    );
                }
            };
            let descriptor = match source.register_selected_file(handle, &path) {
                Ok(descriptor) => descriptor,
                Err(error) => return DesktopAssetImportSelectionResponse::failure(error.code()),
            };
            files.push(DesktopAssetImportDescriptorDto {
                handle: descriptor.handle().as_str().to_string(),
                file_name: descriptor.file_name().as_str().to_string(),
                media_type: descriptor.media_type().as_str().to_string(),
                byte_size: descriptor.byte_size(),
            });
        }
        DesktopAssetImportSelectionResponse::success(false, files)
    }

    pub fn import(&self, request: DesktopAssetImportRequestDto) -> DesktopAssetImportResponse {
        let started = self.start(request.clone());
        match started.operation_id.as_deref() {
            Some(operation_id) => self.run_started(request, operation_id),
            None => started,
        }
    }

    pub fn import_revision_guarded(
        &self,
        request: DesktopRevisionGuardedAssetImportRequestDto,
    ) -> DesktopAssetImportResponse {
        let started = self.start_revision_guarded(&request);
        match started.operation_id.as_deref() {
            Some(operation_id) => self.run_started_revision_guarded(request, operation_id),
            None => started,
        }
    }

    pub fn start_revision_guarded(
        &self,
        request: &DesktopRevisionGuardedAssetImportRequestDto,
    ) -> DesktopAssetImportResponse {
        if DocumentOperationId::new(&request.attachment_operation_id).is_err()
            || VersionId::new(&request.expected_current_version_token).is_err()
        {
            return DesktopAssetImportResponse::failure(
                "asset_import.invalid_revision_guard",
                false,
            );
        }
        self.start(request.import.clone())
    }

    pub fn start(&self, request: DesktopAssetImportRequestDto) -> DesktopAssetImportResponse {
        if request.workspace_id != self.local_workspace_id {
            return DesktopAssetImportResponse::failure(
                "asset_import.workspace_scope_mismatch",
                false,
            );
        }
        let operation_id = match self.next_operation_id() {
            Ok(value) => value,
            Err(code) => return DesktopAssetImportResponse::failure(code, true),
        };
        if let Err(error) = ImportAssetInput::new(
            &request.workspace_id,
            &request.document_id,
            &operation_id,
            &request.handle,
            &request.label,
            self.chunk_bytes,
        ) {
            return DesktopAssetImportResponse::failure(error.code(), false);
        }
        let source = match self.source.lock() {
            Ok(value) => value,
            Err(_) => {
                return DesktopAssetImportResponse::failure(
                    "asset_import.runtime_unavailable",
                    true,
                );
            }
        };
        let handle = match AssetImportHandle::new(&request.handle) {
            Ok(value) => value,
            Err(_) => {
                return DesktopAssetImportResponse::failure("asset_import.invalid_handle", false);
            }
        };
        let descriptor = match source.describe(&handle) {
            Ok(value) => value,
            Err(error) => return DesktopAssetImportResponse::failure(error.code(), false),
        };
        drop(source);
        let operation = match AssetImportOperation::new(
            AssetImportOperationId::new(&operation_id).expect("generated operation id"),
            WorkspaceId::new(&request.workspace_id).expect("validated workspace"),
            DocumentId::new(&request.document_id).expect("validated document"),
            descriptor.byte_size(),
        ) {
            Ok(value) => value,
            Err(_) => {
                return DesktopAssetImportResponse::failure("asset_import.invalid_input", false);
            }
        };
        let mut operations = DurableAssetImportOperationRepository::new(self.app_data_root.clone());
        if let Err(error) = operations.create(operation) {
            return DesktopAssetImportResponse::failure(error.code(), true);
        }
        DesktopAssetImportResponse::accepted(&operation_id)
    }

    pub fn run_started(
        &self,
        request: DesktopAssetImportRequestDto,
        operation_id: &str,
    ) -> DesktopAssetImportResponse {
        let mut associations = DurableAssetAssociationCatalog::new(self.app_data_root.clone());
        self.run_started_with_linker(request, operation_id, &mut associations)
    }

    pub fn run_started_revision_guarded(
        &self,
        request: DesktopRevisionGuardedAssetImportRequestDto,
        operation_id: &str,
    ) -> DesktopAssetImportResponse {
        let mut linker = LocalImportedAssetDocumentRevisionLinker::new(
            LocalMutateDocumentAttachmentsRuntime::new(
                self.app_data_root.clone(),
                self.document_body_policy,
            ),
            &request.attachment_operation_id,
            &request.expected_current_version_token,
            "local-user",
            "첨부 파일 가져오기",
        );
        self.run_started_with_linker(request.import, operation_id, &mut linker)
    }

    fn run_started_with_linker<A: ImportedAssetDocumentLinkPort>(
        &self,
        request: DesktopAssetImportRequestDto,
        operation_id: &str,
        linker: &mut A,
    ) -> DesktopAssetImportResponse {
        let input = match ImportAssetInput::new(
            &request.workspace_id,
            &request.document_id,
            operation_id,
            &request.handle,
            &request.label,
            self.chunk_bytes,
        ) {
            Ok(value) => value,
            Err(error) => return DesktopAssetImportResponse::failure(error.code(), false),
        };
        let source = match self.source.lock() {
            Ok(value) => value,
            Err(_) => {
                return DesktopAssetImportResponse::failure(
                    "asset_import.runtime_unavailable",
                    true,
                );
            }
        };
        let documents = LocalDocumentRepository::new(self.app_data_root.join("authoring-current"));
        let mut staging = LocalAssetStagingWriter::new(self.app_data_root.clone());
        let mut publisher = match LocalContentAddressedAssetPublisher::new(
            self.app_data_root.clone(),
            self.chunk_bytes,
        ) {
            Ok(value) => value,
            Err(error) => return DesktopAssetImportResponse::failure(error.code(), false),
        };
        let mut metadata = DurableAssetMetadataCatalog::new(self.app_data_root.clone());
        let mut operations = DurableAssetImportOperationRepository::new(self.app_data_root.clone());
        let mut logger = DesktopAssetImportProductLogSink::default();
        match ImportAssetUsecase::new().execute(
            input,
            &documents,
            &*source,
            &mut staging,
            &mut publisher,
            &mut metadata,
            linker,
            &mut operations,
            &mut logger,
        ) {
            Ok(output) => match request_asset_graph_reindex(
                &self.app_data_root,
                &request.workspace_id,
                &request.document_id,
                ProjectionChangeKind::AssetAttached,
            ) {
                Ok(()) => DesktopAssetImportResponse::completed(
                    output.operation_id().as_str(),
                    output.asset_id().as_str(),
                ),
                Err(error) => DesktopAssetImportResponse::completed_with_projection_warning(
                    output.operation_id().as_str(),
                    output.asset_id().as_str(),
                    error,
                ),
            },
            Err(error) => DesktopAssetImportResponse::failure(
                error.code(),
                is_retryable_asset_import_error(error),
            ),
        }
    }

    pub fn status(&self, workspace_id: &str, operation_id: &str) -> DesktopAssetImportResponse {
        if workspace_id != self.local_workspace_id {
            return DesktopAssetImportResponse::failure(
                "asset_import.workspace_scope_mismatch",
                false,
            );
        }
        let operation_id = match AssetImportOperationId::new(operation_id) {
            Ok(value) => value,
            Err(_) => {
                return DesktopAssetImportResponse::failure("asset_import.invalid_input", false);
            }
        };
        match DurableAssetImportOperationRepository::new(self.app_data_root.clone())
            .get(&operation_id)
        {
            Ok(Some(operation)) if operation.workspace_id().as_str() == workspace_id => {
                if operation.state() == AssetImportState::Completed {
                    match ensure_asset_graph_reindex(
                        &self.app_data_root,
                        workspace_id,
                        operation.document_id().as_str(),
                        ProjectionChangeKind::AssetAttached,
                    ) {
                        Ok(()) => DesktopAssetImportResponse::operation(&operation),
                        Err(error) => {
                            DesktopAssetImportResponse::operation_with_projection_warning(
                                &operation, error,
                            )
                        }
                    }
                } else {
                    DesktopAssetImportResponse::operation(&operation)
                }
            }
            Ok(_) => DesktopAssetImportResponse::failure("asset_import.operation_not_found", false),
            Err(error) => DesktopAssetImportResponse::failure(error.code(), true),
        }
    }

    pub fn cancel(&self, workspace_id: &str, operation_id: &str) -> DesktopAssetImportResponse {
        if workspace_id != self.local_workspace_id {
            return DesktopAssetImportResponse::failure(
                "asset_import.workspace_scope_mismatch",
                false,
            );
        }
        let input = match CancelAssetImportInput::new(workspace_id, operation_id) {
            Ok(value) => value,
            Err(error) => return DesktopAssetImportResponse::failure(error.code(), false),
        };
        let mut operations = DurableAssetImportOperationRepository::new(self.app_data_root.clone());
        let mut staging = LocalAssetStagingWriter::new(self.app_data_root.clone());
        let mut logger = DesktopAssetImportProductLogSink::default();
        match CancelAssetImportUsecase::new().execute(
            input,
            &mut operations,
            &mut staging,
            &mut logger,
        ) {
            Ok(operation) => DesktopAssetImportResponse::operation(&operation),
            Err(error) => DesktopAssetImportResponse::failure(error.code(), true),
        }
    }

    fn next_operation_id(&self) -> Result<String, &'static str> {
        let mut sequence = self
            .next_operation
            .lock()
            .map_err(|_| "asset_import.runtime_unavailable")?;
        *sequence = sequence
            .checked_add(1)
            .ok_or("asset_import.operation_id_exhausted")?;
        Ok(format!(
            "asset-import-{}-{}",
            self.operation_prefix, *sequence
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetImportRequestDto {
    pub workspace_id: String,
    pub document_id: String,
    pub handle: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopRevisionGuardedAssetImportRequestDto {
    #[serde(flatten)]
    pub import: DesktopAssetImportRequestDto,
    pub attachment_operation_id: String,
    pub expected_current_version_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetImportOperationRequestDto {
    pub workspace_id: String,
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetImportResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
    pub repair_required: bool,
}

impl DesktopAssetImportResponse {
    fn accepted(operation_id: &str) -> Self {
        Self {
            ok: true,
            operation_id: Some(operation_id.to_string()),
            asset_id: None,
            state: Some("selected".to_string()),
            error_code: None,
            retryable: false,
            repair_required: false,
        }
    }

    fn operation(operation: &AssetImportOperation) -> Self {
        Self {
            ok: true,
            operation_id: Some(operation.operation_id().as_str().to_string()),
            asset_id: None,
            state: Some(asset_import_state_name(operation.state()).to_string()),
            error_code: None,
            retryable: false,
            repair_required: false,
        }
    }

    fn completed(operation_id: &str, asset_id: &str) -> Self {
        Self {
            ok: true,
            operation_id: Some(operation_id.to_string()),
            asset_id: Some(asset_id.to_string()),
            state: Some("completed".to_string()),
            error_code: None,
            retryable: false,
            repair_required: false,
        }
    }

    fn completed_with_projection_warning(
        operation_id: &str,
        asset_id: &str,
        error: ReindexAssetGraphProjectionError,
    ) -> Self {
        Self {
            ok: true,
            operation_id: Some(operation_id.to_string()),
            asset_id: Some(asset_id.to_string()),
            state: Some("recovery_required".to_string()),
            error_code: Some(error.code().to_string()),
            retryable: error.retryable(),
            repair_required: true,
        }
    }

    fn operation_with_projection_warning(
        operation: &AssetImportOperation,
        error: ReindexAssetGraphProjectionError,
    ) -> Self {
        Self {
            ok: true,
            operation_id: Some(operation.operation_id().as_str().to_string()),
            asset_id: None,
            state: Some("recovery_required".to_string()),
            error_code: Some(error.code().to_string()),
            retryable: error.retryable(),
            repair_required: true,
        }
    }

    fn failure(error_code: &str, retryable: bool) -> Self {
        Self {
            ok: false,
            operation_id: None,
            asset_id: None,
            state: Some("failed".to_string()),
            error_code: Some(error_code.to_string()),
            retryable,
            repair_required: false,
        }
    }
}

const fn asset_import_state_name(state: AssetImportState) -> &'static str {
    use AssetImportState::*;
    match state {
        Selected => "selected",
        Validating => "validating",
        Staging => "staging",
        Hashing => "hashing",
        PublishingObject => "publishing_object",
        PersistingMetadata => "persisting_metadata",
        Linking => "linking",
        Completed => "completed",
        ValidationFailed => "validation_failed",
        StagingFailed => "staging_failed",
        ObjectPublishFailed => "object_publish_failed",
        MetadataPersistFailed => "metadata_persist_failed",
        LinkFailed => "link_failed",
        Cancelling => "cancelling",
        Cancelled => "cancelled",
        CleanupRequired => "cleanup_required",
    }
}

#[derive(Default)]
struct DesktopAssetImportProductLogSink {
    event_count: usize,
    last_error_code: Option<&'static str>,
}

impl ImportAssetProductLogger for DesktopAssetImportProductLogSink {
    fn write_product(&mut self, event: ImportAssetProductEvent) {
        self.event_count = self.event_count.saturating_add(1);
        self.last_error_code = match event {
            ImportAssetProductEvent::Completed { .. } => None,
            ImportAssetProductEvent::Failed { error_code, .. } => Some(error_code),
        };
    }
}

fn is_retryable_asset_import_error(error: ImportAssetError) -> bool {
    matches!(
        error,
        ImportAssetError::Document(_)
            | ImportAssetError::Source(_)
            | ImportAssetError::Staging(_)
            | ImportAssetError::Publish(_)
            | ImportAssetError::Metadata(_)
            | ImportAssetError::Association(_)
            | ImportAssetError::Repository(_)
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetImportSelectionResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DesktopAssetImportSelectionDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
}

impl DesktopAssetImportSelectionResponse {
    fn success(cancelled: bool, files: Vec<DesktopAssetImportDescriptorDto>) -> Self {
        Self {
            ok: true,
            data: Some(DesktopAssetImportSelectionDataDto { cancelled, files }),
            error_code: None,
            retryable: false,
        }
    }

    pub fn failure(error_code: &str) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(error_code.to_string()),
            retryable: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetImportSelectionDataDto {
    pub cancelled: bool,
    pub files: Vec<DesktopAssetImportDescriptorDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetImportDescriptorDto {
    pub handle: String,
    pub file_name: String,
    pub media_type: String,
    pub byte_size: u64,
}

impl DocumentProductLogger for DesktopDocumentProductLogSink {
    fn write_product(&mut self, event: CreateDocumentProductEvent) {
        self.event_count += 1;
        self.last_error_code = match event {
            CreateDocumentProductEvent::UsecaseFailed { error_code } => Some(error_code),
            _ => None,
        };
    }
}

impl RestoreProductLogger for DesktopDocumentProductLogSink {
    fn write_restore_product(&mut self, event: RestoreProductEvent) {
        self.event_count += 1;
        self.restore_event_names.push(event.name());
    }
}

#[derive(Debug, Clone)]
pub struct DesktopDocumentNavigatorRuntime {
    projection_store: LocalDocumentNavigatorProjectionStore,
}

impl DesktopDocumentNavigatorRuntime {
    pub fn new(projection_root: PathBuf, capacity: usize) -> Result<Self, &'static str> {
        Ok(Self {
            projection_store: LocalDocumentNavigatorProjectionStore::new(projection_root, capacity)
                .map_err(|error| error.code())?,
        })
    }

    pub fn execute(
        &self,
        request: DesktopDocumentNavigatorRequestDto,
    ) -> DesktopDocumentNavigatorCommandResponse {
        let Some(view) = parse_navigator_view(&request.view) else {
            return DesktopDocumentNavigatorCommandResponse::invalid_input();
        };
        match execute_document_navigator_command(
            DocumentNavigatorCommandRequest {
                workspace_id: request.workspace_id,
                view,
                view_key: request.view_key,
                filter: request.filter,
                limit: request.limit,
                cursor: request.cursor,
            },
            &self.projection_store,
        ) {
            Ok(result) => DesktopDocumentNavigatorCommandResponse::success(result),
            Err(error) => DesktopDocumentNavigatorCommandResponse::failure(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DesktopDocumentNavigatorRequestDto {
    pub workspace_id: String,
    pub view: String,
    pub view_key: Option<String>,
    pub filter: Option<String>,
    pub limit: u16,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentNavigatorCommandResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DesktopDocumentNavigatorDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
}

impl DesktopDocumentNavigatorCommandResponse {
    fn success(result: DocumentNavigatorCommandResult) -> Self {
        Self {
            ok: true,
            data: Some(DesktopDocumentNavigatorDataDto::from(result)),
            error_code: None,
            retryable: false,
        }
    }

    fn failure(error: DocumentNavigatorCommandFailure) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(error.error_code.to_string()),
            retryable: error.retryable,
        }
    }

    fn invalid_input() -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some("DOCUMENT_NAVIGATOR_INVALID_INPUT".to_string()),
            retryable: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentNavigatorDataDto {
    pub workspace_id: String,
    pub view: String,
    pub state: String,
    pub items: Vec<DesktopDocumentNavigatorItemDto>,
    pub next_cursor: Option<String>,
}

impl From<DocumentNavigatorCommandResult> for DesktopDocumentNavigatorDataDto {
    fn from(result: DocumentNavigatorCommandResult) -> Self {
        Self {
            workspace_id: result.workspace_id,
            view: navigator_view_name(result.view).to_string(),
            state: match result.state {
                DocumentNavigatorCommandLoadState::Ready => "Ready",
                DocumentNavigatorCommandLoadState::EmptyResult => "EmptyResult",
                DocumentNavigatorCommandLoadState::Degraded => "Degraded",
            }
            .to_string(),
            items: result
                .items
                .into_iter()
                .map(|item| DesktopDocumentNavigatorItemDto {
                    document_id: item.document_id,
                    title: item.title,
                    path: item.path,
                    collections: item.collections,
                    tags: item.tags,
                    favorite: item.favorite,
                })
                .collect(),
            next_cursor: result.next_cursor,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentNavigatorItemDto {
    pub document_id: String,
    pub title: String,
    pub path: String,
    pub collections: Vec<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
}

#[derive(Debug, Clone)]
pub struct DesktopDocumentSearchRuntime {
    search_index: DurableLocalSearchIndex,
}

impl DesktopDocumentSearchRuntime {
    pub fn new(root: PathBuf, body_policy: DocumentBodyPolicy) -> Self {
        Self {
            search_index: DurableLocalSearchIndex::new(root, body_policy),
        }
    }

    pub fn execute(
        &self,
        request: DesktopDocumentSearchRequestDto,
    ) -> DesktopDocumentSearchCommandResponse {
        let workspace_id = request.workspace_id.clone();
        let text = request.text.clone();
        match SearchDocumentsUsecase::new().execute(
            SearchDocumentsInput::new(&workspace_id, &text, request.limit),
            &self.search_index,
        ) {
            Ok(output) => {
                DesktopDocumentSearchCommandResponse::success(workspace_id, text, output.page())
            }
            Err(error) => DesktopDocumentSearchCommandResponse::failure(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DesktopDocumentSearchRequestDto {
    pub workspace_id: String,
    pub text: String,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentSearchCommandResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DesktopDocumentSearchDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
}

impl DesktopDocumentSearchCommandResponse {
    fn success(
        workspace_id: String,
        text: String,
        page: &cabinet_ports::search_index::SearchPage,
    ) -> Self {
        Self {
            ok: true,
            data: Some(DesktopDocumentSearchDataDto {
                query_name: "search-documents".to_string(),
                workspace_id: workspace_id.clone(),
                text,
                results: page
                    .results()
                    .iter()
                    .map(|result| DesktopDocumentSearchResultDto {
                        workspace_id: workspace_id.clone(),
                        document_id: result.document_id().as_str().to_string(),
                        title: result.title().as_str().to_string(),
                        path: result.path().as_str().to_string(),
                        snippet: result.snippet().to_string(),
                        score: result.score(),
                    })
                    .collect(),
            }),
            error_code: None,
            retryable: false,
        }
    }

    fn failure(error: SearchDocumentsError) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(
                match error {
                    SearchDocumentsError::InvalidInput => "SEARCH_INVALID_INPUT",
                    SearchDocumentsError::StorageUnavailable => "SEARCH_INDEX_UNAVAILABLE",
                }
                .to_string(),
            ),
            retryable: error == SearchDocumentsError::StorageUnavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentSearchDataDto {
    pub query_name: String,
    pub workspace_id: String,
    pub text: String,
    pub results: Vec<DesktopDocumentSearchResultDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentSearchResultDto {
    pub workspace_id: String,
    pub document_id: String,
    pub title: String,
    pub path: String,
    pub snippet: String,
    pub score: u32,
}

#[derive(Debug, Clone)]
pub struct DesktopAssetSearchRuntime {
    metadata: DurableAssetMetadataCatalog,
}

impl DesktopAssetSearchRuntime {
    pub fn new(root: PathBuf) -> Self {
        Self {
            metadata: DurableAssetMetadataCatalog::new(root),
        }
    }

    pub fn execute(&self, request: DesktopAssetSearchRequestDto) -> DesktopAssetSearchResponse {
        let workspace_id = request.workspace_id.clone();
        let workspace = match WorkspaceId::new(&workspace_id) {
            Ok(value) => value,
            Err(_) => {
                return DesktopAssetSearchResponse::failure(AssetSearchCommandFailure {
                    error_code: "ASSET_SEARCH_INVALID_INPUT",
                    retryable: false,
                    product_log_event_name: None,
                });
            }
        };
        let mut cursor = None;
        let mut index = LocalAssetSearchIndex::default();
        loop {
            let page = match self.metadata.list(&workspace, cursor.as_deref(), 100) {
                Ok(page) => page,
                Err(_) => {
                    return DesktopAssetSearchResponse::failure(AssetSearchCommandFailure {
                        error_code: "ASSET_SEARCH_STORAGE_UNAVAILABLE",
                        retryable: true,
                        product_log_event_name: None,
                    });
                }
            };
            for record in page.records() {
                index.upsert_asset(&workspace, record.clone());
            }
            cursor = page.next_cursor().map(str::to_string);
            if cursor.is_none() {
                break;
            }
        }

        match execute_asset_search_command(
            AssetSearchCommandRequest {
                workspace_id,
                text: request.text,
                limit: request.limit as u16,
            },
            &index,
        ) {
            Ok(output) => DesktopAssetSearchResponse::success(output),
            Err(error) => DesktopAssetSearchResponse::failure(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DesktopAssetSearchRequestDto {
    pub workspace_id: String,
    pub text: String,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetSearchResponse {
    pub ok: bool,
    pub data: Option<DesktopAssetSearchDataDto>,
    pub error_code: Option<String>,
    pub retryable: bool,
}

impl DesktopAssetSearchResponse {
    fn success(output: cabinet_platform::asset_search_command::AssetSearchCommandResult) -> Self {
        Self {
            ok: true,
            data: Some(DesktopAssetSearchDataDto {
                query_name: "search-assets".to_string(),
                workspace_id: output.workspace_id,
                text: output.text,
                results: output
                    .results
                    .into_iter()
                    .map(|result| DesktopAssetSearchResultDto {
                        asset_id: result.asset_id,
                        file_name: result.file_name,
                        media_type: result.media_type,
                        byte_size: result.byte_size,
                        score: result.score,
                    })
                    .collect(),
            }),
            error_code: None,
            retryable: false,
        }
    }

    fn failure(error: AssetSearchCommandFailure) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(error.error_code.to_string()),
            retryable: error.retryable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetSearchDataDto {
    pub query_name: String,
    pub workspace_id: String,
    pub text: String,
    pub results: Vec<DesktopAssetSearchResultDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetSearchResultDto {
    pub asset_id: String,
    pub file_name: String,
    pub media_type: String,
    pub byte_size: u64,
    pub score: u32,
}

fn parse_navigator_view(value: &str) -> Option<DocumentNavigatorCommandView> {
    match value {
        "Tree" => Some(DocumentNavigatorCommandView::Tree),
        "Collection" => Some(DocumentNavigatorCommandView::Collection),
        "Tag" => Some(DocumentNavigatorCommandView::Tag),
        "Recent" => Some(DocumentNavigatorCommandView::Recent),
        "Favorite" => Some(DocumentNavigatorCommandView::Favorite),
        _ => None,
    }
}

const fn navigator_view_name(view: DocumentNavigatorCommandView) -> &'static str {
    match view {
        DocumentNavigatorCommandView::Tree => "Tree",
        DocumentNavigatorCommandView::Collection => "Collection",
        DocumentNavigatorCommandView::Tag => "Tag",
        DocumentNavigatorCommandView::Recent => "Recent",
        DocumentNavigatorCommandView::Favorite => "Favorite",
    }
}

pub struct DesktopDocumentAttachmentMutationRuntime {
    app_data_root: PathBuf,
    runtime: Mutex<LocalMutateDocumentAttachmentsRuntime>,
    metadata: DurableAssetMetadataCatalog,
}

impl DesktopDocumentAttachmentMutationRuntime {
    pub fn new(app_data_root: PathBuf, max_body_bytes: usize) -> Result<Self, &'static str> {
        let body_policy = DocumentBodyPolicy::new(max_body_bytes)
            .map_err(|_| "DOCUMENT_ATTACHMENT_INVALID_BODY_POLICY")?;
        let mut runtime =
            LocalMutateDocumentAttachmentsRuntime::new(app_data_root.clone(), body_policy);
        let recovered = runtime
            .recover_committed(1000)
            .map_err(|_| "DOCUMENT_ATTACHMENT_STARTUP_RECOVERY_FAILED")?;
        for candidate in recovered.recovered() {
            request_authoritative_asset_graph_reindex(
                &app_data_root,
                candidate.workspace_id().as_str(),
                candidate.document_id().as_str(),
                candidate.change_kind(),
            )
            .map_err(|_| "DOCUMENT_ATTACHMENT_STARTUP_RECOVERY_FAILED")?;
        }
        Ok(Self {
            app_data_root: app_data_root.clone(),
            runtime: Mutex::new(runtime),
            metadata: DurableAssetMetadataCatalog::new(app_data_root),
        })
    }

    pub fn execute(
        &self,
        request: DesktopDocumentAttachmentMutationRequestDto,
    ) -> DesktopDocumentAttachmentMutationResponse {
        let (input, workspace_id, document_id) = match request {
            DesktopDocumentAttachmentMutationRequestDto::Link {
                operation_id,
                workspace_id,
                document_id,
                expected_current_version_token,
                asset_id,
                label,
            } => {
                let workspace = match WorkspaceId::new(&workspace_id) {
                    Ok(value) => value,
                    Err(_) => return DesktopDocumentAttachmentMutationResponse::invalid_input(),
                };
                let asset = match cabinet_domain::asset::AssetId::from_sha256_hex(&asset_id) {
                    Ok(value) => value,
                    Err(_) => return DesktopDocumentAttachmentMutationResponse::invalid_input(),
                };
                match self.metadata.get(&workspace, &asset) {
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        return DesktopDocumentAttachmentMutationResponse::failure(
                            "DOCUMENT_ATTACHMENT_ASSET_NOT_FOUND",
                            false,
                            false,
                        );
                    }
                    Err(_) => {
                        return DesktopDocumentAttachmentMutationResponse::failure(
                            "DOCUMENT_ATTACHMENT_STORAGE_UNAVAILABLE",
                            true,
                            false,
                        );
                    }
                }
                (
                    MutateDocumentAttachmentsInput::link(
                        &operation_id,
                        &workspace_id,
                        &document_id,
                        &expected_current_version_token,
                        &asset_id,
                        &label,
                        "local-user",
                        "첨부 파일 연결",
                    ),
                    workspace_id,
                    document_id,
                )
            }
            DesktopDocumentAttachmentMutationRequestDto::Unlink {
                operation_id,
                workspace_id,
                document_id,
                expected_current_version_token,
                asset_id,
            } => (
                MutateDocumentAttachmentsInput::unlink(
                    &operation_id,
                    &workspace_id,
                    &document_id,
                    &expected_current_version_token,
                    &asset_id,
                    "local-user",
                    "첨부 파일 해제",
                ),
                workspace_id,
                document_id,
            ),
        };
        let mut runtime = match self.runtime.lock() {
            Ok(value) => value,
            Err(_) => {
                return DesktopDocumentAttachmentMutationResponse::failure(
                    "DOCUMENT_ATTACHMENT_RECOVERY_REQUIRED",
                    true,
                    true,
                );
            }
        };
        match runtime.execute(input) {
            Ok(output) => {
                let delta = output.delta();
                let change_kind = match delta {
                    AttachmentSnapshotDelta::Unlinked => ProjectionChangeKind::AssetDetached,
                    AttachmentSnapshotDelta::Linked
                    | AttachmentSnapshotDelta::Relabeled
                    | AttachmentSnapshotDelta::Unchanged => ProjectionChangeKind::AssetAttached,
                };
                if request_authoritative_asset_graph_reindex(
                    &self.app_data_root,
                    &workspace_id,
                    &document_id,
                    change_kind,
                )
                .is_err()
                {
                    return DesktopDocumentAttachmentMutationResponse::failure(
                        "DOCUMENT_ATTACHMENT_RECOVERY_REQUIRED",
                        true,
                        true,
                    );
                }
                let outcome_name = match output.kind() {
                    MutateDocumentAttachmentsOutcomeKind::Fresh => "fresh",
                    MutateDocumentAttachmentsOutcomeKind::Replayed => "replayed",
                    MutateDocumentAttachmentsOutcomeKind::NoChange => "no_change",
                };
                let delta_name = match delta {
                    AttachmentSnapshotDelta::Linked => "linked",
                    AttachmentSnapshotDelta::Relabeled => "relabeled",
                    AttachmentSnapshotDelta::Unlinked => "unlinked",
                    AttachmentSnapshotDelta::Unchanged => "unchanged",
                };
                DesktopDocumentAttachmentMutationResponse::success(
                    outcome_name,
                    delta_name,
                    output.revision_number().value(),
                )
            }
            Err(error) => map_document_attachment_mutation_error(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum DesktopDocumentAttachmentMutationRequestDto {
    Link {
        operation_id: String,
        workspace_id: String,
        document_id: String,
        expected_current_version_token: String,
        asset_id: String,
        label: String,
    },
    Unlink {
        operation_id: String,
        workspace_id: String,
        document_id: String,
        expected_current_version_token: String,
        asset_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentAttachmentMutationResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
    pub repair_required: bool,
}

impl DesktopDocumentAttachmentMutationResponse {
    fn success(outcome: &str, delta: &str, revision_number: u64) -> Self {
        Self {
            ok: true,
            outcome: Some(outcome.to_string()),
            delta: Some(delta.to_string()),
            revision_number: Some(revision_number),
            error_code: None,
            retryable: false,
            repair_required: false,
        }
    }

    fn invalid_input() -> Self {
        Self::failure("DOCUMENT_ATTACHMENT_INVALID_INPUT", false, false)
    }

    fn failure(error_code: &str, retryable: bool, repair_required: bool) -> Self {
        Self {
            ok: false,
            outcome: None,
            delta: None,
            revision_number: None,
            error_code: Some(error_code.to_string()),
            retryable,
            repair_required,
        }
    }
}

fn map_document_attachment_mutation_error(
    error: MutateDocumentAttachmentsError,
) -> DesktopDocumentAttachmentMutationResponse {
    match error {
        MutateDocumentAttachmentsError::InvalidInput => {
            DesktopDocumentAttachmentMutationResponse::invalid_input()
        }
        MutateDocumentAttachmentsError::NotFound => {
            DesktopDocumentAttachmentMutationResponse::failure(
                "DOCUMENT_ATTACHMENT_DOCUMENT_NOT_FOUND",
                false,
                false,
            )
        }
        MutateDocumentAttachmentsError::LegacyBaselineRequired => {
            DesktopDocumentAttachmentMutationResponse::failure(
                "DOCUMENT_ATTACHMENT_LEGACY_BASELINE_REQUIRED",
                false,
                true,
            )
        }
        MutateDocumentAttachmentsError::OperationConflict
        | MutateDocumentAttachmentsError::CommitConflict => {
            DesktopDocumentAttachmentMutationResponse::failure(
                "DOCUMENT_ATTACHMENT_CONFLICT",
                false,
                false,
            )
        }
        MutateDocumentAttachmentsError::RecoveryRequired => {
            DesktopDocumentAttachmentMutationResponse::failure(
                "DOCUMENT_ATTACHMENT_RECOVERY_REQUIRED",
                true,
                true,
            )
        }
        MutateDocumentAttachmentsError::CorruptedData => {
            DesktopDocumentAttachmentMutationResponse::failure(
                "DOCUMENT_ATTACHMENT_CORRUPTED_DATA",
                false,
                true,
            )
        }
        MutateDocumentAttachmentsError::StorageUnavailable
        | MutateDocumentAttachmentsError::FingerprintUnavailable
        | MutateDocumentAttachmentsError::MetadataUnavailable
        | MutateDocumentAttachmentsError::JournalUnavailable
        | MutateDocumentAttachmentsError::CommitUnavailable => {
            DesktopDocumentAttachmentMutationResponse::failure(
                "DOCUMENT_ATTACHMENT_STORAGE_UNAVAILABLE",
                true,
                false,
            )
        }
    }
}

#[derive(Clone)]
pub struct DesktopDocumentAssetsRuntime {
    app_data_root: PathBuf,
    documents: LocalDocumentRepository,
    metadata: DurableAssetMetadataCatalog,
    page_limit: usize,
    preview_max_bytes: usize,
    external_opener: Arc<dyn AssetExternalOpener>,
}

impl DesktopDocumentAssetsRuntime {
    pub fn new(app_data_root: PathBuf, max_body_bytes: usize) -> Result<Self, &'static str> {
        Self::with_preview_limit(app_data_root, max_body_bytes, 2 * 1024 * 1024)
    }

    pub fn with_preview_limit(
        app_data_root: PathBuf,
        max_body_bytes: usize,
        preview_max_bytes: usize,
    ) -> Result<Self, &'static str> {
        Self::with_preview_limit_and_opener(
            app_data_root.clone(),
            max_body_bytes,
            preview_max_bytes,
            Arc::new(LocalAssetExternalOpener::new(app_data_root)),
        )
    }

    pub fn with_preview_limit_and_opener(
        app_data_root: PathBuf,
        max_body_bytes: usize,
        preview_max_bytes: usize,
        external_opener: Arc<dyn AssetExternalOpener>,
    ) -> Result<Self, &'static str> {
        if preview_max_bytes == 0 {
            return Err("ASSET_INVALID_PREVIEW_POLICY");
        }
        let policy =
            DocumentBodyPolicy::new(max_body_bytes).map_err(|_| "ASSET_INVALID_BODY_POLICY")?;
        Ok(Self {
            app_data_root: app_data_root.clone(),
            documents: LocalDocumentRepository::with_body_policy(
                app_data_root.join("authoring-current"),
                policy,
            ),
            metadata: DurableAssetMetadataCatalog::new(app_data_root.clone()),
            page_limit: 200,
            preview_max_bytes,
            external_opener,
        })
    }

    pub fn execute(
        &self,
        request: DesktopLocalCommandRequestDto,
    ) -> DesktopDocumentAssetsCommandResponse {
        let (input, workspace_id, document_id) =
            match map_core_local_desktop_command_request(to_platform_runtime_request(request)) {
                Ok(LocalDesktopUsecaseInput::ListDocumentAssets {
                    workspace_id,
                    document_id,
                }) => (
                    ListCatalogDocumentAssetsInput::new(
                        &workspace_id,
                        &document_id,
                        self.page_limit,
                    ),
                    workspace_id,
                    document_id,
                ),
                _ => {
                    return DesktopDocumentAssetsCommandResponse::failure(
                        ListCatalogDocumentAssetsError::InvalidInput,
                    );
                }
            };
        let input = match input {
            Ok(value) => value,
            Err(error) => return DesktopDocumentAssetsCommandResponse::failure(error),
        };
        match ListCatalogDocumentAssetsUsecase::new().execute(
            input,
            &self.documents,
            &DurableAssetAssociationCatalog::new(self.app_data_root.clone()),
            &self.metadata,
        ) {
            Ok(output) => {
                DesktopDocumentAssetsCommandResponse::success(&workspace_id, &document_id, output)
            }
            Err(error) => DesktopDocumentAssetsCommandResponse::failure(error),
        }
    }

    pub fn detail(&self, request: DesktopAssetDetailRequestDto) -> DesktopAssetDetailResponse {
        let input = match GetAssetDetailInput::new(
            &request.workspace_id,
            &request.asset_id,
            self.page_limit,
        ) {
            Ok(value) => value,
            Err(error) => return DesktopAssetDetailResponse::failure(error),
        };
        match GetAssetDetailUsecase::new().execute(
            input,
            &self.metadata,
            &DurableAssetAssociationCatalog::new(self.app_data_root.clone()),
        ) {
            Ok(output) => {
                let workspace_id = match WorkspaceId::new(&request.workspace_id) {
                    Ok(value) => value,
                    Err(_) => return DesktopAssetDetailResponse::invalid_title_request(),
                };
                let document_ids: Vec<DocumentId> = output
                    .linked_documents()
                    .iter()
                    .map(|association| association.document_id().clone())
                    .collect();
                let linked_documents = match self
                    .documents
                    .get_current_titles(&workspace_id, &document_ids)
                {
                    Ok(lookups) => lookups
                        .into_iter()
                        .map(|lookup| DesktopAssetLinkedDocumentDto {
                            document_id: lookup.document_id().as_str().to_string(),
                            title: lookup.title().map(|title| title.as_str().to_string()),
                            state: if lookup.title().is_some() {
                                "available".to_string()
                            } else {
                                "missing".to_string()
                            },
                        })
                        .collect(),
                    Err(_) => return DesktopAssetDetailResponse::title_resolution_failure(),
                };
                DesktopAssetDetailResponse::success(output, linked_documents)
            }
            Err(error) => DesktopAssetDetailResponse::failure(error),
        }
    }

    pub fn preview(&self, request: DesktopAssetDetailRequestDto) -> DesktopAssetPreviewResponse {
        let input = match GetAssetPreviewInput::new(
            &request.workspace_id,
            &request.asset_id,
            self.preview_max_bytes,
        ) {
            Ok(value) => value,
            Err(error) => return DesktopAssetPreviewResponse::failure(error),
        };
        match GetAssetPreviewUsecase::new().execute(
            input,
            &self.metadata,
            &LocalAssetPreviewReader::new(self.app_data_root.clone()),
        ) {
            Ok(output) => DesktopAssetPreviewResponse::success(&request.asset_id, output),
            Err(error) => DesktopAssetPreviewResponse::failure(error),
        }
    }

    pub fn open_external(
        &self,
        request: DesktopAssetDetailRequestDto,
    ) -> DesktopAssetExternalOpenResponse {
        let input = match OpenAssetExternallyInput::new(&request.workspace_id, &request.asset_id) {
            Ok(value) => value,
            Err(error) => return DesktopAssetExternalOpenResponse::failure(error),
        };
        let mut product_logger = DesktopAssetExternalOpenProductLogSink::default();
        match OpenAssetExternallyUsecase::new().execute(
            input,
            &self.metadata,
            self.external_opener.as_ref(),
            &mut product_logger,
        ) {
            Ok(output) => DesktopAssetExternalOpenResponse::success(output.opened()),
            Err(error) => DesktopAssetExternalOpenResponse::failure(error),
        }
    }

    pub fn list_workspace(
        &self,
        request: DesktopWorkspaceAssetsRequestDto,
    ) -> DesktopWorkspaceAssetsResponse {
        let input = match ListWorkspaceAssetsInput::new(
            &request.workspace_id,
            request.cursor.as_deref(),
            request.limit,
        ) {
            Ok(value) => value,
            Err(error) => return DesktopWorkspaceAssetsResponse::failure(error),
        };
        match ListWorkspaceAssetsUsecase::new().execute(input, &self.metadata) {
            Ok(output) => DesktopWorkspaceAssetsResponse::success(&request.workspace_id, output),
            Err(error) => DesktopWorkspaceAssetsResponse::failure(error),
        }
    }

    pub fn link(&self, request: DesktopAssetLinkRequestDto) -> DesktopAssetLinkResponse {
        let input = match LinkAssetInput::new(
            &request.workspace_id,
            &request.document_id,
            &request.asset_id,
            &request.label,
        ) {
            Ok(value) => value,
            Err(error) => return DesktopAssetLinkResponse::failure(error),
        };
        let mut associations = DurableAssetAssociationCatalog::new(self.app_data_root.clone());
        let mut logger = DesktopAssetLifecycleProductLogSink::default();
        match LinkAssetUsecase::new().execute(
            input,
            &self.documents,
            &self.metadata,
            &mut associations,
            &mut logger,
        ) {
            Ok(output) => {
                if let Err(error) = request_asset_graph_reindex(
                    &self.app_data_root,
                    &request.workspace_id,
                    &request.document_id,
                    ProjectionChangeKind::AssetAttached,
                ) {
                    return DesktopAssetLinkResponse::projection_failure(error);
                }
                DesktopAssetLinkResponse {
                    ok: true,
                    linked: output.linked(),
                    reference_count: output.reference_count(),
                    error_code: None,
                    retryable: false,
                }
            }
            Err(error) => DesktopAssetLinkResponse::failure(error),
        }
    }

    pub fn unlink(&self, request: DesktopAssetUnlinkRequestDto) -> DesktopAssetUnlinkResponse {
        let input = match UnlinkAssetInput::new(
            &request.workspace_id,
            &request.document_id,
            &request.asset_id,
        ) {
            Ok(value) => value,
            Err(error) => return DesktopAssetUnlinkResponse::failure(error),
        };
        let mut associations = DurableAssetAssociationCatalog::new(self.app_data_root.clone());
        let mut logger = DesktopAssetLifecycleProductLogSink::default();
        match UnlinkAssetUsecase::new().execute(
            input,
            &self.documents,
            &self.metadata,
            &mut associations,
            &mut logger,
        ) {
            Ok(output) => {
                if let Err(error) = request_asset_graph_reindex(
                    &self.app_data_root,
                    &request.workspace_id,
                    &request.document_id,
                    ProjectionChangeKind::AssetDetached,
                ) {
                    return DesktopAssetUnlinkResponse::projection_failure(error);
                }
                DesktopAssetUnlinkResponse {
                    ok: true,
                    removed: output.removed(),
                    remaining_references: output.remaining_references(),
                    error_code: None,
                    retryable: false,
                }
            }
            Err(error) => DesktopAssetUnlinkResponse::failure(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetDetailRequestDto {
    pub workspace_id: String,
    pub asset_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetUnlinkRequestDto {
    pub workspace_id: String,
    pub document_id: String,
    pub asset_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopWorkspaceAssetsRequestDto {
    pub workspace_id: String,
    pub cursor: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetLinkRequestDto {
    pub workspace_id: String,
    pub document_id: String,
    pub asset_id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopWorkspaceAssetsResponse {
    pub ok: bool,
    pub data: Option<DesktopWorkspaceAssetsDataDto>,
    pub error_code: Option<String>,
    pub retryable: bool,
}
impl DesktopWorkspaceAssetsResponse {
    fn success(
        workspace_id: &str,
        output: cabinet_usecases::asset_lifecycle::ListWorkspaceAssetsOutput,
    ) -> Self {
        Self {
            ok: true,
            data: Some(DesktopWorkspaceAssetsDataDto {
                workspace_id: workspace_id.to_string(),
                assets: output
                    .records()
                    .iter()
                    .map(|record| DesktopAssetMetadataDto {
                        asset_id: record.metadata().id().as_str().to_string(),
                        label: record.metadata().file_name().as_str().to_string(),
                        file_name: record.metadata().file_name().as_str().to_string(),
                        media_type: record.metadata().media_type().as_str().to_string(),
                        byte_size: record.metadata().byte_size(),
                        status: "available".to_string(),
                    })
                    .collect(),
                next_cursor: output.next_cursor().map(str::to_string),
            }),
            error_code: None,
            retryable: false,
        }
    }
    fn failure(error: AssetLifecycleError) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(error.code().to_string()),
            retryable: asset_lifecycle_retryable(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopWorkspaceAssetsDataDto {
    pub workspace_id: String,
    pub assets: Vec<DesktopAssetMetadataDto>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetLinkResponse {
    pub ok: bool,
    pub linked: bool,
    pub reference_count: u64,
    pub error_code: Option<String>,
    pub retryable: bool,
}
impl DesktopAssetLinkResponse {
    fn failure(error: AssetLifecycleError) -> Self {
        Self {
            ok: false,
            linked: false,
            reference_count: 0,
            error_code: Some(error.code().to_string()),
            retryable: asset_lifecycle_retryable(error),
        }
    }

    fn projection_failure(error: ReindexAssetGraphProjectionError) -> Self {
        Self {
            ok: false,
            linked: false,
            reference_count: 0,
            error_code: Some(error.code().to_string()),
            retryable: error.retryable(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetDetailResponse {
    pub ok: bool,
    pub data: Option<DesktopAssetDetailDto>,
    pub error_code: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetPreviewResponse {
    pub ok: bool,
    pub data: Option<DesktopAssetPreviewDto>,
    pub error_code: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetExternalOpenResponse {
    pub ok: bool,
    pub opened: bool,
    pub error_code: Option<String>,
    pub retryable: bool,
}

impl DesktopAssetExternalOpenResponse {
    fn success(opened: bool) -> Self {
        Self {
            ok: true,
            opened,
            error_code: None,
            retryable: false,
        }
    }

    fn failure(error: OpenAssetExternallyError) -> Self {
        Self {
            ok: false,
            opened: false,
            error_code: Some(error.code().to_string()),
            retryable: error.retryable(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetPreviewDto {
    pub asset_id: String,
    pub capability: String,
    pub media_type: String,
    pub presentation: String,
    pub content: Option<String>,
}

impl DesktopAssetPreviewResponse {
    fn success(
        asset_id: &str,
        output: cabinet_usecases::asset_preview::GetAssetPreviewOutput,
    ) -> Self {
        let capability = asset_preview_name(output.capability()).to_string();
        let media_type = output.media_type().to_string();
        let (presentation, content) = match output.result() {
            AssetPreviewResult::Unsupported => ("unsupported".to_string(), None),
            AssetPreviewResult::Content(bytes)
                if output.capability() == cabinet_domain::asset::AssetPreviewCapability::Text =>
            {
                match String::from_utf8(bytes.clone()) {
                    Ok(text) => ("text".to_string(), Some(text)),
                    Err(_) => {
                        return Self::failure(AssetPreviewError::Read(
                            cabinet_ports::asset_preview::AssetPreviewReadError::Corrupted,
                        ));
                    }
                }
            }
            AssetPreviewResult::Content(bytes) => (
                "data_url".to_string(),
                Some(format!("data:{media_type};base64,{}", BASE64.encode(bytes))),
            ),
        };
        Self {
            ok: true,
            data: Some(DesktopAssetPreviewDto {
                asset_id: asset_id.to_string(),
                capability,
                media_type,
                presentation,
                content,
            }),
            error_code: None,
            retryable: false,
        }
    }
    fn failure(error: AssetPreviewError) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(error.code().to_string()),
            retryable: error.retryable(),
        }
    }
}
impl DesktopAssetDetailResponse {
    fn success(
        output: cabinet_usecases::asset_lifecycle::GetAssetDetailOutput,
        linked_documents: Vec<DesktopAssetLinkedDocumentDto>,
    ) -> Self {
        let record = output.record();
        Self {
            ok: true,
            data: Some(DesktopAssetDetailDto {
                asset_id: record.metadata().id().as_str().to_string(),
                file_name: record.metadata().file_name().as_str().to_string(),
                media_type: record.metadata().media_type().as_str().to_string(),
                byte_size: record.metadata().byte_size(),
                version: record.version(),
                preview_capability: asset_preview_name(record.preview()).to_string(),
                extraction_status: asset_extraction_name(record.extraction()).to_string(),
                reference_count: output.reference_count(),
                linked_document_ids: output
                    .linked_documents()
                    .iter()
                    .map(|value| value.document_id().as_str().to_string())
                    .collect(),
                linked_documents,
            }),
            error_code: None,
            retryable: false,
        }
    }
    fn failure(error: AssetLifecycleError) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(error.code().to_string()),
            retryable: asset_lifecycle_retryable(error),
        }
    }

    fn invalid_title_request() -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some("asset_detail.invalid_input".to_string()),
            retryable: false,
        }
    }

    fn title_resolution_failure() -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some("asset_detail.document_titles_unavailable".to_string()),
            retryable: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetDetailDto {
    pub asset_id: String,
    pub file_name: String,
    pub media_type: String,
    pub byte_size: u64,
    pub version: u32,
    pub preview_capability: String,
    pub extraction_status: String,
    pub reference_count: u64,
    pub linked_document_ids: Vec<String>,
    pub linked_documents: Vec<DesktopAssetLinkedDocumentDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetLinkedDocumentDto {
    pub document_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetUnlinkResponse {
    pub ok: bool,
    pub removed: bool,
    pub remaining_references: u64,
    pub error_code: Option<String>,
    pub retryable: bool,
}
impl DesktopAssetUnlinkResponse {
    fn failure(error: AssetLifecycleError) -> Self {
        Self {
            ok: false,
            removed: false,
            remaining_references: 0,
            error_code: Some(error.code().to_string()),
            retryable: asset_lifecycle_retryable(error),
        }
    }

    fn projection_failure(error: ReindexAssetGraphProjectionError) -> Self {
        Self {
            ok: false,
            removed: false,
            remaining_references: 0,
            error_code: Some(error.code().to_string()),
            retryable: error.retryable(),
        }
    }
}

#[derive(Default)]
struct DesktopAssetLifecycleProductLogSink(usize);
impl AssetLifecycleProductLogger for DesktopAssetLifecycleProductLogSink {
    fn write_product(&mut self, event: AssetLifecycleProductEvent) {
        match event {
            AssetLifecycleProductEvent::Linked { .. }
            | AssetLifecycleProductEvent::Unlinked { .. } => self.0 = self.0.saturating_add(1),
        }
    }
}

#[derive(Default)]
struct DesktopAssetExternalOpenProductLogSink {
    event_count: usize,
    last_error_code: Option<&'static str>,
}

impl AssetExternalOpenProductLogger for DesktopAssetExternalOpenProductLogSink {
    fn write_product(&mut self, event: AssetExternalOpenProductEvent) {
        self.event_count = self.event_count.saturating_add(1);
        let AssetExternalOpenProductEvent::Failed { error_code } = event;
        self.last_error_code = Some(error_code);
    }
}

const fn asset_lifecycle_retryable(error: AssetLifecycleError) -> bool {
    matches!(
        error,
        AssetLifecycleError::Document(_)
            | AssetLifecycleError::Metadata(_)
            | AssetLifecycleError::Association(_)
    )
}
const fn asset_preview_name(value: cabinet_domain::asset::AssetPreviewCapability) -> &'static str {
    use cabinet_domain::asset::AssetPreviewCapability::*;
    match value {
        Image => "image",
        Pdf => "pdf",
        Text => "text",
        Unsupported => "unsupported",
    }
}
const fn asset_extraction_name(
    value: cabinet_domain::asset::AssetExtractionStatus,
) -> &'static str {
    use cabinet_domain::asset::AssetExtractionStatus::*;
    match value {
        NotRequested => "not_requested",
        Pending => "pending",
        Ready => "ready",
        Unsupported => "unsupported",
        Failed => "failed",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentAssetsCommandResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DesktopDocumentAssetsDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
}

impl DesktopDocumentAssetsCommandResponse {
    fn success(
        workspace_id: &str,
        document_id: &str,
        output: cabinet_usecases::asset_import::ListCatalogDocumentAssetsOutput,
    ) -> Self {
        let assets = output
            .assets()
            .iter()
            .map(|record| DesktopAssetMetadataDto {
                asset_id: record.asset_id().as_str().to_string(),
                label: record.label().to_string(),
                file_name: record.record().metadata().file_name().as_str().to_string(),
                media_type: record.record().metadata().media_type().as_str().to_string(),
                byte_size: record.record().metadata().byte_size(),
                status: "available".to_string(),
            })
            .collect();
        Self {
            ok: true,
            data: Some(DesktopDocumentAssetsDataDto {
                query_name: "list-document-assets".to_string(),
                workspace_id: workspace_id.to_string(),
                document_id: document_id.to_string(),
                assets,
            }),
            error_code: None,
            retryable: false,
        }
    }

    fn failure(error: ListCatalogDocumentAssetsError) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(error.code().to_string()),
            retryable: matches!(
                error,
                ListCatalogDocumentAssetsError::Document(_)
                    | ListCatalogDocumentAssetsError::Association(_)
                    | ListCatalogDocumentAssetsError::Metadata(_)
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopDocumentAssetsDataDto {
    pub query_name: String,
    pub workspace_id: String,
    pub document_id: String,
    pub assets: Vec<DesktopAssetMetadataDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAssetMetadataDto {
    pub asset_id: String,
    pub label: String,
    pub file_name: String,
    pub media_type: String,
    pub byte_size: u64,
    pub status: String,
}

type DesktopCompositeGraphProjectionStore = CompositeGraphProjectionStore<
    DurableLocalGraphProjectionStore,
    DurableCanvasGraphRelationProjectionStore,
>;

const DESKTOP_CANVAS_GRAPH_SOURCE_LIMIT: usize = 256;

fn desktop_composite_graph_store(app_data_root: &Path) -> DesktopCompositeGraphProjectionStore {
    CompositeGraphProjectionStore::new(
        DurableLocalGraphProjectionStore::new(app_data_root.to_path_buf()),
        DurableCanvasGraphRelationProjectionStore::new(app_data_root.to_path_buf()),
        DESKTOP_CANVAS_GRAPH_SOURCE_LIMIT,
    )
    .expect("fixed desktop Canvas graph source policy must be valid")
}

const DESKTOP_GRAPH_PREFERENCE_SCHEMA_VERSION: u8 = 2;
const DESKTOP_GRAPH_PREFERENCE_ROOT: &str = "ui-settings/graph";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopGraphCameraPreferenceDto {
    pub center_x: f64,
    pub center_y: f64,
    pub zoom_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopGraphPreferenceDto {
    pub schema_version: u8,
    pub depth: u8,
    pub direction: String,
    pub include_unresolved: bool,
    pub include_assets: bool,
    #[serde(default)]
    pub include_external: bool,
    pub camera: DesktopGraphCameraPreferenceDto,
}

impl Default for DesktopGraphPreferenceDto {
    fn default() -> Self {
        Self {
            schema_version: DESKTOP_GRAPH_PREFERENCE_SCHEMA_VERSION,
            depth: 1,
            direction: "both".to_string(),
            include_unresolved: true,
            include_assets: true,
            include_external: false,
            camera: DesktopGraphCameraPreferenceDto {
                center_x: 0.0,
                center_y: 0.0,
                zoom_percent: 100.0,
            },
        }
    }
}

impl DesktopGraphPreferenceDto {
    fn is_valid(&self) -> bool {
        self.schema_version == DESKTOP_GRAPH_PREFERENCE_SCHEMA_VERSION
            && matches!(self.depth, 1 | 2)
            && matches!(self.direction.as_str(), "incoming" | "outgoing" | "both")
            && self.camera.center_x.is_finite()
            && self.camera.center_x.abs() <= 1_000_000.0
            && self.camera.center_y.is_finite()
            && self.camera.center_y.abs() <= 1_000_000.0
            && self.camera.zoom_percent.is_finite()
            && (25.0..=400.0).contains(&self.camera.zoom_percent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopGraphPreferenceLoadRequestDto {
    pub workspace_id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopGraphPreferenceSaveRequestDto {
    pub workspace_id: String,
    pub preference: DesktopGraphPreferenceDto,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopGraphPreferenceLoadResponse {
    pub ok: bool,
    pub data: Option<DesktopGraphPreferenceDto>,
    pub error_code: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopGraphPreferenceSaveDataDto {
    pub saved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopGraphPreferenceSaveResponse {
    pub ok: bool,
    pub data: Option<DesktopGraphPreferenceSaveDataDto>,
    pub error_code: Option<String>,
    pub retryable: bool,
}

#[derive(Debug)]
pub struct DesktopGraphPreferenceRuntime {
    root: PathBuf,
    write_lock: Mutex<()>,
}

impl DesktopGraphPreferenceRuntime {
    pub fn new(app_data_root: PathBuf) -> Self {
        Self {
            root: app_data_root.join(DESKTOP_GRAPH_PREFERENCE_ROOT),
            write_lock: Mutex::new(()),
        }
    }

    pub fn load(
        &self,
        request: DesktopGraphPreferenceLoadRequestDto,
    ) -> DesktopGraphPreferenceLoadResponse {
        let Some(path) = self.preference_path(&request.workspace_id) else {
            return graph_preference_load_failure("GRAPH_PREFERENCE_WORKSPACE_INVALID", false);
        };
        let preference = fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<DesktopGraphPreferenceDto>(&bytes).ok())
            .map(|mut preference| {
                if preference.schema_version == 1 {
                    preference.schema_version = DESKTOP_GRAPH_PREFERENCE_SCHEMA_VERSION;
                    preference.include_external = false;
                }
                preference
            })
            .filter(DesktopGraphPreferenceDto::is_valid)
            .unwrap_or_default();
        DesktopGraphPreferenceLoadResponse {
            ok: true,
            data: Some(preference),
            error_code: None,
            retryable: false,
        }
    }

    pub fn save(
        &self,
        request: DesktopGraphPreferenceSaveRequestDto,
    ) -> DesktopGraphPreferenceSaveResponse {
        let Some(path) = self.preference_path(&request.workspace_id) else {
            return graph_preference_save_failure("GRAPH_PREFERENCE_WORKSPACE_INVALID", false);
        };
        if !request.preference.is_valid() {
            return graph_preference_save_failure("GRAPH_PREFERENCE_INVALID", false);
        }
        let Ok(_guard) = self.write_lock.lock() else {
            return graph_preference_save_failure("GRAPH_PREFERENCE_STORAGE_FAILED", true);
        };
        let Some(parent) = path.parent() else {
            return graph_preference_save_failure("GRAPH_PREFERENCE_STORAGE_FAILED", false);
        };
        if fs::create_dir_all(parent).is_err() {
            return graph_preference_save_failure("GRAPH_PREFERENCE_STORAGE_FAILED", true);
        }
        let temporary = path.with_extension("json.tmp");
        let write_result = serde_json::to_vec(&request.preference)
            .map_err(|_| ())
            .and_then(|bytes| fs::write(&temporary, bytes).map_err(|_| ()))
            .and_then(|_| fs::rename(&temporary, &path).map_err(|_| ()));
        if write_result.is_err() {
            let _ = fs::remove_file(temporary);
            return graph_preference_save_failure("GRAPH_PREFERENCE_STORAGE_FAILED", true);
        }
        DesktopGraphPreferenceSaveResponse {
            ok: true,
            data: Some(DesktopGraphPreferenceSaveDataDto { saved: true }),
            error_code: None,
            retryable: false,
        }
    }

    fn preference_path(&self, workspace_id: &str) -> Option<PathBuf> {
        let trimmed = workspace_id.trim();
        if trimmed.is_empty() || trimmed.len() > 128 {
            return None;
        }
        let key = trimmed
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Some(self.root.join(format!("{key}.json")))
    }
}

fn graph_preference_load_failure(
    code: &str,
    retryable: bool,
) -> DesktopGraphPreferenceLoadResponse {
    DesktopGraphPreferenceLoadResponse {
        ok: false,
        data: None,
        error_code: Some(code.to_string()),
        retryable,
    }
}

fn graph_preference_save_failure(
    code: &str,
    retryable: bool,
) -> DesktopGraphPreferenceSaveResponse {
    DesktopGraphPreferenceSaveResponse {
        ok: false,
        data: None,
        error_code: Some(code.to_string()),
        retryable,
    }
}

#[derive(Debug, Clone)]
pub struct DesktopKnowledgeGraphRuntime {
    projection_store: DesktopCompositeGraphProjectionStore,
    documents: LocalDocumentRepository,
    assets: DurableAssetMetadataCatalog,
}

#[derive(Debug, Clone)]
pub struct DesktopGlobalKnowledgeGraphRuntime {
    projection_store: DesktopCompositeGraphProjectionStore,
    current_versions: LocalCurrentDocumentVersionPointer,
    documents: LocalDocumentRepository,
    assets: DurableAssetMetadataCatalog,
}
impl DesktopGlobalKnowledgeGraphRuntime {
    pub fn new(app_data_root: PathBuf) -> Self {
        Self {
            projection_store: desktop_composite_graph_store(&app_data_root),
            current_versions: LocalCurrentDocumentVersionPointer::new(
                app_data_root.join(LOCAL_DOCUMENT_POINTER_ROOT),
            ),
            documents: LocalDocumentRepository::new(app_data_root.join("authoring-current")),
            assets: DurableAssetMetadataCatalog::new(app_data_root),
        }
    }
    pub fn execute(
        &self,
        request: DesktopGlobalKnowledgeGraphRequestDto,
    ) -> DesktopGlobalKnowledgeGraphCommandResponse {
        match GetGlobalKnowledgeGraphUsecase::new().execute(
            GetGlobalKnowledgeGraphInput::new(
                &request.workspace_id,
                request.cursor.as_deref(),
                request.include_unresolved,
                request.include_assets,
                request.projection_limit,
                request.node_limit,
                request.edge_limit,
            ),
            &self.projection_store,
            &self.current_versions,
        ) {
            Ok(output) => DesktopGlobalKnowledgeGraphCommandResponse::success(
                output,
                &request.workspace_id,
                &self.documents,
                &self.assets,
            ),
            Err(error) => DesktopGlobalKnowledgeGraphCommandResponse::failure(error),
        }
    }
}

fn request_asset_graph_reindex(
    app_data_root: &Path,
    workspace_id: &str,
    document_id: &str,
    change_kind: ProjectionChangeKind,
) -> Result<(), ReindexAssetGraphProjectionError> {
    let input = ReindexAssetGraphProjectionInput::new(workspace_id, document_id, change_kind)?;
    let usecase = ReindexAssetGraphProjectionUsecase::new();
    let mut repository = DurableProjectionWorkRepository::new(app_data_root.to_path_buf());
    match usecase.execute(
        input.clone(),
        &LocalCurrentDocumentVersionPointer::new(app_data_root.join(LOCAL_DOCUMENT_POINTER_ROOT)),
        &mut repository,
    ) {
        Ok(_) => {}
        Err(ReindexAssetGraphProjectionError::CurrentVersionNotFound) => {
            usecase.execute(
                input,
                &LocalCurrentDocumentVersionPointer::new(
                    app_data_root.join("authoring-current-version"),
                ),
                &mut repository,
            )?;
        }
        Err(error) => return Err(error),
    }
    Ok(())
}

fn request_authoritative_asset_graph_reindex(
    app_data_root: &Path,
    workspace_id: &str,
    document_id: &str,
    change_kind: ProjectionChangeKind,
) -> Result<(), ReindexAssetGraphProjectionError> {
    let input = ReindexAssetGraphProjectionInput::new(workspace_id, document_id, change_kind)?;
    ReindexAssetGraphProjectionUsecase::new().ensure(
        input,
        &LocalCurrentDocumentVersionPointer::new(app_data_root.join(LOCAL_DOCUMENT_POINTER_ROOT)),
        &mut DurableProjectionWorkRepository::new(app_data_root.to_path_buf()),
    )?;
    Ok(())
}

fn ensure_asset_graph_reindex(
    app_data_root: &Path,
    workspace_id: &str,
    document_id: &str,
    change_kind: ProjectionChangeKind,
) -> Result<(), ReindexAssetGraphProjectionError> {
    let input = ReindexAssetGraphProjectionInput::new(workspace_id, document_id, change_kind)?;
    let usecase = ReindexAssetGraphProjectionUsecase::new();
    let mut repository = DurableProjectionWorkRepository::new(app_data_root.to_path_buf());
    match usecase.ensure(
        input.clone(),
        &LocalCurrentDocumentVersionPointer::new(app_data_root.join(LOCAL_DOCUMENT_POINTER_ROOT)),
        &mut repository,
    ) {
        Ok(_) => {}
        Err(ReindexAssetGraphProjectionError::CurrentVersionNotFound) => {
            usecase.ensure(
                input,
                &LocalCurrentDocumentVersionPointer::new(
                    app_data_root.join("authoring-current-version"),
                ),
                &mut repository,
            )?;
        }
        Err(error) => return Err(error),
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopGlobalKnowledgeGraphRequestDto {
    pub workspace_id: String,
    pub cursor: Option<String>,
    pub include_unresolved: bool,
    pub include_assets: bool,
    pub projection_limit: usize,
    pub node_limit: usize,
    pub edge_limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopGlobalKnowledgeGraphCommandResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DesktopGlobalKnowledgeGraphDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
}
impl DesktopGlobalKnowledgeGraphCommandResponse {
    fn success(
        output: cabinet_usecases::global_graph::GetGlobalKnowledgeGraphOutput,
        workspace_id: &str,
        documents: &LocalDocumentRepository,
        assets: &DurableAssetMetadataCatalog,
    ) -> Self {
        Self {
            ok: true,
            data: Some(DesktopGlobalKnowledgeGraphDataDto {
                status: graph_status_name(output.status()).into(),
                nodes: output
                    .nodes()
                    .iter()
                    .map(|node| graph_node_dto(node, workspace_id, documents, assets))
                    .collect(),
                edges: output
                    .edges()
                    .iter()
                    .map(|edge| DesktopKnowledgeGraphEdgeDto {
                        id: edge.id().into(),
                        source_id: edge.source_id().into(),
                        target_id: edge.target_id().into(),
                        kind: graph_edge_kind_name(edge.kind()).into(),
                    })
                    .collect(),
                candidate_count: output.candidate_count(),
                next_cursor: output.next_cursor().map(str::to_string),
            }),
            error_code: None,
            retryable: false,
        }
    }
    fn failure(error: GetGlobalKnowledgeGraphError) -> Self {
        let (code, retryable) = match error {
            GetGlobalKnowledgeGraphError::InvalidInput => ("GLOBAL_GRAPH_INVALID_INPUT", false),
            GetGlobalKnowledgeGraphError::ProjectionUnavailable => {
                ("GLOBAL_GRAPH_PROJECTION_UNAVAILABLE", true)
            }
            GetGlobalKnowledgeGraphError::CorruptedProjection => {
                ("GLOBAL_GRAPH_PROJECTION_CORRUPTED", false)
            }
        };
        Self {
            ok: false,
            data: None,
            error_code: Some(code.into()),
            retryable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopGlobalKnowledgeGraphDataDto {
    pub status: String,
    pub nodes: Vec<DesktopKnowledgeGraphNodeDto>,
    pub edges: Vec<DesktopKnowledgeGraphEdgeDto>,
    pub candidate_count: usize,
    pub next_cursor: Option<String>,
}

impl DesktopKnowledgeGraphRuntime {
    pub fn new(app_data_root: PathBuf) -> Self {
        Self {
            projection_store: desktop_composite_graph_store(&app_data_root),
            documents: LocalDocumentRepository::new(app_data_root.join("authoring-current")),
            assets: DurableAssetMetadataCatalog::new(app_data_root),
        }
    }

    pub fn execute(
        &self,
        request: DesktopLocalCommandRequestDto,
    ) -> DesktopKnowledgeGraphCommandResponse {
        let (input, workspace_id) =
            match map_core_local_desktop_command_request(to_platform_runtime_request(request)) {
                Ok(LocalDesktopUsecaseInput::GetGraphProjection {
                    workspace_id,
                    document_id,
                    depth,
                    direction,
                    include_unresolved,
                    include_assets,
                    node_limit,
                    edge_limit,
                }) => (
                    GetLocalKnowledgeGraphInput::new(
                        &workspace_id,
                        &document_id,
                        depth,
                        match direction.as_str() {
                            "incoming" => LocalGraphDirection::Incoming,
                            "outgoing" => LocalGraphDirection::Outgoing,
                            "both" => LocalGraphDirection::Both,
                            _ => return DesktopKnowledgeGraphCommandResponse::invalid_input(),
                        },
                        include_unresolved,
                        include_assets,
                        usize::from(node_limit),
                        usize::from(edge_limit),
                    ),
                    workspace_id,
                ),
                _ => return DesktopKnowledgeGraphCommandResponse::invalid_input(),
            };

        match GetLocalKnowledgeGraphUsecase::new().execute(input, &self.projection_store) {
            Ok(output) => DesktopKnowledgeGraphCommandResponse::success(
                output,
                &workspace_id,
                &self.documents,
                &self.assets,
            ),
            Err(error) => DesktopKnowledgeGraphCommandResponse::failure(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopKnowledgeGraphCommandResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DesktopKnowledgeGraphDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
}

impl DesktopKnowledgeGraphCommandResponse {
    fn success(
        output: cabinet_usecases::graph::GetLocalKnowledgeGraphOutput,
        workspace_id: &str,
        documents: &LocalDocumentRepository,
        assets: &DurableAssetMetadataCatalog,
    ) -> Self {
        let graph = output.graph();
        let data = DesktopKnowledgeGraphDataDto {
            center_document_id: graph.center_document_id().as_str().to_string(),
            status: graph_status_name(graph.status()).to_string(),
            nodes: graph
                .nodes()
                .iter()
                .map(|node| graph_node_dto(node, workspace_id, documents, assets))
                .collect(),
            edges: graph
                .edges()
                .iter()
                .map(|edge| DesktopKnowledgeGraphEdgeDto {
                    id: edge.id().to_string(),
                    source_id: edge.source_id().to_string(),
                    target_id: edge.target_id().to_string(),
                    kind: graph_edge_kind_name(edge.kind()).to_string(),
                })
                .collect(),
            stats: DesktopKnowledgeGraphStatsDto {
                candidate_count: output.candidate_count(),
                filtered_count: output.filtered_count(),
            },
            freshness_revision: output.freshness_revision().to_string(),
        };
        Self {
            ok: true,
            data: Some(data),
            error_code: None,
            retryable: false,
        }
    }

    fn invalid_input() -> Self {
        Self::failure(GetLocalKnowledgeGraphError::InvalidInput)
    }

    fn failure(error: GetLocalKnowledgeGraphError) -> Self {
        let error_code = match error {
            GetLocalKnowledgeGraphError::InvalidInput => "GRAPH_INVALID_INPUT",
            GetLocalKnowledgeGraphError::ProjectionNotFound => "GRAPH_PROJECTION_NOT_FOUND",
            GetLocalKnowledgeGraphError::ProjectionUnavailable => "GRAPH_PROJECTION_UNAVAILABLE",
            GetLocalKnowledgeGraphError::CorruptedProjection => "GRAPH_PROJECTION_CORRUPTED",
        };
        Self {
            ok: false,
            data: None,
            error_code: Some(error_code.to_string()),
            retryable: error.retryable(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopKnowledgeGraphDataDto {
    pub center_document_id: String,
    pub status: String,
    pub nodes: Vec<DesktopKnowledgeGraphNodeDto>,
    pub edges: Vec<DesktopKnowledgeGraphEdgeDto>,
    pub stats: DesktopKnowledgeGraphStatsDto,
    pub freshness_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopKnowledgeGraphNodeDto {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub breadcrumb_label: String,
    pub availability: String,
    pub can_navigate: bool,
}

fn graph_node_dto(
    node: &GraphNode,
    workspace_id: &str,
    documents: &LocalDocumentRepository,
    assets: &DurableAssetMetadataCatalog,
) -> DesktopKnowledgeGraphNodeDto {
    let kind = graph_node_kind_name(node.kind()).to_string();
    let fallback = || DesktopKnowledgeGraphNodeDto {
        id: node.id().to_string(),
        label: graph_node_fallback_label(node.kind()).to_string(),
        kind: kind.clone(),
        breadcrumb_label: String::new(),
        availability: "missing".to_string(),
        can_navigate: false,
    };
    let Ok(workspace_id) = WorkspaceId::new(workspace_id) else {
        return fallback();
    };
    match node.kind() {
        GraphNodeKind::Document => {
            let Ok(document_id) = DocumentId::new(node.id()) else {
                return fallback();
            };
            let Ok(Some(record)) = documents.get_current_by_id(&workspace_id, &document_id) else {
                return fallback();
            };
            DesktopKnowledgeGraphNodeDto {
                id: node.id().to_string(),
                kind,
                label: DocumentTitle::from_markdown_body(record.body())
                    .as_str()
                    .to_string(),
                breadcrumb_label: document_parent_breadcrumb(record.path().as_str()),
                availability: "available".to_string(),
                can_navigate: true,
            }
        }
        GraphNodeKind::Attachment => {
            let Ok(asset_id) = AssetId::from_sha256_hex(node.id()) else {
                return fallback();
            };
            let Ok(Some(record)) = assets.get(&workspace_id, &asset_id) else {
                return fallback();
            };
            DesktopKnowledgeGraphNodeDto {
                id: node.id().to_string(),
                kind,
                label: record.metadata().file_name().as_str().to_string(),
                breadcrumb_label: String::new(),
                availability: "available".to_string(),
                can_navigate: true,
            }
        }
        GraphNodeKind::UnresolvedLink => DesktopKnowledgeGraphNodeDto {
            id: node.id().to_string(),
            kind,
            label: bounded_safe_graph_label(node.id(), "미해결 링크"),
            breadcrumb_label: String::new(),
            availability: "missing".to_string(),
            can_navigate: false,
        },
        GraphNodeKind::ExternalLink => DesktopKnowledgeGraphNodeDto {
            id: node.id().to_string(),
            kind,
            label: safe_external_graph_label(node.id()),
            breadcrumb_label: String::new(),
            availability: "available".to_string(),
            can_navigate: false,
        },
    }
}

fn bounded_safe_graph_label(value: &str, fallback: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    let bounded = sanitized.trim().chars().take(120).collect::<String>();
    if bounded.is_empty() {
        fallback.to_string()
    } else {
        bounded
    }
}

fn safe_external_graph_label(value: &str) -> String {
    let authority = value
        .split_once("://")
        .map(|(_, remainder)| remainder)
        .unwrap_or(value)
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default();
    let host = authority.split(':').next().unwrap_or_default();
    bounded_safe_graph_label(host, "외부 링크")
}

fn graph_node_fallback_label(kind: GraphNodeKind) -> &'static str {
    match kind {
        GraphNodeKind::Document => "찾을 수 없는 문서",
        GraphNodeKind::UnresolvedLink => "미해결 링크",
        GraphNodeKind::Attachment => "첨부 파일",
        GraphNodeKind::ExternalLink => "외부 링크",
    }
}

fn document_parent_breadcrumb(path: &str) -> String {
    let mut segments = path
        .split('/')
        .filter(|segment| !segment.trim().is_empty())
        .collect::<Vec<_>>();
    segments.pop();
    segments.join(" / ")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopKnowledgeGraphEdgeDto {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopKnowledgeGraphStatsDto {
    pub candidate_count: usize,
    pub filtered_count: usize,
}

const fn graph_status_name(status: GraphProjectionStatus) -> &'static str {
    match status {
        GraphProjectionStatus::Clean => "clean",
        GraphProjectionStatus::ReindexRequested => "reindex_requested",
        GraphProjectionStatus::Reindexing => "reindexing",
        GraphProjectionStatus::Degraded => "degraded",
    }
}

const fn graph_node_kind_name(kind: GraphNodeKind) -> &'static str {
    match kind {
        GraphNodeKind::Document => "document",
        GraphNodeKind::UnresolvedLink => "unresolved_link",
        GraphNodeKind::Attachment => "attachment",
        GraphNodeKind::ExternalLink => "external_link",
    }
}

const fn graph_edge_kind_name(kind: GraphEdgeKind) -> &'static str {
    match kind {
        GraphEdgeKind::DocumentLink => "document_link",
        GraphEdgeKind::AttachmentReference => "attachment_reference",
        GraphEdgeKind::ExternalReference => "external_reference",
        GraphEdgeKind::CanvasRelation => "canvas_relation",
    }
}

/// Request DTO accepted at the desktop shell boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopShellRequest {
    pub command: String,
}

/// Response DTO returned from the desktop shell boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopShellResponse {
    pub boundary: &'static str,
    pub command: String,
}

/// Serializable response DTO exposed to the Tauri command boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesktopShellCommandResponse {
    pub boundary: String,
    pub command: String,
}

/// Serializable route response for the Phase 008 local desktop command boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesktopLocalCommandRouteResponse {
    pub boundary: String,
    pub command_name: String,
    pub accepted: bool,
    pub error_code: Option<String>,
}

/// Request DTO accepted by the local desktop command runtime boundary.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DesktopLocalCommandRequestDto {
    pub command_name: String,
    pub payload: DesktopLocalCommandPayloadDto,
}

/// Typed payload DTO accepted at the desktop shell boundary.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DesktopLocalCommandPayloadDto {
    Empty,
    WorkspaceHome {
        workspace_id: String,
        recent_documents: u16,
        favorites: u16,
        tags: u16,
        recent_changes: u16,
        unfinished_items: u16,
    },
    DocumentIdentity {
        workspace_id: String,
        document_id: String,
    },
    DocumentUpdate {
        workspace_id: String,
        document_id: String,
        title: String,
        path: String,
        body: String,
        expected_version_id: String,
    },
    DocumentHistory {
        workspace_id: String,
        document_id: String,
        limit: u16,
    },
    DocumentVersion {
        workspace_id: String,
        document_id: String,
        version_id: String,
    },
    Search {
        workspace_id: String,
        text: String,
        limit: u16,
    },
    GraphProjection {
        workspace_id: String,
        document_id: String,
        depth: u8,
        direction: String,
        include_unresolved: bool,
        include_assets: bool,
        node_limit: u16,
        edge_limit: u16,
    },
    AssetAttachment {
        workspace_id: String,
        document_id: String,
        asset_id: String,
        label: String,
        file_name: String,
        media_type: String,
        byte_size: u64,
    },
    Workspace {
        workspace_id: String,
    },
    ImportPreview {
        workspace_id: String,
        source_label: String,
        file_count: u16,
    },
    RestorePackage {
        workspace_id: String,
        package_label: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBackupPackageRequestDto {
    pub workspace_id: String,
    pub package_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBackupOperationRequestDto {
    pub workspace_id: String,
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBackupOperationResponse {
    pub ok: bool,
    pub operation_id: String,
    pub state: String,
    pub progress_completed_units: u64,
    pub progress_total_units: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopRestoreConfirmRequestDto {
    pub workspace_id: String,
    pub package_id: String,
    pub operation_id: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopRestoreCancelRequestDto {
    pub workspace_id: String,
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBackupRecoveryRequestDto {
    pub workspace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBackupCatalogRequestDto {
    pub workspace_id: String,
    pub cursor: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBackupManifestEntryDto {
    pub data_class: String,
    pub record_count: u64,
    pub byte_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBackupManifestDto {
    pub package_id: String,
    pub schema_version: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at_epoch_ms: Option<u64>,
    pub entries: Vec<DesktopBackupManifestEntryDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBackupCatalogResponse {
    pub ok: bool,
    pub state: String,
    pub records: Vec<DesktopBackupManifestDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopStartupRecoveryDto {
    pub cleaned_staging_count: u64,
    pub rolled_back_operation_ids: Vec<String>,
    pub cleanup_required_operation_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBackupRecoveryResponse {
    pub ok: bool,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<DesktopBackupManifestDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_ready: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery: Option<DesktopStartupRecoveryDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopBackupProductEvent {
    Package {
        event_name: String,
        error_code: Option<String>,
    },
    Restore {
        event_name: String,
        state: String,
        error_code: Option<String>,
    },
    Recovery {
        event_name: String,
    },
    Operation {
        event_name: String,
        state: String,
        error_code: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct DesktopBackupRecoveryRuntime {
    root: PathBuf,
    policy: LocalBackupPackagePolicy,
    projection_body_max_bytes: usize,
    projection_batch_limit: usize,
    projection_max_attempts: u32,
    product_events: Arc<Mutex<Vec<DesktopBackupProductEvent>>>,
}

impl DesktopBackupRecoveryRuntime {
    pub fn new(
        root: PathBuf,
        max_file_count: u64,
        max_total_bytes: u64,
    ) -> Result<Self, &'static str> {
        Self::new_with_projection_policy(
            root,
            max_file_count,
            max_total_bytes,
            10 * 1024 * 1024,
            64,
            3,
        )
    }

    pub fn new_with_projection_policy(
        root: PathBuf,
        max_file_count: u64,
        max_total_bytes: u64,
        projection_body_max_bytes: usize,
        projection_batch_limit: usize,
        projection_max_attempts: u32,
    ) -> Result<Self, &'static str> {
        let policy = LocalBackupPackagePolicy::new(max_file_count, max_total_bytes)
            .map_err(|_| "BACKUP_INVALID_POLICY")?;
        ProjectionWorkerPolicy::new(projection_batch_limit, projection_max_attempts)
            .map_err(|_| "BACKUP_INVALID_PROJECTION_POLICY")?;
        DocumentBodyPolicy::new(projection_body_max_bytes)
            .map_err(|_| "BACKUP_INVALID_PROJECTION_POLICY")?;
        Ok(Self {
            root,
            policy,
            projection_body_max_bytes,
            projection_batch_limit,
            projection_max_attempts,
            product_events: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn product_events(&self) -> Vec<DesktopBackupProductEvent> {
        self.product_events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn list_catalog(
        &self,
        request: DesktopBackupCatalogRequestDto,
    ) -> DesktopBackupCatalogResponse {
        let catalog = LocalBackupPackageStore::new(self.root.clone(), self.policy);
        match ListBackupCatalogUsecase::new().execute(
            ListBackupCatalogInput::new(
                &request.workspace_id,
                request.cursor.as_deref(),
                request.limit,
            ),
            &catalog,
        ) {
            Ok(output) => DesktopBackupCatalogResponse {
                ok: true,
                state: "Ready".to_string(),
                records: output
                    .records()
                    .iter()
                    .map(|record| to_backup_manifest(record.package_id(), record.summary()))
                    .collect(),
                next_cursor: output.next_cursor().map(str::to_string),
                error_code: None,
                retryable: false,
            },
            Err(error) => DesktopBackupCatalogResponse {
                ok: false,
                state: "Failed".to_string(),
                records: vec![],
                next_cursor: None,
                error_code: Some(error.code().to_string()),
                retryable: matches!(error, ListBackupCatalogError::CatalogUnavailable),
            },
        }
    }

    pub fn start_operation(
        &self,
        request: DesktopBackupOperationRequestDto,
    ) -> DesktopBackupOperationResponse {
        let mut jobs = LocalBackupStore::new(self.root.clone());
        let mut logger = self.logger();
        match StartBackupPackageOperationUsecase::new().execute(
            StartBackupPackageOperationInput::new(
                "local-user",
                &request.workspace_id,
                &request.operation_id,
            ),
            &mut jobs,
            &mut logger,
        ) {
            Ok(output) => to_operation_response(output),
            Err(error) => backup_operation_failure(&request.operation_id, error.code()),
        }
    }

    pub fn run_operation(
        &self,
        request: DesktopBackupOperationRequestDto,
    ) -> DesktopBackupOperationResponse {
        let mut jobs = LocalBackupStore::new(self.root.clone());
        let mut packages = LocalBackupPackageStore::new(self.root.clone(), self.policy);
        let mut logger = self.logger();
        match RunBackupPackageOperationUsecase::new().execute(
            RunBackupPackageOperationInput::new(
                "local-user",
                &request.workspace_id,
                &request.operation_id,
            ),
            &mut jobs,
            &mut packages,
            &mut logger,
        ) {
            Ok(output) => to_operation_response(output),
            Err(error) => backup_operation_failure(&request.operation_id, error.code()),
        }
    }

    pub fn operation_status(
        &self,
        request: DesktopBackupOperationRequestDto,
    ) -> DesktopBackupOperationResponse {
        let jobs = LocalBackupStore::new(self.root.clone());
        match GetBackupPackageOperationUsecase::new().execute(
            GetBackupPackageOperationInput::new(
                "local-user",
                &request.workspace_id,
                &request.operation_id,
            ),
            &jobs,
        ) {
            Ok(output) => to_operation_response(output),
            Err(error) => backup_operation_failure(&request.operation_id, error.code()),
        }
    }

    pub fn cancel_operation(
        &self,
        request: DesktopBackupOperationRequestDto,
    ) -> DesktopBackupOperationResponse {
        let mut jobs = LocalBackupStore::new(self.root.clone());
        let mut logger = self.logger();
        match CancelBackupPackageOperationUsecase::new().execute(
            CancelBackupPackageOperationInput::new(
                "local-user",
                &request.workspace_id,
                &request.operation_id,
            ),
            &mut jobs,
            &mut logger,
        ) {
            Ok(output) => to_operation_response(output),
            Err(error) => backup_operation_failure(&request.operation_id, error.code()),
        }
    }

    pub fn create(&self, request: DesktopBackupPackageRequestDto) -> DesktopBackupRecoveryResponse {
        let mut store = LocalBackupPackageStore::new(self.root.clone(), self.policy);
        let mut logger = self.logger();
        match CreateBackupPackageUsecase::new().execute(
            CreateBackupPackageInput::new("local-user", &request.workspace_id, &request.package_id),
            &mut store,
            &mut logger,
        ) {
            Ok(output) => DesktopBackupRecoveryResponse::success(
                "Ready",
                Some(to_backup_manifest(output.package_id(), output.summary())),
            ),
            Err(error) => DesktopBackupRecoveryResponse::failure(
                "Failed",
                error.code(),
                matches!(
                    error,
                    cabinet_usecases::backup_package::BackupPackageUsecaseError::StorageUnavailable
                ),
            ),
        }
    }

    pub fn preview(
        &self,
        request: DesktopBackupPackageRequestDto,
    ) -> DesktopBackupRecoveryResponse {
        let mut store = LocalBackupPackageStore::new(self.root.clone(), self.policy);
        let mut logger = self.logger();
        match PreviewBackupRestoreUsecase::new().execute(
            PreviewBackupRestoreInput::new(
                "local-user",
                &request.workspace_id,
                &request.package_id,
            ),
            &mut store,
            &mut logger,
        ) {
            Ok(output) => {
                let mut response = DesktopBackupRecoveryResponse::success(
                    restore_state_name(output.state()),
                    Some(to_backup_manifest(output.package_id(), output.summary())),
                );
                response.confirmation_ready = Some(output.confirmation_ready());
                response.error_code = output.validation_error_code().map(str::to_string);
                response
            }
            Err(error) => DesktopBackupRecoveryResponse::failure(
                "Failed",
                error.code(),
                matches!(
                    error,
                    cabinet_usecases::backup_package::BackupPackageUsecaseError::StorageUnavailable
                ),
            ),
        }
    }

    pub fn confirm(
        &self,
        request: DesktopRestoreConfirmRequestDto,
    ) -> DesktopBackupRecoveryResponse {
        let mut packages = LocalBackupPackageStore::new(self.root.clone(), self.policy);
        let mut restores = LocalBackupRestoreStore::new(self.root.clone(), self.policy);
        let mut reopener = LocalWorkspaceReopener::new(self.root.clone());
        let mut logger = self.logger();
        match ConfirmBackupRestoreUsecase::new().execute(
            ConfirmBackupRestoreInput::new(
                "local-user",
                &request.workspace_id,
                &request.package_id,
                &request.operation_id,
                request.confirmed,
            ),
            &mut packages,
            &mut restores,
            &mut reopener,
            &mut logger,
        ) {
            Ok(output) => {
                let mut response = DesktopBackupRecoveryResponse::success(
                    restore_state_name(output.state()),
                    None,
                );
                response.operation_id = Some(output.operation_id().to_string());
                response.error_code = output.error_code().map(str::to_string);
                response.retryable = output.state() == RestoreState::RecoveryRequired;
                response
            }
            Err(error) => DesktopBackupRecoveryResponse::failure(
                "Failed",
                error.code(),
                matches!(
                    error,
                    cabinet_usecases::backup_restore::ConfirmBackupRestoreError::StorageUnavailable
                ),
            ),
        }
    }

    pub fn start_restore_operation(
        &self,
        request: DesktopRestoreConfirmRequestDto,
    ) -> DesktopBackupRecoveryResponse {
        let mut packages = LocalBackupPackageStore::new(self.root.clone(), self.policy);
        let mut restores = LocalBackupRestoreStore::new(self.root.clone(), self.policy);
        let mut logger = self.logger();
        match StartBackupRestoreOperationUsecase::new().execute(
            StartBackupRestoreOperationInput::new(
                "local-user",
                &request.workspace_id,
                &request.package_id,
                &request.operation_id,
                request.confirmed,
            ),
            &mut packages,
            &mut restores,
            &mut logger,
        ) {
            Ok(output) => restore_operation_response(output),
            Err(error) => DesktopBackupRecoveryResponse::failure(
                "Failed",
                error.code(),
                matches!(
                    error,
                    cabinet_usecases::backup_restore::ConfirmBackupRestoreError::StorageUnavailable
                ),
            ),
        }
    }

    pub fn run_restore_operation(
        &self,
        request: DesktopRestoreConfirmRequestDto,
    ) -> DesktopBackupRecoveryResponse {
        let mut response = self.confirm(request.clone());
        if response.ok && response.state == "Completed" {
            let catalog = LocalCurrentDocumentProjectionCatalog::new(self.root.clone());
            let mut repository = DurableProjectionWorkRepository::new(self.root.clone());
            match RebuildRestoreProjectionsUsecase::new(100_000).execute(
                RebuildRestoreProjectionsInput::new(&request.workspace_id),
                &catalog,
                &mut repository,
            ) {
                Ok(_) => {
                    self.logger().push(DesktopBackupProductEvent::Operation {
                        event_name: "restore.projection_rebuild.requested".into(),
                        state: "Pending".into(),
                        error_code: None,
                    });
                    if let Err(error_code) =
                        self.process_restored_projections(&request.workspace_id)
                    {
                        self.mark_projection_rebuild_failed(
                            &request.workspace_id,
                            &request.operation_id,
                            error_code,
                            &mut response,
                        );
                    } else {
                        self.logger().push(DesktopBackupProductEvent::Operation {
                            event_name: "restore.projection_rebuild.completed".into(),
                            state: "Completed".into(),
                            error_code: None,
                        });
                    }
                }
                Err(error) => {
                    self.mark_projection_rebuild_failed(
                        &request.workspace_id,
                        &request.operation_id,
                        error.code(),
                        &mut response,
                    );
                }
            }
        }
        response
    }

    fn process_restored_projections(&self, workspace_id: &str) -> Result<(), &'static str> {
        let projection = DesktopProjectionRuntime::new(
            self.root.clone(),
            self.projection_body_max_bytes,
            self.projection_batch_limit,
            self.projection_max_attempts,
        )
        .map_err(|_| "RESTORE_PROJECTION_RUNTIME_INVALID")?;
        for _ in 0..=self.projection_max_attempts {
            let pending_count = DurableProjectionWorkRepository::new(self.root.clone())
                .list_resumable(300_000)
                .map_err(|_| "RESTORE_PROJECTION_REPOSITORY_UNAVAILABLE")?
                .len();
            let max_runs = pending_count
                .div_ceil(self.projection_batch_limit)
                .saturating_mul(self.projection_max_attempts as usize)
                .saturating_add(1);
            let mut drained = false;
            for _ in 0..max_runs {
                let processed = projection.run_once();
                if !processed.ok {
                    return Err("RESTORE_PROJECTION_PROCESSING_FAILED");
                }
                if processed.ready_count == 0
                    && processed.retry_scheduled_count == 0
                    && processed.failed_count == 0
                {
                    drained = true;
                    break;
                }
            }
            if !drained {
                return Err("RESTORE_PROJECTION_PROCESSING_INCOMPLETE");
            }
            let reconciled = projection.reconcile_current(workspace_id, 100_000);
            if !reconciled.ok {
                return Err("RESTORE_PROJECTION_RECONCILE_FAILED");
            }
            if reconciled.ready_document_count == reconciled.document_count {
                return Ok(());
            }
            if reconciled.enqueued_count == 0 && reconciled.reset_count == 0 {
                return Err("RESTORE_PROJECTION_PROCESSING_FAILED");
            }
        }
        Err("RESTORE_PROJECTION_PROCESSING_INCOMPLETE")
    }

    fn mark_projection_rebuild_failed(
        &self,
        workspace_id: &str,
        operation_id: &str,
        error_code: &'static str,
        response: &mut DesktopBackupRecoveryResponse,
    ) {
        let workspace = WorkspaceId::new(workspace_id);
        let operation = BackupJobId::new(operation_id);
        let mut restores = LocalBackupRestoreStore::new(self.root.clone(), self.policy);
        if let (Ok(workspace), Ok(operation)) = (workspace, operation) {
            let _ = restores.mark_cleanup_required(&workspace, &operation);
        }
        self.logger().push(DesktopBackupProductEvent::Operation {
            event_name: "restore.projection_rebuild.failed".into(),
            state: "CleanupRequired".into(),
            error_code: Some(error_code.into()),
        });
        response.state = "CleanupRequired".into();
        response.error_code = Some(error_code.into());
        response.retryable = true;
    }

    pub fn restore_operation_status(
        &self,
        request: DesktopRestoreCancelRequestDto,
    ) -> DesktopBackupRecoveryResponse {
        let restores = LocalBackupRestoreStore::new(self.root.clone(), self.policy);
        match GetBackupRestoreOperationUsecase::new().execute(
            GetBackupRestoreOperationInput::new(
                "local-user",
                &request.workspace_id,
                &request.operation_id,
            ),
            &restores,
        ) {
            Ok(output) => restore_operation_response(output),
            Err(error) => DesktopBackupRecoveryResponse::failure(
                "Failed",
                error.code(),
                matches!(
                    error,
                    cabinet_usecases::backup_restore::ConfirmBackupRestoreError::StorageUnavailable
                ),
            ),
        }
    }

    pub fn cancel(&self, request: DesktopRestoreCancelRequestDto) -> DesktopBackupRecoveryResponse {
        let mut restores = LocalBackupRestoreStore::new(self.root.clone(), self.policy);
        let mut logger = self.logger();
        match CancelBackupRestoreUsecase::new().execute(
            CancelBackupRestoreInput::new(
                "local-user",
                &request.workspace_id,
                &request.operation_id,
            ),
            &mut restores,
            &mut logger,
        ) {
            Ok(output) => {
                let mut response = DesktopBackupRecoveryResponse::success(
                    restore_state_name(output.state()),
                    None,
                );
                response.operation_id = Some(output.operation_id().to_string());
                response.error_code = output.error_code().map(str::to_string);
                response
            }
            Err(error) => DesktopBackupRecoveryResponse::failure(
                "Failed",
                error.code(),
                matches!(
                    error,
                    cabinet_usecases::backup_restore::ConfirmBackupRestoreError::StorageUnavailable
                ),
            ),
        }
    }

    pub fn recover_startup(
        &self,
        request: DesktopBackupRecoveryRequestDto,
    ) -> DesktopBackupRecoveryResponse {
        let mut restores = LocalBackupRestoreStore::new(self.root.clone(), self.policy);
        let mut logger = self.logger();
        match RecoverBackupStartupUsecase::new().execute(
            RecoverBackupStartupInput::new("local-user", &request.workspace_id),
            &mut restores,
            &mut logger,
        ) {
            Ok(output) => {
                let mut response = DesktopBackupRecoveryResponse::success("Completed", None);
                response.recovery = Some(DesktopStartupRecoveryDto {
                    cleaned_staging_count: output.cleaned_staging_count(),
                    rolled_back_operation_ids: output.rolled_back_operation_ids().to_vec(),
                    cleanup_required_operation_ids: output.cleanup_required_operation_ids().to_vec(),
                });
                response
            }
            Err(error) => DesktopBackupRecoveryResponse::failure(
                "Failed",
                match error {
                    cabinet_usecases::backup_recovery::BackupRecoveryUsecaseError::InvalidInput => "BACKUP_RECOVERY_INVALID_INPUT",
                    cabinet_usecases::backup_recovery::BackupRecoveryUsecaseError::StorageUnavailable => "BACKUP_RECOVERY_STORAGE_UNAVAILABLE",
                },
                matches!(error, cabinet_usecases::backup_recovery::BackupRecoveryUsecaseError::StorageUnavailable),
            ),
        }
    }

    fn logger(&self) -> DesktopBackupLogSink {
        DesktopBackupLogSink {
            events: Arc::clone(&self.product_events),
        }
    }
}

impl DesktopBackupRecoveryResponse {
    fn success(state: &str, manifest: Option<DesktopBackupManifestDto>) -> Self {
        Self {
            ok: true,
            state: state.to_string(),
            manifest,
            confirmation_ready: None,
            operation_id: None,
            recovery: None,
            error_code: None,
            retryable: false,
        }
    }

    fn failure(state: &str, error_code: &str, retryable: bool) -> Self {
        Self {
            ok: false,
            state: state.to_string(),
            manifest: None,
            confirmation_ready: None,
            operation_id: None,
            recovery: None,
            error_code: Some(error_code.to_string()),
            retryable,
        }
    }
}

struct DesktopBackupLogSink {
    events: Arc<Mutex<Vec<DesktopBackupProductEvent>>>,
}

impl BackupPackageUsecaseLogger for DesktopBackupLogSink {
    fn write_product(&mut self, event: BackupPackageProductEvent) {
        self.push(DesktopBackupProductEvent::Package {
            event_name: event.event_name().to_string(),
            error_code: event.error_code().map(str::to_string),
        });
    }
}

impl BackupRestoreUsecaseLogger for DesktopBackupLogSink {
    fn write_product(&mut self, event: BackupRestoreProductEvent) {
        self.push(DesktopBackupProductEvent::Restore {
            event_name: event.event_name().to_string(),
            state: restore_state_name(event.state()).to_string(),
            error_code: event.error_code().map(str::to_string),
        });
    }
}

impl BackupRecoveryUsecaseLogger for DesktopBackupLogSink {
    fn write_product(&mut self, event: BackupRecoveryProductEvent) {
        self.push(DesktopBackupProductEvent::Recovery {
            event_name: event.event_name().to_string(),
        });
    }
}

impl BackupPackageOperationLogger for DesktopBackupLogSink {
    fn write_product(&mut self, event: BackupPackageOperationEvent) {
        self.push(DesktopBackupProductEvent::Operation {
            event_name: event.event_name().to_string(),
            state: backup_job_state_name(event.state()).to_string(),
            error_code: event.error_code().map(str::to_string),
        });
    }
}

impl DesktopBackupLogSink {
    fn push(&self, event: DesktopBackupProductEvent) {
        self.events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(event);
    }
}

fn to_backup_manifest(
    package_id: &str,
    summary: &BackupPackageSummary,
) -> DesktopBackupManifestDto {
    DesktopBackupManifestDto {
        package_id: package_id.to_string(),
        schema_version: summary.schema_version(),
        created_at_epoch_ms: summary.created_at_epoch_ms(),
        entries: summary
            .entries()
            .iter()
            .map(|entry| DesktopBackupManifestEntryDto {
                data_class: backup_data_class_name(entry.data_class()).to_string(),
                record_count: entry.record_count(),
                byte_count: entry.byte_count(),
            })
            .collect(),
    }
}

const fn backup_data_class_name(data_class: BackupDataClass) -> &'static str {
    match data_class {
        BackupDataClass::CurrentDocuments => "current_documents",
        BackupDataClass::VersionHistory => "version_history",
        BackupDataClass::CanvasRecords => "canvas_records",
        BackupDataClass::AssetMetadata => "asset_metadata",
        BackupDataClass::AssetObjects => "asset_objects",
        BackupDataClass::AssetAssociations => "asset_associations",
        BackupDataClass::GraphRebuildMetadata => "graph_rebuild_metadata",
        BackupDataClass::SearchRebuildMetadata => "search_rebuild_metadata",
    }
}

const fn restore_state_name(state: RestoreState) -> &'static str {
    match state {
        RestoreState::Requested => "Requested",
        RestoreState::Previewing => "Previewing",
        RestoreState::Validating => "Validating",
        RestoreState::AwaitingConfirmation => "AwaitingConfirmation",
        RestoreState::Staging => "Staging",
        RestoreState::Applying => "Applying",
        RestoreState::Reopening => "Reopening",
        RestoreState::CleanupRequired => "CleanupRequired",
        RestoreState::RollbackRequired => "RollbackRequired",
        RestoreState::RecoveryRequired => "RecoveryRequired",
        RestoreState::Completed => "Completed",
        RestoreState::Failed => "Failed",
        RestoreState::Cancelled => "Cancelled",
        RestoreState::RolledBack => "RolledBack",
    }
}

fn to_operation_response(output: BackupPackageOperationOutput) -> DesktopBackupOperationResponse {
    DesktopBackupOperationResponse {
        ok: true,
        operation_id: output.operation_id().to_string(),
        state: backup_job_state_name(output.state()).to_string(),
        progress_completed_units: output.progress_completed_units(),
        progress_total_units: output.progress_total_units(),
        error_code: output.error_code().map(str::to_string),
        retryable: output.state() == BackupJobState::Failed,
    }
}

fn restore_operation_response(
    output: cabinet_usecases::backup_restore::ConfirmBackupRestoreOutput,
) -> DesktopBackupRecoveryResponse {
    let mut response =
        DesktopBackupRecoveryResponse::success(restore_state_name(output.state()), None);
    response.operation_id = Some(output.operation_id().to_string());
    response.error_code = output.error_code().map(str::to_string);
    response.retryable = output.state() == RestoreState::RecoveryRequired;
    response
}

fn backup_operation_failure(
    operation_id: &str,
    error_code: &str,
) -> DesktopBackupOperationResponse {
    DesktopBackupOperationResponse {
        ok: false,
        operation_id: operation_id.to_string(),
        state: "Failed".to_string(),
        progress_completed_units: 0,
        progress_total_units: 1,
        error_code: Some(error_code.to_string()),
        retryable: error_code == "BACKUP_OPERATION_STORAGE_UNAVAILABLE",
    }
}

const fn backup_job_state_name(state: BackupJobState) -> &'static str {
    match state {
        BackupJobState::Queued => "Queued",
        BackupJobState::Running => "Running",
        BackupJobState::Completed => "Completed",
        BackupJobState::Failed => "Failed",
        BackupJobState::Retrying => "Retrying",
        BackupJobState::Abandoned => "Abandoned",
    }
}

/// UI-safe command response DTO exposed by the desktop shell boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesktopLocalCommandRuntimeResponse {
    pub boundary: String,
    pub command_name: String,
    pub accepted: bool,
    pub usecase_name: Option<String>,
    pub error_code: Option<String>,
    pub retryable: bool,
    pub body_byte_len: Option<usize>,
    pub asset_byte_len: Option<u64>,
    pub result_limit: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct DesktopWorkspaceHomeRuntime {
    projection_store: LocalWorkspaceHomeQueryStore,
}

impl DesktopWorkspaceHomeRuntime {
    pub fn new(projection_root: PathBuf) -> Self {
        Self {
            projection_store: LocalWorkspaceHomeQueryStore::new(projection_root),
        }
    }

    pub fn execute(
        &self,
        request: DesktopLocalCommandRequestDto,
    ) -> DesktopWorkspaceHomeCommandResponse {
        let platform_request = to_platform_runtime_request(request);
        match map_core_local_desktop_command_request(platform_request) {
            Ok(input) => match execute_workspace_home_command(input, &self.projection_store) {
                Ok(result) => DesktopWorkspaceHomeCommandResponse::success(result),
                Err(error) => DesktopWorkspaceHomeCommandResponse::failure(error),
            },
            Err(error) => DesktopWorkspaceHomeCommandResponse {
                ok: false,
                data: None,
                error_code: Some(map_platform_error_code(error.error_code).to_string()),
                retryable: false,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopWorkspaceHomeCommandResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DesktopWorkspaceHomeDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub retryable: bool,
}

impl DesktopWorkspaceHomeCommandResponse {
    fn success(result: WorkspaceHomeCommandResult) -> Self {
        Self {
            ok: true,
            data: Some(DesktopWorkspaceHomeDataDto::from(result)),
            error_code: None,
            retryable: false,
        }
    }

    fn failure(error: WorkspaceHomeCommandFailure) -> Self {
        Self {
            ok: false,
            data: None,
            error_code: Some(error.error_code.to_string()),
            retryable: error.retryable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopWorkspaceHomeDataDto {
    pub workspace_id: String,
    pub state: String,
    pub recent_documents: Vec<DesktopWorkspaceHomeDocumentDto>,
    pub favorites: Vec<DesktopWorkspaceHomeDocumentDto>,
    pub tags: Vec<DesktopWorkspaceHomeTagDto>,
    pub recent_changes: Vec<DesktopWorkspaceHomeChangeDto>,
    pub unfinished_items: Vec<DesktopWorkspaceHomeUnfinishedDto>,
    pub backup_status: String,
    pub health_status: String,
    pub document_count: u32,
    pub asset_count: u32,
    pub canvas_count: u32,
    pub summary_unavailable: Vec<String>,
}

impl From<WorkspaceHomeCommandResult> for DesktopWorkspaceHomeDataDto {
    fn from(result: WorkspaceHomeCommandResult) -> Self {
        Self {
            workspace_id: result.workspace_id,
            state: match result.state {
                WorkspaceHomeCommandLoadState::Ready => "Ready",
                WorkspaceHomeCommandLoadState::Empty => "Empty",
                WorkspaceHomeCommandLoadState::Degraded => "Degraded",
            }
            .to_string(),
            recent_documents: result
                .recent_documents
                .into_iter()
                .map(|item| DesktopWorkspaceHomeDocumentDto {
                    document_id: item.document_id,
                    title: item.title,
                    path: item.path,
                })
                .collect(),
            favorites: result
                .favorites
                .into_iter()
                .map(|item| DesktopWorkspaceHomeDocumentDto {
                    document_id: item.document_id,
                    title: item.title,
                    path: item.path,
                })
                .collect(),
            tags: result
                .tags
                .into_iter()
                .map(|item| DesktopWorkspaceHomeTagDto {
                    label: item.label,
                    document_count: item.document_count,
                })
                .collect(),
            recent_changes: result
                .recent_changes
                .into_iter()
                .map(|item| DesktopWorkspaceHomeChangeDto {
                    document_id: item.document_id,
                    summary: item.summary,
                })
                .collect(),
            unfinished_items: result
                .unfinished_items
                .into_iter()
                .map(|item| DesktopWorkspaceHomeUnfinishedDto {
                    document_id: item.document_id,
                    label: item.label,
                })
                .collect(),
            backup_status: result.backup_status.to_string(),
            health_status: result.health_status.to_string(),
            document_count: result.document_count,
            asset_count: result.asset_count,
            canvas_count: result.canvas_count,
            summary_unavailable: result
                .summary_unavailable
                .into_iter()
                .map(str::to_string)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopWorkspaceHomeDocumentDto {
    pub document_id: String,
    pub title: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopWorkspaceHomeTagDto {
    pub label: String,
    pub document_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopWorkspaceHomeChangeDto {
    pub document_id: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopWorkspaceHomeUnfinishedDto {
    pub document_id: String,
    pub label: String,
}

/// Report returned by the packaged desktop smoke path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopPackageSmokeReport {
    pub boundary: &'static str,
    pub dist_dir: PathBuf,
    pub index_html_exists: bool,
    pub app_bundle_exists: bool,
    pub styles_css_exists: bool,
    pub node_runtime_required: bool,
}

/// Routes a command through the desktop shell boundary.
pub fn route_desktop_command(request: DesktopShellRequest) -> DesktopShellResponse {
    DesktopShellResponse {
        boundary: cabinet_platform::layer_name(),
        command: request.command,
    }
}

/// Routes a Tauri command DTO without exposing Tauri types to the domain or usecase layers.
pub fn route_tauri_command(command: String) -> DesktopShellCommandResponse {
    let response = route_desktop_command(DesktopShellRequest { command });
    DesktopShellCommandResponse {
        boundary: response.boundary.to_string(),
        command: response.command,
    }
}

const LOCAL_DESKTOP_COMMAND_NAMES: &[&str] = &[
    "local_workspace_bootstrap",
    "local_workspace_home",
    "local_document_navigator",
    "create_document",
    "save_document_revision",
    "get_current_document",
    "update_current_document",
    "get_document_history",
    "get_document_version",
    "preview_document_restore",
    "restore_document_version",
    "search_documents",
    "search_assets",
    "get_link_overview",
    "get_graph_projection",
    "list_document_assets",
    "attach_document_asset",
    "create_backup",
    "preview_import",
    "preview_restore",
    "apply_restore",
];

/// Returns the Phase 009 command names accepted by the local desktop command bridge.
pub const fn local_desktop_command_names() -> &'static [&'static str] {
    LOCAL_DESKTOP_COMMAND_NAMES
}

/// Routes a local desktop command name without performing persistence or business logic.
pub fn route_local_desktop_command(command_name: String) -> DesktopLocalCommandRouteResponse {
    let accepted = LOCAL_DESKTOP_COMMAND_NAMES.contains(&command_name.as_str());
    DesktopLocalCommandRouteResponse {
        boundary: cabinet_platform::layer_name().to_string(),
        command_name,
        accepted,
        error_code: if accepted {
            None
        } else {
            Some("COMMAND_UNSUPPORTED".to_string())
        },
    }
}

/// Routes a typed local desktop command DTO through the platform mapper.
pub fn route_local_desktop_command_request(
    request: DesktopLocalCommandRequestDto,
) -> DesktopLocalCommandRuntimeResponse {
    let platform_request = to_platform_runtime_request(request);
    let summary = summarize_local_desktop_command_for_product_log(&platform_request);
    let command_name = platform_request.command_name.clone();

    match map_core_local_desktop_command_request(platform_request) {
        Ok(input) => DesktopLocalCommandRuntimeResponse {
            boundary: cabinet_platform::layer_name().to_string(),
            command_name,
            accepted: true,
            usecase_name: Some(usecase_input_name(&input).to_string()),
            error_code: None,
            retryable: false,
            body_byte_len: summary.body_byte_len,
            asset_byte_len: summary.asset_byte_len,
            result_limit: summary.result_limit,
        },
        Err(error) => DesktopLocalCommandRuntimeResponse {
            boundary: cabinet_platform::layer_name().to_string(),
            command_name,
            accepted: false,
            usecase_name: None,
            error_code: Some(map_platform_error_code(error.error_code).to_string()),
            retryable: false,
            body_byte_len: summary.body_byte_len,
            asset_byte_len: summary.asset_byte_len,
            result_limit: summary.result_limit,
        },
    }
}

fn to_platform_runtime_request(
    request: DesktopLocalCommandRequestDto,
) -> LocalDesktopRuntimeCommandRequest {
    LocalDesktopRuntimeCommandRequest::new(
        &request.command_name,
        match request.payload {
            DesktopLocalCommandPayloadDto::Empty => LocalDesktopCommandPayload::Empty,
            DesktopLocalCommandPayloadDto::WorkspaceHome {
                workspace_id,
                recent_documents,
                favorites,
                tags,
                recent_changes,
                unfinished_items,
            } => LocalDesktopCommandPayload::WorkspaceHome {
                workspace_id,
                recent_documents,
                favorites,
                tags,
                recent_changes,
                unfinished_items,
            },
            DesktopLocalCommandPayloadDto::DocumentIdentity {
                workspace_id,
                document_id,
            } => LocalDesktopCommandPayload::DocumentIdentity {
                workspace_id,
                document_id,
            },
            DesktopLocalCommandPayloadDto::DocumentUpdate {
                workspace_id,
                document_id,
                title,
                path,
                body,
                expected_version_id,
            } => LocalDesktopCommandPayload::DocumentUpdate {
                workspace_id,
                document_id,
                title,
                path,
                body,
                expected_version_id,
            },
            DesktopLocalCommandPayloadDto::DocumentHistory {
                workspace_id,
                document_id,
                limit,
            } => LocalDesktopCommandPayload::DocumentHistory {
                workspace_id,
                document_id,
                limit,
            },
            DesktopLocalCommandPayloadDto::DocumentVersion {
                workspace_id,
                document_id,
                version_id,
            } => LocalDesktopCommandPayload::DocumentVersion {
                workspace_id,
                document_id,
                version_id,
            },
            DesktopLocalCommandPayloadDto::Search {
                workspace_id,
                text,
                limit,
            } => LocalDesktopCommandPayload::Search {
                workspace_id,
                text,
                limit,
            },
            DesktopLocalCommandPayloadDto::GraphProjection {
                workspace_id,
                document_id,
                depth,
                direction,
                include_unresolved,
                include_assets,
                node_limit,
                edge_limit,
            } => LocalDesktopCommandPayload::GraphProjection {
                workspace_id,
                document_id,
                depth,
                direction,
                include_unresolved,
                include_assets,
                node_limit,
                edge_limit,
            },
            DesktopLocalCommandPayloadDto::AssetAttachment {
                workspace_id,
                document_id,
                asset_id,
                label,
                file_name,
                media_type,
                byte_size,
            } => LocalDesktopCommandPayload::AssetAttachment {
                workspace_id,
                document_id,
                asset_id,
                label,
                file_name,
                media_type,
                byte_size,
            },
            DesktopLocalCommandPayloadDto::Workspace { workspace_id } => {
                LocalDesktopCommandPayload::Workspace { workspace_id }
            }
            DesktopLocalCommandPayloadDto::ImportPreview {
                workspace_id,
                source_label,
                file_count,
            } => LocalDesktopCommandPayload::ImportPreview {
                workspace_id,
                source_label,
                file_count,
            },
            DesktopLocalCommandPayloadDto::RestorePackage {
                workspace_id,
                package_label,
            } => LocalDesktopCommandPayload::RestorePackage {
                workspace_id,
                package_label,
            },
        },
    )
}

fn usecase_input_name(input: &LocalDesktopUsecaseInput) -> &'static str {
    match input {
        LocalDesktopUsecaseInput::BootstrapWorkspace => "BootstrapWorkspace",
        LocalDesktopUsecaseInput::WorkspaceHome { .. } => "WorkspaceHome",
        LocalDesktopUsecaseInput::GetCurrentDocument { .. } => "GetCurrentDocument",
        LocalDesktopUsecaseInput::UpdateCurrentDocument { .. } => "UpdateCurrentDocument",
        LocalDesktopUsecaseInput::GetDocumentHistory { .. } => "GetDocumentHistory",
        LocalDesktopUsecaseInput::GetDocumentVersion { .. } => "GetDocumentVersion",
        LocalDesktopUsecaseInput::PreviewDocumentRestore { .. } => "PreviewDocumentRestore",
        LocalDesktopUsecaseInput::RestoreDocumentVersion { .. } => "RestoreDocumentVersion",
        LocalDesktopUsecaseInput::SearchDocuments { .. } => "SearchDocuments",
        LocalDesktopUsecaseInput::SearchAssets { .. } => "SearchAssets",
        LocalDesktopUsecaseInput::GetLinkOverview { .. } => "GetLinkOverview",
        LocalDesktopUsecaseInput::GetGraphProjection { .. } => "GetGraphProjection",
        LocalDesktopUsecaseInput::ListDocumentAssets { .. } => "ListDocumentAssets",
        LocalDesktopUsecaseInput::AttachDocumentAsset { .. } => "AttachDocumentAsset",
        LocalDesktopUsecaseInput::CreateBackup { .. } => "CreateBackup",
        LocalDesktopUsecaseInput::PreviewImport { .. } => "PreviewImport",
        LocalDesktopUsecaseInput::PreviewRestore { .. } => "PreviewRestore",
        LocalDesktopUsecaseInput::ApplyRestore { .. } => "ApplyRestore",
    }
}

fn map_platform_error_code(error_code: LocalDesktopCommandErrorCode) -> &'static str {
    match error_code {
        LocalDesktopCommandErrorCode::UnsupportedCommand => "COMMAND_UNSUPPORTED",
        LocalDesktopCommandErrorCode::InvalidInput => "COMMAND_INVALID_INPUT",
        LocalDesktopCommandErrorCode::UsecaseFailed => "COMMAND_USECASE_FAILED",
        LocalDesktopCommandErrorCode::ResultMappingFailed => "COMMAND_RESULT_MAPPING_FAILED",
        LocalDesktopCommandErrorCode::InvalidTransition => "COMMAND_INVALID_TRANSITION",
    }
}

/// Returns the expected bundled desktop asset directory.
pub fn bundled_desktop_dist_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri must have a desktop parent directory")
        .join("dist")
}

/// Checks whether the packaged desktop runtime can load bundled Web assets without Node.js.
pub fn create_desktop_package_smoke_report() -> DesktopPackageSmokeReport {
    let dist_dir = bundled_desktop_dist_dir();
    DesktopPackageSmokeReport {
        boundary: cabinet_platform::layer_name(),
        index_html_exists: dist_dir.join("index.html").is_file(),
        app_bundle_exists: dist_dir.join("app.bundle.js").is_file(),
        styles_css_exists: dist_dir.join("styles.css").is_file(),
        dist_dir,
        node_runtime_required: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabinet_domain::asset_import_operation::AssetImportEvent;
    use cabinet_domain::document::{DocumentBody, DocumentMetadata, DocumentPath, DocumentTitle};
    use cabinet_domain::version::CurrentDocumentSnapshot;
    use cabinet_ports::document_repository::{CurrentDocumentRecord, DocumentRepository};
    use cabinet_ports::graph_projection::GraphProjectionStore;
    use cabinet_ports::link_target_resolver::{DocumentLinkTargetResolver, LinkTargetResolution};

    #[test]
    fn asset_import_projection_warning_requires_recovery_without_exposing_details() {
        let response = DesktopAssetImportResponse::completed_with_projection_warning(
            "operation-1",
            "asset-1",
            ReindexAssetGraphProjectionError::RepositoryUnavailable,
        );

        assert!(response.ok);
        assert_eq!(response.state.as_deref(), Some("recovery_required"));
        assert!(response.repair_required);
        assert_eq!(
            response.error_code.as_deref(),
            Some("asset_graph_reindex.repository_unavailable")
        );
        assert!(response.retryable);
        let json = serde_json::to_string(&response).expect("response json");
        assert!(json.contains("\"repairRequired\":true"));
        assert!(!json.contains("/private/"));
        assert!(!json.contains("document body"));
    }

    #[test]
    fn asset_import_status_projection_warning_requires_recovery() {
        let workspace = WorkspaceId::new("workspace-1").expect("workspace");
        let mut operation = AssetImportOperation::new(
            AssetImportOperationId::new("operation-2").expect("operation"),
            workspace,
            DocumentId::new("document-1").expect("document"),
            1,
        )
        .expect("operation");
        operation.apply(AssetImportEvent::Begin, 1).expect("begin");
        operation
            .apply(AssetImportEvent::ValidationSucceeded, 1)
            .expect("validation");
        operation
            .apply(AssetImportEvent::StagingSucceeded, 1)
            .expect("staging");
        operation
            .apply(AssetImportEvent::HashingSucceeded, 1)
            .expect("hashing");
        operation
            .apply(AssetImportEvent::ObjectPublished, 1)
            .expect("object");
        operation
            .apply(AssetImportEvent::MetadataPersisted, 1)
            .expect("metadata");
        operation
            .apply(AssetImportEvent::LinkSucceeded, 1)
            .expect("association");

        let response = DesktopAssetImportResponse::operation_with_projection_warning(
            &operation,
            ReindexAssetGraphProjectionError::CorruptedState,
        );

        assert!(response.ok);
        assert_eq!(response.state.as_deref(), Some("recovery_required"));
        assert!(response.repair_required);
        assert!(!response.retryable);
    }

    #[test]
    fn asset_import_terminal_responses_do_not_require_recovery_by_default() {
        let completed = DesktopAssetImportResponse::completed("operation-3", "asset-3");
        let failed = DesktopAssetImportResponse::failure("asset_import.invalid_input", false);

        assert!(!completed.repair_required);
        assert!(!failed.repair_required);
    }

    #[test]
    fn document_change_sink_persists_rename_and_delete_catalog_lifecycle() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cabinet-desktop-catalog-lifecycle-{}-{suffix}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut sink = DesktopDocumentChangeSink::new(root.clone());
        seed_current_document(&mut sink.documents, "Original", "notes/original.md");
        sink.publish(DocumentChangeEvent::DocumentCreated {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            version_id: "version-1".to_string(),
            title: "Original".to_string(),
            path: "notes/original.md".to_string(),
        });
        assert_eq!(sink.last_error_code, None);
        seed_current_document(&mut sink.documents, "Renamed", "notes/renamed.md");
        sink.publish(DocumentChangeEvent::DocumentRenamed {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            version_id: "version-1".to_string(),
            title: "Renamed".to_string(),
            old_path: "notes/original.md".to_string(),
            new_path: "notes/renamed.md".to_string(),
        });
        assert_eq!(sink.last_error_code, None);
        drop(sink);

        let workspace = WorkspaceId::new("workspace-1").unwrap();
        let catalog = DurableDocumentLinkCatalog::new(root.clone());
        assert!(matches!(
            catalog.resolve(&workspace, "Original").unwrap(),
            LinkTargetResolution::Unresolved(_)
        ));
        assert!(matches!(
            catalog.resolve(&workspace, "Renamed").unwrap(),
            LinkTargetResolution::Resolved(_)
        ));
        drop(catalog);

        let mut sink = DesktopDocumentChangeSink::new(root.clone());
        sink.publish(DocumentChangeEvent::DocumentDeleted {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            version_id: "version-1".to_string(),
        });
        assert_eq!(sink.last_error_code, None);
        drop(sink);
        let catalog = DurableDocumentLinkCatalog::new(root.clone());
        assert!(matches!(
            catalog.resolve(&workspace, "Renamed").unwrap(),
            LinkTargetResolution::Unresolved(_)
        ));
        let _ = std::fs::remove_dir_all(root);
    }

    fn seed_current_document(repository: &mut LocalDocumentRepository, title: &str, path: &str) {
        let workspace = WorkspaceId::new("workspace-1").expect("workspace");
        let document = DocumentId::new("doc-1").expect("document");
        let metadata = DocumentMetadata::new(
            document.clone(),
            DocumentTitle::new(title).expect("title"),
            DocumentPath::new(path).expect("path"),
        )
        .expect("metadata");
        let snapshot = CurrentDocumentSnapshot::new(
            document,
            DocumentBody::new(
                "catalog lifecycle fixture",
                DocumentBodyPolicy::new(1024).expect("body policy"),
            )
            .expect("body"),
        );
        repository
            .put_current(
                &workspace,
                CurrentDocumentRecord::new(metadata, snapshot).expect("current record"),
            )
            .expect("seed current document");
    }

    #[test]
    fn document_change_sink_fans_out_rename_resolution_to_source_graph() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cabinet-desktop-reference-fanout-{}-{suffix}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let authoring = DesktopDocumentAuthoringRuntime::new(root.clone(), 4096).unwrap();
        for request in [
            DesktopDocumentAuthoringRequestDto::Create {
                workspace_id: "workspace-1".into(),
                document_id: "target".into(),
                path: "target.md".into(),
                body: "# Target\ntarget body".into(),
                version_id: "tv1".into(),
                snapshot_ref: "snapshot-tv1".into(),
                author: "local-user".into(),
                summary: "Created".into(),
            },
            DesktopDocumentAuthoringRequestDto::Create {
                workspace_id: "workspace-1".into(),
                document_id: "source".into(),
                path: "source.md".into(),
                body: "# Source\n[[Target]]".into(),
                version_id: "sv1".into(),
                snapshot_ref: "snapshot-sv1".into(),
                author: "local-user".into(),
                summary: "Created".into(),
            },
        ] {
            assert!(authoring.execute(request).ok);
        }
        drop(authoring);
        let projection = DesktopProjectionRuntime::new(root.clone(), 4096, 20, 3).unwrap();
        let initial_projection = projection.run_once();
        assert_eq!(
            (
                initial_projection.ready_count,
                initial_projection.retry_scheduled_count,
                initial_projection.failed_count,
            ),
            (6, 0, 0)
        );
        drop(projection);

        let mut sink = DesktopDocumentChangeSink::new(root.clone());
        sink.publish(DocumentChangeEvent::DocumentRenamed {
            workspace_id: "workspace-1".into(),
            document_id: "target".into(),
            version_id: "tv1".into(),
            title: "Renamed".into(),
            old_path: "target.md".into(),
            new_path: "renamed.md".into(),
        });
        assert_eq!(sink.last_error_code, None);
        drop(sink);
        let projection = DesktopProjectionRuntime::new(root.clone(), 4096, 20, 3).unwrap();
        assert_eq!(projection.run_once().ready_count, 6);
        drop(projection);

        let workspace = WorkspaceId::new("workspace-1").unwrap();
        let source = DocumentId::new("source").unwrap();
        let graph = DurableLocalGraphProjectionStore::new(root.clone())
            .get_projection(&workspace, &source)
            .unwrap()
            .unwrap();
        assert!(
            graph
                .graph()
                .nodes()
                .iter()
                .any(|node| node.kind() == GraphNodeKind::UnresolvedLink)
        );

        let mut sink = DesktopDocumentChangeSink::new(root.clone());
        sink.publish(DocumentChangeEvent::DocumentRenamed {
            workspace_id: "workspace-1".into(),
            document_id: "target".into(),
            version_id: "tv1".into(),
            title: "Target".into(),
            old_path: "renamed.md".into(),
            new_path: "target.md".into(),
        });
        assert_eq!(sink.last_error_code, None);
        drop(sink);
        let projection = DesktopProjectionRuntime::new(root.clone(), 4096, 20, 3).unwrap();
        assert_eq!(projection.run_once().ready_count, 3);
        drop(projection);

        let graph = DurableLocalGraphProjectionStore::new(root.clone())
            .get_projection(&workspace, &source)
            .unwrap()
            .unwrap();
        assert!(
            graph
                .graph()
                .nodes()
                .iter()
                .any(|node| node.kind() == GraphNodeKind::Document && node.id().contains("target"))
        );
        assert!(
            !graph
                .graph()
                .nodes()
                .iter()
                .any(|node| node.kind() == GraphNodeKind::UnresolvedLink)
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn desktop_shell_routes_to_platform_boundary() {
        let response = route_desktop_command(DesktopShellRequest {
            command: "open_workspace".to_string(),
        });

        assert_eq!(response.boundary, "platform");
        assert_eq!(response.command, "open_workspace");
    }

    #[test]
    fn tauri_command_boundary_maps_to_platform_response_dto() {
        let response = route_tauri_command("open_workspace".to_string());

        assert_eq!(response.boundary, "platform");
        assert_eq!(response.command, "open_workspace");
    }

    #[test]
    fn local_desktop_command_contract_exposes_phase009_command_names() {
        assert_eq!(
            local_desktop_command_names(),
            &[
                "local_workspace_bootstrap",
                "local_workspace_home",
                "local_document_navigator",
                "create_document",
                "save_document_revision",
                "get_current_document",
                "update_current_document",
                "get_document_history",
                "get_document_version",
                "preview_document_restore",
                "restore_document_version",
                "search_documents",
                "search_assets",
                "get_link_overview",
                "get_graph_projection",
                "list_document_assets",
                "attach_document_asset",
                "create_backup",
                "preview_import",
                "preview_restore",
                "apply_restore",
            ]
        );
    }

    #[test]
    fn local_desktop_command_route_accepts_known_command_without_payload_logging() {
        let response = route_local_desktop_command("update_current_document".to_string());

        assert_eq!(response.boundary, "platform");
        assert_eq!(response.command_name, "update_current_document");
        assert!(response.accepted);
        assert_eq!(response.error_code, None);
        assert!(!format!("{response:?}").contains("raw document body"));
    }

    #[test]
    fn local_desktop_command_route_rejects_old_alias_hosted_or_admin_commands() {
        for command_name in [
            "open_default_workspace",
            "save_current_document",
            "list_document_history",
            "get_asset_metadata",
            "server_workspace_connect",
            "tenant_admin",
            "billing",
            "sso_settings",
            "unknown",
        ] {
            let response = route_local_desktop_command(command_name.to_string());

            assert_eq!(response.boundary, "platform");
            assert_eq!(response.command_name, command_name);
            assert!(!response.accepted);
            assert_eq!(response.error_code, Some("COMMAND_UNSUPPORTED".to_string()));
        }
    }

    #[test]
    fn local_desktop_command_request_dto_maps_core_document_command_to_safe_response() {
        let response = route_local_desktop_command_request(DesktopLocalCommandRequestDto {
            command_name: "get_current_document".to_string(),
            payload: DesktopLocalCommandPayloadDto::DocumentIdentity {
                workspace_id: "workspace-1".to_string(),
                document_id: "doc-1".to_string(),
            },
        });

        assert_eq!(response.boundary, "platform");
        assert_eq!(response.command_name, "get_current_document");
        assert!(response.accepted);
        assert_eq!(
            response.usecase_name,
            Some("GetCurrentDocument".to_string())
        );
        assert_eq!(response.error_code, None);
        assert_eq!(response.body_byte_len, None);
    }

    #[test]
    fn workspace_home_request_dto_preserves_workspace_and_bounded_limits() {
        let response = route_local_desktop_command_request(DesktopLocalCommandRequestDto {
            command_name: "local_workspace_home".to_string(),
            payload: DesktopLocalCommandPayloadDto::WorkspaceHome {
                workspace_id: "workspace-1".to_string(),
                recent_documents: 12,
                favorites: 8,
                tags: 10,
                recent_changes: 14,
                unfinished_items: 6,
            },
        });

        assert!(response.accepted);
        assert_eq!(response.usecase_name, Some("WorkspaceHome".to_string()));
        assert_eq!(response.result_limit, Some(50));
        assert_eq!(response.error_code, None);
    }

    #[test]
    fn workspace_home_request_dto_rejects_invalid_limit_safely() {
        let response = route_local_desktop_command_request(DesktopLocalCommandRequestDto {
            command_name: "local_workspace_home".to_string(),
            payload: DesktopLocalCommandPayloadDto::WorkspaceHome {
                workspace_id: "private-workspace-id".to_string(),
                recent_documents: 101,
                favorites: 8,
                tags: 10,
                recent_changes: 14,
                unfinished_items: 6,
            },
        });

        assert!(!response.accepted);
        assert_eq!(
            response.error_code,
            Some("COMMAND_INVALID_INPUT".to_string())
        );
        assert!(!response.retryable);
        assert!(!format!("{response:?}").contains("private-workspace-id"));
    }

    #[test]
    fn local_desktop_command_request_dto_sanitizes_update_payload_response() {
        let response = route_local_desktop_command_request(DesktopLocalCommandRequestDto {
            command_name: "update_current_document".to_string(),
            payload: DesktopLocalCommandPayloadDto::DocumentUpdate {
                workspace_id: "workspace-1".to_string(),
                document_id: "doc-1".to_string(),
                title: "Source".to_string(),
                path: "docs/source.md".to_string(),
                body: "raw document body fixture must not leak".to_string(),
                expected_version_id: "version-1".to_string(),
            },
        });

        assert!(response.accepted);
        assert_eq!(
            response.usecase_name,
            Some("UpdateCurrentDocument".to_string())
        );
        assert_eq!(response.body_byte_len, Some(39));
        assert!(!format!("{response:?}").contains("raw document body fixture"));
        assert!(!format!("{response:?}").contains("docs/source.md"));
    }

    #[test]
    fn local_desktop_command_request_dto_maps_invalid_payload_to_stable_error_code() {
        let invalid_payload = route_local_desktop_command_request(DesktopLocalCommandRequestDto {
            command_name: "get_current_document".to_string(),
            payload: DesktopLocalCommandPayloadDto::Empty,
        });
        let unsupported = route_local_desktop_command_request(DesktopLocalCommandRequestDto {
            command_name: "unsupported_future_command".to_string(),
            payload: DesktopLocalCommandPayloadDto::Empty,
        });

        assert!(!invalid_payload.accepted);
        assert_eq!(
            invalid_payload.error_code,
            Some("COMMAND_INVALID_INPUT".to_string())
        );
        assert_eq!(invalid_payload.retryable, false);
        assert!(!unsupported.accepted);
        assert_eq!(
            unsupported.error_code,
            Some("COMMAND_UNSUPPORTED".to_string())
        );
        assert_eq!(unsupported.retryable, false);
    }

    #[test]
    fn local_desktop_command_request_dto_covers_discovery_asset_backup_and_restore_commands() {
        let commands = [
            DesktopLocalCommandRequestDto {
                command_name: "search_documents".to_string(),
                payload: DesktopLocalCommandPayloadDto::Search {
                    workspace_id: "workspace-1".to_string(),
                    text: "needle".to_string(),
                    limit: 10,
                },
            },
            DesktopLocalCommandRequestDto {
                command_name: "search_assets".to_string(),
                payload: DesktopLocalCommandPayloadDto::Search {
                    workspace_id: "workspace-1".to_string(),
                    text: "needle".to_string(),
                    limit: 10,
                },
            },
            DesktopLocalCommandRequestDto {
                command_name: "get_graph_projection".to_string(),
                payload: DesktopLocalCommandPayloadDto::GraphProjection {
                    workspace_id: "workspace-1".to_string(),
                    document_id: "doc-1".to_string(),
                    depth: 2,
                    direction: "both".to_string(),
                    include_unresolved: true,
                    include_assets: false,
                    node_limit: 120,
                    edge_limit: 240,
                },
            },
            DesktopLocalCommandRequestDto {
                command_name: "attach_document_asset".to_string(),
                payload: DesktopLocalCommandPayloadDto::AssetAttachment {
                    workspace_id: "workspace-1".to_string(),
                    document_id: "doc-1".to_string(),
                    asset_id: "asset-1".to_string(),
                    label: "Reference".to_string(),
                    file_name: "/Users/example/private/source.pdf".to_string(),
                    media_type: "application/pdf".to_string(),
                    byte_size: 42,
                },
            },
            DesktopLocalCommandRequestDto {
                command_name: "create_backup".to_string(),
                payload: DesktopLocalCommandPayloadDto::Workspace {
                    workspace_id: "workspace-1".to_string(),
                },
            },
            DesktopLocalCommandRequestDto {
                command_name: "preview_import".to_string(),
                payload: DesktopLocalCommandPayloadDto::ImportPreview {
                    workspace_id: "workspace-1".to_string(),
                    source_label: "/Users/example/private/import".to_string(),
                    file_count: 3,
                },
            },
            DesktopLocalCommandRequestDto {
                command_name: "preview_restore".to_string(),
                payload: DesktopLocalCommandPayloadDto::RestorePackage {
                    workspace_id: "workspace-1".to_string(),
                    package_label: "/Users/example/private/backup.zip".to_string(),
                },
            },
        ];

        let responses = commands
            .into_iter()
            .map(route_local_desktop_command_request)
            .collect::<Vec<_>>();

        assert!(responses.iter().all(|response| response.accepted));
        assert_eq!(responses[1].usecase_name, Some("SearchAssets".to_string()));
        assert_eq!(
            responses[3].usecase_name,
            Some("AttachDocumentAsset".to_string())
        );
        assert_eq!(responses[3].asset_byte_len, Some(42));
        assert!(!format!("{responses:?}").contains("/Users/example/private"));
        assert!(!format!("{responses:?}").contains("source.pdf"));
        assert!(!format!("{responses:?}").contains("backup.zip"));
    }

    #[test]
    fn bundled_desktop_dist_dir_is_outside_src_tauri() {
        let dist_dir = bundled_desktop_dist_dir();

        assert!(dist_dir.ends_with("dist"));
        assert!(!dist_dir.ends_with("src-tauri"));
    }

    #[test]
    fn packaged_runtime_smoke_does_not_require_node() {
        let report = create_desktop_package_smoke_report();

        assert_eq!(report.boundary, "platform");
        assert!(!report.node_runtime_required);
    }
}
