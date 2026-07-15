use cabinet_domain::document::{DocumentId, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;

pub trait DocumentTitleReader {
    fn get_current_title(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentTitle>, DocumentTitleReaderError>;

    fn get_current_titles(
        &self,
        workspace_id: &WorkspaceId,
        document_ids: &[DocumentId],
    ) -> Result<Vec<DocumentTitleLookup>, DocumentTitleReaderError> {
        document_ids
            .iter()
            .map(|document_id| {
                self.get_current_title(workspace_id, document_id)
                    .map(|title| DocumentTitleLookup::new(document_id.clone(), title))
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTitleLookup {
    document_id: DocumentId,
    title: Option<DocumentTitle>,
}

impl DocumentTitleLookup {
    pub const fn new(document_id: DocumentId, title: Option<DocumentTitle>) -> Self {
        Self { document_id, title }
    }

    pub const fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub const fn title(&self) -> Option<&DocumentTitle> {
        self.title.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentTitleReaderError {
    StorageUnavailable,
    CorruptedMetadata,
}
