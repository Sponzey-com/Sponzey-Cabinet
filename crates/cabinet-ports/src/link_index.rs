use cabinet_domain::document::DocumentId;
use cabinet_domain::link::{Backlink, DocumentLink, LinkStatus};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkProjectionRecord {
    source_document_id: DocumentId,
    backlinks: Vec<Backlink>,
    unresolved_links: Vec<DocumentLink>,
}

impl LinkProjectionRecord {
    pub fn new(
        source_document_id: DocumentId,
        backlinks: Vec<Backlink>,
        unresolved_links: Vec<DocumentLink>,
    ) -> Result<Self, LinkIndexError> {
        if backlinks
            .iter()
            .any(|backlink| backlink.source_document_id() != &source_document_id)
            || unresolved_links
                .iter()
                .any(|link| link.source_document_id() != &source_document_id)
        {
            return Err(LinkIndexError::MismatchedSourceDocument);
        }
        if unresolved_links
            .iter()
            .any(|link| link.status() != LinkStatus::Unresolved)
        {
            return Err(LinkIndexError::ResolvedLinkInUnresolvedProjection);
        }
        Ok(Self {
            source_document_id,
            backlinks,
            unresolved_links,
        })
    }

    pub fn source_document_id(&self) -> &DocumentId {
        &self.source_document_id
    }

    pub fn backlinks(&self) -> &[Backlink] {
        &self.backlinks
    }

    pub fn unresolved_links(&self) -> &[DocumentLink] {
        &self.unresolved_links
    }
}

pub trait LinkIndex {
    fn replace_document_links(
        &mut self,
        workspace_id: &WorkspaceId,
        record: LinkProjectionRecord,
    ) -> Result<(), LinkIndexError>;

    fn get_document_links(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<LinkProjectionRecord>, LinkIndexError>;

    fn list_backlinks(
        &self,
        workspace_id: &WorkspaceId,
        target_document_id: &DocumentId,
    ) -> Result<Vec<Backlink>, LinkIndexError>;

    fn list_unresolved_links(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<DocumentLink>, LinkIndexError>;

    fn list_orphan_documents(
        &self,
        workspace_id: &WorkspaceId,
        document_ids: &[DocumentId],
    ) -> Result<Vec<DocumentId>, LinkIndexError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkIndexError {
    MismatchedSourceDocument,
    ResolvedLinkInUnresolvedProjection,
    StorageUnavailable,
    CorruptedProjection,
}

impl LinkIndexError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::MismatchedSourceDocument => "link_index.mismatched_source_document",
            Self::ResolvedLinkInUnresolvedProjection => {
                "link_index.resolved_link_in_unresolved_projection"
            }
            Self::StorageUnavailable => "link_index.storage_unavailable",
            Self::CorruptedProjection => "link_index.corrupted_projection",
        }
    }
}
