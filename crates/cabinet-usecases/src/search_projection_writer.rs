use cabinet_domain::document::DocumentBody;
use cabinet_domain::projection_work::{ProjectionKind, ProjectionWorkIdentity};
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::document_repository::{DocumentRepository, DocumentRepositoryError};
use cabinet_ports::markdown_parser::ParsedMarkdown;
use cabinet_ports::projection_writer::{ProjectionWriteError, VersionedProjectionWriter};
use cabinet_ports::search_index::{SearchDocumentRecord, SearchIndex, SearchIndexError};

pub struct SearchProjectionWriter<'a, P, D, S> {
    pointer: &'a P,
    documents: &'a D,
    index: &'a mut S,
}

impl<'a, P, D, S> SearchProjectionWriter<'a, P, D, S> {
    pub fn new(pointer: &'a P, documents: &'a D, index: &'a mut S) -> Self {
        Self {
            pointer,
            documents,
            index,
        }
    }
}

impl<P: CurrentDocumentVersionPointerPort, D: DocumentRepository, S: SearchIndex>
    VersionedProjectionWriter for SearchProjectionWriter<'_, P, D, S>
{
    fn write(
        &mut self,
        identity: &ProjectionWorkIdentity,
        body: &DocumentBody,
        _: &ParsedMarkdown,
    ) -> Result<(), ProjectionWriteError> {
        if identity.kind() != ProjectionKind::Search {
            return Err(ProjectionWriteError::Permanent);
        }
        let current_version = self
            .pointer
            .load_current_version(identity.workspace_id(), identity.document_id())
            .map_err(map_pointer_error)?
            .ok_or(ProjectionWriteError::Permanent)?;
        if &current_version != identity.version_id() {
            return Err(ProjectionWriteError::Permanent);
        }
        let current = self
            .documents
            .get_current_by_id(identity.workspace_id(), identity.document_id())
            .map_err(map_document_error)?
            .ok_or(ProjectionWriteError::Permanent)?;
        if current.document_id() != identity.document_id() {
            return Err(ProjectionWriteError::Permanent);
        }
        let record = SearchDocumentRecord::new(
            current.document_id().clone(),
            current.metadata().title().clone(),
            current.path().clone(),
            body.clone(),
        );
        self.index
            .upsert_document(identity.workspace_id(), record)
            .map_err(map_search_error)
    }

    fn delete(&mut self, identity: &ProjectionWorkIdentity) -> Result<(), ProjectionWriteError> {
        if identity.kind() != ProjectionKind::Search {
            return Err(ProjectionWriteError::Permanent);
        }
        ensure_current_version(self.pointer, identity)?;
        self.index
            .delete_document(identity.workspace_id(), identity.document_id())
            .map_err(map_search_error)
    }
}

fn ensure_current_version(
    pointer: &impl CurrentDocumentVersionPointerPort,
    identity: &ProjectionWorkIdentity,
) -> Result<(), ProjectionWriteError> {
    let current_version = pointer
        .load_current_version(identity.workspace_id(), identity.document_id())
        .map_err(map_pointer_error)?
        .ok_or(ProjectionWriteError::Permanent)?;
    if &current_version == identity.version_id() {
        Ok(())
    } else {
        Err(ProjectionWriteError::Permanent)
    }
}

const fn map_pointer_error(error: CurrentDocumentVersionPointerError) -> ProjectionWriteError {
    match error {
        CurrentDocumentVersionPointerError::StorageUnavailable => ProjectionWriteError::Retryable,
        CurrentDocumentVersionPointerError::Conflict
        | CurrentDocumentVersionPointerError::CorruptedPointer => ProjectionWriteError::Permanent,
    }
}

const fn map_document_error(error: DocumentRepositoryError) -> ProjectionWriteError {
    match error {
        DocumentRepositoryError::StorageUnavailable | DocumentRepositoryError::Conflict => {
            ProjectionWriteError::Retryable
        }
        DocumentRepositoryError::MismatchedDocumentIdentity
        | DocumentRepositoryError::CorruptedMetadata => ProjectionWriteError::Permanent,
    }
}

const fn map_search_error(error: SearchIndexError) -> ProjectionWriteError {
    match error {
        SearchIndexError::StorageUnavailable => ProjectionWriteError::Retryable,
        SearchIndexError::InvalidQuery
        | SearchIndexError::InvalidLimit
        | SearchIndexError::InvalidSnippet
        | SearchIndexError::CorruptedIndex => ProjectionWriteError::Permanent,
    }
}
