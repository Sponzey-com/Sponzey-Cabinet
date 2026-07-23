use cabinet_ports::workspace_home::{
    WorkspaceHomeBackupStatus, WorkspaceHomeHealthStatus, WorkspaceHomeProjectionPort,
    WorkspaceHomeSummaryKind,
};
use cabinet_usecases::workspace_home::{
    GetWorkspaceHomeError, GetWorkspaceHomeInput, GetWorkspaceHomeOutput, GetWorkspaceHomeUsecase,
    WorkspaceHomeLoadState,
};

use crate::local_desktop_runtime::LocalDesktopUsecaseInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceHomeCommandLoadState {
    Ready,
    Empty,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceHomeCommandDocumentItem {
    pub document_id: String,
    pub title: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceHomeCommandTagItem {
    pub label: String,
    pub document_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceHomeCommandChangeItem {
    pub document_id: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceHomeCommandUnfinishedItem {
    pub document_id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceHomeCommandResult {
    pub workspace_id: String,
    pub state: WorkspaceHomeCommandLoadState,
    pub recent_documents: Vec<WorkspaceHomeCommandDocumentItem>,
    pub favorites: Vec<WorkspaceHomeCommandDocumentItem>,
    pub tags: Vec<WorkspaceHomeCommandTagItem>,
    pub recent_changes: Vec<WorkspaceHomeCommandChangeItem>,
    pub unfinished_items: Vec<WorkspaceHomeCommandUnfinishedItem>,
    pub backup_status: &'static str,
    pub health_status: &'static str,
    pub document_count: u32,
    pub asset_count: u32,
    pub canvas_count: u32,
    pub summary_unavailable: Vec<&'static str>,
    pub product_log_event_name: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceHomeCommandFailure {
    pub error_code: &'static str,
    pub retryable: bool,
    pub product_log_event_name: Option<&'static str>,
}

pub fn execute_workspace_home_command(
    input: LocalDesktopUsecaseInput,
    projection_port: &impl WorkspaceHomeProjectionPort,
) -> Result<WorkspaceHomeCommandResult, WorkspaceHomeCommandFailure> {
    let LocalDesktopUsecaseInput::WorkspaceHome {
        workspace_id,
        recent_documents,
        favorites,
        tags,
        recent_changes,
        unfinished_items,
    } = input
    else {
        return Err(WorkspaceHomeCommandFailure {
            error_code: "WORKSPACE_HOME_COMMAND_UNSUPPORTED",
            retryable: false,
            product_log_event_name: None,
        });
    };

    let output = GetWorkspaceHomeUsecase::new()
        .execute(
            GetWorkspaceHomeInput::new(
                &workspace_id,
                recent_documents,
                favorites,
                tags,
                recent_changes,
                unfinished_items,
            ),
            projection_port,
        )
        .map_err(map_usecase_error)?;

    map_output(output)
}

fn map_output(
    output: GetWorkspaceHomeOutput,
) -> Result<WorkspaceHomeCommandResult, WorkspaceHomeCommandFailure> {
    let state = match output.state() {
        WorkspaceHomeLoadState::Ready => WorkspaceHomeCommandLoadState::Ready,
        WorkspaceHomeLoadState::Empty => WorkspaceHomeCommandLoadState::Empty,
        WorkspaceHomeLoadState::Degraded => WorkspaceHomeCommandLoadState::Degraded,
        WorkspaceHomeLoadState::Pending
        | WorkspaceHomeLoadState::Loading
        | WorkspaceHomeLoadState::Failed => {
            return Err(WorkspaceHomeCommandFailure {
                error_code: "WORKSPACE_HOME_RESULT_MAPPING_FAILED",
                retryable: false,
                product_log_event_name: None,
            });
        }
    };

    let summary = output.summary();
    Ok(WorkspaceHomeCommandResult {
        workspace_id: output.workspace_id().to_string(),
        state,
        recent_documents: output
            .recent_documents()
            .iter()
            .map(|item| WorkspaceHomeCommandDocumentItem {
                document_id: item.document_id().to_string(),
                title: item.title().to_string(),
                path: item.path().to_string(),
            })
            .collect(),
        favorites: output
            .favorites()
            .iter()
            .map(|item| WorkspaceHomeCommandDocumentItem {
                document_id: item.document_id().to_string(),
                title: item.title().to_string(),
                path: item.path().to_string(),
            })
            .collect(),
        tags: output
            .tags()
            .iter()
            .map(|item| WorkspaceHomeCommandTagItem {
                label: item.label().to_string(),
                document_count: item.document_count(),
            })
            .collect(),
        recent_changes: output
            .recent_changes()
            .iter()
            .map(|item| WorkspaceHomeCommandChangeItem {
                document_id: item.document_id().to_string(),
                summary: item.summary().to_string(),
            })
            .collect(),
        unfinished_items: output
            .unfinished_items()
            .iter()
            .map(|item| WorkspaceHomeCommandUnfinishedItem {
                document_id: item.document_id().to_string(),
                label: item.label().to_string(),
            })
            .collect(),
        backup_status: map_backup_status(output.backup_status()),
        health_status: map_health_status(output.health_status()),
        document_count: summary.document_count(),
        asset_count: summary.asset_count(),
        canvas_count: summary.canvas_count(),
        summary_unavailable: [
            (WorkspaceHomeSummaryKind::Documents, "Documents"),
            (WorkspaceHomeSummaryKind::Assets, "Assets"),
            (WorkspaceHomeSummaryKind::Canvases, "Canvases"),
        ]
        .into_iter()
        .filter_map(|(kind, label)| (!summary.is_available(kind)).then_some(label))
        .collect(),
        product_log_event_name: output.product_log_event_name(),
    })
}

fn map_usecase_error(error: GetWorkspaceHomeError) -> WorkspaceHomeCommandFailure {
    match error {
        GetWorkspaceHomeError::InvalidInput => WorkspaceHomeCommandFailure {
            error_code: "WORKSPACE_HOME_INVALID_INPUT",
            retryable: false,
            product_log_event_name: error.product_log_event_name(),
        },
        GetWorkspaceHomeError::ProjectionUnavailable => WorkspaceHomeCommandFailure {
            error_code: "WORKSPACE_HOME_PROJECTION_UNAVAILABLE",
            retryable: true,
            product_log_event_name: error.product_log_event_name(),
        },
    }
}

const fn map_backup_status(status: WorkspaceHomeBackupStatus) -> &'static str {
    match status {
        WorkspaceHomeBackupStatus::NeverCreated => "NeverCreated",
        WorkspaceHomeBackupStatus::Fresh => "Fresh",
        WorkspaceHomeBackupStatus::Stale => "Stale",
        WorkspaceHomeBackupStatus::Failed => "Failed",
    }
}

const fn map_health_status(status: WorkspaceHomeHealthStatus) -> &'static str {
    match status {
        WorkspaceHomeHealthStatus::Healthy => "Healthy",
        WorkspaceHomeHealthStatus::Degraded => "Degraded",
        WorkspaceHomeHealthStatus::ReadOnlyRecovery => "ReadOnlyRecovery",
    }
}
