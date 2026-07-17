use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_domain::document::DocumentId;
use cabinet_domain::document_revision::DocumentExpectedCurrentVersion;
use cabinet_domain::version::{DocumentRevisionNumber, DocumentSnapshotRef, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::document_revision_metadata::{
    DocumentRevisionClock, DocumentRevisionMetadataPortError, DocumentRevisionNumberAllocator,
    DocumentSnapshotRefGenerator, DocumentVersionIdGenerator,
};
use cabinet_ports::version_store::{HistoryPageRequest, VersionStore};
use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalDocumentRevisionMetadataSource;

impl LocalDocumentRevisionMetadataSource {
    pub const fn new() -> Self {
        Self
    }
}

impl DocumentVersionIdGenerator for LocalDocumentRevisionMetadataSource {
    fn generate_version_id(&self) -> Result<VersionId, DocumentRevisionMetadataPortError> {
        VersionId::new(&format!("version:{}", Uuid::new_v4()))
            .map_err(|_| DocumentRevisionMetadataPortError::GenerationUnavailable)
    }
}

impl DocumentSnapshotRefGenerator for LocalDocumentRevisionMetadataSource {
    fn generate_snapshot_ref(
        &self,
        version_id: &VersionId,
    ) -> Result<DocumentSnapshotRef, DocumentRevisionMetadataPortError> {
        DocumentSnapshotRef::new(&format!("snapshot:{}", version_id.as_str()))
            .map_err(|_| DocumentRevisionMetadataPortError::GenerationUnavailable)
    }
}

impl DocumentRevisionClock for LocalDocumentRevisionMetadataSource {
    fn now_epoch_ms(&self) -> Result<u64, DocumentRevisionMetadataPortError> {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| DocumentRevisionMetadataPortError::GenerationUnavailable)?
            .as_millis();
        u64::try_from(millis)
            .ok()
            .filter(|value| *value > 0)
            .ok_or(DocumentRevisionMetadataPortError::GenerationUnavailable)
    }
}

pub struct LocalDocumentRevisionNumberAllocator<'a, V, P> {
    versions: &'a V,
    pointer: &'a P,
}

impl<'a, V, P> LocalDocumentRevisionNumberAllocator<'a, V, P> {
    pub const fn new(versions: &'a V, pointer: &'a P) -> Self {
        Self { versions, pointer }
    }
}

impl<V, P> DocumentRevisionNumberAllocator for LocalDocumentRevisionNumberAllocator<'_, V, P>
where
    V: VersionStore,
    P: CurrentDocumentVersionPointerPort,
{
    fn allocate_next_revision(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        expected_current: &DocumentExpectedCurrentVersion,
    ) -> Result<DocumentRevisionNumber, DocumentRevisionMetadataPortError> {
        let current = self
            .pointer
            .load_current_version(workspace_id, document_id)
            .map_err(|_| DocumentRevisionMetadataPortError::StorageUnavailable)?;
        match expected_current {
            DocumentExpectedCurrentVersion::MustNotExist => {
                if current.is_some() {
                    return Err(DocumentRevisionMetadataPortError::Conflict);
                }
                DocumentRevisionNumber::new(1)
                    .map_err(|_| DocumentRevisionMetadataPortError::GenerationUnavailable)
            }
            DocumentExpectedCurrentVersion::MustMatch(expected) => {
                if current.as_ref() != Some(expected) {
                    return Err(DocumentRevisionMetadataPortError::Conflict);
                }
                self.next_after_history_entry(workspace_id, document_id, expected)
            }
        }
    }
}

impl<V, P> LocalDocumentRevisionNumberAllocator<'_, V, P>
where
    V: VersionStore,
{
    fn next_after_history_entry(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        expected: &VersionId,
    ) -> Result<DocumentRevisionNumber, DocumentRevisionMetadataPortError> {
        let mut request = HistoryPageRequest::first(100)
            .map_err(|_| DocumentRevisionMetadataPortError::StorageUnavailable)?;
        loop {
            let page = self
                .versions
                .list_history(workspace_id, document_id, request)
                .map_err(|_| DocumentRevisionMetadataPortError::StorageUnavailable)?;
            if let Some(entry) = page
                .entries()
                .iter()
                .find(|entry| entry.version_id() == expected)
            {
                let current_revision = entry
                    .revision_number()
                    .ok_or(DocumentRevisionMetadataPortError::StorageUnavailable)?;
                let next = current_revision
                    .value()
                    .checked_add(1)
                    .ok_or(DocumentRevisionMetadataPortError::StorageUnavailable)?;
                return DocumentRevisionNumber::new(next)
                    .map_err(|_| DocumentRevisionMetadataPortError::StorageUnavailable);
            }
            let Some(cursor) = page.next_cursor().cloned() else {
                return Err(DocumentRevisionMetadataPortError::StorageUnavailable);
            };
            request = HistoryPageRequest::after(cursor, 100)
                .map_err(|_| DocumentRevisionMetadataPortError::StorageUnavailable)?;
        }
    }
}
