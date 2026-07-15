use std::fmt;

use cabinet_domain::document::DocumentBodyPolicy;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::document_repository::DocumentRepository;
use cabinet_ports::version_store::VersionStore;
use cabinet_usecases::document::{DocumentChangeEventPublisher, DocumentProductLogger};
use cabinet_usecases::guarded_authoring::{
    GuardedAuthoringError, GuardedAuthoringUsecase, GuardedCreateDocumentInput,
    GuardedGetCurrentDocumentInput, GuardedUpdateDocumentInput,
};

#[derive(Clone, PartialEq, Eq)]
pub enum DocumentAuthoringCommandRequest {
    Create {
        workspace_id: String,
        document_id: String,
        path: String,
        body: String,
        version_id: String,
        snapshot_ref: String,
        author: String,
        summary: String,
    },
    Update {
        workspace_id: String,
        document_id: String,
        body: String,
        expected_version_id: String,
        version_id: String,
        snapshot_ref: String,
        author: String,
        summary: String,
    },
    GetCurrent {
        workspace_id: String,
        document_id: String,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub enum DocumentAuthoringCommandResult {
    Created {
        document_id: String,
        current_version_id: String,
    },
    Updated {
        document_id: String,
        current_version_id: String,
    },
    Current {
        document_id: String,
        title: String,
        path: String,
        body: String,
        current_version_id: String,
    },
}

impl fmt::Debug for DocumentAuthoringCommandResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created {
                document_id,
                current_version_id,
            } => formatter
                .debug_struct("Created")
                .field("document_id", document_id)
                .field("current_version_id", current_version_id)
                .finish(),
            Self::Updated {
                document_id,
                current_version_id,
            } => formatter
                .debug_struct("Updated")
                .field("document_id", document_id)
                .field("current_version_id", current_version_id)
                .finish(),
            Self::Current {
                document_id,
                current_version_id,
                ..
            } => formatter
                .debug_struct("Current")
                .field("document_id", document_id)
                .field("current_version_id", current_version_id)
                .field("content", &"[REDACTED]")
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentAuthoringCommandFailure {
    pub error_code: &'static str,
    pub retryable: bool,
    pub repair_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentAuthoringCommandExecutor {
    body_policy: DocumentBodyPolicy,
}

impl DocumentAuthoringCommandExecutor {
    pub const fn new(body_policy: DocumentBodyPolicy) -> Self {
        Self { body_policy }
    }

    pub fn execute(
        &self,
        request: DocumentAuthoringCommandRequest,
        document_repository: &mut impl DocumentRepository,
        version_store: &mut impl VersionStore,
        pointer: &mut impl CurrentDocumentVersionPointerPort,
        event_publisher: &mut impl DocumentChangeEventPublisher,
        product_logger: &mut impl DocumentProductLogger,
    ) -> Result<DocumentAuthoringCommandResult, DocumentAuthoringCommandFailure> {
        let usecase = GuardedAuthoringUsecase::new(self.body_policy);
        match request {
            DocumentAuthoringCommandRequest::Create {
                workspace_id,
                document_id,
                path,
                body,
                version_id,
                snapshot_ref,
                author,
                summary,
            } => usecase
                .create(
                    GuardedCreateDocumentInput::new(
                        &workspace_id,
                        &document_id,
                        &path,
                        &body,
                        &version_id,
                        &snapshot_ref,
                        &author,
                        &summary,
                    ),
                    document_repository,
                    version_store,
                    pointer,
                    event_publisher,
                    product_logger,
                )
                .map(|output| DocumentAuthoringCommandResult::Created {
                    document_id: output.document_id().to_string(),
                    current_version_id: output.current_version_id().to_string(),
                })
                .map_err(map_error),
            DocumentAuthoringCommandRequest::Update {
                workspace_id,
                document_id,
                body,
                expected_version_id,
                version_id,
                snapshot_ref,
                author,
                summary,
            } => {
                let result_document_id = document_id.clone();
                usecase
                    .update(
                        GuardedUpdateDocumentInput::new(
                            &workspace_id,
                            &document_id,
                            &body,
                            &expected_version_id,
                            &version_id,
                            &snapshot_ref,
                            &author,
                            &summary,
                        ),
                        document_repository,
                        version_store,
                        pointer,
                        event_publisher,
                        product_logger,
                    )
                    .map(|output| DocumentAuthoringCommandResult::Updated {
                        document_id: result_document_id,
                        current_version_id: output.current_version_id().to_string(),
                    })
                    .map_err(map_error)
            }
            DocumentAuthoringCommandRequest::GetCurrent {
                workspace_id,
                document_id,
            } => usecase
                .get_current(
                    GuardedGetCurrentDocumentInput::new(&workspace_id, &document_id),
                    document_repository,
                    pointer,
                )
                .map(|output| DocumentAuthoringCommandResult::Current {
                    document_id: output.record().document_id().as_str().to_string(),
                    title: output.record().metadata().title().as_str().to_string(),
                    path: output.record().path().as_str().to_string(),
                    body: output.record().body().as_str().to_string(),
                    current_version_id: output.current_version_id().to_string(),
                })
                .map_err(map_error),
        }
    }
}

const fn map_error(error: GuardedAuthoringError) -> DocumentAuthoringCommandFailure {
    match error {
        GuardedAuthoringError::InvalidInput => {
            failure("DOCUMENT_AUTHORING_INVALID_INPUT", false, false)
        }
        GuardedAuthoringError::NotFound => failure("DOCUMENT_AUTHORING_NOT_FOUND", false, false),
        GuardedAuthoringError::VersionConflict => {
            failure("DOCUMENT_AUTHORING_VERSION_CONFLICT", false, false)
        }
        GuardedAuthoringError::PointerMissing => {
            failure("DOCUMENT_AUTHORING_POINTER_MISSING", true, true)
        }
        GuardedAuthoringError::PointerUpdateFailed => {
            failure("DOCUMENT_AUTHORING_POINTER_UPDATE_FAILED", true, true)
        }
        GuardedAuthoringError::StorageUnavailable => {
            failure("DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE", true, false)
        }
    }
}

const fn failure(
    error_code: &'static str,
    retryable: bool,
    repair_required: bool,
) -> DocumentAuthoringCommandFailure {
    DocumentAuthoringCommandFailure {
        error_code,
        retryable,
        repair_required,
    }
}
