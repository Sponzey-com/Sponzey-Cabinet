use crate::asset::AssetReference;
use crate::document::{DocumentBody, DocumentId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentSnapshot {
    references: Vec<AssetReference>,
}

impl AttachmentSnapshot {
    pub fn new(mut references: Vec<AssetReference>) -> Result<Self, AttachmentSnapshotError> {
        references.sort_by(|left, right| {
            left.asset_id()
                .as_str()
                .cmp(right.asset_id().as_str())
                .then_with(|| left.label().cmp(right.label()))
        });

        if references
            .windows(2)
            .any(|pair| pair[0].asset_id() == pair[1].asset_id())
        {
            return Err(AttachmentSnapshotError::DuplicateAssetReference);
        }

        Ok(Self { references })
    }

    pub fn references(&self) -> &[AssetReference] {
        &self.references
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentSnapshotState {
    Known(AttachmentSnapshot),
    LegacyUnknown,
}

impl AttachmentSnapshotState {
    pub fn known(references: Vec<AssetReference>) -> Result<Self, AttachmentSnapshotError> {
        AttachmentSnapshot::new(references).map(Self::Known)
    }

    pub const fn legacy_unknown() -> Self {
        Self::LegacyUnknown
    }

    pub fn references(&self) -> Option<&[AssetReference]> {
        match self {
            Self::Known(snapshot) => Some(snapshot.references()),
            Self::LegacyUnknown => None,
        }
    }

    pub const fn is_legacy_unknown(&self) -> bool {
        matches!(self, Self::LegacyUnknown)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentSnapshotError {
    DuplicateAssetReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DocumentRevisionNumber {
    value: u64,
}

impl DocumentRevisionNumber {
    pub fn new(value: u64) -> Result<Self, VersionError> {
        if value == 0 {
            return Err(VersionError::InvalidRevisionNumber);
        }
        Ok(Self { value })
    }

    pub const fn value(self) -> u64 {
        self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentRevisionNumberState {
    Assigned(DocumentRevisionNumber),
    LegacyUnassigned,
}

impl AttachmentSnapshotError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::DuplicateAssetReference => "version.duplicate_attachment_reference",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentDocumentSnapshot {
    document_id: DocumentId,
    body: DocumentBody,
}

impl CurrentDocumentSnapshot {
    pub fn new(document_id: DocumentId, body: DocumentBody) -> Self {
        Self { document_id, body }
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn body(&self) -> &DocumentBody {
        &self.body
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionEntry {
    version_id: VersionId,
    document_id: DocumentId,
    snapshot_ref: DocumentSnapshotRef,
    author: VersionAuthor,
    summary: VersionSummary,
    created_at_epoch_ms: Option<u64>,
    revision_number_state: DocumentRevisionNumberState,
}

impl VersionEntry {
    pub fn new(
        version_id: VersionId,
        document_id: DocumentId,
        snapshot_ref: DocumentSnapshotRef,
        author: VersionAuthor,
        summary: VersionSummary,
    ) -> Result<Self, VersionError> {
        Ok(Self {
            version_id,
            document_id,
            snapshot_ref,
            author,
            summary,
            created_at_epoch_ms: None,
            revision_number_state: DocumentRevisionNumberState::LegacyUnassigned,
        })
    }

    pub fn with_created_at_epoch_ms(mut self, value: u64) -> Result<Self, VersionError> {
        if value == 0 {
            return Err(VersionError::InvalidCreatedAt);
        }
        self.created_at_epoch_ms = Some(value);
        Ok(self)
    }

    pub fn with_revision_number(
        mut self,
        revision_number: DocumentRevisionNumber,
    ) -> Result<Self, VersionError> {
        if matches!(
            self.revision_number_state,
            DocumentRevisionNumberState::Assigned(_)
        ) {
            return Err(VersionError::RevisionNumberAlreadyAssigned);
        }
        self.revision_number_state = DocumentRevisionNumberState::Assigned(revision_number);
        Ok(self)
    }

    pub fn version_id(&self) -> &VersionId {
        &self.version_id
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn snapshot_ref(&self) -> &DocumentSnapshotRef {
        &self.snapshot_ref
    }

    pub fn author(&self) -> &VersionAuthor {
        &self.author
    }

    pub fn summary(&self) -> &VersionSummary {
        &self.summary
    }

    pub const fn created_at_epoch_ms(&self) -> Option<u64> {
        self.created_at_epoch_ms
    }

    pub const fn revision_number_state(&self) -> &DocumentRevisionNumberState {
        &self.revision_number_state
    }

    pub const fn revision_number(&self) -> Option<DocumentRevisionNumber> {
        match self.revision_number_state {
            DocumentRevisionNumberState::Assigned(value) => Some(value),
            DocumentRevisionNumberState::LegacyUnassigned => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionId {
    value: String,
}

impl VersionId {
    pub fn new(value: &str) -> Result<Self, VersionError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(VersionError::EmptyVersionId);
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
pub struct DocumentSnapshotRef {
    value: String,
}

impl DocumentSnapshotRef {
    pub fn new(value: &str) -> Result<Self, VersionError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(VersionError::EmptySnapshotRef);
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
pub struct VersionAuthor {
    value: String,
}

impl VersionAuthor {
    pub fn new(value: &str) -> Result<Self, VersionError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(VersionError::EmptyAuthor);
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
pub struct VersionSummary {
    value: String,
}

impl VersionSummary {
    pub fn new(value: &str) -> Result<Self, VersionError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(VersionError::EmptySummary);
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
pub enum VersionError {
    EmptyVersionId,
    EmptySnapshotRef,
    EmptyAuthor,
    EmptySummary,
    InvalidCreatedAt,
    InvalidRevisionNumber,
    RevisionNumberAlreadyAssigned,
}

impl VersionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyVersionId => "version.empty_version_id",
            Self::EmptySnapshotRef => "version.empty_snapshot_ref",
            Self::EmptyAuthor => "version.empty_author",
            Self::EmptySummary => "version.empty_summary",
            Self::InvalidCreatedAt => "version.invalid_created_at",
            Self::InvalidRevisionNumber => "version.invalid_revision_number",
            Self::RevisionNumberAlreadyAssigned => "version.revision_number_already_assigned",
        }
    }
}
