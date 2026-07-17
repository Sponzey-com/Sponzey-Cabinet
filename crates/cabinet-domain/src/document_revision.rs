use crate::document::DocumentId;
use crate::version::VersionId;
use crate::workspace::WorkspaceId;

pub const MAX_DOCUMENT_OPERATION_ID_LENGTH: usize = 128;
pub const MAX_DOCUMENT_MUTATION_FINGERPRINT_LENGTH: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentOperationId {
    value: String,
}

impl DocumentOperationId {
    pub fn new(value: &str) -> Result<Self, DocumentOperationError> {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
            return Err(DocumentOperationError::InvalidOperationId);
        }
        if trimmed.len() > MAX_DOCUMENT_OPERATION_ID_LENGTH {
            return Err(DocumentOperationError::OperationIdTooLong);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentMutationFingerprint {
    value: String,
}

impl DocumentMutationFingerprint {
    pub fn new(value: &str) -> Result<Self, DocumentOperationError> {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
            return Err(DocumentOperationError::InvalidRequestFingerprint);
        }
        if trimmed.len() > MAX_DOCUMENT_MUTATION_FINGERPRINT_LENGTH {
            return Err(DocumentOperationError::RequestFingerprintTooLong);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentMutationKind {
    Create,
    Update,
    AttachAsset,
    LinkAsset,
    UnlinkAsset,
    Restore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentExpectedCurrentVersion {
    MustNotExist,
    MustMatch(VersionId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentOperationIdentity {
    operation_id: DocumentOperationId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    kind: DocumentMutationKind,
    expected_current: DocumentExpectedCurrentVersion,
    request_fingerprint: Option<DocumentMutationFingerprint>,
}

impl DocumentOperationIdentity {
    pub fn new(
        operation_id: DocumentOperationId,
        workspace_id: WorkspaceId,
        document_id: DocumentId,
        kind: DocumentMutationKind,
        expected_current: DocumentExpectedCurrentVersion,
    ) -> Result<Self, DocumentOperationError> {
        let guard_is_valid = matches!(
            (kind, &expected_current),
            (
                DocumentMutationKind::Create,
                DocumentExpectedCurrentVersion::MustNotExist
            ) | (
                DocumentMutationKind::Update
                    | DocumentMutationKind::AttachAsset
                    | DocumentMutationKind::LinkAsset
                    | DocumentMutationKind::UnlinkAsset
                    | DocumentMutationKind::Restore,
                DocumentExpectedCurrentVersion::MustMatch(_)
            )
        );
        if !guard_is_valid {
            return Err(DocumentOperationError::InvalidExpectedCurrentGuard);
        }
        Ok(Self {
            operation_id,
            workspace_id,
            document_id,
            kind,
            expected_current,
            request_fingerprint: None,
        })
    }

    pub fn with_request_fingerprint(
        mut self,
        request_fingerprint: DocumentMutationFingerprint,
    ) -> Self {
        self.request_fingerprint = Some(request_fingerprint);
        self
    }

    pub const fn operation_id(&self) -> &DocumentOperationId {
        &self.operation_id
    }

    pub const fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub const fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub const fn kind(&self) -> DocumentMutationKind {
        self.kind
    }

    pub const fn expected_current(&self) -> &DocumentExpectedCurrentVersion {
        &self.expected_current
    }

    pub const fn request_fingerprint(&self) -> Option<&DocumentMutationFingerprint> {
        self.request_fingerprint.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentOperationError {
    InvalidOperationId,
    OperationIdTooLong,
    InvalidExpectedCurrentGuard,
    InvalidRequestFingerprint,
    RequestFingerprintTooLong,
}

impl DocumentOperationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidOperationId => "document_operation.invalid_operation_id",
            Self::OperationIdTooLong => "document_operation.operation_id_too_long",
            Self::InvalidExpectedCurrentGuard => {
                "document_operation.invalid_expected_current_guard"
            }
            Self::InvalidRequestFingerprint => "document_operation.invalid_request_fingerprint",
            Self::RequestFingerprintTooLong => "document_operation.request_fingerprint_too_long",
        }
    }
}
