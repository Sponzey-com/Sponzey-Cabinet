use cabinet_domain::document::{DocumentBody, DocumentId, DocumentMetadata, DocumentPath};
use cabinet_domain::version::CurrentDocumentSnapshot;
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentDocumentRecord {
    metadata: DocumentMetadata,
    snapshot: CurrentDocumentSnapshot,
}

impl CurrentDocumentRecord {
    pub fn new(
        metadata: DocumentMetadata,
        snapshot: CurrentDocumentSnapshot,
    ) -> Result<Self, DocumentRepositoryError> {
        if metadata.id() != snapshot.document_id() {
            return Err(DocumentRepositoryError::MismatchedDocumentIdentity);
        }

        Ok(Self { metadata, snapshot })
    }

    pub fn metadata(&self) -> &DocumentMetadata {
        &self.metadata
    }

    pub fn snapshot(&self) -> &CurrentDocumentSnapshot {
        &self.snapshot
    }

    pub fn document_id(&self) -> &DocumentId {
        self.metadata.id()
    }

    pub fn path(&self) -> &DocumentPath {
        self.metadata.path()
    }

    pub fn body(&self) -> &DocumentBody {
        self.snapshot.body()
    }
}

pub trait DocumentRepository {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError>;

    fn get_current_by_id(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError>;

    fn get_current_by_path(
        &self,
        workspace_id: &WorkspaceId,
        path: &DocumentPath,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError>;

    fn delete_current(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<(), DocumentRepositoryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentRepositoryError {
    MismatchedDocumentIdentity,
    StorageUnavailable,
    CorruptedMetadata,
    Conflict,
}

impl DocumentRepositoryError {
    pub fn code(self) -> &'static str {
        match self {
            Self::MismatchedDocumentIdentity => "document_repository.mismatched_document_identity",
            Self::StorageUnavailable => "document_repository.storage_unavailable",
            Self::CorruptedMetadata => "document_repository.corrupted_metadata",
            Self::Conflict => "document_repository.conflict",
        }
    }
}
