use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_repository::{DocumentRepository, DocumentRepositoryError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportMarkdownInput {
    workspace_id: String,
    document_ids: Vec<String>,
}

impl ExportMarkdownInput {
    pub fn new(workspace_id: &str, document_ids: Vec<&str>) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            document_ids: document_ids.into_iter().map(ToString::to_string).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportMarkdownOutput {
    final_state: ExportMarkdownState,
    files: Vec<ExportedMarkdownFile>,
    failed_items: Vec<ExportFailedItem>,
}

impl ExportMarkdownOutput {
    pub fn final_state(&self) -> ExportMarkdownState {
        self.final_state
    }

    pub fn files(&self) -> &[ExportedMarkdownFile] {
        &self.files
    }

    pub fn failed_items(&self) -> &[ExportFailedItem] {
        &self.failed_items
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedMarkdownFile {
    path: String,
    content: String,
}

impl ExportedMarkdownFile {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportFailedItem {
    document_id: String,
    error_code: &'static str,
}

impl ExportFailedItem {
    pub fn document_id(&self) -> &str {
        &self.document_id
    }

    pub fn error_code(&self) -> &'static str {
        self.error_code
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportMarkdownState {
    Requested,
    ReadingCurrent,
    Completed,
    PartiallyFailed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportMarkdownUsecase;

impl ExportMarkdownUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ExportMarkdownInput,
        document_repository: &impl DocumentRepository,
    ) -> Result<ExportMarkdownOutput, ExportMarkdownError> {
        let workspace_id =
            WorkspaceId::new(&input.workspace_id).map_err(|_| ExportMarkdownError::InvalidInput)?;
        let mut files = Vec::new();
        let mut failed_items = Vec::new();

        for document_id in input.document_ids {
            match export_document(&workspace_id, &document_id, document_repository) {
                Ok(file) => files.push(file),
                Err(error) => failed_items.push(ExportFailedItem {
                    document_id,
                    error_code: error.code(),
                }),
            }
        }

        let final_state = match (files.is_empty(), failed_items.is_empty()) {
            (true, _) => ExportMarkdownState::Failed,
            (false, true) => ExportMarkdownState::Completed,
            (false, false) => ExportMarkdownState::PartiallyFailed,
        };

        Ok(ExportMarkdownOutput {
            final_state,
            files,
            failed_items,
        })
    }
}

impl Default for ExportMarkdownUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportMarkdownError {
    InvalidInput,
}

impl ExportMarkdownError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "markdown_export.invalid_input",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportItemError {
    InvalidInput,
    NotFound,
    StorageUnavailable,
}

impl ExportItemError {
    const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "markdown_export.invalid_item",
            Self::NotFound => "markdown_export.not_found",
            Self::StorageUnavailable => "markdown_export.storage_unavailable",
        }
    }

    fn from_document_repository_error(_error: DocumentRepositoryError) -> Self {
        Self::StorageUnavailable
    }
}

fn export_document(
    workspace_id: &WorkspaceId,
    document_id: &str,
    document_repository: &impl DocumentRepository,
) -> Result<ExportedMarkdownFile, ExportItemError> {
    let document_id = DocumentId::new(document_id).map_err(|_| ExportItemError::InvalidInput)?;
    let record = document_repository
        .get_current_by_id(workspace_id, &document_id)
        .map_err(ExportItemError::from_document_repository_error)?
        .ok_or(ExportItemError::NotFound)?;
    Ok(ExportedMarkdownFile {
        path: record.path().as_str().to_string(),
        content: record.body().as_str().to_string(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPdfOutput {
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportPdfUsecase;

impl ExportPdfUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub const fn execute(&self) -> ExportPdfOutput {
        ExportPdfOutput::Unsupported
    }
}

impl Default for ExportPdfUsecase {
    fn default() -> Self {
        Self::new()
    }
}
