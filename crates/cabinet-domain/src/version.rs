use crate::document::{DocumentBody, DocumentId};

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
        })
    }

    pub fn with_created_at_epoch_ms(mut self, value: u64) -> Result<Self, VersionError> {
        if value == 0 {
            return Err(VersionError::InvalidCreatedAt);
        }
        self.created_at_epoch_ms = Some(value);
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
}
