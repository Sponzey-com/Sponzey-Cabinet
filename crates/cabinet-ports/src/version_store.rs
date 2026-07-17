use cabinet_domain::document::{DocumentBody, DocumentId};
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentSnapshotRef, VersionEntry, VersionId,
};
use cabinet_domain::workspace::WorkspaceId;

pub const MAX_HISTORY_PAGE_LIMIT: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionSnapshot {
    document_id: DocumentId,
    snapshot_ref: DocumentSnapshotRef,
    body: DocumentBody,
    attachment_state: AttachmentSnapshotState,
}

impl VersionSnapshot {
    pub fn new(
        document_id: DocumentId,
        snapshot_ref: DocumentSnapshotRef,
        body: DocumentBody,
    ) -> Self {
        Self::with_attachment_state(
            document_id,
            snapshot_ref,
            body,
            AttachmentSnapshotState::legacy_unknown(),
        )
    }

    pub fn with_attachment_state(
        document_id: DocumentId,
        snapshot_ref: DocumentSnapshotRef,
        body: DocumentBody,
        attachment_state: AttachmentSnapshotState,
    ) -> Self {
        Self {
            document_id,
            snapshot_ref,
            body,
            attachment_state,
        }
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn snapshot_ref(&self) -> &DocumentSnapshotRef {
        &self.snapshot_ref
    }

    pub fn body(&self) -> &DocumentBody {
        &self.body
    }

    pub fn attachment_state(&self) -> &AttachmentSnapshotState {
        &self.attachment_state
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRecord {
    entry: VersionEntry,
    snapshot: VersionSnapshot,
}

impl VersionRecord {
    pub fn new(entry: VersionEntry, snapshot: VersionSnapshot) -> Result<Self, VersionStoreError> {
        if entry.document_id() != snapshot.document_id()
            || entry.snapshot_ref() != snapshot.snapshot_ref()
        {
            return Err(VersionStoreError::MismatchedVersionSnapshot);
        }

        Ok(Self { entry, snapshot })
    }

    pub fn entry(&self) -> &VersionEntry {
        &self.entry
    }

    pub fn snapshot(&self) -> &VersionSnapshot {
        &self.snapshot
    }

    pub fn document_id(&self) -> &DocumentId {
        self.entry.document_id()
    }

    pub fn version_id(&self) -> &VersionId {
        self.entry.version_id()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCursor {
    value: String,
}

impl HistoryCursor {
    pub fn new(value: &str) -> Result<Self, VersionStoreError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(VersionStoreError::InvalidHistoryCursor);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryPageRequest {
    cursor: Option<HistoryCursor>,
    limit: usize,
}

impl HistoryPageRequest {
    pub fn first(limit: usize) -> Result<Self, VersionStoreError> {
        Self::new(None, limit)
    }

    pub fn after(cursor: HistoryCursor, limit: usize) -> Result<Self, VersionStoreError> {
        Self::new(Some(cursor), limit)
    }

    fn new(cursor: Option<HistoryCursor>, limit: usize) -> Result<Self, VersionStoreError> {
        if limit == 0 || limit > MAX_HISTORY_PAGE_LIMIT {
            return Err(VersionStoreError::InvalidHistoryPageLimit);
        }
        Ok(Self { cursor, limit })
    }

    pub fn cursor(&self) -> Option<&HistoryCursor> {
        self.cursor.as_ref()
    }

    pub fn limit(&self) -> usize {
        self.limit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryPage {
    entries: Vec<VersionEntry>,
    next_cursor: Option<HistoryCursor>,
}

impl HistoryPage {
    pub fn new(entries: Vec<VersionEntry>, next_cursor: Option<HistoryCursor>) -> Self {
        Self {
            entries,
            next_cursor,
        }
    }

    pub fn entries(&self) -> &[VersionEntry] {
        &self.entries
    }

    pub fn next_cursor(&self) -> Option<&HistoryCursor> {
        self.next_cursor.as_ref()
    }
}

pub trait VersionStore {
    fn append_version(
        &mut self,
        workspace_id: &WorkspaceId,
        record: VersionRecord,
    ) -> Result<(), VersionStoreError>;

    fn get_version_snapshot(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError>;

    fn list_history(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        request: HistoryPageRequest,
    ) -> Result<HistoryPage, VersionStoreError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionStoreError {
    MismatchedVersionSnapshot,
    InvalidHistoryPageLimit,
    InvalidHistoryCursor,
    StorageUnavailable,
    CorruptedHistory,
    Conflict,
}

impl VersionStoreError {
    pub fn code(self) -> &'static str {
        match self {
            Self::MismatchedVersionSnapshot => "version_store.mismatched_version_snapshot",
            Self::InvalidHistoryPageLimit => "version_store.invalid_history_page_limit",
            Self::InvalidHistoryCursor => "version_store.invalid_history_cursor",
            Self::StorageUnavailable => "version_store.storage_unavailable",
            Self::CorruptedHistory => "version_store.corrupted_history",
            Self::Conflict => "version_store.conflict",
        }
    }
}
