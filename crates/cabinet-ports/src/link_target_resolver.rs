use cabinet_domain::document::{DocumentId, DocumentPath, DocumentSlug};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDocumentLinkTarget {
    document_id: DocumentId,
    path: DocumentPath,
}
impl ResolvedDocumentLinkTarget {
    pub fn new(document_id: DocumentId, path: DocumentPath) -> Self {
        Self { document_id, path }
    }
    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }
    pub fn path(&self) -> &DocumentPath {
        &self.path
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTargetResolution {
    Resolved(ResolvedDocumentLinkTarget),
    Unresolved(DocumentSlug),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkTargetResolverError {
    Unavailable,
    InvalidTarget,
    Ambiguous,
}
pub trait DocumentLinkTargetResolver {
    fn resolve(
        &self,
        workspace_id: &WorkspaceId,
        target: &str,
    ) -> Result<LinkTargetResolution, LinkTargetResolverError>;

    fn resolve_relative(
        &self,
        workspace_id: &WorkspaceId,
        source_document_id: &DocumentId,
        target: &str,
    ) -> Result<LinkTargetResolution, LinkTargetResolverError>;
}
