use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_repository::{DocumentRepository, DocumentRepositoryError};
use cabinet_ports::workspace_home::{
    WorkspaceHomeDocumentMutation, WorkspaceHomeDocumentMutationPort,
    WorkspaceHomeDocumentProjection,
};

use crate::document::DocumentChangeEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateWorkspaceHomeOutcome {
    AppliedUpsert,
    AppliedRemove,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateWorkspaceHomeError {
    InvalidPolicy,
    InvalidInput,
    CurrentDocumentMissing,
    RepositoryUnavailable,
    ProjectionUnavailable,
}

impl UpdateWorkspaceHomeError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidPolicy => "workspace_home_update.invalid_policy",
            Self::InvalidInput => "workspace_home_update.invalid_input",
            Self::CurrentDocumentMissing => "workspace_home_update.current_document_missing",
            Self::RepositoryUnavailable => "workspace_home_update.repository_unavailable",
            Self::ProjectionUnavailable => "workspace_home_update.projection_unavailable",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(
            self,
            Self::RepositoryUnavailable | Self::ProjectionUnavailable
        )
    }

    pub const fn product_log_event_name(self) -> Option<&'static str> {
        match self {
            Self::ProjectionUnavailable => Some("workspace.home.projection_update_failed"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateWorkspaceHomeProjectionUsecase {
    capacity: u16,
}

impl UpdateWorkspaceHomeProjectionUsecase {
    pub fn new(capacity: u16) -> Result<Self, UpdateWorkspaceHomeError> {
        if capacity == 0 || capacity > 100 {
            return Err(UpdateWorkspaceHomeError::InvalidPolicy);
        }
        Ok(Self { capacity })
    }

    pub fn execute(
        &self,
        event: DocumentChangeEvent,
        document_repository: &impl DocumentRepository,
        mutation_port: &mut impl WorkspaceHomeDocumentMutationPort,
    ) -> Result<UpdateWorkspaceHomeOutcome, UpdateWorkspaceHomeError> {
        match event {
            DocumentChangeEvent::DocumentCreated {
                workspace_id,
                document_id,
                ..
            } => self.upsert(
                workspace_id,
                document_id,
                "Created document",
                document_repository,
                mutation_port,
            ),
            DocumentChangeEvent::DocumentUpdated {
                workspace_id,
                document_id,
                ..
            } => self.upsert(
                workspace_id,
                document_id,
                "Updated document",
                document_repository,
                mutation_port,
            ),
            DocumentChangeEvent::DocumentRestored {
                workspace_id,
                document_id,
                ..
            } => self.upsert(
                workspace_id,
                document_id,
                "Restored document",
                document_repository,
                mutation_port,
            ),
            DocumentChangeEvent::DocumentRenamed {
                workspace_id,
                document_id,
                ..
            } => self.upsert(
                workspace_id,
                document_id,
                "Renamed document",
                document_repository,
                mutation_port,
            ),
            DocumentChangeEvent::DocumentDeleted {
                workspace_id,
                document_id,
                ..
            } => self.remove(workspace_id, document_id, mutation_port),
            DocumentChangeEvent::DocumentAssetAttached {
                workspace_id,
                document_id,
                ..
            } => {
                validate_identity(workspace_id, document_id)?;
                Ok(UpdateWorkspaceHomeOutcome::Ignored)
            }
        }
    }

    fn upsert(
        &self,
        workspace_id: String,
        document_id: String,
        change_summary: &str,
        document_repository: &impl DocumentRepository,
        mutation_port: &mut impl WorkspaceHomeDocumentMutationPort,
    ) -> Result<UpdateWorkspaceHomeOutcome, UpdateWorkspaceHomeError> {
        let (workspace_id, document_id) = validate_identity(workspace_id, document_id)?;
        let current = document_repository
            .get_current_by_id(&workspace_id, &document_id)
            .map_err(map_repository_error)?
            .ok_or(UpdateWorkspaceHomeError::CurrentDocumentMissing)?;
        let document = WorkspaceHomeDocumentProjection::new(
            current.document_id().clone(),
            current.metadata().title().clone(),
            current.metadata().path().clone(),
        );
        mutation_port
            .apply_document_mutation(
                &workspace_id,
                WorkspaceHomeDocumentMutation::UpsertRecent {
                    document,
                    change_summary: change_summary.to_string(),
                },
                self.capacity,
            )
            .map_err(|_| UpdateWorkspaceHomeError::ProjectionUnavailable)?;
        Ok(UpdateWorkspaceHomeOutcome::AppliedUpsert)
    }

    fn remove(
        &self,
        workspace_id: String,
        document_id: String,
        mutation_port: &mut impl WorkspaceHomeDocumentMutationPort,
    ) -> Result<UpdateWorkspaceHomeOutcome, UpdateWorkspaceHomeError> {
        let (workspace_id, document_id) = validate_identity(workspace_id, document_id)?;
        mutation_port
            .apply_document_mutation(
                &workspace_id,
                WorkspaceHomeDocumentMutation::RemoveDocument { document_id },
                self.capacity,
            )
            .map_err(|_| UpdateWorkspaceHomeError::ProjectionUnavailable)?;
        Ok(UpdateWorkspaceHomeOutcome::AppliedRemove)
    }
}

fn validate_identity(
    workspace_id: String,
    document_id: String,
) -> Result<(WorkspaceId, DocumentId), UpdateWorkspaceHomeError> {
    Ok((
        WorkspaceId::new(&workspace_id).map_err(|_| UpdateWorkspaceHomeError::InvalidInput)?,
        DocumentId::new(&document_id).map_err(|_| UpdateWorkspaceHomeError::InvalidInput)?,
    ))
}

fn map_repository_error(_error: DocumentRepositoryError) -> UpdateWorkspaceHomeError {
    UpdateWorkspaceHomeError::RepositoryUnavailable
}
