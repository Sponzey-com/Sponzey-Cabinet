use crate::document_diff::{DiffComputation, DocumentLineDiffService};
pub use crate::document_diff::{
    DiffHunk, DocumentDiffResult, DocumentTitleDelta, LineDiff, LineDiffKind,
};
use crate::document_diff_query::{ExecuteDocumentDiffQueryError, ExecuteDocumentDiffQueryUsecase};
use cabinet_domain::asset::{
    AssetFileName, AssetId, AssetMediaType, AssetMetadata, AssetReference,
};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::document_diff_query::DocumentDiffQueryTarget;
use cabinet_domain::version::{
    CurrentDocumentSnapshot, DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId,
    VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_store::{AssetObject, AssetRecord, AssetStore, AssetStoreError};
use cabinet_ports::document_asset_repository::{
    DocumentAssetRecord, DocumentAssetRepository, DocumentAssetRepositoryError,
};
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_ports::version_store::{
    HistoryCursor, HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateDocumentInput {
    workspace_id: String,
    document_id: String,
    path: String,
    body: String,
    version_id: String,
    snapshot_ref: String,
    author: String,
    summary: String,
}

impl CreateDocumentInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: &str,
        document_id: &str,
        path: &str,
        body: &str,
        version_id: &str,
        snapshot_ref: &str,
        author: &str,
        summary: &str,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
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
pub struct CreateDocumentOutput {
    document_id: DocumentId,
    version_id: VersionId,
}

impl CreateDocumentOutput {
    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn version_id(&self) -> &VersionId {
        &self.version_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentChangeEvent {
    DocumentCreated {
        workspace_id: String,
        document_id: String,
        version_id: String,
        title: String,
        path: String,
    },
    DocumentRestored {
        workspace_id: String,
        document_id: String,
        target_version_id: String,
        restored_version_id: String,
    },
    DocumentUpdated {
        workspace_id: String,
        document_id: String,
        version_id: String,
        title: String,
        path: String,
    },
    DocumentRenamed {
        workspace_id: String,
        document_id: String,
        version_id: String,
        title: String,
        old_path: String,
        new_path: String,
    },
    DocumentDeleted {
        workspace_id: String,
        document_id: String,
        version_id: String,
    },
    DocumentAssetAttached {
        workspace_id: String,
        document_id: String,
        version_id: String,
        asset_id: String,
    },
}

pub trait DocumentChangeEventPublisher {
    fn publish(&mut self, event: DocumentChangeEvent);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateDocumentProductEvent {
    DocumentCreated {
        document_id: String,
    },
    DocumentRestored {
        document_id: String,
        restored_version_id: String,
    },
    DocumentUpdated {
        document_id: String,
        version_id: String,
    },
    DocumentRenamed {
        document_id: String,
    },
    DocumentDeleted {
        document_id: String,
    },
    DocumentAssetAttached {
        document_id: String,
        asset_id: String,
    },
    UsecaseFailed {
        error_code: &'static str,
    },
}

pub trait DocumentProductLogger {
    fn write_product(&mut self, event: CreateDocumentProductEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateDocumentUsecase {
    body_policy: DocumentBodyPolicy,
}

impl CreateDocumentUsecase {
    pub const fn new(body_policy: DocumentBodyPolicy) -> Self {
        Self { body_policy }
    }

    pub fn execute(
        &self,
        input: CreateDocumentInput,
        document_repository: &mut impl DocumentRepository,
        version_store: &mut impl VersionStore,
        event_publisher: &mut impl DocumentChangeEventPublisher,
        product_logger: &mut impl DocumentProductLogger,
    ) -> Result<CreateDocumentOutput, CreateDocumentError> {
        let command = match CreateDocumentCommand::from_input(input, self.body_policy) {
            Ok(command) => command,
            Err(error) => {
                product_logger.write_product(CreateDocumentProductEvent::UsecaseFailed {
                    error_code: error.code(),
                });
                return Err(error);
            }
        };

        document_repository
            .put_current(&command.workspace_id, command.current_record.clone())
            .map_err(CreateDocumentError::from_document_repository_error)
            .map_err(|error| {
                product_logger.write_product(CreateDocumentProductEvent::UsecaseFailed {
                    error_code: error.code(),
                });
                error
            })?;

        version_store
            .append_version(&command.workspace_id, command.version_record.clone())
            .map_err(CreateDocumentError::from_version_store_error)
            .map_err(|error| {
                product_logger.write_product(CreateDocumentProductEvent::UsecaseFailed {
                    error_code: error.code(),
                });
                error
            })?;

        event_publisher.publish(DocumentChangeEvent::DocumentCreated {
            workspace_id: command.workspace_id.as_str().to_string(),
            document_id: command.document_id.as_str().to_string(),
            version_id: command.version_id.as_str().to_string(),
            title: command
                .current_record
                .metadata()
                .title()
                .as_str()
                .to_string(),
            path: command
                .current_record
                .metadata()
                .path()
                .as_str()
                .to_string(),
        });
        product_logger.write_product(CreateDocumentProductEvent::DocumentCreated {
            document_id: command.document_id.as_str().to_string(),
        });

        Ok(CreateDocumentOutput {
            document_id: command.document_id,
            version_id: command.version_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetCurrentDocumentInput {
    ById {
        workspace_id: String,
        document_id: String,
    },
    ByPath {
        workspace_id: String,
        path: String,
    },
}

impl GetCurrentDocumentInput {
    pub fn by_id(workspace_id: &str, document_id: &str) -> Self {
        Self::ById {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
        }
    }

    pub fn by_path(workspace_id: &str, path: &str) -> Self {
        Self::ByPath {
            workspace_id: workspace_id.to_string(),
            path: path.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetCurrentDocumentOutput {
    record: CurrentDocumentRecord,
}

impl GetCurrentDocumentOutput {
    pub fn record(&self) -> &CurrentDocumentRecord {
        &self.record
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetCurrentDocumentUsecase;

impl GetCurrentDocumentUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetCurrentDocumentInput,
        document_repository: &impl DocumentRepository,
    ) -> Result<GetCurrentDocumentOutput, GetCurrentDocumentError> {
        let record = match input {
            GetCurrentDocumentInput::ById {
                workspace_id,
                document_id,
            } => {
                let workspace_id = WorkspaceId::new(&workspace_id)
                    .map_err(|_| GetCurrentDocumentError::InvalidInput)?;
                let document_id = DocumentId::new(&document_id)
                    .map_err(|_| GetCurrentDocumentError::InvalidInput)?;
                document_repository
                    .get_current_by_id(&workspace_id, &document_id)
                    .map_err(GetCurrentDocumentError::from_repository_error)?
            }
            GetCurrentDocumentInput::ByPath { workspace_id, path } => {
                let workspace_id = WorkspaceId::new(&workspace_id)
                    .map_err(|_| GetCurrentDocumentError::InvalidInput)?;
                let path =
                    DocumentPath::new(&path).map_err(|_| GetCurrentDocumentError::InvalidInput)?;
                document_repository
                    .get_current_by_path(&workspace_id, &path)
                    .map_err(GetCurrentDocumentError::from_repository_error)?
            }
        };

        record
            .map(|record| GetCurrentDocumentOutput { record })
            .ok_or(GetCurrentDocumentError::NotFound)
    }
}

impl Default for GetCurrentDocumentUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetCurrentDocumentError {
    InvalidInput,
    NotFound,
    StorageUnavailable,
}

impl GetCurrentDocumentError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document.invalid_input",
            Self::NotFound => "document.not_found",
            Self::StorageUnavailable => "document.storage_unavailable",
        }
    }

    fn from_repository_error(error: DocumentRepositoryError) -> Self {
        match error {
            DocumentRepositoryError::StorageUnavailable
            | DocumentRepositoryError::CorruptedMetadata
            | DocumentRepositoryError::MismatchedDocumentIdentity
            | DocumentRepositoryError::Conflict => Self::StorageUnavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetDocumentVersionInput {
    workspace_id: String,
    document_id: String,
    version_id: String,
}

impl GetDocumentVersionInput {
    pub fn new(workspace_id: &str, document_id: &str, version_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            version_id: version_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetDocumentVersionOutput {
    snapshot: VersionSnapshot,
}

impl GetDocumentVersionOutput {
    pub fn snapshot(&self) -> &VersionSnapshot {
        &self.snapshot
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetDocumentVersionUsecase;

impl GetDocumentVersionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetDocumentVersionInput,
        version_store: &impl VersionStore,
    ) -> Result<GetDocumentVersionOutput, GetDocumentVersionError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| GetDocumentVersionError::InvalidInput)?;
        let document_id = DocumentId::new(&input.document_id)
            .map_err(|_| GetDocumentVersionError::InvalidInput)?;
        let version_id =
            VersionId::new(&input.version_id).map_err(|_| GetDocumentVersionError::InvalidInput)?;

        version_store
            .get_version_snapshot(&workspace_id, &document_id, &version_id)
            .map_err(GetDocumentVersionError::from_version_store_error)?
            .map(|snapshot| GetDocumentVersionOutput { snapshot })
            .ok_or(GetDocumentVersionError::NotFound)
    }
}

impl Default for GetDocumentVersionUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetDocumentVersionError {
    InvalidInput,
    NotFound,
    StorageUnavailable,
}

impl GetDocumentVersionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document.invalid_input",
            Self::NotFound => "document.version_not_found",
            Self::StorageUnavailable => "document.storage_unavailable",
        }
    }

    fn from_version_store_error(error: VersionStoreError) -> Self {
        match error {
            VersionStoreError::StorageUnavailable
            | VersionStoreError::CorruptedHistory
            | VersionStoreError::InvalidHistoryCursor
            | VersionStoreError::InvalidHistoryPageLimit
            | VersionStoreError::MismatchedVersionSnapshot
            | VersionStoreError::Conflict => Self::StorageUnavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetDocumentHistoryInput {
    workspace_id: String,
    document_id: String,
    cursor: Option<String>,
    limit: usize,
}

impl GetDocumentHistoryInput {
    pub fn new(workspace_id: &str, document_id: &str, cursor: Option<&str>, limit: usize) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            cursor: cursor.map(ToString::to_string),
            limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetDocumentHistoryOutput {
    page: HistoryPage,
}

impl GetDocumentHistoryOutput {
    pub fn page(&self) -> &HistoryPage {
        &self.page
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetDocumentHistoryUsecase;

impl GetDocumentHistoryUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetDocumentHistoryInput,
        version_store: &impl VersionStore,
    ) -> Result<GetDocumentHistoryOutput, GetDocumentHistoryError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| GetDocumentHistoryError::InvalidInput)?;
        let document_id = DocumentId::new(&input.document_id)
            .map_err(|_| GetDocumentHistoryError::InvalidInput)?;
        let request = history_page_request(input.cursor.as_deref(), input.limit)?;

        let page = version_store
            .list_history(&workspace_id, &document_id, request)
            .map_err(GetDocumentHistoryError::from_version_store_error)?;
        Ok(GetDocumentHistoryOutput { page })
    }
}

impl Default for GetDocumentHistoryUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetDocumentHistoryError {
    InvalidInput,
    StorageUnavailable,
}

impl GetDocumentHistoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document.invalid_input",
            Self::StorageUnavailable => "document.storage_unavailable",
        }
    }

    fn from_version_store_error(error: VersionStoreError) -> Self {
        match error {
            VersionStoreError::InvalidHistoryCursor
            | VersionStoreError::InvalidHistoryPageLimit => Self::InvalidInput,
            VersionStoreError::StorageUnavailable
            | VersionStoreError::CorruptedHistory
            | VersionStoreError::MismatchedVersionSnapshot
            | VersionStoreError::Conflict => Self::StorageUnavailable,
        }
    }
}

fn history_page_request(
    cursor: Option<&str>,
    limit: usize,
) -> Result<HistoryPageRequest, GetDocumentHistoryError> {
    match cursor {
        Some(cursor) => {
            let cursor =
                HistoryCursor::new(cursor).map_err(|_| GetDocumentHistoryError::InvalidInput)?;
            HistoryPageRequest::after(cursor, limit)
                .map_err(|_| GetDocumentHistoryError::InvalidInput)
        }
        None => HistoryPageRequest::first(limit).map_err(|_| GetDocumentHistoryError::InvalidInput),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareDocumentVersionsInput {
    CurrentToVersion {
        workspace_id: String,
        document_id: String,
        version_id: String,
    },
    Versions {
        workspace_id: String,
        document_id: String,
        left_version_id: String,
        right_version_id: String,
    },
}

impl CompareDocumentVersionsInput {
    pub fn current_to_version(workspace_id: &str, document_id: &str, version_id: &str) -> Self {
        Self::CurrentToVersion {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            version_id: version_id.to_string(),
        }
    }

    pub fn versions(
        workspace_id: &str,
        document_id: &str,
        left_version_id: &str,
        right_version_id: &str,
    ) -> Self {
        Self::Versions {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            left_version_id: left_version_id.to_string(),
            right_version_id: right_version_id.to_string(),
        }
    }

    fn into_target(self) -> Result<DocumentDiffQueryTarget, CompareDocumentVersionsError> {
        match self {
            Self::CurrentToVersion {
                workspace_id,
                document_id,
                version_id,
            } => DocumentDiffQueryTarget::current_to_version(
                &workspace_id,
                &document_id,
                &version_id,
            ),
            Self::Versions {
                workspace_id,
                document_id,
                left_version_id,
                right_version_id,
            } => DocumentDiffQueryTarget::versions(
                &workspace_id,
                &document_id,
                &left_version_id,
                &right_version_id,
            ),
        }
        .map_err(|_| CompareDocumentVersionsError::InvalidInput)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareDocumentVersionsOutput {
    diff: DocumentDiffResult,
}

impl CompareDocumentVersionsOutput {
    pub fn lines(&self) -> &[LineDiff] {
        self.diff.lines()
    }

    pub fn hunks(&self) -> &[DiffHunk] {
        self.diff.hunks()
    }

    pub fn diff(&self) -> &DocumentDiffResult {
        &self.diff
    }

    pub fn title_delta(&self) -> &DocumentTitleDelta {
        self.diff.title_delta()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompareDocumentVersionsUsecase {
    executor: ExecuteDocumentDiffQueryUsecase,
}

impl CompareDocumentVersionsUsecase {
    pub fn new() -> Self {
        Self::with_diff_service(DocumentLineDiffService::default())
    }

    pub const fn with_diff_service(diff_service: DocumentLineDiffService) -> Self {
        Self {
            executor: ExecuteDocumentDiffQueryUsecase::with_diff_service(diff_service),
        }
    }

    pub fn execute(
        &self,
        input: CompareDocumentVersionsInput,
        document_repository: &impl DocumentRepository,
        version_store: &impl VersionStore,
    ) -> Result<CompareDocumentVersionsOutput, CompareDocumentVersionsError> {
        let target = input.into_target()?;
        match self
            .executor
            .execute(&target, document_repository, version_store)
            .map_err(CompareDocumentVersionsError::from_executor_error)?
        {
            DiffComputation::Complete(diff) => Ok(CompareDocumentVersionsOutput { diff }),
            DiffComputation::TooLarge(_) => Err(CompareDocumentVersionsError::TooLarge),
        }
    }
}

impl Default for CompareDocumentVersionsUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareDocumentVersionsError {
    InvalidInput,
    NotFound,
    TooLarge,
    StorageUnavailable,
}

impl CompareDocumentVersionsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document.invalid_input",
            Self::NotFound => "document.diff_target_not_found",
            Self::TooLarge => "document.diff_too_large",
            Self::StorageUnavailable => "document.storage_unavailable",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(self, Self::TooLarge | Self::StorageUnavailable)
    }

    fn from_executor_error(error: ExecuteDocumentDiffQueryError) -> Self {
        match error {
            ExecuteDocumentDiffQueryError::NotFound => Self::NotFound,
            ExecuteDocumentDiffQueryError::StorageUnavailable => Self::StorageUnavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewDocumentRestoreInput {
    workspace_id: String,
    document_id: String,
    target_version_id: String,
}

impl PreviewDocumentRestoreInput {
    pub fn new(workspace_id: &str, document_id: &str, target_version_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            target_version_id: target_version_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewDocumentRestoreOutput {
    target_version_id: VersionId,
    can_restore: bool,
    diff: DocumentDiffResult,
}

impl PreviewDocumentRestoreOutput {
    pub fn target_version_id(&self) -> &VersionId {
        &self.target_version_id
    }

    pub fn can_restore(&self) -> bool {
        self.can_restore
    }

    pub fn lines(&self) -> &[LineDiff] {
        self.diff.lines()
    }

    pub fn hunks(&self) -> &[DiffHunk] {
        self.diff.hunks()
    }

    pub fn diff(&self) -> &DocumentDiffResult {
        &self.diff
    }

    pub fn title_delta(&self) -> &DocumentTitleDelta {
        self.diff.title_delta()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewDocumentRestoreUsecase {
    diff_service: DocumentLineDiffService,
}

impl PreviewDocumentRestoreUsecase {
    pub fn new() -> Self {
        Self::with_diff_service(DocumentLineDiffService::default())
    }

    pub const fn with_diff_service(diff_service: DocumentLineDiffService) -> Self {
        Self { diff_service }
    }

    pub fn execute(
        &self,
        input: PreviewDocumentRestoreInput,
        document_repository: &impl DocumentRepository,
        version_store: &impl VersionStore,
    ) -> Result<PreviewDocumentRestoreOutput, PreviewDocumentRestoreError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| PreviewDocumentRestoreError::InvalidInput)?;
        let document_id = DocumentId::new(&input.document_id)
            .map_err(|_| PreviewDocumentRestoreError::InvalidInput)?;
        let target_version_id = VersionId::new(&input.target_version_id)
            .map_err(|_| PreviewDocumentRestoreError::InvalidInput)?;

        let current = document_repository
            .get_current_by_id(&workspace_id, &document_id)
            .map_err(PreviewDocumentRestoreError::from_document_repository_error)?
            .ok_or(PreviewDocumentRestoreError::NotFound)?;
        let target = version_store
            .get_version_snapshot(&workspace_id, &document_id, &target_version_id)
            .map_err(PreviewDocumentRestoreError::from_version_store_error)?
            .ok_or(PreviewDocumentRestoreError::NotFound)?;

        match self
            .diff_service
            .compare(current.body().as_str(), target.body().as_str())
        {
            DiffComputation::Complete(diff) => Ok(PreviewDocumentRestoreOutput {
                target_version_id,
                can_restore: true,
                diff,
            }),
            DiffComputation::TooLarge(_) => Err(PreviewDocumentRestoreError::TooLarge),
        }
    }
}

impl Default for PreviewDocumentRestoreUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewDocumentRestoreError {
    InvalidInput,
    NotFound,
    TooLarge,
    StorageUnavailable,
}

impl PreviewDocumentRestoreError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document.invalid_input",
            Self::NotFound => "document.restore_target_not_found",
            Self::TooLarge => "document.diff_too_large",
            Self::StorageUnavailable => "document.storage_unavailable",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(self, Self::TooLarge | Self::StorageUnavailable)
    }

    fn from_document_repository_error(error: DocumentRepositoryError) -> Self {
        match error {
            DocumentRepositoryError::StorageUnavailable
            | DocumentRepositoryError::CorruptedMetadata
            | DocumentRepositoryError::MismatchedDocumentIdentity
            | DocumentRepositoryError::Conflict => Self::StorageUnavailable,
        }
    }

    fn from_version_store_error(error: VersionStoreError) -> Self {
        match error {
            VersionStoreError::StorageUnavailable
            | VersionStoreError::CorruptedHistory
            | VersionStoreError::InvalidHistoryCursor
            | VersionStoreError::InvalidHistoryPageLimit
            | VersionStoreError::MismatchedVersionSnapshot
            | VersionStoreError::Conflict => Self::StorageUnavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreDocumentVersionInput {
    workspace_id: String,
    document_id: String,
    target_version_id: String,
    restored_version_id: String,
    restored_snapshot_ref: String,
    author: String,
    summary: String,
}

impl RestoreDocumentVersionInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: &str,
        document_id: &str,
        target_version_id: &str,
        restored_version_id: &str,
        restored_snapshot_ref: &str,
        author: &str,
        summary: &str,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            target_version_id: target_version_id.to_string(),
            restored_version_id: restored_version_id.to_string(),
            restored_snapshot_ref: restored_snapshot_ref.to_string(),
            author: author.to_string(),
            summary: summary.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreDocumentVersionOutput {
    restored_version_id: VersionId,
    final_state: RestoreDocumentVersionState,
}

impl RestoreDocumentVersionOutput {
    pub fn restored_version_id(&self) -> &VersionId {
        &self.restored_version_id
    }

    pub fn final_state(&self) -> RestoreDocumentVersionState {
        self.final_state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreDocumentVersionState {
    Requested,
    LoadingCurrent,
    LoadingTarget,
    WritingRestoreVersion,
    UpdatingCurrent,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoreDocumentVersionUsecase;

impl RestoreDocumentVersionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: RestoreDocumentVersionInput,
        document_repository: &mut impl DocumentRepository,
        version_store: &mut impl VersionStore,
        event_publisher: &mut impl DocumentChangeEventPublisher,
        product_logger: &mut impl DocumentProductLogger,
    ) -> Result<RestoreDocumentVersionOutput, RestoreDocumentVersionError> {
        let command = match RestoreDocumentVersionCommand::from_input(input) {
            Ok(command) => command,
            Err(error) => {
                write_restore_failure(product_logger, &error);
                return Err(error);
            }
        };

        let current = document_repository
            .get_current_by_id(&command.workspace_id, &command.document_id)
            .map_err(RestoreDocumentVersionError::from_document_repository_error)
            .map_err(|error| {
                write_restore_failure(product_logger, &error);
                error
            })?
            .ok_or_else(|| {
                let error =
                    RestoreDocumentVersionError::not_found(RestoreDocumentVersionState::Failed);
                write_restore_failure(product_logger, &error);
                error
            })?;

        let target = version_store
            .get_version_snapshot(
                &command.workspace_id,
                &command.document_id,
                &command.target_version_id,
            )
            .map_err(RestoreDocumentVersionError::from_version_store_error)
            .map_err(|error| {
                write_restore_failure(product_logger, &error);
                error
            })?
            .ok_or_else(|| {
                let error =
                    RestoreDocumentVersionError::not_found(RestoreDocumentVersionState::Failed);
                write_restore_failure(product_logger, &error);
                error
            })?;

        let restore_record = command
            .restore_version_record(target.body().clone())
            .map_err(|error| {
                write_restore_failure(product_logger, &error);
                error
            })?;
        version_store
            .append_version(&command.workspace_id, restore_record)
            .map_err(RestoreDocumentVersionError::from_version_store_error)
            .map_err(|error| {
                write_restore_failure(product_logger, &error);
                error
            })?;

        let current_record = command
            .restored_current_record(current.metadata().clone(), target.body().clone())
            .map_err(|error| {
                write_restore_failure(product_logger, &error);
                error
            })?;
        document_repository
            .put_current(&command.workspace_id, current_record)
            .map_err(RestoreDocumentVersionError::from_document_repository_error)
            .map_err(|error| {
                write_restore_failure(product_logger, &error);
                error
            })?;

        event_publisher.publish(DocumentChangeEvent::DocumentRestored {
            workspace_id: command.workspace_id.as_str().to_string(),
            document_id: command.document_id.as_str().to_string(),
            target_version_id: command.target_version_id.as_str().to_string(),
            restored_version_id: command.restored_version_id.as_str().to_string(),
        });
        product_logger.write_product(CreateDocumentProductEvent::DocumentRestored {
            document_id: command.document_id.as_str().to_string(),
            restored_version_id: command.restored_version_id.as_str().to_string(),
        });

        Ok(RestoreDocumentVersionOutput {
            restored_version_id: command.restored_version_id,
            final_state: RestoreDocumentVersionState::Completed,
        })
    }
}

impl Default for RestoreDocumentVersionUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreDocumentVersionError {
    InvalidInput {
        final_state: RestoreDocumentVersionState,
    },
    NotFound {
        final_state: RestoreDocumentVersionState,
    },
    StorageUnavailable {
        final_state: RestoreDocumentVersionState,
    },
}

impl RestoreDocumentVersionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput { .. } => "document.invalid_input",
            Self::NotFound { .. } => "document.restore_target_not_found",
            Self::StorageUnavailable { .. } => "document.storage_unavailable",
        }
    }

    pub const fn final_state(self) -> RestoreDocumentVersionState {
        match self {
            Self::InvalidInput { final_state }
            | Self::NotFound { final_state }
            | Self::StorageUnavailable { final_state } => final_state,
        }
    }

    const fn invalid_input(final_state: RestoreDocumentVersionState) -> Self {
        Self::InvalidInput { final_state }
    }

    const fn not_found(final_state: RestoreDocumentVersionState) -> Self {
        Self::NotFound { final_state }
    }

    const fn storage_unavailable(final_state: RestoreDocumentVersionState) -> Self {
        Self::StorageUnavailable { final_state }
    }

    fn from_document_repository_error(_error: DocumentRepositoryError) -> Self {
        Self::storage_unavailable(RestoreDocumentVersionState::Failed)
    }

    fn from_version_store_error(_error: VersionStoreError) -> Self {
        Self::storage_unavailable(RestoreDocumentVersionState::Failed)
    }
}

struct RestoreDocumentVersionCommand {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    target_version_id: VersionId,
    restored_version_id: VersionId,
    restored_snapshot_ref: DocumentSnapshotRef,
    author: VersionAuthor,
    summary: VersionSummary,
}

impl RestoreDocumentVersionCommand {
    fn from_input(input: RestoreDocumentVersionInput) -> Result<Self, RestoreDocumentVersionError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id).map_err(|_| {
            RestoreDocumentVersionError::invalid_input(RestoreDocumentVersionState::Failed)
        })?;
        let document_id = DocumentId::new(&input.document_id).map_err(|_| {
            RestoreDocumentVersionError::invalid_input(RestoreDocumentVersionState::Failed)
        })?;
        let target_version_id = VersionId::new(&input.target_version_id).map_err(|_| {
            RestoreDocumentVersionError::invalid_input(RestoreDocumentVersionState::Failed)
        })?;
        let restored_version_id = VersionId::new(&input.restored_version_id).map_err(|_| {
            RestoreDocumentVersionError::invalid_input(RestoreDocumentVersionState::Failed)
        })?;
        let restored_snapshot_ref = DocumentSnapshotRef::new(&input.restored_snapshot_ref)
            .map_err(|_| {
                RestoreDocumentVersionError::invalid_input(RestoreDocumentVersionState::Failed)
            })?;
        let author = VersionAuthor::new(&input.author).map_err(|_| {
            RestoreDocumentVersionError::invalid_input(RestoreDocumentVersionState::Failed)
        })?;
        let summary = VersionSummary::new(&input.summary).map_err(|_| {
            RestoreDocumentVersionError::invalid_input(RestoreDocumentVersionState::Failed)
        })?;

        Ok(Self {
            workspace_id,
            document_id,
            target_version_id,
            restored_version_id,
            restored_snapshot_ref,
            author,
            summary,
        })
    }

    fn restore_version_record(
        &self,
        body: DocumentBody,
    ) -> Result<VersionRecord, RestoreDocumentVersionError> {
        let entry = VersionEntry::new(
            self.restored_version_id.clone(),
            self.document_id.clone(),
            self.restored_snapshot_ref.clone(),
            self.author.clone(),
            self.summary.clone(),
        )
        .map_err(|_| {
            RestoreDocumentVersionError::invalid_input(RestoreDocumentVersionState::Failed)
        })?;
        let snapshot = VersionSnapshot::new(
            self.document_id.clone(),
            self.restored_snapshot_ref.clone(),
            body,
        );
        VersionRecord::new(entry, snapshot).map_err(|_| {
            RestoreDocumentVersionError::storage_unavailable(RestoreDocumentVersionState::Failed)
        })
    }

    fn restored_current_record(
        &self,
        metadata: DocumentMetadata,
        body: DocumentBody,
    ) -> Result<CurrentDocumentRecord, RestoreDocumentVersionError> {
        let metadata = metadata
            .with_title(DocumentTitle::from_markdown_body(&body))
            .map_err(|_| {
                RestoreDocumentVersionError::storage_unavailable(
                    RestoreDocumentVersionState::Failed,
                )
            })?;
        let snapshot = CurrentDocumentSnapshot::new(self.document_id.clone(), body);
        CurrentDocumentRecord::new(metadata, snapshot).map_err(|_| {
            RestoreDocumentVersionError::storage_unavailable(RestoreDocumentVersionState::Failed)
        })
    }
}

fn write_restore_failure(
    product_logger: &mut impl DocumentProductLogger,
    error: &RestoreDocumentVersionError,
) {
    product_logger.write_product(CreateDocumentProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateDocumentInput {
    workspace_id: String,
    document_id: String,
    body: String,
    version_id: String,
    snapshot_ref: String,
    author: String,
    summary: String,
}

impl UpdateDocumentInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: &str,
        document_id: &str,
        body: &str,
        version_id: &str,
        snapshot_ref: &str,
        author: &str,
        summary: &str,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            body: body.to_string(),
            version_id: version_id.to_string(),
            snapshot_ref: snapshot_ref.to_string(),
            author: author.to_string(),
            summary: summary.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateDocumentOutput {
    version_id: VersionId,
}

impl UpdateDocumentOutput {
    pub fn version_id(&self) -> &VersionId {
        &self.version_id
    }

    pub fn version_id_value(&self) -> &str {
        self.version_id.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateDocumentUsecase {
    body_policy: DocumentBodyPolicy,
}

impl UpdateDocumentUsecase {
    pub const fn new(body_policy: DocumentBodyPolicy) -> Self {
        Self { body_policy }
    }

    pub fn with_body_limit(max_body_bytes: usize) -> Result<Self, UpdateDocumentError> {
        DocumentBodyPolicy::new(max_body_bytes)
            .map(Self::new)
            .map_err(|_| UpdateDocumentError::InvalidDocumentInput)
    }

    pub fn execute(
        &self,
        input: UpdateDocumentInput,
        document_repository: &mut impl DocumentRepository,
        version_store: &mut impl VersionStore,
        event_publisher: &mut impl DocumentChangeEventPublisher,
        product_logger: &mut impl DocumentProductLogger,
    ) -> Result<UpdateDocumentOutput, UpdateDocumentError> {
        let command = match UpdateDocumentCommand::from_input(input, self.body_policy) {
            Ok(command) => command,
            Err(error) => {
                write_update_failure(product_logger, error);
                return Err(error);
            }
        };

        let current = document_repository
            .get_current_by_id(&command.workspace_id, &command.document_id)
            .map_err(UpdateDocumentError::from_document_repository_error)
            .map_err(|error| {
                write_update_failure(product_logger, error);
                error
            })?
            .ok_or_else(|| {
                let error = UpdateDocumentError::NotFound;
                write_update_failure(product_logger, error);
                error
            })?;

        version_store
            .append_version(&command.workspace_id, command.version_record.clone())
            .map_err(UpdateDocumentError::from_version_store_error)
            .map_err(|error| {
                write_update_failure(product_logger, error);
                error
            })?;

        let current_record =
            command
                .current_record(current.metadata().clone())
                .map_err(|error| {
                    write_update_failure(product_logger, error);
                    error
                })?;
        let updated_title = current_record.metadata().title().as_str().to_string();
        let updated_path = current_record.path().as_str().to_string();
        document_repository
            .put_current(&command.workspace_id, current_record)
            .map_err(UpdateDocumentError::from_document_repository_error)
            .map_err(|error| {
                write_update_failure(product_logger, error);
                error
            })?;

        event_publisher.publish(DocumentChangeEvent::DocumentUpdated {
            workspace_id: command.workspace_id.as_str().to_string(),
            document_id: command.document_id.as_str().to_string(),
            version_id: command.version_id.as_str().to_string(),
            title: updated_title,
            path: updated_path,
        });
        product_logger.write_product(CreateDocumentProductEvent::DocumentUpdated {
            document_id: command.document_id.as_str().to_string(),
            version_id: command.version_id.as_str().to_string(),
        });

        Ok(UpdateDocumentOutput {
            version_id: command.version_id,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateDocumentError {
    InvalidDocumentInput,
    NotFound,
    VersionAlreadyExists,
    StorageUnavailable,
}

impl UpdateDocumentError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidDocumentInput => "document.invalid_input",
            Self::NotFound => "document.not_found",
            Self::VersionAlreadyExists => "document.version_already_exists",
            Self::StorageUnavailable => "document.storage_unavailable",
        }
    }

    fn from_document_repository_error(error: DocumentRepositoryError) -> Self {
        match error {
            DocumentRepositoryError::StorageUnavailable
            | DocumentRepositoryError::CorruptedMetadata
            | DocumentRepositoryError::MismatchedDocumentIdentity
            | DocumentRepositoryError::Conflict => Self::StorageUnavailable,
        }
    }

    fn from_version_store_error(error: VersionStoreError) -> Self {
        match error {
            VersionStoreError::Conflict => Self::VersionAlreadyExists,
            VersionStoreError::StorageUnavailable
            | VersionStoreError::CorruptedHistory
            | VersionStoreError::InvalidHistoryCursor
            | VersionStoreError::InvalidHistoryPageLimit
            | VersionStoreError::MismatchedVersionSnapshot => Self::StorageUnavailable,
        }
    }
}

struct UpdateDocumentCommand {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    body: DocumentBody,
    version_id: VersionId,
    version_record: VersionRecord,
}

impl UpdateDocumentCommand {
    fn from_input(
        input: UpdateDocumentInput,
        body_policy: DocumentBodyPolicy,
    ) -> Result<Self, UpdateDocumentError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| UpdateDocumentError::InvalidDocumentInput)?;
        let document_id = DocumentId::new(&input.document_id)
            .map_err(|_| UpdateDocumentError::InvalidDocumentInput)?;
        let body = DocumentBody::new(&input.body, body_policy)
            .map_err(|_| UpdateDocumentError::InvalidDocumentInput)?;
        let version_id = VersionId::new(&input.version_id)
            .map_err(|_| UpdateDocumentError::InvalidDocumentInput)?;
        let snapshot_ref = DocumentSnapshotRef::new(&input.snapshot_ref)
            .map_err(|_| UpdateDocumentError::InvalidDocumentInput)?;
        let author = VersionAuthor::new(&input.author)
            .map_err(|_| UpdateDocumentError::InvalidDocumentInput)?;
        let summary = VersionSummary::new(&input.summary)
            .map_err(|_| UpdateDocumentError::InvalidDocumentInput)?;
        let version_entry = VersionEntry::new(
            version_id.clone(),
            document_id.clone(),
            snapshot_ref.clone(),
            author,
            summary,
        )
        .map_err(|_| UpdateDocumentError::InvalidDocumentInput)?;
        let version_snapshot =
            VersionSnapshot::new(document_id.clone(), snapshot_ref, body.clone());
        let version_record = VersionRecord::new(version_entry, version_snapshot)
            .map_err(|_| UpdateDocumentError::InvalidDocumentInput)?;

        Ok(Self {
            workspace_id,
            document_id,
            body,
            version_id,
            version_record,
        })
    }

    fn current_record(
        &self,
        metadata: DocumentMetadata,
    ) -> Result<CurrentDocumentRecord, UpdateDocumentError> {
        let metadata = metadata
            .with_title(DocumentTitle::from_markdown_body(&self.body))
            .map_err(|_| UpdateDocumentError::InvalidDocumentInput)?;
        let snapshot = CurrentDocumentSnapshot::new(self.document_id.clone(), self.body.clone());
        CurrentDocumentRecord::new(metadata, snapshot)
            .map_err(|_| UpdateDocumentError::InvalidDocumentInput)
    }
}

fn write_update_failure(
    product_logger: &mut impl DocumentProductLogger,
    error: UpdateDocumentError,
) {
    product_logger.write_product(CreateDocumentProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameDocumentInput {
    workspace_id: String,
    document_id: String,
    version_id: String,
    title: String,
    path: String,
}

impl RenameDocumentInput {
    pub fn new(
        workspace_id: &str,
        document_id: &str,
        version_id: &str,
        title: &str,
        path: &str,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            version_id: version_id.to_string(),
            title: title.to_string(),
            path: path.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameDocumentOutput {
    document_id: DocumentId,
    title: DocumentTitle,
    path: DocumentPath,
}

impl RenameDocumentOutput {
    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn title(&self) -> &DocumentTitle {
        &self.title
    }

    pub fn path(&self) -> &DocumentPath {
        &self.path
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenameDocumentUsecase;

impl RenameDocumentUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: RenameDocumentInput,
        document_repository: &mut impl DocumentRepository,
        event_publisher: &mut impl DocumentChangeEventPublisher,
        product_logger: &mut impl DocumentProductLogger,
    ) -> Result<RenameDocumentOutput, RenameDocumentError> {
        let command = match RenameDocumentCommand::from_input(input) {
            Ok(command) => command,
            Err(error) => {
                write_rename_failure(product_logger, error);
                return Err(error);
            }
        };

        let current = document_repository
            .get_current_by_id(&command.workspace_id, &command.document_id)
            .map_err(RenameDocumentError::from_document_repository_error)
            .map_err(|error| {
                write_rename_failure(product_logger, error);
                error
            })?
            .ok_or_else(|| {
                let error = RenameDocumentError::NotFound;
                write_rename_failure(product_logger, error);
                error
            })?;

        let old_path = current.path().as_str().to_string();
        let new_path = command.path.as_str().to_string();
        let renamed_record =
            command
                .renamed_record(current.snapshot().clone())
                .map_err(|error| {
                    write_rename_failure(product_logger, error);
                    error
                })?;
        document_repository
            .put_current(&command.workspace_id, renamed_record)
            .map_err(RenameDocumentError::from_document_repository_error)
            .map_err(|error| {
                write_rename_failure(product_logger, error);
                error
            })?;

        event_publisher.publish(DocumentChangeEvent::DocumentRenamed {
            workspace_id: command.workspace_id.as_str().to_string(),
            document_id: command.document_id.as_str().to_string(),
            version_id: command.version_id.as_str().to_string(),
            title: command.title.as_str().to_string(),
            old_path,
            new_path,
        });
        product_logger.write_product(CreateDocumentProductEvent::DocumentRenamed {
            document_id: command.document_id.as_str().to_string(),
        });

        Ok(RenameDocumentOutput {
            document_id: command.document_id,
            title: command.title,
            path: command.path,
        })
    }
}

impl Default for RenameDocumentUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameDocumentError {
    InvalidDocumentInput,
    NotFound,
    StorageUnavailable,
}

impl RenameDocumentError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidDocumentInput => "document.invalid_input",
            Self::NotFound => "document.not_found",
            Self::StorageUnavailable => "document.storage_unavailable",
        }
    }

    fn from_document_repository_error(_error: DocumentRepositoryError) -> Self {
        Self::StorageUnavailable
    }
}

struct RenameDocumentCommand {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    version_id: VersionId,
    title: DocumentTitle,
    path: DocumentPath,
}

impl RenameDocumentCommand {
    fn from_input(input: RenameDocumentInput) -> Result<Self, RenameDocumentError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| RenameDocumentError::InvalidDocumentInput)?;
        let document_id = DocumentId::new(&input.document_id)
            .map_err(|_| RenameDocumentError::InvalidDocumentInput)?;
        let version_id = VersionId::new(&input.version_id)
            .map_err(|_| RenameDocumentError::InvalidDocumentInput)?;
        let title = DocumentTitle::new(&input.title)
            .map_err(|_| RenameDocumentError::InvalidDocumentInput)?;
        let path = DocumentPath::new(&input.path)
            .map_err(|_| RenameDocumentError::InvalidDocumentInput)?;

        Ok(Self {
            workspace_id,
            document_id,
            version_id,
            title,
            path,
        })
    }

    fn renamed_record(
        &self,
        snapshot: CurrentDocumentSnapshot,
    ) -> Result<CurrentDocumentRecord, RenameDocumentError> {
        let metadata = DocumentMetadata::new(
            self.document_id.clone(),
            self.title.clone(),
            self.path.clone(),
        )
        .map_err(|_| RenameDocumentError::InvalidDocumentInput)?;
        CurrentDocumentRecord::new(metadata, snapshot)
            .map_err(|_| RenameDocumentError::InvalidDocumentInput)
    }
}

fn write_rename_failure(
    product_logger: &mut impl DocumentProductLogger,
    error: RenameDocumentError,
) {
    product_logger.write_product(CreateDocumentProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteDocumentInput {
    workspace_id: String,
    document_id: String,
    version_id: String,
}

impl DeleteDocumentInput {
    pub fn new(workspace_id: &str, document_id: &str, version_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            version_id: version_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteDocumentOutput {
    document_id: DocumentId,
}

impl DeleteDocumentOutput {
    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteDocumentUsecase;

impl DeleteDocumentUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: DeleteDocumentInput,
        document_repository: &mut impl DocumentRepository,
        event_publisher: &mut impl DocumentChangeEventPublisher,
        product_logger: &mut impl DocumentProductLogger,
    ) -> Result<DeleteDocumentOutput, DeleteDocumentError> {
        let command = match DeleteDocumentCommand::from_input(input) {
            Ok(command) => command,
            Err(error) => {
                write_delete_failure(product_logger, error);
                return Err(error);
            }
        };

        let exists = document_repository
            .get_current_by_id(&command.workspace_id, &command.document_id)
            .map_err(DeleteDocumentError::from_document_repository_error)
            .map_err(|error| {
                write_delete_failure(product_logger, error);
                error
            })?
            .is_some();

        if !exists {
            let error = DeleteDocumentError::NotFound;
            write_delete_failure(product_logger, error);
            return Err(error);
        }

        document_repository
            .delete_current(&command.workspace_id, &command.document_id)
            .map_err(DeleteDocumentError::from_document_repository_error)
            .map_err(|error| {
                write_delete_failure(product_logger, error);
                error
            })?;

        event_publisher.publish(DocumentChangeEvent::DocumentDeleted {
            workspace_id: command.workspace_id.as_str().to_string(),
            document_id: command.document_id.as_str().to_string(),
            version_id: command.version_id.as_str().to_string(),
        });
        product_logger.write_product(CreateDocumentProductEvent::DocumentDeleted {
            document_id: command.document_id.as_str().to_string(),
        });

        Ok(DeleteDocumentOutput {
            document_id: command.document_id,
        })
    }
}

impl Default for DeleteDocumentUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteDocumentError {
    InvalidDocumentInput,
    NotFound,
    StorageUnavailable,
}

impl DeleteDocumentError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidDocumentInput => "document.invalid_input",
            Self::NotFound => "document.not_found",
            Self::StorageUnavailable => "document.storage_unavailable",
        }
    }

    fn from_document_repository_error(_error: DocumentRepositoryError) -> Self {
        Self::StorageUnavailable
    }
}

struct DeleteDocumentCommand {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    version_id: VersionId,
}

impl DeleteDocumentCommand {
    fn from_input(input: DeleteDocumentInput) -> Result<Self, DeleteDocumentError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| DeleteDocumentError::InvalidDocumentInput)?;
        let document_id = DocumentId::new(&input.document_id)
            .map_err(|_| DeleteDocumentError::InvalidDocumentInput)?;
        let version_id = VersionId::new(&input.version_id)
            .map_err(|_| DeleteDocumentError::InvalidDocumentInput)?;

        Ok(Self {
            workspace_id,
            document_id,
            version_id,
        })
    }
}

fn write_delete_failure(
    product_logger: &mut impl DocumentProductLogger,
    error: DeleteDocumentError,
) {
    product_logger.write_product(CreateDocumentProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachFileToDocumentInput {
    workspace_id: String,
    document_id: String,
    version_id: String,
    asset_id: String,
    file_name: String,
    media_type: String,
    bytes: Vec<u8>,
    label: String,
}

impl AttachFileToDocumentInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: &str,
        document_id: &str,
        version_id: &str,
        asset_id: &str,
        file_name: &str,
        media_type: &str,
        bytes: Vec<u8>,
        label: &str,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            version_id: version_id.to_string(),
            asset_id: asset_id.to_string(),
            file_name: file_name.to_string(),
            media_type: media_type.to_string(),
            bytes,
            label: label.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachFileToDocumentOutput {
    asset_id: AssetId,
}

impl AttachFileToDocumentOutput {
    pub fn asset_id(&self) -> &AssetId {
        &self.asset_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachFileToDocumentUsecase;

impl AttachFileToDocumentUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: AttachFileToDocumentInput,
        document_repository: &impl DocumentRepository,
        asset_store: &mut impl AssetStore,
        document_asset_repository: &mut impl DocumentAssetRepository,
        event_publisher: &mut impl DocumentChangeEventPublisher,
        product_logger: &mut impl DocumentProductLogger,
    ) -> Result<AttachFileToDocumentOutput, AttachFileToDocumentError> {
        let command = match AttachFileToDocumentCommand::from_input(input) {
            Ok(command) => command,
            Err(error) => {
                write_attach_failure(product_logger, error);
                return Err(error);
            }
        };

        let exists = document_repository
            .get_current_by_id(&command.workspace_id, &command.document_id)
            .map_err(AttachFileToDocumentError::from_document_repository_error)
            .map_err(|error| {
                write_attach_failure(product_logger, error);
                error
            })?
            .is_some();

        if !exists {
            let error = AttachFileToDocumentError::DocumentNotFound;
            write_attach_failure(product_logger, error);
            return Err(error);
        }

        asset_store
            .put_asset(&command.workspace_id, command.asset_record.clone())
            .map_err(AttachFileToDocumentError::from_asset_store_error)
            .map_err(|error| {
                write_attach_failure(product_logger, error);
                error
            })?;

        document_asset_repository
            .attach_asset(
                &command.workspace_id,
                &command.document_id,
                command.document_asset_record.clone(),
            )
            .map_err(AttachFileToDocumentError::from_document_asset_repository_error)
            .map_err(|error| {
                write_attach_failure(product_logger, error);
                error
            })?;

        event_publisher.publish(DocumentChangeEvent::DocumentAssetAttached {
            workspace_id: command.workspace_id.as_str().to_string(),
            document_id: command.document_id.as_str().to_string(),
            version_id: command.version_id.as_str().to_string(),
            asset_id: command.asset_id.as_str().to_string(),
        });
        product_logger.write_product(CreateDocumentProductEvent::DocumentAssetAttached {
            document_id: command.document_id.as_str().to_string(),
            asset_id: command.asset_id.as_str().to_string(),
        });

        Ok(AttachFileToDocumentOutput {
            asset_id: command.asset_id,
        })
    }
}

impl Default for AttachFileToDocumentUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachFileToDocumentError {
    InvalidInput,
    DocumentNotFound,
    StorageUnavailable,
}

impl AttachFileToDocumentError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document_asset.invalid_input",
            Self::DocumentNotFound => "document_asset.document_not_found",
            Self::StorageUnavailable => "document_asset.storage_unavailable",
        }
    }

    fn from_document_repository_error(_error: DocumentRepositoryError) -> Self {
        Self::StorageUnavailable
    }

    fn from_asset_store_error(error: AssetStoreError) -> Self {
        match error {
            AssetStoreError::MismatchedAssetObject | AssetStoreError::InvalidAssetObject => {
                Self::InvalidInput
            }
            AssetStoreError::StorageUnavailable
            | AssetStoreError::CorruptedMetadata
            | AssetStoreError::MissingObject
            | AssetStoreError::Conflict => Self::StorageUnavailable,
        }
    }

    fn from_document_asset_repository_error(error: DocumentAssetRepositoryError) -> Self {
        match error {
            DocumentAssetRepositoryError::MismatchedAssetReference
            | DocumentAssetRepositoryError::InvalidAssociation => Self::InvalidInput,
            DocumentAssetRepositoryError::StorageUnavailable
            | DocumentAssetRepositoryError::CorruptedMetadata
            | DocumentAssetRepositoryError::Conflict => Self::StorageUnavailable,
        }
    }
}

struct AttachFileToDocumentCommand {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    version_id: VersionId,
    asset_id: AssetId,
    asset_record: AssetRecord,
    document_asset_record: DocumentAssetRecord,
}

impl AttachFileToDocumentCommand {
    fn from_input(input: AttachFileToDocumentInput) -> Result<Self, AttachFileToDocumentError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| AttachFileToDocumentError::InvalidInput)?;
        let document_id = DocumentId::new(&input.document_id)
            .map_err(|_| AttachFileToDocumentError::InvalidInput)?;
        let version_id = VersionId::new(&input.version_id)
            .map_err(|_| AttachFileToDocumentError::InvalidInput)?;
        let asset_id = AssetId::from_sha256_hex(&input.asset_id)
            .map_err(|_| AttachFileToDocumentError::InvalidInput)?;
        let file_name = AssetFileName::new(&input.file_name)
            .map_err(|_| AttachFileToDocumentError::InvalidInput)?;
        let media_type = AssetMediaType::new(&input.media_type)
            .map_err(|_| AttachFileToDocumentError::InvalidInput)?;
        let byte_size = input.bytes.len() as u64;
        let metadata = AssetMetadata::new(asset_id.clone(), file_name, media_type, byte_size)
            .map_err(|_| AttachFileToDocumentError::InvalidInput)?;
        let object = AssetObject::new(asset_id.clone(), input.bytes)
            .map_err(|_| AttachFileToDocumentError::InvalidInput)?;
        let asset_record = AssetRecord::new(metadata.clone(), object)
            .map_err(|_| AttachFileToDocumentError::InvalidInput)?;
        let reference = AssetReference::new(asset_id.clone(), &input.label)
            .map_err(|_| AttachFileToDocumentError::InvalidInput)?;
        let document_asset_record = DocumentAssetRecord::new(reference, metadata)
            .map_err(|_| AttachFileToDocumentError::InvalidInput)?;

        Ok(Self {
            workspace_id,
            document_id,
            version_id,
            asset_id,
            asset_record,
            document_asset_record,
        })
    }
}

fn write_attach_failure(
    product_logger: &mut impl DocumentProductLogger,
    error: AttachFileToDocumentError,
) {
    product_logger.write_product(CreateDocumentProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListDocumentAssetsInput {
    workspace_id: String,
    document_id: String,
}

impl ListDocumentAssetsInput {
    pub fn new(workspace_id: &str, document_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListDocumentAssetsOutput {
    assets: Vec<DocumentAssetRecord>,
}

impl ListDocumentAssetsOutput {
    pub fn assets(&self) -> &[DocumentAssetRecord] {
        &self.assets
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListDocumentAssetsUsecase;

impl ListDocumentAssetsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ListDocumentAssetsInput,
        document_repository: &impl DocumentRepository,
        document_asset_repository: &impl DocumentAssetRepository,
    ) -> Result<ListDocumentAssetsOutput, ListDocumentAssetsError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ListDocumentAssetsError::InvalidInput)?;
        let document_id = DocumentId::new(&input.document_id)
            .map_err(|_| ListDocumentAssetsError::InvalidInput)?;

        let exists = document_repository
            .get_current_by_id(&workspace_id, &document_id)
            .map_err(ListDocumentAssetsError::from_document_repository_error)?
            .is_some();

        if !exists {
            return Err(ListDocumentAssetsError::NotFound);
        }

        let assets = document_asset_repository
            .list_assets(&workspace_id, &document_id)
            .map_err(ListDocumentAssetsError::from_document_asset_repository_error)?;

        Ok(ListDocumentAssetsOutput { assets })
    }
}

impl Default for ListDocumentAssetsUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListDocumentAssetsError {
    InvalidInput,
    NotFound,
    StorageUnavailable,
}

impl ListDocumentAssetsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document_asset.invalid_input",
            Self::NotFound => "document_asset.document_not_found",
            Self::StorageUnavailable => "document_asset.storage_unavailable",
        }
    }

    fn from_document_repository_error(_error: DocumentRepositoryError) -> Self {
        Self::StorageUnavailable
    }

    fn from_document_asset_repository_error(_error: DocumentAssetRepositoryError) -> Self {
        Self::StorageUnavailable
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateDocumentError {
    InvalidDocumentInput,
    DocumentAlreadyExists,
    VersionAlreadyExists,
    StorageUnavailable,
}

impl CreateDocumentError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidDocumentInput => "document.invalid_input",
            Self::DocumentAlreadyExists => "document.already_exists",
            Self::VersionAlreadyExists => "document.version_already_exists",
            Self::StorageUnavailable => "document.storage_unavailable",
        }
    }

    fn from_document_repository_error(error: DocumentRepositoryError) -> Self {
        match error {
            DocumentRepositoryError::Conflict => Self::DocumentAlreadyExists,
            DocumentRepositoryError::StorageUnavailable
            | DocumentRepositoryError::CorruptedMetadata
            | DocumentRepositoryError::MismatchedDocumentIdentity => Self::StorageUnavailable,
        }
    }

    fn from_version_store_error(error: VersionStoreError) -> Self {
        match error {
            VersionStoreError::Conflict => Self::VersionAlreadyExists,
            VersionStoreError::StorageUnavailable
            | VersionStoreError::CorruptedHistory
            | VersionStoreError::InvalidHistoryCursor
            | VersionStoreError::InvalidHistoryPageLimit
            | VersionStoreError::MismatchedVersionSnapshot => Self::StorageUnavailable,
        }
    }
}

struct CreateDocumentCommand {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    version_id: VersionId,
    current_record: CurrentDocumentRecord,
    version_record: VersionRecord,
}

impl CreateDocumentCommand {
    fn from_input(
        input: CreateDocumentInput,
        body_policy: DocumentBodyPolicy,
    ) -> Result<Self, CreateDocumentError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| CreateDocumentError::InvalidDocumentInput)?;
        let document_id = DocumentId::new(&input.document_id)
            .map_err(|_| CreateDocumentError::InvalidDocumentInput)?;
        let path = DocumentPath::new(&input.path)
            .map_err(|_| CreateDocumentError::InvalidDocumentInput)?;
        let body = DocumentBody::new(&input.body, body_policy)
            .map_err(|_| CreateDocumentError::InvalidDocumentInput)?;
        let title = DocumentTitle::from_markdown_body(&body);
        let version_id = VersionId::new(&input.version_id)
            .map_err(|_| CreateDocumentError::InvalidDocumentInput)?;
        let snapshot_ref = DocumentSnapshotRef::new(&input.snapshot_ref)
            .map_err(|_| CreateDocumentError::InvalidDocumentInput)?;
        let author = VersionAuthor::new(&input.author)
            .map_err(|_| CreateDocumentError::InvalidDocumentInput)?;
        let summary = VersionSummary::new(&input.summary)
            .map_err(|_| CreateDocumentError::InvalidDocumentInput)?;

        let metadata = DocumentMetadata::new(document_id.clone(), title, path)
            .map_err(|_| CreateDocumentError::InvalidDocumentInput)?;
        let current_snapshot = CurrentDocumentSnapshot::new(document_id.clone(), body.clone());
        let current_record = CurrentDocumentRecord::new(metadata, current_snapshot)
            .map_err(|_| CreateDocumentError::InvalidDocumentInput)?;
        let version_entry = VersionEntry::new(
            version_id.clone(),
            document_id.clone(),
            snapshot_ref.clone(),
            author,
            summary,
        )
        .map_err(|_| CreateDocumentError::InvalidDocumentInput)?;
        let version_snapshot = VersionSnapshot::new(document_id.clone(), snapshot_ref, body);
        let version_record = VersionRecord::new(version_entry, version_snapshot)
            .map_err(|_| CreateDocumentError::InvalidDocumentInput)?;

        Ok(Self {
            workspace_id,
            document_id,
            version_id,
            current_record,
            version_record,
        })
    }
}
