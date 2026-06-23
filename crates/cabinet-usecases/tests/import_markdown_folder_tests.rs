use std::collections::HashMap;

use cabinet_domain::document::{DocumentId, DocumentPath};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_ports::version_store::{
    HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};
use cabinet_usecases::import::{
    ImportMarkdownEntryInput, ImportMarkdownFolderInput, ImportMarkdownFolderUsecase,
    MarkdownImportState,
};

#[derive(Default)]
struct FakeDocumentRepository {
    records: HashMap<(String, String), CurrentDocumentRecord>,
    conflict_document_id: Option<String>,
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        if self.conflict_document_id.as_deref() == Some(record.document_id().as_str()) {
            return Err(DocumentRepositoryError::Conflict);
        }
        self.records.insert(
            (
                workspace_id.as_str().to_string(),
                record.document_id().as_str().to_string(),
            ),
            record,
        );
        Ok(())
    }

    fn get_current_by_id(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        Ok(self
            .records
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
            ))
            .cloned())
    }

    fn get_current_by_path(
        &self,
        _workspace_id: &WorkspaceId,
        _path: &DocumentPath,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        Ok(None)
    }

    fn delete_current(
        &mut self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<(), DocumentRepositoryError> {
        Ok(())
    }
}

#[derive(Default)]
struct FakeVersionStore {
    records: Vec<VersionRecord>,
}

impl VersionStore for FakeVersionStore {
    fn append_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        self.records.push(record);
        Ok(())
    }

    fn get_version_snapshot(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        Ok(None)
    }

    fn list_history(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _request: HistoryPageRequest,
    ) -> Result<HistoryPage, VersionStoreError> {
        Ok(HistoryPage::new(Vec::new(), None))
    }
}

#[test]
fn import_markdown_folder_stores_current_documents_and_versions() {
    let mut documents = FakeDocumentRepository::default();
    let mut versions = FakeVersionStore::default();
    let usecase = ImportMarkdownFolderUsecase::new();

    let output = usecase
        .execute(
            ImportMarkdownFolderInput::new(
                "workspace-1",
                vec![
                    entry("doc-1", "One", "docs/one.md"),
                    entry("doc-2", "Two", "docs/two.md"),
                ],
            ),
            &mut documents,
            &mut versions,
        )
        .expect("import");

    assert_eq!(output.final_state(), MarkdownImportState::Completed);
    assert_eq!(output.imported_count(), 2);
    assert!(output.failed_items().is_empty());
    assert_eq!(documents.records.len(), 2);
    assert_eq!(versions.records.len(), 2);
}

#[test]
fn import_markdown_folder_continues_after_duplicate_entry_as_partial_failure() {
    let mut documents = FakeDocumentRepository {
        conflict_document_id: Some("doc-1".to_string()),
        ..FakeDocumentRepository::default()
    };
    let mut versions = FakeVersionStore::default();
    let usecase = ImportMarkdownFolderUsecase::new();

    let output = usecase
        .execute(
            ImportMarkdownFolderInput::new(
                "workspace-1",
                vec![
                    entry("doc-1", "One", "docs/one.md"),
                    entry("doc-2", "Two", "docs/two.md"),
                ],
            ),
            &mut documents,
            &mut versions,
        )
        .expect("partial import");

    assert_eq!(output.final_state(), MarkdownImportState::PartiallyFailed);
    assert_eq!(output.imported_count(), 1);
    assert_eq!(output.failed_items().len(), 1);
    assert_eq!(output.failed_items()[0].document_id(), "doc-1");
    assert_eq!(documents.records.len(), 1);
    assert_eq!(versions.records.len(), 1);
}

#[test]
fn import_markdown_folder_invalid_entry_does_not_stop_valid_entry() {
    let mut documents = FakeDocumentRepository::default();
    let mut versions = FakeVersionStore::default();
    let usecase = ImportMarkdownFolderUsecase::new();

    let output = usecase
        .execute(
            ImportMarkdownFolderInput::new(
                "workspace-1",
                vec![
                    entry("doc-1", "One", "/absolute.md"),
                    entry("doc-2", "Two", "docs/two.md"),
                ],
            ),
            &mut documents,
            &mut versions,
        )
        .expect("partial import");

    assert_eq!(output.final_state(), MarkdownImportState::PartiallyFailed);
    assert_eq!(output.imported_count(), 1);
    assert_eq!(output.failed_items().len(), 1);
    assert_eq!(documents.records.len(), 1);
    assert_eq!(versions.records.len(), 1);
}

fn entry(document_id: &str, title: &str, path: &str) -> ImportMarkdownEntryInput {
    ImportMarkdownEntryInput::new(
        document_id,
        title,
        path,
        "Imported body",
        &format!("version-{document_id}"),
        &format!("snapshot-{document_id}"),
        "importer",
        "Imported from folder",
    )
}
