use crate::document::{DocumentId, DocumentPath, DocumentSlug};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    start: usize,
    end: usize,
}

impl SourceRange {
    pub fn new(start: usize, end: usize) -> Result<Self, LinkError> {
        if start >= end {
            return Err(LinkError::InvalidSourceRange);
        }
        Ok(Self { start, end })
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    Resolved(DocumentPath),
    Unresolved(DocumentSlug),
}

impl LinkTarget {
    pub fn resolved(path: DocumentPath) -> Self {
        Self::Resolved(path)
    }

    pub fn unresolved(slug: DocumentSlug) -> Self {
        Self::Unresolved(slug)
    }

    fn status(&self) -> LinkStatus {
        match self {
            Self::Resolved(_) => LinkStatus::Resolved,
            Self::Unresolved(_) => LinkStatus::Unresolved,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStatus {
    Resolved,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLink {
    source_document_id: DocumentId,
    target: LinkTarget,
    source_range: SourceRange,
}

impl DocumentLink {
    pub fn new(
        source_document_id: DocumentId,
        target: LinkTarget,
        source_range: SourceRange,
    ) -> Self {
        Self {
            source_document_id,
            target,
            source_range,
        }
    }

    pub fn source_document_id(&self) -> &DocumentId {
        &self.source_document_id
    }

    pub fn target(&self) -> &LinkTarget {
        &self.target
    }

    pub fn status(&self) -> LinkStatus {
        self.target.status()
    }

    pub fn source_range(&self) -> SourceRange {
        self.source_range
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backlink {
    source_document_id: DocumentId,
    target_document_id: DocumentId,
    source_range: SourceRange,
}

impl Backlink {
    pub fn new(
        source_document_id: DocumentId,
        target_document_id: DocumentId,
        source_range: SourceRange,
    ) -> Self {
        Self {
            source_document_id,
            target_document_id,
            source_range,
        }
    }

    pub fn source_document_id(&self) -> &DocumentId {
        &self.source_document_id
    }

    pub fn target_document_id(&self) -> &DocumentId {
        &self.target_document_id
    }

    pub fn source_range(&self) -> SourceRange {
        self.source_range
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkError {
    InvalidSourceRange,
}
