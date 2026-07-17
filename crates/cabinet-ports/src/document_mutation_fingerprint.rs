use cabinet_domain::document::{DocumentBody, DocumentId};
use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationFingerprint, DocumentMutationKind,
};
use cabinet_domain::version::{AttachmentSnapshotState, VersionAuthor, VersionSummary};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentMutationFingerprintInput {
    kind: DocumentMutationKind,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    expected_current: DocumentExpectedCurrentVersion,
    body: DocumentBody,
    author: VersionAuthor,
    summary: VersionSummary,
    attachment_state: AttachmentSnapshotState,
}

impl DocumentMutationFingerprintInput {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        kind: DocumentMutationKind,
        workspace_id: WorkspaceId,
        document_id: DocumentId,
        expected_current: DocumentExpectedCurrentVersion,
        body: DocumentBody,
        author: VersionAuthor,
        summary: VersionSummary,
        attachment_state: AttachmentSnapshotState,
    ) -> Self {
        Self {
            kind,
            workspace_id,
            document_id,
            expected_current,
            body,
            author,
            summary,
            attachment_state,
        }
    }

    pub const fn kind(&self) -> DocumentMutationKind {
        self.kind
    }

    pub const fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub const fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub const fn expected_current(&self) -> &DocumentExpectedCurrentVersion {
        &self.expected_current
    }

    pub const fn body(&self) -> &DocumentBody {
        &self.body
    }

    pub const fn author(&self) -> &VersionAuthor {
        &self.author
    }

    pub const fn summary(&self) -> &VersionSummary {
        &self.summary
    }

    pub const fn attachment_state(&self) -> &AttachmentSnapshotState {
        &self.attachment_state
    }
}

pub trait DocumentMutationFingerprintPort {
    fn fingerprint(
        &self,
        input: &DocumentMutationFingerprintInput,
    ) -> Result<DocumentMutationFingerprint, DocumentMutationFingerprintPortError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentMutationFingerprintPortError {
    GenerationUnavailable,
}

impl DocumentMutationFingerprintPortError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::GenerationUnavailable => "document_mutation_fingerprint.generation_unavailable",
        }
    }
}
