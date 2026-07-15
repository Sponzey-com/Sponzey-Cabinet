use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;

pub trait DocumentExistenceReader {
    fn exists(
        &self,
        workspace: &WorkspaceId,
        document: &DocumentId,
    ) -> Result<bool, DocumentExistenceError>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentExistenceError {
    StorageUnavailable,
    CorruptedRecord,
}
impl DocumentExistenceError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "document_existence.storage_unavailable",
            Self::CorruptedRecord => "document_existence.corrupted",
        }
    }
}
