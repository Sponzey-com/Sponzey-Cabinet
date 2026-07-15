use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;

const HOME_LIMIT_MAX: u16 = 100;
const TAG_LABEL_MAX: usize = 64;
const CHANGE_SUMMARY_MAX: usize = 160;
const UNFINISHED_LABEL_MAX: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceHomeBackupStatus {
    NeverCreated,
    Fresh,
    Stale,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceHomeHealthStatus {
    Healthy,
    Degraded,
    ReadOnlyRecovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceHomeProjectionLimits {
    recent_documents: u16,
    favorites: u16,
    tags: u16,
    recent_changes: u16,
    unfinished_items: u16,
}

impl WorkspaceHomeProjectionLimits {
    pub fn new(
        recent_documents: u16,
        favorites: u16,
        tags: u16,
        recent_changes: u16,
        unfinished_items: u16,
    ) -> Result<Self, WorkspaceHomeProjectionError> {
        let limits = [
            recent_documents,
            favorites,
            tags,
            recent_changes,
            unfinished_items,
        ];
        if limits
            .iter()
            .any(|limit| *limit == 0 || *limit > HOME_LIMIT_MAX)
        {
            return Err(WorkspaceHomeProjectionError::InvalidLimit);
        }
        Ok(Self {
            recent_documents,
            favorites,
            tags,
            recent_changes,
            unfinished_items,
        })
    }

    pub const fn recent_documents(self) -> u16 {
        self.recent_documents
    }

    pub const fn favorites(self) -> u16 {
        self.favorites
    }

    pub const fn tags(self) -> u16 {
        self.tags
    }

    pub const fn recent_changes(self) -> u16 {
        self.recent_changes
    }

    pub const fn unfinished_items(self) -> u16 {
        self.unfinished_items
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceHomeDocumentProjection {
    document_id: DocumentId,
    title: DocumentTitle,
    path: DocumentPath,
}

impl WorkspaceHomeDocumentProjection {
    pub fn new(document_id: DocumentId, title: DocumentTitle, path: DocumentPath) -> Self {
        Self {
            document_id,
            title,
            path,
        }
    }

    pub fn document_id(&self) -> &str {
        self.document_id.as_str()
    }

    pub fn title(&self) -> &str {
        self.title.as_str()
    }

    pub fn path(&self) -> &str {
        self.path.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceHomeTagProjection {
    label: String,
    document_count: u32,
}

impl WorkspaceHomeTagProjection {
    pub fn new(label: &str, document_count: u32) -> Result<Self, WorkspaceHomeProjectionError> {
        Ok(Self {
            label: validate_text(label, TAG_LABEL_MAX)?,
            document_count,
        })
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn document_count(&self) -> u32 {
        self.document_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceHomeChangeProjection {
    document_id: DocumentId,
    summary: String,
}

impl WorkspaceHomeChangeProjection {
    pub fn new(
        document_id: DocumentId,
        summary: &str,
    ) -> Result<Self, WorkspaceHomeProjectionError> {
        Ok(Self {
            document_id,
            summary: validate_text(summary, CHANGE_SUMMARY_MAX)?,
        })
    }

    pub fn document_id(&self) -> &str {
        self.document_id.as_str()
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceHomeUnfinishedProjection {
    document_id: DocumentId,
    label: String,
}

impl WorkspaceHomeUnfinishedProjection {
    pub fn new(document_id: DocumentId, label: &str) -> Result<Self, WorkspaceHomeProjectionError> {
        Ok(Self {
            document_id,
            label: validate_text(label, UNFINISHED_LABEL_MAX)?,
        })
    }

    pub fn document_id(&self) -> &str {
        self.document_id.as_str()
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceHomeProjection {
    recent_documents: Vec<WorkspaceHomeDocumentProjection>,
    favorites: Vec<WorkspaceHomeDocumentProjection>,
    tags: Vec<WorkspaceHomeTagProjection>,
    recent_changes: Vec<WorkspaceHomeChangeProjection>,
    unfinished_items: Vec<WorkspaceHomeUnfinishedProjection>,
    backup_status: WorkspaceHomeBackupStatus,
    health_status: WorkspaceHomeHealthStatus,
}

impl WorkspaceHomeProjection {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        recent_documents: Vec<WorkspaceHomeDocumentProjection>,
        favorites: Vec<WorkspaceHomeDocumentProjection>,
        tags: Vec<WorkspaceHomeTagProjection>,
        recent_changes: Vec<WorkspaceHomeChangeProjection>,
        unfinished_items: Vec<WorkspaceHomeUnfinishedProjection>,
        backup_status: WorkspaceHomeBackupStatus,
        health_status: WorkspaceHomeHealthStatus,
    ) -> Self {
        Self {
            recent_documents,
            favorites,
            tags,
            recent_changes,
            unfinished_items,
            backup_status,
            health_status,
        }
    }

    pub fn empty(
        backup_status: WorkspaceHomeBackupStatus,
        health_status: WorkspaceHomeHealthStatus,
    ) -> Self {
        Self::new(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            backup_status,
            health_status,
        )
    }

    pub fn recent_documents(&self) -> &[WorkspaceHomeDocumentProjection] {
        &self.recent_documents
    }

    pub fn favorites(&self) -> &[WorkspaceHomeDocumentProjection] {
        &self.favorites
    }

    pub fn tags(&self) -> &[WorkspaceHomeTagProjection] {
        &self.tags
    }

    pub fn recent_changes(&self) -> &[WorkspaceHomeChangeProjection] {
        &self.recent_changes
    }

    pub fn unfinished_items(&self) -> &[WorkspaceHomeUnfinishedProjection] {
        &self.unfinished_items
    }

    pub const fn backup_status(&self) -> WorkspaceHomeBackupStatus {
        self.backup_status
    }

    pub const fn health_status(&self) -> WorkspaceHomeHealthStatus {
        self.health_status
    }

    pub fn total_item_count(&self) -> usize {
        self.recent_documents.len()
            + self.favorites.len()
            + self.tags.len()
            + self.recent_changes.len()
            + self.unfinished_items.len()
    }
}

pub trait WorkspaceHomeProjectionPort {
    fn load_workspace_home(
        &self,
        workspace_id: &WorkspaceId,
        limits: WorkspaceHomeProjectionLimits,
    ) -> Result<WorkspaceHomeProjection, WorkspaceHomeProjectionError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceHomeDocumentMutation {
    UpsertRecent {
        document: WorkspaceHomeDocumentProjection,
        change_summary: String,
    },
    RemoveDocument {
        document_id: DocumentId,
    },
}

pub trait WorkspaceHomeDocumentMutationPort {
    fn apply_document_mutation(
        &mut self,
        workspace_id: &WorkspaceId,
        mutation: WorkspaceHomeDocumentMutation,
        capacity: u16,
    ) -> Result<(), WorkspaceHomeProjectionError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceHomeProjectionError {
    InvalidLimit,
    InvalidProjectionText,
    StorageUnavailable,
    CorruptedProjection,
}

impl WorkspaceHomeProjectionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimit => "workspace_home_projection.invalid_limit",
            Self::InvalidProjectionText => "workspace_home_projection.invalid_text",
            Self::StorageUnavailable => "workspace_home_projection.storage_unavailable",
            Self::CorruptedProjection => "workspace_home_projection.corrupted",
        }
    }
}

fn validate_text(value: &str, max_len: usize) -> Result<String, WorkspaceHomeProjectionError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > max_len
        || trimmed.chars().any(char::is_control)
    {
        return Err(WorkspaceHomeProjectionError::InvalidProjectionText);
    }
    Ok(trimmed.to_string())
}
