use cabinet_domain::document::DocumentId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;

use crate::version_store::VersionRecord;

pub trait CommittedVersionRecordReader {
    fn get_committed_version_record(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<VersionRecord>, CommittedVersionRecordReadError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommittedVersionRecordReadError {
    StorageUnavailable,
    CorruptedRecord,
}

impl CommittedVersionRecordReadError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "committed_version_record.storage_unavailable",
            Self::CorruptedRecord => "committed_version_record.corrupted_record",
        }
    }
}
