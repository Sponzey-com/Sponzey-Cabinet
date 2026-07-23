use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_work::{
    ProjectionKind, ProjectionWork, ProjectionWorkIdentity, ProjectionWorkState,
};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_projection_catalog::{
    CurrentDocumentProjectionCatalog, CurrentDocumentProjectionCatalogError,
    CurrentDocumentProjectionIdentity,
};
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::projection_work::{
    ProjectionEnqueueOutcome, ProjectionWorkRepository, ProjectionWorkRepositoryError,
};
use cabinet_usecases::reconcile_current_projections::{
    ReconcileCurrentProjectionsError, ReconcileCurrentProjectionsInput,
    ReconcileCurrentProjectionsUsecase,
};

#[test]
fn reconcile_enqueues_only_missing_current_document_projections() {
    let catalog = Catalog::new([("doc-a", "v1"), ("doc-b", "v1")]);
    let pointer = Pointer;
    let mut repository = Repository::ready_for("doc-b");

    let first = ReconcileCurrentProjectionsUsecase::new()
        .execute(input(100), &catalog, &pointer, &mut repository)
        .unwrap();

    assert_eq!(first.document_count(), 2);
    assert_eq!(first.ready_document_count(), 1);
    assert_eq!(first.enqueued_count(), 3);
    assert_eq!(first.reset_count(), 0);
    assert_eq!(repository.records.len(), 6);

    let second = ReconcileCurrentProjectionsUsecase::new()
        .execute(input(100), &catalog, &pointer, &mut repository)
        .unwrap();
    assert_eq!(second.ready_document_count(), 1);
    assert_eq!(second.enqueued_count(), 0);
    assert_eq!(second.already_active_count(), 3);
    assert_eq!(repository.records.len(), 6);
}

#[test]
fn reconcile_rejects_zero_limit_before_reading_the_catalog() {
    let error = ReconcileCurrentProjectionsUsecase::new()
        .execute(
            input(0),
            &Catalog::new([]),
            &Pointer,
            &mut Repository::default(),
        )
        .unwrap_err();
    assert_eq!(error, ReconcileCurrentProjectionsError::InvalidInput);
}

#[test]
fn reconcile_stops_when_catalog_identity_is_no_longer_current() {
    let error = ReconcileCurrentProjectionsUsecase::new()
        .execute(
            input(10),
            &Catalog::new([("doc-a", "older")]),
            &Pointer,
            &mut Repository::default(),
        )
        .unwrap_err();
    assert_eq!(
        error,
        ReconcileCurrentProjectionsError::CurrentVersionChanged
    );
    assert!(error.retryable());
}

fn input(limit: usize) -> ReconcileCurrentProjectionsInput {
    ReconcileCurrentProjectionsInput::new("workspace-1", limit)
}

struct Catalog {
    identities: Vec<CurrentDocumentProjectionIdentity>,
}

impl Catalog {
    fn new<const N: usize>(values: [(&str, &str); N]) -> Self {
        Self {
            identities: values
                .into_iter()
                .map(|(document, version)| {
                    CurrentDocumentProjectionIdentity::new(
                        DocumentId::new(document).unwrap(),
                        VersionId::new(version).unwrap(),
                    )
                })
                .collect(),
        }
    }
}

impl CurrentDocumentProjectionCatalog for Catalog {
    fn list_current_projection_identities(
        &self,
        _: &WorkspaceId,
        _: usize,
    ) -> Result<Vec<CurrentDocumentProjectionIdentity>, CurrentDocumentProjectionCatalogError> {
        Ok(self.identities.clone())
    }
}

struct Pointer;

impl CurrentDocumentVersionPointerPort for Pointer {
    fn load_current_version(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<Option<VersionId>, CurrentDocumentVersionPointerError> {
        Ok(Some(VersionId::new("v1").unwrap()))
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

#[derive(Default)]
struct Repository {
    records: Vec<ProjectionWork>,
}

impl Repository {
    fn ready_for(document_id: &str) -> Self {
        let workspace = WorkspaceId::new("workspace-1").unwrap();
        let document = DocumentId::new(document_id).unwrap();
        let version = VersionId::new("v1").unwrap();
        Self {
            records: [
                ProjectionKind::Search,
                ProjectionKind::Links,
                ProjectionKind::Graph,
            ]
            .into_iter()
            .map(|kind| {
                ProjectionWork::restore(
                    ProjectionWorkIdentity::new(
                        workspace.clone(),
                        document.clone(),
                        version.clone(),
                        kind,
                    ),
                    ProjectionWorkState::Ready,
                    1,
                )
                .unwrap()
            })
            .collect(),
        }
    }
}

impl ProjectionWorkRepository for Repository {
    fn enqueue(
        &mut self,
        work: ProjectionWork,
    ) -> Result<ProjectionEnqueueOutcome, ProjectionWorkRepositoryError> {
        if self
            .records
            .iter()
            .any(|record| record.identity() == work.identity())
        {
            return Ok(ProjectionEnqueueOutcome::AlreadyExists);
        }
        self.records.push(work);
        Ok(ProjectionEnqueueOutcome::Enqueued)
    }

    fn get(
        &self,
        identity: &ProjectionWorkIdentity,
    ) -> Result<Option<ProjectionWork>, ProjectionWorkRepositoryError> {
        Ok(self
            .records
            .iter()
            .find(|record| record.identity() == identity)
            .cloned())
    }

    fn replace(
        &mut self,
        work: ProjectionWork,
        expected_state: ProjectionWorkState,
    ) -> Result<(), ProjectionWorkRepositoryError> {
        let current = self
            .records
            .iter_mut()
            .find(|record| record.identity() == work.identity())
            .ok_or(ProjectionWorkRepositoryError::NotFound)?;
        if current.state() != expected_state {
            return Err(ProjectionWorkRepositoryError::Conflict);
        }
        *current = work;
        Ok(())
    }

    fn list_resumable(
        &self,
        _: usize,
    ) -> Result<Vec<ProjectionWork>, ProjectionWorkRepositoryError> {
        Ok(self
            .records
            .iter()
            .filter(|record| record.state().is_resumable())
            .cloned()
            .collect())
    }
}
