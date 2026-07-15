use cabinet_domain::document::DocumentId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentDocumentProjectionIdentity {
    document_id: DocumentId,
    version_id: VersionId,
}

impl CurrentDocumentProjectionIdentity {
    pub fn new(document_id: DocumentId, version_id: VersionId) -> Self {
        Self {
            document_id,
            version_id,
        }
    }
    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }
    pub fn version_id(&self) -> &VersionId {
        &self.version_id
    }
}

pub trait CurrentDocumentProjectionCatalog {
    fn list_current_projection_identities(
        &self,
        workspace_id: &WorkspaceId,
        limit: usize,
    ) -> Result<Vec<CurrentDocumentProjectionIdentity>, CurrentDocumentProjectionCatalogError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentDocumentProjectionCatalogError {
    InvalidLimit,
    LimitExceeded,
    StorageUnavailable,
    CorruptedRecord,
}
