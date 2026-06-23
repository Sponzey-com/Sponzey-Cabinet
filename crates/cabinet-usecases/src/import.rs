use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::{
    CurrentDocumentSnapshot, DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId,
    VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_ports::version_store::{
    VersionRecord, VersionSnapshot, VersionStore, VersionStoreError,
};

const IMPORT_BODY_MAX_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportMarkdownFolderInput {
    workspace_id: String,
    entries: Vec<ImportMarkdownEntryInput>,
}

impl ImportMarkdownFolderInput {
    pub fn new(workspace_id: &str, entries: Vec<ImportMarkdownEntryInput>) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            entries,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportMarkdownEntryInput {
    document_id: String,
    title: String,
    path: String,
    body: String,
    version_id: String,
    snapshot_ref: String,
    author: String,
    summary: String,
}

impl ImportMarkdownEntryInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        document_id: &str,
        title: &str,
        path: &str,
        body: &str,
        version_id: &str,
        snapshot_ref: &str,
        author: &str,
        summary: &str,
    ) -> Self {
        Self {
            document_id: document_id.to_string(),
            title: title.to_string(),
            path: path.to_string(),
            body: body.to_string(),
            version_id: version_id.to_string(),
            snapshot_ref: snapshot_ref.to_string(),
            author: author.to_string(),
            summary: summary.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportMarkdownFolderOutput {
    final_state: MarkdownImportState,
    imported_count: usize,
    failed_items: Vec<ImportFailedItem>,
}

impl ImportMarkdownFolderOutput {
    pub fn final_state(&self) -> MarkdownImportState {
        self.final_state
    }

    pub fn imported_count(&self) -> usize {
        self.imported_count
    }

    pub fn failed_items(&self) -> &[ImportFailedItem] {
        &self.failed_items
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFailedItem {
    document_id: String,
    error_code: &'static str,
}

impl ImportFailedItem {
    pub fn document_id(&self) -> &str {
        &self.document_id
    }

    pub fn error_code(&self) -> &'static str {
        self.error_code
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownImportState {
    Requested,
    Validating,
    Importing,
    Completed,
    PartiallyFailed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportMarkdownFolderUsecase;

impl ImportMarkdownFolderUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ImportMarkdownFolderInput,
        document_repository: &mut impl DocumentRepository,
        version_store: &mut impl VersionStore,
    ) -> Result<ImportMarkdownFolderOutput, ImportMarkdownFolderError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ImportMarkdownFolderError::InvalidInput)?;
        let mut imported_count = 0;
        let mut failed_items = Vec::new();

        for entry in input.entries {
            let document_id = entry.document_id.clone();
            match import_entry(&workspace_id, entry, document_repository, version_store) {
                Ok(()) => imported_count += 1,
                Err(error) => failed_items.push(ImportFailedItem {
                    document_id,
                    error_code: error.code(),
                }),
            }
        }

        let final_state = match (imported_count, failed_items.is_empty()) {
            (0, false) | (0, true) => MarkdownImportState::Failed,
            (_, true) => MarkdownImportState::Completed,
            (_, false) => MarkdownImportState::PartiallyFailed,
        };

        Ok(ImportMarkdownFolderOutput {
            final_state,
            imported_count,
            failed_items,
        })
    }
}

impl Default for ImportMarkdownFolderUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMarkdownFolderError {
    InvalidInput,
}

impl ImportMarkdownFolderError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "markdown_import.invalid_input",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportEntryError {
    InvalidInput,
    Conflict,
    StorageUnavailable,
}

impl ImportEntryError {
    const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "markdown_import.invalid_entry",
            Self::Conflict => "markdown_import.conflict",
            Self::StorageUnavailable => "markdown_import.storage_unavailable",
        }
    }
}

fn import_entry(
    workspace_id: &WorkspaceId,
    entry: ImportMarkdownEntryInput,
    document_repository: &mut impl DocumentRepository,
    version_store: &mut impl VersionStore,
) -> Result<(), ImportEntryError> {
    let command = ImportEntryCommand::from_input(entry)?;
    document_repository
        .put_current(workspace_id, command.current_record.clone())
        .map_err(ImportEntryError::from_document_repository_error)?;
    version_store
        .append_version(workspace_id, command.version_record)
        .map_err(ImportEntryError::from_version_store_error)
}

struct ImportEntryCommand {
    current_record: CurrentDocumentRecord,
    version_record: VersionRecord,
}

impl ImportEntryCommand {
    fn from_input(input: ImportMarkdownEntryInput) -> Result<Self, ImportEntryError> {
        let body_policy = DocumentBodyPolicy::new(IMPORT_BODY_MAX_BYTES)
            .map_err(|_| ImportEntryError::InvalidInput)?;
        let document_id =
            DocumentId::new(&input.document_id).map_err(|_| ImportEntryError::InvalidInput)?;
        let title = DocumentTitle::new(&input.title).map_err(|_| ImportEntryError::InvalidInput)?;
        let path = DocumentPath::new(&input.path).map_err(|_| ImportEntryError::InvalidInput)?;
        let body = DocumentBody::new(&input.body, body_policy)
            .map_err(|_| ImportEntryError::InvalidInput)?;
        let version_id =
            VersionId::new(&input.version_id).map_err(|_| ImportEntryError::InvalidInput)?;
        let snapshot_ref = DocumentSnapshotRef::new(&input.snapshot_ref)
            .map_err(|_| ImportEntryError::InvalidInput)?;
        let author =
            VersionAuthor::new(&input.author).map_err(|_| ImportEntryError::InvalidInput)?;
        let summary =
            VersionSummary::new(&input.summary).map_err(|_| ImportEntryError::InvalidInput)?;
        let metadata = DocumentMetadata::new(document_id.clone(), title, path)
            .map_err(|_| ImportEntryError::InvalidInput)?;
        let current_snapshot = CurrentDocumentSnapshot::new(document_id.clone(), body.clone());
        let current_record = CurrentDocumentRecord::new(metadata, current_snapshot)
            .map_err(|_| ImportEntryError::InvalidInput)?;
        let version_entry = VersionEntry::new(
            version_id,
            document_id.clone(),
            snapshot_ref.clone(),
            author,
            summary,
        )
        .map_err(|_| ImportEntryError::InvalidInput)?;
        let version_snapshot = VersionSnapshot::new(document_id, snapshot_ref, body);
        let version_record = VersionRecord::new(version_entry, version_snapshot)
            .map_err(|_| ImportEntryError::InvalidInput)?;
        Ok(Self {
            current_record,
            version_record,
        })
    }
}

impl ImportEntryError {
    fn from_document_repository_error(error: DocumentRepositoryError) -> Self {
        match error {
            DocumentRepositoryError::Conflict => Self::Conflict,
            DocumentRepositoryError::StorageUnavailable
            | DocumentRepositoryError::CorruptedMetadata
            | DocumentRepositoryError::MismatchedDocumentIdentity => Self::StorageUnavailable,
        }
    }

    fn from_version_store_error(error: VersionStoreError) -> Self {
        match error {
            VersionStoreError::Conflict => Self::Conflict,
            VersionStoreError::StorageUnavailable
            | VersionStoreError::CorruptedHistory
            | VersionStoreError::InvalidHistoryCursor
            | VersionStoreError::InvalidHistoryPageLimit
            | VersionStoreError::MismatchedVersionSnapshot => Self::StorageUnavailable,
        }
    }
}
