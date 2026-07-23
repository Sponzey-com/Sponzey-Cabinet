use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::workspace_home::{
    WorkspaceHomeBackupStatus, WorkspaceHomeChangeProjection, WorkspaceHomeDocumentProjection,
    WorkspaceHomeHealthStatus, WorkspaceHomeProjection, WorkspaceHomeProjectionLimits,
    WorkspaceHomeProjectionPort, WorkspaceHomeSummaryProjection, WorkspaceHomeTagProjection,
    WorkspaceHomeUnfinishedProjection,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceHomeLoadState {
    Pending,
    Loading,
    Ready,
    Empty,
    Degraded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceHomeLoadEvent {
    LoadRequested,
    ProjectionLoaded {
        total_item_count: usize,
        health: WorkspaceHomeHealthStatus,
    },
    ProjectionFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceHomeLoadTransition {
    pub state: WorkspaceHomeLoadState,
    pub error_code: Option<&'static str>,
}

pub fn transition_workspace_home_load(
    state: WorkspaceHomeLoadState,
    event: WorkspaceHomeLoadEvent,
) -> WorkspaceHomeLoadTransition {
    match (state, event) {
        (WorkspaceHomeLoadState::Pending, WorkspaceHomeLoadEvent::LoadRequested) => {
            WorkspaceHomeLoadTransition {
                state: WorkspaceHomeLoadState::Loading,
                error_code: None,
            }
        }
        (
            WorkspaceHomeLoadState::Loading,
            WorkspaceHomeLoadEvent::ProjectionLoaded { health, .. },
        ) if health != WorkspaceHomeHealthStatus::Healthy => WorkspaceHomeLoadTransition {
            state: WorkspaceHomeLoadState::Degraded,
            error_code: None,
        },
        (
            WorkspaceHomeLoadState::Loading,
            WorkspaceHomeLoadEvent::ProjectionLoaded {
                total_item_count: 0,
                health: WorkspaceHomeHealthStatus::Healthy,
            },
        ) => WorkspaceHomeLoadTransition {
            state: WorkspaceHomeLoadState::Empty,
            error_code: None,
        },
        (
            WorkspaceHomeLoadState::Loading,
            WorkspaceHomeLoadEvent::ProjectionLoaded {
                health: WorkspaceHomeHealthStatus::Healthy,
                ..
            },
        ) => WorkspaceHomeLoadTransition {
            state: WorkspaceHomeLoadState::Ready,
            error_code: None,
        },
        (WorkspaceHomeLoadState::Loading, WorkspaceHomeLoadEvent::ProjectionFailed) => {
            WorkspaceHomeLoadTransition {
                state: WorkspaceHomeLoadState::Failed,
                error_code: Some("workspace_home.projection_unavailable"),
            }
        }
        _ => WorkspaceHomeLoadTransition {
            state: WorkspaceHomeLoadState::Failed,
            error_code: Some("workspace_home.invalid_transition"),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetWorkspaceHomeInput {
    workspace_id: String,
    recent_documents: u16,
    favorites: u16,
    tags: u16,
    recent_changes: u16,
    unfinished_items: u16,
}

impl GetWorkspaceHomeInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: &str,
        recent_documents: u16,
        favorites: u16,
        tags: u16,
        recent_changes: u16,
        unfinished_items: u16,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            recent_documents,
            favorites,
            tags,
            recent_changes,
            unfinished_items,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetWorkspaceHomeOutput {
    workspace_id: WorkspaceId,
    projection: WorkspaceHomeProjection,
    state: WorkspaceHomeLoadState,
}

impl GetWorkspaceHomeOutput {
    pub fn workspace_id(&self) -> &str {
        self.workspace_id.as_str()
    }

    pub const fn state(&self) -> WorkspaceHomeLoadState {
        self.state
    }

    pub fn recent_documents(&self) -> &[WorkspaceHomeDocumentProjection] {
        self.projection.recent_documents()
    }

    pub fn favorites(&self) -> &[WorkspaceHomeDocumentProjection] {
        self.projection.favorites()
    }

    pub fn tags(&self) -> &[WorkspaceHomeTagProjection] {
        self.projection.tags()
    }

    pub fn recent_changes(&self) -> &[WorkspaceHomeChangeProjection] {
        self.projection.recent_changes()
    }

    pub fn unfinished_items(&self) -> &[WorkspaceHomeUnfinishedProjection] {
        self.projection.unfinished_items()
    }

    pub const fn backup_status(&self) -> WorkspaceHomeBackupStatus {
        self.projection.backup_status()
    }

    pub const fn health_status(&self) -> WorkspaceHomeHealthStatus {
        self.projection.health_status()
    }

    pub const fn summary(&self) -> WorkspaceHomeSummaryProjection {
        self.projection.summary()
    }

    pub fn total_item_count(&self) -> usize {
        self.projection.total_item_count()
    }

    pub const fn product_log_event_name(&self) -> Option<&'static str> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetWorkspaceHomeError {
    InvalidInput,
    ProjectionUnavailable,
}

impl GetWorkspaceHomeError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "workspace_home.invalid_input",
            Self::ProjectionUnavailable => "workspace_home.projection_unavailable",
        }
    }

    pub const fn product_log_event_name(self) -> Option<&'static str> {
        match self {
            Self::InvalidInput => None,
            Self::ProjectionUnavailable => Some("workspace.home.failed"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GetWorkspaceHomeUsecase;

impl GetWorkspaceHomeUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetWorkspaceHomeInput,
        projection_port: &impl WorkspaceHomeProjectionPort,
    ) -> Result<GetWorkspaceHomeOutput, GetWorkspaceHomeError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| GetWorkspaceHomeError::InvalidInput)?;
        let limits = WorkspaceHomeProjectionLimits::new(
            input.recent_documents,
            input.favorites,
            input.tags,
            input.recent_changes,
            input.unfinished_items,
        )
        .map_err(|_| GetWorkspaceHomeError::InvalidInput)?;

        let loading = transition_workspace_home_load(
            WorkspaceHomeLoadState::Pending,
            WorkspaceHomeLoadEvent::LoadRequested,
        );
        debug_assert_eq!(loading.state, WorkspaceHomeLoadState::Loading);

        let projection = projection_port
            .load_workspace_home(&workspace_id, limits)
            .map_err(|_| GetWorkspaceHomeError::ProjectionUnavailable)?;
        let loaded = transition_workspace_home_load(
            loading.state,
            WorkspaceHomeLoadEvent::ProjectionLoaded {
                total_item_count: projection.total_item_count(),
                health: projection.health_status(),
            },
        );

        Ok(GetWorkspaceHomeOutput {
            workspace_id,
            projection,
            state: loaded.state,
        })
    }
}
