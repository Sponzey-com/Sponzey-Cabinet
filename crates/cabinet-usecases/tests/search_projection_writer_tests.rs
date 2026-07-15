use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::projection_work::{ProjectionKind, ProjectionWorkIdentity};
use cabinet_domain::version::{CurrentDocumentSnapshot, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_ports::markdown_parser::ParsedMarkdown;
use cabinet_ports::projection_writer::{ProjectionWriteError, VersionedProjectionWriter};
use cabinet_ports::search_index::{
    SearchDocumentRecord, SearchIndex, SearchIndexError, SearchPage, SearchQuery,
};
use cabinet_usecases::search_projection_writer::SearchProjectionWriter;

#[test]
fn writer_indexes_current_metadata_with_exact_version_body_once() {
    let pointer = Pointer::current("v2");
    let documents = Documents::with_record(record("repository current body"));
    let mut index = Index::default();
    let exact_body = body("exact version body needle");
    let mut writer = SearchProjectionWriter::new(&pointer, &documents, &mut index);

    assert_eq!(
        writer.write(
            &identity(ProjectionKind::Search, "v2"),
            &exact_body,
            &ParsedMarkdown::new(vec![], vec![], vec![]),
        ),
        Ok(())
    );
    let indexed = index.record.expect("indexed record");
    assert_eq!(indexed.document_id().as_str(), "doc-1");
    assert_eq!(indexed.title().as_str(), "Searchable Document");
    assert_eq!(indexed.path().as_str(), "notes/searchable.md");
    assert_eq!(indexed.body().as_str(), "exact version body needle");
    assert_eq!(index.write_count, 1);
}

#[test]
fn writer_rejects_stale_and_non_search_work_without_index_write() {
    for identity in [
        identity(ProjectionKind::Search, "v1"),
        identity(ProjectionKind::Graph, "v2"),
    ] {
        let pointer = Pointer::current("v2");
        let documents = Documents::with_record(record("current"));
        let mut index = Index::default();
        let mut writer = SearchProjectionWriter::new(&pointer, &documents, &mut index);
        assert_eq!(
            writer.write(
                &identity,
                &body("exact"),
                &ParsedMarkdown::new(vec![], vec![], vec![]),
            ),
            Err(ProjectionWriteError::Permanent)
        );
        assert_eq!(index.write_count, 0);
    }
}

#[test]
fn writer_maps_boundary_failures_without_exposing_document_content() {
    let documents = Documents::with_record(record("private current"));
    let mut index = Index::default();
    let unavailable = Pointer {
        version: None,
        error: Some(CurrentDocumentVersionPointerError::StorageUnavailable),
    };
    let mut writer = SearchProjectionWriter::new(&unavailable, &documents, &mut index);
    assert_eq!(
        writer.write(
            &identity(ProjectionKind::Search, "v2"),
            &body("private exact"),
            &ParsedMarkdown::new(vec![], vec![], vec![]),
        ),
        Err(ProjectionWriteError::Retryable)
    );

    let pointer = Pointer::current("v2");
    let corrupt_documents = Documents {
        record: None,
        error: Some(DocumentRepositoryError::CorruptedMetadata),
    };
    let mut writer = SearchProjectionWriter::new(&pointer, &corrupt_documents, &mut index);
    assert_eq!(
        writer.write(
            &identity(ProjectionKind::Search, "v2"),
            &body("private exact"),
            &ParsedMarkdown::new(vec![], vec![], vec![]),
        ),
        Err(ProjectionWriteError::Permanent)
    );

    index.error = Some(SearchIndexError::StorageUnavailable);
    let mut writer = SearchProjectionWriter::new(&pointer, &documents, &mut index);
    let error = writer
        .write(
            &identity(ProjectionKind::Search, "v2"),
            &body("private exact"),
            &ParsedMarkdown::new(vec![], vec![], vec![]),
        )
        .expect_err("index unavailable");
    assert_eq!(error, ProjectionWriteError::Retryable);
    assert!(!format!("{error:?}").contains("private exact"));
}

#[test]
fn writer_deletes_only_current_search_projection_and_maps_storage_failure() {
    let pointer = Pointer::current("v2");
    let documents = Documents::with_record(record("unused"));
    let mut index = Index::default();
    let mut writer = SearchProjectionWriter::new(&pointer, &documents, &mut index);
    assert_eq!(
        writer.delete(&identity(ProjectionKind::Search, "v2")),
        Ok(())
    );
    assert_eq!(
        writer.delete(&identity(ProjectionKind::Search, "v1")),
        Err(ProjectionWriteError::Permanent)
    );
    drop(writer);
    assert_eq!(index.delete_count, 1);

    index.error = Some(SearchIndexError::StorageUnavailable);
    let mut writer = SearchProjectionWriter::new(&pointer, &documents, &mut index);
    assert_eq!(
        writer.delete(&identity(ProjectionKind::Search, "v2")),
        Err(ProjectionWriteError::Retryable)
    );
    drop(writer);
    assert_eq!(index.delete_count, 2);
}

struct Pointer {
    version: Option<VersionId>,
    error: Option<CurrentDocumentVersionPointerError>,
}

impl Pointer {
    fn current(version: &str) -> Self {
        Self {
            version: Some(VersionId::new(version).unwrap()),
            error: None,
        }
    }
}

impl CurrentDocumentVersionPointerPort for Pointer {
    fn load_current_version(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<Option<VersionId>, CurrentDocumentVersionPointerError> {
        self.error.map_or_else(|| Ok(self.version.clone()), Err)
    }

    fn compare_and_set_current_version(
        &mut self,
        _: &WorkspaceId,
        _: &DocumentId,
        _: Option<&VersionId>,
        _: VersionId,
    ) -> Result<(), CurrentDocumentVersionPointerError> {
        unreachable!()
    }
}

struct Documents {
    record: Option<CurrentDocumentRecord>,
    error: Option<DocumentRepositoryError>,
}

impl Documents {
    fn with_record(record: CurrentDocumentRecord) -> Self {
        Self {
            record: Some(record),
            error: None,
        }
    }
}

impl DocumentRepository for Documents {
    fn put_current(
        &mut self,
        _: &WorkspaceId,
        _: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        unreachable!()
    }

    fn get_current_by_id(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        self.error.map_or_else(|| Ok(self.record.clone()), Err)
    }

    fn get_current_by_path(
        &self,
        _: &WorkspaceId,
        _: &DocumentPath,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        unreachable!()
    }

    fn delete_current(
        &mut self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<(), DocumentRepositoryError> {
        unreachable!()
    }
}

#[derive(Default)]
struct Index {
    record: Option<SearchDocumentRecord>,
    write_count: usize,
    delete_count: usize,
    error: Option<SearchIndexError>,
}

impl SearchIndex for Index {
    fn upsert_document(
        &mut self,
        _: &WorkspaceId,
        record: SearchDocumentRecord,
    ) -> Result<(), SearchIndexError> {
        self.write_count += 1;
        if let Some(error) = self.error {
            return Err(error);
        }
        self.record = Some(record);
        Ok(())
    }

    fn delete_document(&mut self, _: &WorkspaceId, _: &DocumentId) -> Result<(), SearchIndexError> {
        self.delete_count += 1;
        self.error.map_or(Ok(()), Err)
    }

    fn search(&self, _: &WorkspaceId, _: SearchQuery) -> Result<SearchPage, SearchIndexError> {
        unreachable!()
    }
}

fn identity(kind: ProjectionKind, version: &str) -> ProjectionWorkIdentity {
    ProjectionWorkIdentity::new(
        WorkspaceId::new("workspace-1").unwrap(),
        DocumentId::new("doc-1").unwrap(),
        VersionId::new(version).unwrap(),
        kind,
    )
}

fn record(current_body: &str) -> CurrentDocumentRecord {
    let document_id = DocumentId::new("doc-1").unwrap();
    CurrentDocumentRecord::new(
        DocumentMetadata::new(
            document_id.clone(),
            DocumentTitle::new("Searchable Document").unwrap(),
            DocumentPath::new("notes/searchable.md").unwrap(),
        )
        .unwrap(),
        CurrentDocumentSnapshot::new(document_id, body(current_body)),
    )
    .unwrap()
}

fn body(value: &str) -> DocumentBody {
    DocumentBody::new(value, DocumentBodyPolicy::new(1024).unwrap()).unwrap()
}
