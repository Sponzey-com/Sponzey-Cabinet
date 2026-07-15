use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_link_catalog::{
    DocumentLinkCatalog, DocumentLinkCatalogError, DocumentLinkCatalogRecord,
};

use crate::document::DocumentChangeEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLinkCatalogChangeOutcome {
    Upserted,
    Removed,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyDocumentLinkCatalogChangeError {
    InvalidEvent,
    RepositoryUnavailable,
    CorruptedCatalog,
}

impl ApplyDocumentLinkCatalogChangeError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidEvent => "document_catalog.invalid_event",
            Self::RepositoryUnavailable => "document_catalog.storage_unavailable",
            Self::CorruptedCatalog => "document_catalog.corrupted",
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ApplyDocumentLinkCatalogChangeUsecase;

impl ApplyDocumentLinkCatalogChangeUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        event: &DocumentChangeEvent,
        catalog: &mut impl DocumentLinkCatalog,
    ) -> Result<DocumentLinkCatalogChangeOutcome, ApplyDocumentLinkCatalogChangeError> {
        match event {
            DocumentChangeEvent::DocumentCreated {
                workspace_id,
                document_id,
                title,
                path,
                ..
            } => self.upsert(workspace_id, document_id, title, path, catalog),
            DocumentChangeEvent::DocumentRenamed {
                workspace_id,
                document_id,
                title,
                new_path,
                ..
            } => self.upsert(workspace_id, document_id, title, new_path, catalog),
            DocumentChangeEvent::DocumentDeleted {
                workspace_id,
                document_id,
                ..
            } => {
                let workspace_id = parse_workspace(workspace_id)?;
                let document_id = DocumentId::new(document_id)
                    .map_err(|_| ApplyDocumentLinkCatalogChangeError::InvalidEvent)?;
                catalog
                    .remove(&workspace_id, &document_id)
                    .map_err(map_catalog_error)?;
                Ok(DocumentLinkCatalogChangeOutcome::Removed)
            }
            _ => Ok(DocumentLinkCatalogChangeOutcome::Ignored),
        }
    }

    fn upsert(
        &self,
        workspace_id: &str,
        document_id: &str,
        title: &str,
        path: &str,
        catalog: &mut impl DocumentLinkCatalog,
    ) -> Result<DocumentLinkCatalogChangeOutcome, ApplyDocumentLinkCatalogChangeError> {
        let workspace_id = parse_workspace(workspace_id)?;
        let record = DocumentLinkCatalogRecord::new(
            DocumentId::new(document_id)
                .map_err(|_| ApplyDocumentLinkCatalogChangeError::InvalidEvent)?,
            DocumentTitle::new(title)
                .map_err(|_| ApplyDocumentLinkCatalogChangeError::InvalidEvent)?,
            DocumentPath::new(path)
                .map_err(|_| ApplyDocumentLinkCatalogChangeError::InvalidEvent)?,
        )
        .map_err(map_catalog_error)?;
        catalog
            .upsert(&workspace_id, record)
            .map_err(map_catalog_error)?;
        Ok(DocumentLinkCatalogChangeOutcome::Upserted)
    }
}

fn parse_workspace(value: &str) -> Result<WorkspaceId, ApplyDocumentLinkCatalogChangeError> {
    WorkspaceId::new(value).map_err(|_| ApplyDocumentLinkCatalogChangeError::InvalidEvent)
}

const fn map_catalog_error(error: DocumentLinkCatalogError) -> ApplyDocumentLinkCatalogChangeError {
    match error {
        DocumentLinkCatalogError::InvalidRecord => {
            ApplyDocumentLinkCatalogChangeError::InvalidEvent
        }
        DocumentLinkCatalogError::StorageUnavailable => {
            ApplyDocumentLinkCatalogChangeError::RepositoryUnavailable
        }
        DocumentLinkCatalogError::CorruptedCatalog => {
            ApplyDocumentLinkCatalogChangeError::CorruptedCatalog
        }
    }
}
