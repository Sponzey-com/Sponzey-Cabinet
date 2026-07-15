use cabinet_domain::document::{DocumentId, DocumentPath, DocumentSlug, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLinkCatalogRecord {
    document_id: DocumentId,
    title: DocumentTitle,
    path: DocumentPath,
    slug: DocumentSlug,
}

impl DocumentLinkCatalogRecord {
    pub fn new(
        document_id: DocumentId,
        title: DocumentTitle,
        path: DocumentPath,
    ) -> Result<Self, DocumentLinkCatalogError> {
        let slug = DocumentSlug::from_title(&title)
            .map_err(|_| DocumentLinkCatalogError::InvalidRecord)?;
        Ok(Self {
            document_id,
            title,
            path,
            slug,
        })
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn title(&self) -> &DocumentTitle {
        &self.title
    }

    pub fn path(&self) -> &DocumentPath {
        &self.path
    }

    pub fn slug(&self) -> &DocumentSlug {
        &self.slug
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLinkCatalogError {
    InvalidRecord,
    StorageUnavailable,
    CorruptedCatalog,
}

pub trait DocumentLinkCatalog {
    fn upsert(
        &mut self,
        workspace_id: &WorkspaceId,
        record: DocumentLinkCatalogRecord,
    ) -> Result<(), DocumentLinkCatalogError>;

    fn remove(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<bool, DocumentLinkCatalogError>;

    fn list(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<DocumentLinkCatalogRecord>, DocumentLinkCatalogError>;
}
