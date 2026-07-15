use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_work::{
    ProjectionChangeKind, ProjectionKind, ProjectionWork, ProjectionWorkIdentity,
    ProjectionWorkState,
};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::projection_work::{
    ProjectionEnqueueOutcome, ProjectionWorkRepository, ProjectionWorkRepositoryError,
};
use cabinet_usecases::reindex_projection::{
    ReindexCurrentProjectionError, ReindexCurrentProjectionInput, ReindexCurrentProjectionUsecase,
};

#[test]
fn reindex_enqueues_missing_resets_terminal_and_reports_counts() {
    let pointer = Pointer::current();
    let mut repository = Repository::new([
        (ProjectionKind::Links, ProjectionWorkState::Ready),
        (ProjectionKind::Graph, ProjectionWorkState::Failed),
    ]);
    let output = ReindexCurrentProjectionUsecase::new()
        .execute(input(), &pointer, &mut repository)
        .unwrap();

    assert_eq!(output.enqueued_count(), 1);
    assert_eq!(output.reset_count(), 2);
    assert_eq!(output.already_active_count(), 0);
    assert_eq!(repository.enqueued, vec![ProjectionKind::Search]);
    assert_eq!(repository.replaced.len(), 2);
    assert!(repository.replaced.iter().all(|(work, expected)| {
        work.state() == ProjectionWorkState::Pending
            && work.attempt() == 0
            && matches!(
                expected,
                ProjectionWorkState::Ready | ProjectionWorkState::Failed
            )
    }));
}

#[test]
fn reindex_keeps_active_work_idempotently_without_writes() {
    let mut repository = Repository::new([
        (ProjectionKind::Search, ProjectionWorkState::Pending),
        (ProjectionKind::Links, ProjectionWorkState::Indexing),
        (ProjectionKind::Graph, ProjectionWorkState::RetryScheduled),
    ]);
    let output = ReindexCurrentProjectionUsecase::new()
        .execute(input(), &Pointer::current(), &mut repository)
        .unwrap();
    assert_eq!(output.already_active_count(), 3);
    assert!(repository.enqueued.is_empty());
    assert!(repository.replaced.is_empty());
}

#[test]
fn reindex_resets_terminal_work_created_for_the_current_version() {
    let mut repository = Repository::with_change([
        (
            ProjectionKind::Search,
            ProjectionChangeKind::Created,
            ProjectionWorkState::Ready,
        ),
        (
            ProjectionKind::Links,
            ProjectionChangeKind::Created,
            ProjectionWorkState::Ready,
        ),
        (
            ProjectionKind::Graph,
            ProjectionChangeKind::Created,
            ProjectionWorkState::Ready,
        ),
    ]);

    let output = ReindexCurrentProjectionUsecase::new()
        .execute(input(), &Pointer::current(), &mut repository)
        .unwrap();

    assert_eq!(output.reset_count(), 3);
    assert_eq!(output.enqueued_count(), 0);
    assert!(repository.replaced.iter().all(|(work, _)| {
        work.identity().change_kind() == ProjectionChangeKind::Created
            && work.state() == ProjectionWorkState::Pending
    }));
}

#[test]
fn reindex_maps_pointer_and_cas_failures_to_stable_errors() {
    let mut repository = Repository::new([]);
    let unavailable = Pointer {
        version: None,
        error: Some(CurrentDocumentVersionPointerError::StorageUnavailable),
    };
    assert_eq!(
        ReindexCurrentProjectionUsecase::new()
            .execute(input(), &unavailable, &mut repository)
            .unwrap_err(),
        ReindexCurrentProjectionError::PointerUnavailable
    );
    assert!(ReindexCurrentProjectionError::PointerUnavailable.retryable());

    let mut repository = Repository::new([(ProjectionKind::Search, ProjectionWorkState::Ready)]);
    repository.error = Some(ProjectionWorkRepositoryError::Conflict);
    assert_eq!(
        ReindexCurrentProjectionUsecase::new()
            .execute(input(), &Pointer::current(), &mut repository)
            .unwrap_err(),
        ReindexCurrentProjectionError::RepositoryConflict
    );
}

struct Pointer {
    version: Option<VersionId>,
    error: Option<CurrentDocumentVersionPointerError>,
}
impl Pointer {
    fn current() -> Self {
        Self {
            version: Some(VersionId::new("v2").unwrap()),
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

struct Repository {
    records: Vec<ProjectionWork>,
    enqueued: Vec<ProjectionKind>,
    replaced: Vec<(ProjectionWork, ProjectionWorkState)>,
    error: Option<ProjectionWorkRepositoryError>,
}
impl Repository {
    fn new<const N: usize>(states: [(ProjectionKind, ProjectionWorkState); N]) -> Self {
        Self::with_change(states.map(|(kind, state)| (kind, ProjectionChangeKind::Updated, state)))
    }

    fn with_change<const N: usize>(
        states: [(ProjectionKind, ProjectionChangeKind, ProjectionWorkState); N],
    ) -> Self {
        Self {
            records: states
                .into_iter()
                .map(|(kind, change, state)| {
                    ProjectionWork::restore(
                        identity_for_change(kind, change),
                        state,
                        u32::from(state != ProjectionWorkState::Pending),
                    )
                    .unwrap()
                })
                .collect(),
            enqueued: Vec::new(),
            replaced: Vec::new(),
            error: None,
        }
    }
}
impl ProjectionWorkRepository for Repository {
    fn enqueue(
        &mut self,
        work: ProjectionWork,
    ) -> Result<ProjectionEnqueueOutcome, ProjectionWorkRepositoryError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        self.enqueued.push(work.identity().kind());
        Ok(ProjectionEnqueueOutcome::Enqueued)
    }
    fn get(
        &self,
        identity: &ProjectionWorkIdentity,
    ) -> Result<Option<ProjectionWork>, ProjectionWorkRepositoryError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        Ok(self
            .records
            .iter()
            .find(|work| work.identity().idempotency_key() == identity.idempotency_key())
            .cloned())
    }
    fn replace(
        &mut self,
        work: ProjectionWork,
        expected: ProjectionWorkState,
    ) -> Result<(), ProjectionWorkRepositoryError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        self.replaced.push((work, expected));
        Ok(())
    }
    fn list_resumable(
        &self,
        _: usize,
    ) -> Result<Vec<ProjectionWork>, ProjectionWorkRepositoryError> {
        unreachable!()
    }
}
fn input() -> ReindexCurrentProjectionInput {
    ReindexCurrentProjectionInput::new("workspace-1", "doc-1")
}
fn identity_for_change(
    kind: ProjectionKind,
    change: ProjectionChangeKind,
) -> ProjectionWorkIdentity {
    ProjectionWorkIdentity::for_change(
        WorkspaceId::new("workspace-1").unwrap(),
        DocumentId::new("doc-1").unwrap(),
        VersionId::new("v2").unwrap(),
        kind,
        change,
    )
}
