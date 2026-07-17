use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_link_catalog::{
    DocumentLinkCatalog, DocumentLinkCatalogError, DocumentLinkCatalogRecord,
};
use cabinet_usecases::document::DocumentChangeEvent;
use cabinet_usecases::document_link_catalog_projection::{
    ApplyDocumentLinkCatalogChangeUsecase, DocumentLinkCatalogChangeOutcome,
};

#[test]
fn create_and_rename_upsert_current_identity_title_and_path() {
    let mut catalog = FakeCatalog::default();
    let usecase = ApplyDocumentLinkCatalogChangeUsecase::new();
    assert_eq!(
        usecase.execute(&created(), &mut catalog).unwrap(),
        DocumentLinkCatalogChangeOutcome::Upserted
    );
    assert_eq!(
        usecase.execute(&renamed(), &mut catalog).unwrap(),
        DocumentLinkCatalogChangeOutcome::Upserted
    );
    assert_eq!(catalog.records.len(), 1);
    assert_eq!(catalog.records[0].document_id().as_str(), "doc-1");
    assert_eq!(catalog.records[0].title().as_str(), "Renamed");
    assert_eq!(catalog.records[0].path().as_str(), "notes/renamed.md");
}

#[test]
fn update_upserts_current_metadata_and_delete_removes_identity() {
    let mut catalog = FakeCatalog::default();
    let usecase = ApplyDocumentLinkCatalogChangeUsecase::new();
    usecase.execute(&created(), &mut catalog).unwrap();
    assert_eq!(
        usecase
            .execute(
                &DocumentChangeEvent::DocumentUpdated {
                    workspace_id: "workspace-1".to_string(),
                    document_id: "doc-1".to_string(),
                    version_id: "version-2".to_string(),
                    title: "Updated".to_string(),
                    path: "notes/original.md".to_string(),
                },
                &mut catalog,
            )
            .unwrap(),
        DocumentLinkCatalogChangeOutcome::Upserted
    );
    assert_eq!(catalog.records[0].title().as_str(), "Updated");
    assert_eq!(
        usecase.execute(&deleted(), &mut catalog).unwrap(),
        DocumentLinkCatalogChangeOutcome::Removed
    );
    assert!(catalog.records.is_empty());
}

#[derive(Default)]
struct FakeCatalog {
    records: Vec<DocumentLinkCatalogRecord>,
}

impl DocumentLinkCatalog for FakeCatalog {
    fn upsert(
        &mut self,
        _: &WorkspaceId,
        record: DocumentLinkCatalogRecord,
    ) -> Result<(), DocumentLinkCatalogError> {
        self.records
            .retain(|current| current.document_id() != record.document_id());
        self.records.push(record);
        Ok(())
    }

    fn remove(
        &mut self,
        _: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<bool, DocumentLinkCatalogError> {
        let before = self.records.len();
        self.records
            .retain(|record| record.document_id() != document_id);
        Ok(before != self.records.len())
    }

    fn list(
        &self,
        _: &WorkspaceId,
    ) -> Result<Vec<DocumentLinkCatalogRecord>, DocumentLinkCatalogError> {
        Ok(self.records.clone())
    }
}

fn created() -> DocumentChangeEvent {
    DocumentChangeEvent::DocumentCreated {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        version_id: "version-1".to_string(),
        title: "Original".to_string(),
        path: "notes/original.md".to_string(),
    }
}

fn renamed() -> DocumentChangeEvent {
    DocumentChangeEvent::DocumentRenamed {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        version_id: "version-1".to_string(),
        title: "Renamed".to_string(),
        old_path: "notes/original.md".to_string(),
        new_path: "notes/renamed.md".to_string(),
    }
}

fn deleted() -> DocumentChangeEvent {
    DocumentChangeEvent::DocumentDeleted {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        version_id: "version-1".to_string(),
    }
}
