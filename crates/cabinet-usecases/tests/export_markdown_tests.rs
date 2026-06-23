use std::collections::HashMap;

use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::CurrentDocumentSnapshot;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_usecases::export::{ExportMarkdownInput, ExportMarkdownState, ExportMarkdownUsecase};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(Default)]
struct FakeDocumentRepository {
    records: HashMap<(String, String), CurrentDocumentRecord>,
}

impl FakeDocumentRepository {
    fn insert(&mut self, workspace_id: &str, record: CurrentDocumentRecord) {
        self.records.insert(
            (
                workspace_id.to_string(),
                record.document_id().as_str().to_string(),
            ),
            record,
        );
    }
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        self.insert(workspace_id.as_str(), record);
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

#[test]
fn export_markdown_returns_current_documents_as_file_plan() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert(
        "workspace-1",
        current_record("doc-1", "Title", "docs/one.md", "Body"),
    );
    let usecase = ExportMarkdownUsecase::new();

    let output = usecase
        .execute(
            ExportMarkdownInput::new("workspace-1", vec!["doc-1"]),
            &documents,
        )
        .expect("export");

    assert_eq!(output.final_state(), ExportMarkdownState::Completed);
    assert_eq!(output.files().len(), 1);
    assert_eq!(output.files()[0].path(), "docs/one.md");
    assert_eq!(output.files()[0].content(), "Body");
    assert!(output.failed_items().is_empty());
}

#[test]
fn export_markdown_preserves_asset_reference_text() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert(
        "workspace-1",
        current_record(
            "doc-1",
            "Title",
            "docs/one.md",
            &format!("Diagram: ![[asset:{HASH_A}|Diagram]]"),
        ),
    );
    let usecase = ExportMarkdownUsecase::new();

    let output = usecase
        .execute(
            ExportMarkdownInput::new("workspace-1", vec!["doc-1"]),
            &documents,
        )
        .expect("export");

    assert!(
        output.files()[0]
            .content()
            .contains(&format!("asset:{HASH_A}"))
    );
}

#[test]
fn export_markdown_records_missing_document_as_partial_failure() {
    let mut documents = FakeDocumentRepository::default();
    documents.insert(
        "workspace-1",
        current_record("doc-1", "Title", "docs/one.md", "Body"),
    );
    let usecase = ExportMarkdownUsecase::new();

    let output = usecase
        .execute(
            ExportMarkdownInput::new("workspace-1", vec!["doc-404", "doc-1"]),
            &documents,
        )
        .expect("partial export");

    assert_eq!(output.final_state(), ExportMarkdownState::PartiallyFailed);
    assert_eq!(output.files().len(), 1);
    assert_eq!(output.failed_items().len(), 1);
    assert_eq!(output.failed_items()[0].document_id(), "doc-404");
}

fn current_record(document_id: &str, title: &str, path: &str, body: &str) -> CurrentDocumentRecord {
    let metadata = DocumentMetadata::new(
        DocumentId::new(document_id).expect("document id"),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
    )
    .expect("metadata");
    let snapshot = CurrentDocumentSnapshot::new(
        DocumentId::new(document_id).expect("document id"),
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
    );
    CurrentDocumentRecord::new(metadata, snapshot).expect("record")
}
