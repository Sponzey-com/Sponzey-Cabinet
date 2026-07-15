use std::cell::{Cell, RefCell};

use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::CurrentDocumentSnapshot;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_ports::workspace_home::{
    WorkspaceHomeDocumentMutation, WorkspaceHomeDocumentMutationPort, WorkspaceHomeProjectionError,
};
use cabinet_usecases::document::DocumentChangeEvent;
use cabinet_usecases::workspace_home_update::{
    UpdateWorkspaceHomeError, UpdateWorkspaceHomeOutcome, UpdateWorkspaceHomeProjectionUsecase,
};

struct FakeDocumentRepository {
    record: Option<CurrentDocumentRecord>,
    fail_read: bool,
    read_count: Cell<usize>,
}

impl FakeDocumentRepository {
    fn with_record() -> Self {
        Self {
            record: Some(current_record()),
            fail_read: false,
            read_count: Cell::new(0),
        }
    }
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        _workspace_id: &WorkspaceId,
        _record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        Ok(())
    }

    fn get_current_by_id(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        self.read_count.set(self.read_count.get() + 1);
        if self.fail_read {
            Err(DocumentRepositoryError::StorageUnavailable)
        } else {
            Ok(self.record.clone())
        }
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
struct FakeMutationPort {
    calls: RefCell<Vec<(String, WorkspaceHomeDocumentMutation, u16)>>,
    fail: bool,
}

impl WorkspaceHomeDocumentMutationPort for FakeMutationPort {
    fn apply_document_mutation(
        &mut self,
        workspace_id: &WorkspaceId,
        mutation: WorkspaceHomeDocumentMutation,
        capacity: u16,
    ) -> Result<(), WorkspaceHomeProjectionError> {
        if self.fail {
            return Err(WorkspaceHomeProjectionError::StorageUnavailable);
        }
        self.calls
            .borrow_mut()
            .push((workspace_id.as_str().to_string(), mutation, capacity));
        Ok(())
    }
}

#[test]
fn create_update_restore_and_rename_events_upsert_current_document_without_body() {
    let events = [
        DocumentChangeEvent::DocumentCreated {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            version_id: "version-1".to_string(),
            title: "Document".to_string(),
            path: "document.md".to_string(),
        },
        DocumentChangeEvent::DocumentUpdated {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            version_id: "version-2".to_string(),
        },
        DocumentChangeEvent::DocumentRestored {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            target_version_id: "version-1".to_string(),
            restored_version_id: "version-3".to_string(),
        },
        DocumentChangeEvent::DocumentRenamed {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            version_id: "version-2".to_string(),
            title: "New title".to_string(),
            old_path: "/private/old.md".to_string(),
            new_path: "/private/new.md".to_string(),
        },
    ];

    for event in events {
        let documents = FakeDocumentRepository::with_record();
        let mut mutations = FakeMutationPort::default();
        let output = UpdateWorkspaceHomeProjectionUsecase::new(50)
            .expect("policy")
            .execute(event, &documents, &mut mutations)
            .expect("projection update");

        assert_eq!(output, UpdateWorkspaceHomeOutcome::AppliedUpsert);
        assert_eq!(documents.read_count.get(), 1);
        let calls = mutations.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "workspace-1");
        assert_eq!(calls[0].2, 50);
        match &calls[0].1 {
            WorkspaceHomeDocumentMutation::UpsertRecent {
                document,
                change_summary,
            } => {
                assert_eq!(document.document_id(), "doc-1");
                assert_eq!(document.title(), "Source");
                assert_eq!(document.path(), "notes/source.md");
                assert!(!change_summary.is_empty());
            }
            mutation => panic!("unexpected mutation: {mutation:?}"),
        }
        let debug = format!("{calls:?}");
        assert!(!debug.contains("private document body"));
        assert!(!debug.contains("/private/old.md"));
        assert!(!debug.contains("/private/new.md"));
    }
}

#[test]
fn delete_removes_without_repository_read_and_asset_event_is_ignored() {
    let documents = FakeDocumentRepository::with_record();
    let mut mutations = FakeMutationPort::default();
    let usecase = UpdateWorkspaceHomeProjectionUsecase::new(100).expect("policy");

    let removed = usecase
        .execute(
            DocumentChangeEvent::DocumentDeleted {
                workspace_id: "workspace-1".to_string(),
                document_id: "doc-1".to_string(),
                version_id: "version-2".to_string(),
            },
            &documents,
            &mut mutations,
        )
        .expect("remove");
    let ignored = usecase
        .execute(
            DocumentChangeEvent::DocumentAssetAttached {
                workspace_id: "workspace-1".to_string(),
                document_id: "doc-1".to_string(),
                version_id: "version-2".to_string(),
                asset_id: "asset-1".to_string(),
            },
            &documents,
            &mut mutations,
        )
        .expect("ignore");

    assert_eq!(removed, UpdateWorkspaceHomeOutcome::AppliedRemove);
    assert_eq!(ignored, UpdateWorkspaceHomeOutcome::Ignored);
    assert_eq!(documents.read_count.get(), 0);
    let calls = mutations.calls.borrow();
    assert_eq!(calls.len(), 1);
    assert!(matches!(
        calls[0].1,
        WorkspaceHomeDocumentMutation::RemoveDocument { .. }
    ));
}

#[test]
fn policy_rejects_capacity_outside_one_to_one_hundred() {
    assert_eq!(
        UpdateWorkspaceHomeProjectionUsecase::new(0).expect_err("zero fails"),
        UpdateWorkspaceHomeError::InvalidPolicy
    );
    assert_eq!(
        UpdateWorkspaceHomeProjectionUsecase::new(101).expect_err("101 fails"),
        UpdateWorkspaceHomeError::InvalidPolicy
    );
    assert!(UpdateWorkspaceHomeProjectionUsecase::new(1).is_ok());
    assert!(UpdateWorkspaceHomeProjectionUsecase::new(100).is_ok());
}

#[test]
fn projector_distinguishes_invalid_missing_repository_and_projection_failures() {
    let usecase = UpdateWorkspaceHomeProjectionUsecase::new(50).expect("policy");
    let invalid = usecase
        .execute(
            DocumentChangeEvent::DocumentCreated {
                workspace_id: "".to_string(),
                document_id: "doc-1".to_string(),
                version_id: "version-1".to_string(),
                title: "Document".to_string(),
                path: "document.md".to_string(),
            },
            &FakeDocumentRepository::with_record(),
            &mut FakeMutationPort::default(),
        )
        .expect_err("invalid fails");
    let missing = usecase
        .execute(
            created_event(),
            &FakeDocumentRepository {
                record: None,
                fail_read: false,
                read_count: Cell::new(0),
            },
            &mut FakeMutationPort::default(),
        )
        .expect_err("missing fails");
    let repository = usecase
        .execute(
            created_event(),
            &FakeDocumentRepository {
                record: None,
                fail_read: true,
                read_count: Cell::new(0),
            },
            &mut FakeMutationPort::default(),
        )
        .expect_err("repository fails");
    let mut failing_mutation = FakeMutationPort {
        fail: true,
        ..FakeMutationPort::default()
    };
    let projection = usecase
        .execute(
            created_event(),
            &FakeDocumentRepository::with_record(),
            &mut failing_mutation,
        )
        .expect_err("projection fails");

    assert_eq!(invalid.code(), "workspace_home_update.invalid_input");
    assert_eq!(
        missing.code(),
        "workspace_home_update.current_document_missing"
    );
    assert_eq!(
        repository.code(),
        "workspace_home_update.repository_unavailable"
    );
    assert_eq!(
        projection.code(),
        "workspace_home_update.projection_unavailable"
    );
    assert!(!invalid.retryable());
    assert!(repository.retryable());
    assert!(projection.retryable());
    assert_eq!(
        projection.product_log_event_name(),
        Some("workspace.home.projection_update_failed")
    );
    assert!(!format!("{projection:?}").contains("private document body"));
}

fn created_event() -> DocumentChangeEvent {
    DocumentChangeEvent::DocumentCreated {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        version_id: "version-1".to_string(),
        title: "Document".to_string(),
        path: "document.md".to_string(),
    }
}

fn current_record() -> CurrentDocumentRecord {
    let document_id = DocumentId::new("doc-1").expect("id");
    let metadata = DocumentMetadata::new(
        document_id.clone(),
        DocumentTitle::new("Source").expect("title"),
        DocumentPath::new("notes/source.md").expect("path"),
    )
    .expect("metadata");
    let body = DocumentBody::new(
        "private document body",
        DocumentBodyPolicy::new(1024).expect("policy"),
    )
    .expect("body");
    CurrentDocumentRecord::new(metadata, CurrentDocumentSnapshot::new(document_id, body))
        .expect("record")
}
