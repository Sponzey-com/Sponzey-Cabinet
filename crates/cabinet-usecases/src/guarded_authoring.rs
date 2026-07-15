use cabinet_domain::document::{DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::document_repository::{CurrentDocumentRecord, DocumentRepository};
use cabinet_ports::version_store::VersionStore;

use crate::document::{
    CreateDocumentError, CreateDocumentInput, CreateDocumentUsecase, DocumentChangeEventPublisher,
    DocumentProductLogger, GetCurrentDocumentError, GetCurrentDocumentInput,
    GetCurrentDocumentUsecase, UpdateDocumentError, UpdateDocumentInput, UpdateDocumentUsecase,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardedCreateDocumentInput {
    workspace_id: String,
    document_id: String,
    path: String,
    body: String,
    version_id: String,
    snapshot_ref: String,
    author: String,
    summary: String,
}

impl GuardedCreateDocumentInput {
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
pub struct GuardedUpdateDocumentInput {
    workspace_id: String,
    document_id: String,
    body: String,
    expected_version_id: String,
    version_id: String,
    snapshot_ref: String,
    author: String,
    summary: String,
}

impl GuardedUpdateDocumentInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: &str,
        document_id: &str,
        body: &str,
        expected_version_id: &str,
        version_id: &str,
        snapshot_ref: &str,
        author: &str,
        summary: &str,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            body: body.to_string(),
            expected_version_id: expected_version_id.to_string(),
            version_id: version_id.to_string(),
            snapshot_ref: snapshot_ref.to_string(),
            author: author.to_string(),
            summary: summary.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardedGetCurrentDocumentInput {
    workspace_id: String,
    document_id: String,
}

impl GuardedGetCurrentDocumentInput {
    pub fn new(workspace_id: &str, document_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardedCreateDocumentOutput {
    document_id: DocumentId,
    current_version_id: VersionId,
}

impl GuardedCreateDocumentOutput {
    pub fn document_id(&self) -> &str {
        self.document_id.as_str()
    }

    pub fn current_version_id(&self) -> &str {
        self.current_version_id.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardedUpdateDocumentOutput {
    current_version_id: VersionId,
}

impl GuardedUpdateDocumentOutput {
    pub fn current_version_id(&self) -> &str {
        self.current_version_id.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardedGetCurrentDocumentOutput {
    record: CurrentDocumentRecord,
    current_version_id: VersionId,
}

impl GuardedGetCurrentDocumentOutput {
    pub fn record(&self) -> &CurrentDocumentRecord {
        &self.record
    }

    pub fn current_version_id(&self) -> &str {
        self.current_version_id.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardedAuthoringError {
    InvalidInput,
    NotFound,
    VersionConflict,
    PointerMissing,
    PointerUpdateFailed,
    StorageUnavailable,
}

impl GuardedAuthoringError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "guarded_authoring.invalid_input",
            Self::NotFound => "guarded_authoring.not_found",
            Self::VersionConflict => "guarded_authoring.version_conflict",
            Self::PointerMissing => "guarded_authoring.pointer_missing",
            Self::PointerUpdateFailed => "guarded_authoring.pointer_update_failed",
            Self::StorageUnavailable => "guarded_authoring.storage_unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardedAuthoringUsecase {
    body_policy: DocumentBodyPolicy,
}

impl GuardedAuthoringUsecase {
    pub const fn new(body_policy: DocumentBodyPolicy) -> Self {
        Self { body_policy }
    }

    pub fn create(
        &self,
        input: GuardedCreateDocumentInput,
        document_repository: &mut impl DocumentRepository,
        version_store: &mut impl VersionStore,
        pointer: &mut impl CurrentDocumentVersionPointerPort,
        event_publisher: &mut impl DocumentChangeEventPublisher,
        product_logger: &mut impl DocumentProductLogger,
    ) -> Result<GuardedCreateDocumentOutput, GuardedAuthoringError> {
        let workspace_id = parse_workspace_id(&input.workspace_id)?;
        let document_id = parse_document_id(&input.document_id)?;
        let version_id = parse_version_id(&input.version_id)?;

        if pointer
            .load_current_version(&workspace_id, &document_id)
            .map_err(map_pointer_read_error)?
            .is_some()
        {
            return Err(GuardedAuthoringError::VersionConflict);
        }

        let created = CreateDocumentUsecase::new(self.body_policy)
            .execute(
                CreateDocumentInput::new(
                    &input.workspace_id,
                    &input.document_id,
                    &input.path,
                    &input.body,
                    &input.version_id,
                    &input.snapshot_ref,
                    &input.author,
                    &input.summary,
                ),
                document_repository,
                version_store,
                event_publisher,
                product_logger,
            )
            .map_err(map_create_error)?;

        pointer
            .compare_and_set_current_version(&workspace_id, &document_id, None, version_id.clone())
            .map_err(|_| GuardedAuthoringError::PointerUpdateFailed)?;

        Ok(GuardedCreateDocumentOutput {
            document_id: created.document_id().clone(),
            current_version_id: version_id,
        })
    }

    pub fn update(
        &self,
        input: GuardedUpdateDocumentInput,
        document_repository: &mut impl DocumentRepository,
        version_store: &mut impl VersionStore,
        pointer: &mut impl CurrentDocumentVersionPointerPort,
        event_publisher: &mut impl DocumentChangeEventPublisher,
        product_logger: &mut impl DocumentProductLogger,
    ) -> Result<GuardedUpdateDocumentOutput, GuardedAuthoringError> {
        let workspace_id = parse_workspace_id(&input.workspace_id)?;
        let document_id = parse_document_id(&input.document_id)?;
        let expected_version_id = parse_version_id(&input.expected_version_id)?;
        let version_id = parse_version_id(&input.version_id)?;
        let current_version = pointer
            .load_current_version(&workspace_id, &document_id)
            .map_err(map_pointer_read_error)?
            .ok_or(GuardedAuthoringError::PointerMissing)?;

        if current_version != expected_version_id {
            return Err(GuardedAuthoringError::VersionConflict);
        }

        UpdateDocumentUsecase::new(self.body_policy)
            .execute(
                UpdateDocumentInput::new(
                    &input.workspace_id,
                    &input.document_id,
                    &input.body,
                    &input.version_id,
                    &input.snapshot_ref,
                    &input.author,
                    &input.summary,
                ),
                document_repository,
                version_store,
                event_publisher,
                product_logger,
            )
            .map_err(map_update_error)?;

        pointer
            .compare_and_set_current_version(
                &workspace_id,
                &document_id,
                Some(&expected_version_id),
                version_id.clone(),
            )
            .map_err(|_| GuardedAuthoringError::PointerUpdateFailed)?;

        Ok(GuardedUpdateDocumentOutput {
            current_version_id: version_id,
        })
    }

    pub fn get_current(
        &self,
        input: GuardedGetCurrentDocumentInput,
        document_repository: &impl DocumentRepository,
        pointer: &impl CurrentDocumentVersionPointerPort,
    ) -> Result<GuardedGetCurrentDocumentOutput, GuardedAuthoringError> {
        let workspace_id = parse_workspace_id(&input.workspace_id)?;
        let document_id = parse_document_id(&input.document_id)?;
        let record = GetCurrentDocumentUsecase::new()
            .execute(
                GetCurrentDocumentInput::by_id(&input.workspace_id, &input.document_id),
                document_repository,
            )
            .map_err(map_get_current_error)?
            .record()
            .clone();
        let current_version_id = pointer
            .load_current_version(&workspace_id, &document_id)
            .map_err(map_pointer_read_error)?
            .ok_or(GuardedAuthoringError::PointerMissing)?;

        Ok(GuardedGetCurrentDocumentOutput {
            record,
            current_version_id,
        })
    }
}

fn parse_workspace_id(value: &str) -> Result<WorkspaceId, GuardedAuthoringError> {
    WorkspaceId::new(value).map_err(|_| GuardedAuthoringError::InvalidInput)
}

fn parse_document_id(value: &str) -> Result<DocumentId, GuardedAuthoringError> {
    DocumentId::new(value).map_err(|_| GuardedAuthoringError::InvalidInput)
}

fn parse_version_id(value: &str) -> Result<VersionId, GuardedAuthoringError> {
    VersionId::new(value).map_err(|_| GuardedAuthoringError::InvalidInput)
}

fn map_pointer_read_error(error: CurrentDocumentVersionPointerError) -> GuardedAuthoringError {
    match error {
        CurrentDocumentVersionPointerError::Conflict => GuardedAuthoringError::VersionConflict,
        CurrentDocumentVersionPointerError::StorageUnavailable
        | CurrentDocumentVersionPointerError::CorruptedPointer => {
            GuardedAuthoringError::StorageUnavailable
        }
    }
}

fn map_create_error(error: CreateDocumentError) -> GuardedAuthoringError {
    match error {
        CreateDocumentError::InvalidDocumentInput => GuardedAuthoringError::InvalidInput,
        CreateDocumentError::DocumentAlreadyExists | CreateDocumentError::VersionAlreadyExists => {
            GuardedAuthoringError::VersionConflict
        }
        CreateDocumentError::StorageUnavailable => GuardedAuthoringError::StorageUnavailable,
    }
}

fn map_update_error(error: UpdateDocumentError) -> GuardedAuthoringError {
    match error {
        UpdateDocumentError::InvalidDocumentInput => GuardedAuthoringError::InvalidInput,
        UpdateDocumentError::NotFound => GuardedAuthoringError::NotFound,
        UpdateDocumentError::VersionAlreadyExists => GuardedAuthoringError::VersionConflict,
        UpdateDocumentError::StorageUnavailable => GuardedAuthoringError::StorageUnavailable,
    }
}

fn map_get_current_error(error: GetCurrentDocumentError) -> GuardedAuthoringError {
    match error {
        GetCurrentDocumentError::InvalidInput => GuardedAuthoringError::InvalidInput,
        GetCurrentDocumentError::NotFound => GuardedAuthoringError::NotFound,
        GetCurrentDocumentError::StorageUnavailable => GuardedAuthoringError::StorageUnavailable,
    }
}
