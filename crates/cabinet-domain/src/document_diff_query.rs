use crate::document::DocumentId;
use crate::version::VersionId;
use crate::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentDiffQueryTarget {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    kind: DocumentDiffQueryKind,
}

impl DocumentDiffQueryTarget {
    pub fn current_to_version(
        workspace_id: &str,
        document_id: &str,
        version_id: &str,
    ) -> Result<Self, DocumentDiffQueryTargetError> {
        Ok(Self {
            workspace_id: parse_workspace_id(workspace_id)?,
            document_id: parse_document_id(document_id)?,
            kind: DocumentDiffQueryKind::CurrentToVersion {
                version_id: parse_version_id(version_id)?,
            },
        })
    }

    pub fn versions(
        workspace_id: &str,
        document_id: &str,
        left_version_id: &str,
        right_version_id: &str,
    ) -> Result<Self, DocumentDiffQueryTargetError> {
        Ok(Self {
            workspace_id: parse_workspace_id(workspace_id)?,
            document_id: parse_document_id(document_id)?,
            kind: DocumentDiffQueryKind::Versions {
                left_version_id: parse_version_id(left_version_id)?,
                right_version_id: parse_version_id(right_version_id)?,
            },
        })
    }

    pub const fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub const fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub const fn kind(&self) -> &DocumentDiffQueryKind {
        &self.kind
    }

    pub const fn current_version_id(&self) -> Option<&VersionId> {
        match &self.kind {
            DocumentDiffQueryKind::CurrentToVersion { version_id } => Some(version_id),
            DocumentDiffQueryKind::Versions { .. } => None,
        }
    }

    pub const fn version_pair(&self) -> Option<(&VersionId, &VersionId)> {
        match &self.kind {
            DocumentDiffQueryKind::CurrentToVersion { .. } => None,
            DocumentDiffQueryKind::Versions {
                left_version_id,
                right_version_id,
            } => Some((left_version_id, right_version_id)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentDiffQueryKind {
    CurrentToVersion {
        version_id: VersionId,
    },
    Versions {
        left_version_id: VersionId,
        right_version_id: VersionId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentDiffQueryTargetError {
    InvalidTarget,
}

impl DocumentDiffQueryTargetError {
    pub const fn code(self) -> &'static str {
        "document_diff_query.invalid_target"
    }
}

fn parse_workspace_id(value: &str) -> Result<WorkspaceId, DocumentDiffQueryTargetError> {
    WorkspaceId::new(value).map_err(|_| DocumentDiffQueryTargetError::InvalidTarget)
}

fn parse_document_id(value: &str) -> Result<DocumentId, DocumentDiffQueryTargetError> {
    DocumentId::new(value).map_err(|_| DocumentDiffQueryTargetError::InvalidTarget)
}

fn parse_version_id(value: &str) -> Result<VersionId, DocumentDiffQueryTargetError> {
    VersionId::new(value).map_err(|_| DocumentDiffQueryTargetError::InvalidTarget)
}
