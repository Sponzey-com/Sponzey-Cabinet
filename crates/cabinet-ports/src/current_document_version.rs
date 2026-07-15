use cabinet_domain::document::DocumentId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;

pub trait CurrentDocumentVersionPointerPort {
    fn load_current_version(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<VersionId>, CurrentDocumentVersionPointerError>;

    fn compare_and_set_current_version(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        expected: Option<&VersionId>,
        next: VersionId,
    ) -> Result<(), CurrentDocumentVersionPointerError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentDocumentVersionPointerError {
    Conflict,
    StorageUnavailable,
    CorruptedPointer,
}

impl CurrentDocumentVersionPointerError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Conflict => "current_document_version.conflict",
            Self::StorageUnavailable => "current_document_version.storage_unavailable",
            Self::CorruptedPointer => "current_document_version.corrupted",
        }
    }
}
