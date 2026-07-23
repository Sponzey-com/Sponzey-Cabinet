use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_projection_catalog::{
    CurrentDocumentProjectionCatalog, CurrentDocumentProjectionCatalogError,
};
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::projection_work::ProjectionWorkRepository;

use crate::projection_freshness::{
    GetCurrentProjectionFreshnessError, GetCurrentProjectionFreshnessInput,
    GetCurrentProjectionFreshnessUsecase, ProjectionFreshnessState,
};
use crate::reindex_projection::{
    ReindexCurrentProjectionError, ReindexCurrentProjectionInput, ReindexCurrentProjectionUsecase,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCurrentProjectionsInput {
    workspace_id: String,
    document_limit: usize,
}

impl ReconcileCurrentProjectionsInput {
    pub fn new(workspace_id: &str, document_limit: usize) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_limit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileCurrentProjectionsOutput {
    document_count: usize,
    ready_document_count: usize,
    enqueued_count: usize,
    reset_count: usize,
    already_active_count: usize,
}

impl ReconcileCurrentProjectionsOutput {
    pub const fn document_count(self) -> usize {
        self.document_count
    }

    pub const fn ready_document_count(self) -> usize {
        self.ready_document_count
    }

    pub const fn enqueued_count(self) -> usize {
        self.enqueued_count
    }

    pub const fn reset_count(self) -> usize {
        self.reset_count
    }

    pub const fn already_active_count(self) -> usize {
        self.already_active_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileCurrentProjectionsError {
    InvalidInput,
    CatalogLimitExceeded,
    CatalogUnavailable,
    CorruptedState,
    CurrentVersionChanged,
    PointerUnavailable,
    RepositoryUnavailable,
    RepositoryConflict,
    InvalidTransition,
}

impl ReconcileCurrentProjectionsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "projection_reconcile.invalid_input",
            Self::CatalogLimitExceeded => "projection_reconcile.catalog_limit_exceeded",
            Self::CatalogUnavailable => "projection_reconcile.catalog_unavailable",
            Self::CorruptedState => "projection_reconcile.corrupted_state",
            Self::CurrentVersionChanged => "projection_reconcile.current_version_changed",
            Self::PointerUnavailable => "projection_reconcile.pointer_unavailable",
            Self::RepositoryUnavailable => "projection_reconcile.repository_unavailable",
            Self::RepositoryConflict => "projection_reconcile.repository_conflict",
            Self::InvalidTransition => "projection_reconcile.invalid_transition",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(
            self,
            Self::CatalogUnavailable
                | Self::CurrentVersionChanged
                | Self::PointerUnavailable
                | Self::RepositoryUnavailable
                | Self::RepositoryConflict
        )
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ReconcileCurrentProjectionsUsecase;

impl ReconcileCurrentProjectionsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ReconcileCurrentProjectionsInput,
        catalog: &impl CurrentDocumentProjectionCatalog,
        pointer: &impl CurrentDocumentVersionPointerPort,
        repository: &mut impl ProjectionWorkRepository,
    ) -> Result<ReconcileCurrentProjectionsOutput, ReconcileCurrentProjectionsError> {
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ReconcileCurrentProjectionsError::InvalidInput)?;
        if input.document_limit == 0 {
            return Err(ReconcileCurrentProjectionsError::InvalidInput);
        }
        let identities = catalog
            .list_current_projection_identities(&workspace, input.document_limit)
            .map_err(map_catalog_error)?;
        let mut output = ReconcileCurrentProjectionsOutput {
            document_count: identities.len(),
            ready_document_count: 0,
            enqueued_count: 0,
            reset_count: 0,
            already_active_count: 0,
        };

        for identity in identities {
            let document_id = identity.document_id().as_str();
            let freshness = GetCurrentProjectionFreshnessUsecase::new()
                .execute(
                    GetCurrentProjectionFreshnessInput::new(&input.workspace_id, document_id),
                    pointer,
                    repository,
                )
                .map_err(map_freshness_error)?;
            if freshness.current_version_id() != identity.version_id() {
                return Err(ReconcileCurrentProjectionsError::CurrentVersionChanged);
            }
            if freshness.aggregate_state() == ProjectionFreshnessState::Ready {
                output.ready_document_count += 1;
                continue;
            }
            let reindexed = ReindexCurrentProjectionUsecase::new()
                .execute(
                    ReindexCurrentProjectionInput::new(&input.workspace_id, document_id),
                    pointer,
                    repository,
                )
                .map_err(map_reindex_error)?;
            output.enqueued_count += reindexed.enqueued_count();
            output.reset_count += reindexed.reset_count();
            output.already_active_count += reindexed.already_active_count();
        }
        Ok(output)
    }
}

const fn map_catalog_error(
    error: CurrentDocumentProjectionCatalogError,
) -> ReconcileCurrentProjectionsError {
    match error {
        CurrentDocumentProjectionCatalogError::InvalidLimit => {
            ReconcileCurrentProjectionsError::InvalidInput
        }
        CurrentDocumentProjectionCatalogError::LimitExceeded => {
            ReconcileCurrentProjectionsError::CatalogLimitExceeded
        }
        CurrentDocumentProjectionCatalogError::StorageUnavailable => {
            ReconcileCurrentProjectionsError::CatalogUnavailable
        }
        CurrentDocumentProjectionCatalogError::CorruptedRecord => {
            ReconcileCurrentProjectionsError::CorruptedState
        }
    }
}

const fn map_freshness_error(
    error: GetCurrentProjectionFreshnessError,
) -> ReconcileCurrentProjectionsError {
    match error {
        GetCurrentProjectionFreshnessError::InvalidInput => {
            ReconcileCurrentProjectionsError::InvalidInput
        }
        GetCurrentProjectionFreshnessError::CurrentVersionNotFound
        | GetCurrentProjectionFreshnessError::CorruptedState => {
            ReconcileCurrentProjectionsError::CorruptedState
        }
        GetCurrentProjectionFreshnessError::PointerUnavailable => {
            ReconcileCurrentProjectionsError::PointerUnavailable
        }
        GetCurrentProjectionFreshnessError::RepositoryUnavailable => {
            ReconcileCurrentProjectionsError::RepositoryUnavailable
        }
    }
}

const fn map_reindex_error(
    error: ReindexCurrentProjectionError,
) -> ReconcileCurrentProjectionsError {
    match error {
        ReindexCurrentProjectionError::InvalidInput => {
            ReconcileCurrentProjectionsError::InvalidInput
        }
        ReindexCurrentProjectionError::CurrentVersionNotFound
        | ReindexCurrentProjectionError::CorruptedState => {
            ReconcileCurrentProjectionsError::CorruptedState
        }
        ReindexCurrentProjectionError::PointerUnavailable => {
            ReconcileCurrentProjectionsError::PointerUnavailable
        }
        ReindexCurrentProjectionError::RepositoryUnavailable => {
            ReconcileCurrentProjectionsError::RepositoryUnavailable
        }
        ReindexCurrentProjectionError::RepositoryConflict => {
            ReconcileCurrentProjectionsError::RepositoryConflict
        }
        ReindexCurrentProjectionError::InvalidTransition => {
            ReconcileCurrentProjectionsError::InvalidTransition
        }
    }
}
