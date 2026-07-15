#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBootstrapState {
    Pending,
    ReadingConfig,
    ResolvingAppData,
    InitializingStores,
    OpeningDefaultWorkspace,
    Ready,
    Failed {
        error_code: NativeBootstrapErrorCode,
        retryable: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBootstrapEvent {
    Start,
    ConfigRead,
    AppDataResolved,
    StoresInitialized,
    DefaultWorkspaceOpened,
    Fail(NativeBootstrapErrorCode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBootstrapErrorCode {
    ConfigInvalid,
    AppDataResolutionFailed,
    StoreInitializationFailed,
    DefaultWorkspaceOpenFailed,
    InvalidTransition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeBootstrapTransition {
    pub previous_state: NativeBootstrapState,
    pub event: NativeBootstrapEvent,
    pub next_state: NativeBootstrapState,
    pub retryable: bool,
    pub error_code: Option<NativeBootstrapErrorCode>,
}

pub fn transition_native_bootstrap(
    state: NativeBootstrapState,
    event: NativeBootstrapEvent,
) -> NativeBootstrapTransition {
    let next_state = match (state, event) {
        (NativeBootstrapState::Pending, NativeBootstrapEvent::Start) => {
            NativeBootstrapState::ReadingConfig
        }
        (NativeBootstrapState::ReadingConfig, NativeBootstrapEvent::ConfigRead) => {
            NativeBootstrapState::ResolvingAppData
        }
        (NativeBootstrapState::ResolvingAppData, NativeBootstrapEvent::AppDataResolved) => {
            NativeBootstrapState::InitializingStores
        }
        (NativeBootstrapState::InitializingStores, NativeBootstrapEvent::StoresInitialized) => {
            NativeBootstrapState::OpeningDefaultWorkspace
        }
        (
            NativeBootstrapState::OpeningDefaultWorkspace,
            NativeBootstrapEvent::DefaultWorkspaceOpened,
        ) => NativeBootstrapState::Ready,
        (
            NativeBootstrapState::Pending
            | NativeBootstrapState::ReadingConfig
            | NativeBootstrapState::ResolvingAppData
            | NativeBootstrapState::InitializingStores
            | NativeBootstrapState::OpeningDefaultWorkspace,
            NativeBootstrapEvent::Fail(error_code),
        ) => NativeBootstrapState::Failed {
            error_code,
            retryable: true,
        },
        _ => NativeBootstrapState::Failed {
            error_code: NativeBootstrapErrorCode::InvalidTransition,
            retryable: false,
        },
    };

    let (retryable, error_code) = match next_state {
        NativeBootstrapState::Failed {
            error_code,
            retryable,
        } => (retryable, Some(error_code)),
        _ => (false, None),
    };

    NativeBootstrapTransition {
        previous_state: state,
        event,
        next_state,
        retryable,
        error_code,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalDesktopCommandState {
    Idle,
    ValidatingInput,
    ExecutingUsecase,
    MappingResult,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalDesktopCommandEvent {
    Start,
    InputValidated,
    UsecaseExecuted,
    ResultMapped,
    Fail(LocalDesktopCommandErrorCode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalDesktopCommandErrorCode {
    UnsupportedCommand,
    InvalidInput,
    UsecaseFailed,
    ResultMappingFailed,
    InvalidTransition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalDesktopCommandTransition {
    pub previous_state: LocalDesktopCommandState,
    pub event: LocalDesktopCommandEvent,
    pub next_state: LocalDesktopCommandState,
    pub retryable: bool,
    pub error_code: Option<LocalDesktopCommandErrorCode>,
}

pub fn transition_local_desktop_command(
    state: LocalDesktopCommandState,
    event: LocalDesktopCommandEvent,
) -> LocalDesktopCommandTransition {
    let next_state = match (state, event) {
        (LocalDesktopCommandState::Idle, LocalDesktopCommandEvent::Start) => {
            LocalDesktopCommandState::ValidatingInput
        }
        (LocalDesktopCommandState::ValidatingInput, LocalDesktopCommandEvent::InputValidated) => {
            LocalDesktopCommandState::ExecutingUsecase
        }
        (LocalDesktopCommandState::ExecutingUsecase, LocalDesktopCommandEvent::UsecaseExecuted) => {
            LocalDesktopCommandState::MappingResult
        }
        (LocalDesktopCommandState::MappingResult, LocalDesktopCommandEvent::ResultMapped) => {
            LocalDesktopCommandState::Completed
        }
        (
            LocalDesktopCommandState::ValidatingInput
            | LocalDesktopCommandState::ExecutingUsecase
            | LocalDesktopCommandState::MappingResult,
            LocalDesktopCommandEvent::Fail(_),
        ) => LocalDesktopCommandState::Failed,
        _ => LocalDesktopCommandState::Failed,
    };

    let (retryable, error_code) = match (next_state, event) {
        (LocalDesktopCommandState::Failed, LocalDesktopCommandEvent::Fail(error_code)) => {
            (true, Some(error_code))
        }
        (LocalDesktopCommandState::Failed, _) => {
            (false, Some(LocalDesktopCommandErrorCode::InvalidTransition))
        }
        _ => (false, None),
    };

    LocalDesktopCommandTransition {
        previous_state: state,
        event,
        next_state,
        retryable,
        error_code,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDesktopRuntimeCommandRequest {
    pub command_name: String,
    pub payload: LocalDesktopCommandPayload,
}

impl LocalDesktopRuntimeCommandRequest {
    pub fn new(command_name: &str, payload: LocalDesktopCommandPayload) -> Self {
        Self {
            command_name: command_name.to_string(),
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalDesktopCommandPayload {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalDesktopUsecaseInput {
    BootstrapWorkspace,
    WorkspaceHome {
        workspace_id: String,
        recent_documents: u16,
        favorites: u16,
        tags: u16,
        recent_changes: u16,
        unfinished_items: u16,
    },
    GetCurrentDocument {
        workspace_id: String,
        document_id: String,
    },
    UpdateCurrentDocument {
        workspace_id: String,
        document_id: String,
        title: String,
        path: String,
        body: String,
        expected_version_id: String,
    },
    GetDocumentHistory {
        workspace_id: String,
        document_id: String,
        limit: u16,
    },
    GetDocumentVersion {
        workspace_id: String,
        document_id: String,
        version_id: String,
    },
    PreviewDocumentRestore {
        workspace_id: String,
        document_id: String,
        version_id: String,
    },
    RestoreDocumentVersion {
        workspace_id: String,
        document_id: String,
        version_id: String,
    },
    SearchDocuments {
        workspace_id: String,
        text: String,
        limit: u16,
    },
    GetLinkOverview {
        workspace_id: String,
        document_id: String,
    },
    GetGraphProjection {
        workspace_id: String,
        document_id: String,
        depth: u8,
        direction: String,
        include_unresolved: bool,
        include_assets: bool,
        node_limit: u16,
        edge_limit: u16,
    },
    ListDocumentAssets {
        workspace_id: String,
        document_id: String,
    },
    AttachDocumentAsset {
        workspace_id: String,
        document_id: String,
        asset_id: String,
        label: String,
        file_name: String,
        media_type: String,
        byte_size: u64,
    },
    CreateBackup {
        workspace_id: String,
    },
    PreviewImport {
        workspace_id: String,
        source_label: String,
        file_count: u16,
    },
    PreviewRestore {
        workspace_id: String,
        package_label: String,
    },
    ApplyRestore {
        workspace_id: String,
        package_label: String,
    },
}

impl LocalDesktopUsecaseInput {
    pub const fn phase009_name(&self) -> &'static str {
        match self {
            Self::BootstrapWorkspace => "BootstrapWorkspace",
            Self::WorkspaceHome { .. } => "WorkspaceHome",
            Self::GetCurrentDocument { .. } => "GetCurrentDocument",
            Self::UpdateCurrentDocument { .. } => "UpdateCurrentDocument",
            Self::GetDocumentHistory { .. } => "GetDocumentHistory",
            Self::GetDocumentVersion { .. } => "GetDocumentVersion",
            Self::PreviewDocumentRestore { .. } => "PreviewDocumentRestore",
            Self::RestoreDocumentVersion { .. } => "RestoreDocumentVersion",
            Self::SearchDocuments { .. } => "SearchDocuments",
            Self::GetLinkOverview { .. } => "GetLinkOverview",
            Self::GetGraphProjection { .. } => "GetGraphProjection",
            Self::ListDocumentAssets { .. } => "ListDocumentAssets",
            Self::AttachDocumentAsset { .. } => "AttachDocumentAsset",
            Self::CreateBackup { .. } => "CreateBackup",
            Self::PreviewImport { .. } => "PreviewImport",
            Self::PreviewRestore { .. } => "PreviewRestore",
            Self::ApplyRestore { .. } => "ApplyRestore",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDesktopCommandMappingError {
    pub error_code: LocalDesktopCommandErrorCode,
    pub finding_id: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDesktopCommandProductLogSummary {
    pub command_name: String,
    pub workspace_id_present: bool,
    pub document_id_present: bool,
    pub body_byte_len: Option<usize>,
    pub asset_byte_len: Option<u64>,
    pub result_limit: Option<u16>,
}

pub fn map_core_local_desktop_command_request(
    request: LocalDesktopRuntimeCommandRequest,
) -> Result<LocalDesktopUsecaseInput, LocalDesktopCommandMappingError> {
    match (request.command_name.as_str(), request.payload) {
        ("local_workspace_bootstrap", LocalDesktopCommandPayload::Empty) => {
            Ok(LocalDesktopUsecaseInput::BootstrapWorkspace)
        }
        (
            "local_workspace_home",
            LocalDesktopCommandPayload::WorkspaceHome {
                workspace_id,
                recent_documents,
                favorites,
                tags,
                recent_changes,
                unfinished_items,
            },
        ) => {
            let workspace_id = validate_workspace_id(workspace_id)?;
            validate_workspace_home_limits([
                recent_documents,
                favorites,
                tags,
                recent_changes,
                unfinished_items,
            ])?;
            Ok(LocalDesktopUsecaseInput::WorkspaceHome {
                workspace_id,
                recent_documents,
                favorites,
                tags,
                recent_changes,
                unfinished_items,
            })
        }
        (
            "get_current_document",
            LocalDesktopCommandPayload::DocumentIdentity {
                workspace_id,
                document_id,
            },
        ) => validate_document_identity(workspace_id, document_id).map(
            |(workspace_id, document_id)| LocalDesktopUsecaseInput::GetCurrentDocument {
                workspace_id,
                document_id,
            },
        ),
        (
            "update_current_document",
            LocalDesktopCommandPayload::DocumentUpdate {
                workspace_id,
                document_id,
                title,
                path,
                body,
                expected_version_id,
            },
        ) => {
            let (workspace_id, document_id) =
                validate_document_identity(workspace_id, document_id)?;
            if expected_version_id.trim().is_empty() {
                return Err(mapping_error(
                    LocalDesktopCommandErrorCode::InvalidInput,
                    "expected_version_id",
                ));
            }
            Ok(LocalDesktopUsecaseInput::UpdateCurrentDocument {
                workspace_id,
                document_id,
                title,
                path,
                body,
                expected_version_id,
            })
        }
        (
            "get_document_history",
            LocalDesktopCommandPayload::DocumentHistory {
                workspace_id,
                document_id,
                limit,
            },
        ) => {
            let (workspace_id, document_id) =
                validate_document_identity(workspace_id, document_id)?;
            if limit == 0 {
                return Err(mapping_error(
                    LocalDesktopCommandErrorCode::InvalidInput,
                    "limit",
                ));
            }
            Ok(LocalDesktopUsecaseInput::GetDocumentHistory {
                workspace_id,
                document_id,
                limit,
            })
        }
        (
            "get_document_version",
            LocalDesktopCommandPayload::DocumentVersion {
                workspace_id,
                document_id,
                version_id,
            },
        ) => {
            let (workspace_id, document_id) =
                validate_document_identity(workspace_id, document_id)?;
            if version_id.trim().is_empty() {
                return Err(mapping_error(
                    LocalDesktopCommandErrorCode::InvalidInput,
                    "version_id",
                ));
            }
            Ok(LocalDesktopUsecaseInput::GetDocumentVersion {
                workspace_id,
                document_id,
                version_id,
            })
        }
        (
            "preview_document_restore",
            LocalDesktopCommandPayload::DocumentVersion {
                workspace_id,
                document_id,
                version_id,
            },
        ) => {
            let (workspace_id, document_id) =
                validate_document_identity(workspace_id, document_id)?;
            let version_id = validate_non_empty(version_id, "version_id")?;
            Ok(LocalDesktopUsecaseInput::PreviewDocumentRestore {
                workspace_id,
                document_id,
                version_id,
            })
        }
        (
            "restore_document_version",
            LocalDesktopCommandPayload::DocumentVersion {
                workspace_id,
                document_id,
                version_id,
            },
        ) => {
            let (workspace_id, document_id) =
                validate_document_identity(workspace_id, document_id)?;
            let version_id = validate_non_empty(version_id, "version_id")?;
            Ok(LocalDesktopUsecaseInput::RestoreDocumentVersion {
                workspace_id,
                document_id,
                version_id,
            })
        }
        (
            "search_documents",
            LocalDesktopCommandPayload::Search {
                workspace_id,
                text,
                limit,
            },
        ) => {
            let workspace_id = validate_workspace_id(workspace_id)?;
            let text = validate_non_empty(text, "text")?;
            if limit == 0 {
                return Err(mapping_error(
                    LocalDesktopCommandErrorCode::InvalidInput,
                    "limit",
                ));
            }
            Ok(LocalDesktopUsecaseInput::SearchDocuments {
                workspace_id,
                text,
                limit,
            })
        }
        (
            "get_link_overview",
            LocalDesktopCommandPayload::DocumentIdentity {
                workspace_id,
                document_id,
            },
        ) => validate_document_identity(workspace_id, document_id).map(
            |(workspace_id, document_id)| LocalDesktopUsecaseInput::GetLinkOverview {
                workspace_id,
                document_id,
            },
        ),
        (
            "get_graph_projection",
            LocalDesktopCommandPayload::GraphProjection {
                workspace_id,
                document_id,
                depth,
                direction,
                include_unresolved,
                include_assets,
                node_limit,
                edge_limit,
            },
        ) => {
            let (workspace_id, document_id) =
                validate_document_identity(workspace_id, document_id)?;
            if !matches!(depth, 1 | 2)
                || !matches!(direction.as_str(), "incoming" | "outgoing" | "both")
                || node_limit == 0
                || node_limit > 500
                || edge_limit == 0
                || edge_limit > 1_000
            {
                return Err(mapping_error(
                    LocalDesktopCommandErrorCode::InvalidInput,
                    "graph_query",
                ));
            }
            Ok(LocalDesktopUsecaseInput::GetGraphProjection {
                workspace_id,
                document_id,
                depth,
                direction,
                include_unresolved,
                include_assets,
                node_limit,
                edge_limit,
            })
        }
        (
            "list_document_assets",
            LocalDesktopCommandPayload::DocumentIdentity {
                workspace_id,
                document_id,
            },
        ) => validate_document_identity(workspace_id, document_id).map(
            |(workspace_id, document_id)| LocalDesktopUsecaseInput::ListDocumentAssets {
                workspace_id,
                document_id,
            },
        ),
        (
            "attach_document_asset",
            LocalDesktopCommandPayload::AssetAttachment {
                workspace_id,
                document_id,
                asset_id,
                label,
                file_name,
                media_type,
                byte_size,
            },
        ) => {
            let (workspace_id, document_id) =
                validate_document_identity(workspace_id, document_id)?;
            let asset_id = validate_non_empty(asset_id, "asset_id")?;
            let label = validate_non_empty(label, "label")?;
            let file_name = validate_non_empty(file_name, "file_name")?;
            let media_type = validate_non_empty(media_type, "media_type")?;
            if byte_size == 0 {
                return Err(mapping_error(
                    LocalDesktopCommandErrorCode::InvalidInput,
                    "byte_size",
                ));
            }
            Ok(LocalDesktopUsecaseInput::AttachDocumentAsset {
                workspace_id,
                document_id,
                asset_id,
                label,
                file_name,
                media_type,
                byte_size,
            })
        }
        ("create_backup", LocalDesktopCommandPayload::Workspace { workspace_id }) => {
            validate_workspace_id(workspace_id)
                .map(|workspace_id| LocalDesktopUsecaseInput::CreateBackup { workspace_id })
        }
        (
            "preview_import",
            LocalDesktopCommandPayload::ImportPreview {
                workspace_id,
                source_label,
                file_count,
            },
        ) => {
            let workspace_id = validate_workspace_id(workspace_id)?;
            let source_label = validate_non_empty(source_label, "source_label")?;
            if file_count == 0 {
                return Err(mapping_error(
                    LocalDesktopCommandErrorCode::InvalidInput,
                    "file_count",
                ));
            }
            Ok(LocalDesktopUsecaseInput::PreviewImport {
                workspace_id,
                source_label,
                file_count,
            })
        }
        (
            "preview_restore",
            LocalDesktopCommandPayload::RestorePackage {
                workspace_id,
                package_label,
            },
        ) => {
            let workspace_id = validate_workspace_id(workspace_id)?;
            let package_label = validate_non_empty(package_label, "package_label")?;
            Ok(LocalDesktopUsecaseInput::PreviewRestore {
                workspace_id,
                package_label,
            })
        }
        (
            "apply_restore",
            LocalDesktopCommandPayload::RestorePackage {
                workspace_id,
                package_label,
            },
        ) => {
            let workspace_id = validate_workspace_id(workspace_id)?;
            let package_label = validate_non_empty(package_label, "package_label")?;
            Ok(LocalDesktopUsecaseInput::ApplyRestore {
                workspace_id,
                package_label,
            })
        }
        (
            "local_workspace_bootstrap"
            | "local_workspace_home"
            | "get_current_document"
            | "update_current_document"
            | "get_document_history"
            | "get_document_version"
            | "preview_document_restore"
            | "restore_document_version"
            | "search_documents"
            | "get_link_overview"
            | "get_graph_projection"
            | "list_document_assets"
            | "attach_document_asset"
            | "create_backup"
            | "preview_import"
            | "preview_restore"
            | "apply_restore",
            _,
        ) => Err(mapping_error(
            LocalDesktopCommandErrorCode::InvalidInput,
            "payload",
        )),
        _ => Err(mapping_error(
            LocalDesktopCommandErrorCode::UnsupportedCommand,
            "command_name",
        )),
    }
}

pub fn summarize_local_desktop_command_for_product_log(
    request: &LocalDesktopRuntimeCommandRequest,
) -> LocalDesktopCommandProductLogSummary {
    let (workspace_id_present, document_id_present, body_byte_len, asset_byte_len, result_limit) =
        match &request.payload {
            LocalDesktopCommandPayload::Empty => (false, false, None, None, None),
            LocalDesktopCommandPayload::WorkspaceHome { workspace_id, .. }
            | LocalDesktopCommandPayload::Workspace { workspace_id }
            | LocalDesktopCommandPayload::ImportPreview { workspace_id, .. }
            | LocalDesktopCommandPayload::RestorePackage { workspace_id, .. }
            | LocalDesktopCommandPayload::Search { workspace_id, .. } => (
                !workspace_id.trim().is_empty(),
                false,
                None,
                None,
                payload_limit(&request.payload),
            ),
            LocalDesktopCommandPayload::DocumentIdentity {
                workspace_id,
                document_id,
            }
            | LocalDesktopCommandPayload::DocumentHistory {
                workspace_id,
                document_id,
                ..
            }
            | LocalDesktopCommandPayload::DocumentVersion {
                workspace_id,
                document_id,
                ..
            }
            | LocalDesktopCommandPayload::GraphProjection {
                workspace_id,
                document_id,
                ..
            } => (
                !workspace_id.trim().is_empty(),
                !document_id.trim().is_empty(),
                None,
                None,
                payload_limit(&request.payload),
            ),
            LocalDesktopCommandPayload::DocumentUpdate {
                workspace_id,
                document_id,
                body,
                ..
            } => (
                !workspace_id.trim().is_empty(),
                !document_id.trim().is_empty(),
                Some(body.len()),
                None,
                None,
            ),
            LocalDesktopCommandPayload::AssetAttachment {
                workspace_id,
                document_id,
                byte_size,
                ..
            } => (
                !workspace_id.trim().is_empty(),
                !document_id.trim().is_empty(),
                None,
                Some(*byte_size),
                None,
            ),
        };

    LocalDesktopCommandProductLogSummary {
        command_name: request.command_name.clone(),
        workspace_id_present,
        document_id_present,
        body_byte_len,
        asset_byte_len,
        result_limit,
    }
}

fn payload_limit(payload: &LocalDesktopCommandPayload) -> Option<u16> {
    match payload {
        LocalDesktopCommandPayload::WorkspaceHome {
            recent_documents,
            favorites,
            tags,
            recent_changes,
            unfinished_items,
            ..
        } => Some(recent_documents + favorites + tags + recent_changes + unfinished_items),
        LocalDesktopCommandPayload::DocumentHistory { limit, .. }
        | LocalDesktopCommandPayload::Search { limit, .. } => Some(*limit),
        LocalDesktopCommandPayload::ImportPreview { file_count, .. } => Some(*file_count),
        _ => None,
    }
}

fn validate_workspace_home_limits(limits: [u16; 5]) -> Result<(), LocalDesktopCommandMappingError> {
    if limits.iter().any(|limit| *limit == 0 || *limit > 100) {
        return Err(mapping_error(
            LocalDesktopCommandErrorCode::InvalidInput,
            "workspace_home_limit",
        ));
    }
    Ok(())
}

fn validate_workspace_id(workspace_id: String) -> Result<String, LocalDesktopCommandMappingError> {
    validate_non_empty(workspace_id, "workspace_id")
}

fn validate_non_empty(
    value: String,
    finding_id: &'static str,
) -> Result<String, LocalDesktopCommandMappingError> {
    if value.trim().is_empty() {
        return Err(mapping_error(
            LocalDesktopCommandErrorCode::InvalidInput,
            finding_id,
        ));
    }
    Ok(value)
}

fn validate_document_identity(
    workspace_id: String,
    document_id: String,
) -> Result<(String, String), LocalDesktopCommandMappingError> {
    if workspace_id.trim().is_empty() {
        return Err(mapping_error(
            LocalDesktopCommandErrorCode::InvalidInput,
            "workspace_id",
        ));
    }
    if document_id.trim().is_empty() {
        return Err(mapping_error(
            LocalDesktopCommandErrorCode::InvalidInput,
            "document_id",
        ));
    }
    Ok((workspace_id, document_id))
}

fn mapping_error(
    error_code: LocalDesktopCommandErrorCode,
    finding_id: &'static str,
) -> LocalDesktopCommandMappingError {
    LocalDesktopCommandMappingError {
        error_code,
        finding_id,
    }
}
