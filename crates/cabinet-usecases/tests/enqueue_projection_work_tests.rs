use cabinet_domain::projection_work::{
    ProjectionChangeKind, ProjectionKind, ProjectionWork, ProjectionWorkIdentity,
    ProjectionWorkState,
};
use cabinet_ports::projection_work::{
    ProjectionEnqueueOutcome, ProjectionWorkRepository, ProjectionWorkRepositoryError,
};
use cabinet_usecases::document::DocumentChangeEvent;
use cabinet_usecases::projection_work::{EnqueueProjectionWorkError, EnqueueProjectionWorkUsecase};

#[test]
fn supported_document_changes_enqueue_distinct_search_links_and_graph_pending_work() {
    for (event, change_kind) in [
        (created(), ProjectionChangeKind::Created),
        (updated(), ProjectionChangeKind::Updated),
        (restored(), ProjectionChangeKind::Restored),
        (renamed(), ProjectionChangeKind::Renamed),
        (deleted(), ProjectionChangeKind::Deleted),
        (asset_attached(), ProjectionChangeKind::AssetAttached),
    ] {
        let mut repository = FakeRepository::default();
        let output = EnqueueProjectionWorkUsecase::new()
            .execute(event, &mut repository)
            .expect("enqueue");

        assert_eq!(output.enqueued_count(), 3);
        assert_eq!(output.duplicate_count(), 0);
        assert_eq!(repository.works.len(), 3);
        assert_eq!(
            repository.works[0].identity().kind(),
            ProjectionKind::Search
        );
        assert_eq!(repository.works[1].identity().kind(), ProjectionKind::Links);
        assert_eq!(repository.works[2].identity().kind(), ProjectionKind::Graph);
        assert!(
            repository
                .works
                .iter()
                .all(|work| work.state() == ProjectionWorkState::Pending)
        );
        assert!(
            repository
                .works
                .iter()
                .all(|work| work.identity().version_id().as_str() == "version-1")
        );
        assert!(
            repository
                .works
                .iter()
                .all(|work| work.identity().change_kind() == change_kind)
        );
    }
}

#[test]
fn projection_enqueue_counts_duplicates_and_maps_safe_failures() {
    let mut duplicate = FakeRepository {
        duplicate: true,
        ..FakeRepository::default()
    };
    let output = EnqueueProjectionWorkUsecase::new()
        .execute(updated(), &mut duplicate)
        .expect("duplicates");
    assert_eq!(output.enqueued_count(), 0);
    assert_eq!(output.duplicate_count(), 3);

    let mut failed = FakeRepository {
        failure: Some(ProjectionWorkRepositoryError::StorageUnavailable),
        ..FakeRepository::default()
    };
    let error = EnqueueProjectionWorkUsecase::new()
        .execute(updated(), &mut failed)
        .expect_err("failure");
    assert_eq!(error, EnqueueProjectionWorkError::RepositoryUnavailable);
    assert_eq!(error.code(), "projection_enqueue.repository_unavailable");
}

#[test]
fn projection_enqueue_rejects_invalid_identity_without_writes() {
    let mut repository = FakeRepository::default();
    let invalid = EnqueueProjectionWorkUsecase::new().execute(
        DocumentChangeEvent::DocumentUpdated {
            workspace_id: " ".to_string(),
            document_id: "doc-1".to_string(),
            version_id: "version-1".to_string(),
            title: "Document".to_string(),
            path: "document.md".to_string(),
        },
        &mut repository,
    );

    assert_eq!(invalid, Err(EnqueueProjectionWorkError::InvalidIdentity));
    assert!(repository.works.is_empty());
}

#[derive(Default)]
struct FakeRepository {
    works: Vec<ProjectionWork>,
    duplicate: bool,
    failure: Option<ProjectionWorkRepositoryError>,
}

impl ProjectionWorkRepository for FakeRepository {
    fn enqueue(
        &mut self,
        work: ProjectionWork,
    ) -> Result<ProjectionEnqueueOutcome, ProjectionWorkRepositoryError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if self.duplicate {
            return Ok(ProjectionEnqueueOutcome::AlreadyExists);
        }
        self.works.push(work);
        Ok(ProjectionEnqueueOutcome::Enqueued)
    }

    fn get(
        &self,
        _identity: &ProjectionWorkIdentity,
    ) -> Result<Option<ProjectionWork>, ProjectionWorkRepositoryError> {
        Ok(None)
    }
    fn replace(
        &mut self,
        _work: ProjectionWork,
        _expected_state: ProjectionWorkState,
    ) -> Result<(), ProjectionWorkRepositoryError> {
        Ok(())
    }
    fn list_resumable(
        &self,
        _limit: usize,
    ) -> Result<Vec<ProjectionWork>, ProjectionWorkRepositoryError> {
        Ok(Vec::new())
    }
}

fn created() -> DocumentChangeEvent {
    DocumentChangeEvent::DocumentCreated {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        version_id: "version-1".to_string(),
        title: "Document".to_string(),
        path: "document.md".to_string(),
    }
}

fn updated() -> DocumentChangeEvent {
    DocumentChangeEvent::DocumentUpdated {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        version_id: "version-1".to_string(),
        title: "Document".to_string(),
        path: "document.md".to_string(),
    }
}

fn restored() -> DocumentChangeEvent {
    DocumentChangeEvent::DocumentRestored {
        workspace_id: "workspace-1".to_string(),
        document_id: "doc-1".to_string(),
        target_version_id: "old-version".to_string(),
        restored_version_id: "version-1".to_string(),
    }
}

fn renamed() -> DocumentChangeEvent {
    DocumentChangeEvent::DocumentRenamed {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        version_id: "version-1".into(),
        title: "Renamed".into(),
        old_path: "old.md".into(),
        new_path: "new.md".into(),
    }
}

fn deleted() -> DocumentChangeEvent {
    DocumentChangeEvent::DocumentDeleted {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        version_id: "version-1".into(),
    }
}

fn asset_attached() -> DocumentChangeEvent {
    DocumentChangeEvent::DocumentAssetAttached {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        version_id: "version-1".into(),
        asset_id: "asset-1".into(),
    }
}
