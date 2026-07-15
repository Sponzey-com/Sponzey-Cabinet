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
use cabinet_usecases::projection_freshness::{
    GetCurrentProjectionFreshnessError, GetCurrentProjectionFreshnessInput,
    GetCurrentProjectionFreshnessUsecase, ProjectionFreshnessState,
};
use std::cell::RefCell;

#[test]
fn freshness_queries_only_current_version_and_reports_all_ready() {
    let pointer = Pointer::current("v2");
    let repository = Repository::states([
        (ProjectionKind::Search, Some(ProjectionWorkState::Ready)),
        (ProjectionKind::Links, Some(ProjectionWorkState::Ready)),
        (ProjectionKind::Graph, Some(ProjectionWorkState::Ready)),
    ]);
    let output = GetCurrentProjectionFreshnessUsecase::new()
        .execute(input(), &pointer, &repository)
        .unwrap();

    assert_eq!(output.current_version_id().as_str(), "v2");
    assert_eq!(output.aggregate_state(), ProjectionFreshnessState::Ready);
    assert_eq!(output.projections().len(), 3);
    assert!(
        output
            .projections()
            .iter()
            .all(|item| item.state() == ProjectionFreshnessState::Ready)
    );
    assert_eq!(
        repository.requested_versions.borrow().as_slice(),
        ["v2"; 21]
    );
}

#[test]
fn freshness_combines_all_change_causes_for_the_current_version() {
    let repository = Repository::cause_states([
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
        (
            ProjectionKind::Search,
            ProjectionChangeKind::Renamed,
            ProjectionWorkState::Pending,
        ),
    ]);

    let output = GetCurrentProjectionFreshnessUsecase::new()
        .execute(input(), &Pointer::current("v2"), &repository)
        .unwrap();

    assert_eq!(output.aggregate_state(), ProjectionFreshnessState::Stale);
    assert_eq!(
        output
            .projections()
            .iter()
            .find(|item| item.kind() == ProjectionKind::Search)
            .unwrap()
            .state(),
        ProjectionFreshnessState::Stale
    );
    assert!(
        output
            .projections()
            .iter()
            .filter(|item| item.kind() != ProjectionKind::Search)
            .all(|item| item.state() == ProjectionFreshnessState::Ready)
    );
}

#[test]
fn freshness_maps_missing_and_work_states_with_explicit_precedence() {
    for (states, expected) in [
        (
            [
                (ProjectionKind::Search, Some(ProjectionWorkState::Ready)),
                (ProjectionKind::Links, None),
                (ProjectionKind::Graph, Some(ProjectionWorkState::Ready)),
            ],
            ProjectionFreshnessState::Stale,
        ),
        (
            [
                (ProjectionKind::Search, Some(ProjectionWorkState::Pending)),
                (
                    ProjectionKind::Links,
                    Some(ProjectionWorkState::RetryScheduled),
                ),
                (ProjectionKind::Graph, Some(ProjectionWorkState::Indexing)),
            ],
            ProjectionFreshnessState::Repairing,
        ),
        (
            [
                (ProjectionKind::Search, Some(ProjectionWorkState::Indexing)),
                (ProjectionKind::Links, Some(ProjectionWorkState::Failed)),
                (ProjectionKind::Graph, Some(ProjectionWorkState::Ready)),
            ],
            ProjectionFreshnessState::Failed,
        ),
    ] {
        let output = GetCurrentProjectionFreshnessUsecase::new()
            .execute(
                input(),
                &Pointer::current("v2"),
                &Repository::states(states),
            )
            .unwrap();
        assert_eq!(output.aggregate_state(), expected);
    }
}

#[test]
fn freshness_returns_stable_boundary_errors_and_retryability() {
    let repository = Repository::states([]);
    assert_eq!(
        GetCurrentProjectionFreshnessUsecase::new()
            .execute(
                input(),
                &Pointer {
                    version: None,
                    error: Some(CurrentDocumentVersionPointerError::StorageUnavailable),
                },
                &repository,
            )
            .unwrap_err(),
        GetCurrentProjectionFreshnessError::PointerUnavailable
    );
    assert!(GetCurrentProjectionFreshnessError::PointerUnavailable.retryable());
    assert_eq!(
        GetCurrentProjectionFreshnessUsecase::new()
            .execute(
                input(),
                &Pointer::current("v2"),
                &Repository {
                    records: Vec::new(),
                    error: Some(ProjectionWorkRepositoryError::CorruptedRecord),
                    requested_versions: RefCell::new(Vec::new()),
                },
            )
            .unwrap_err(),
        GetCurrentProjectionFreshnessError::CorruptedState
    );
    assert!(!GetCurrentProjectionFreshnessError::CorruptedState.retryable());
    assert_eq!(
        GetCurrentProjectionFreshnessUsecase::new()
            .execute(
                input(),
                &Pointer {
                    version: None,
                    error: None
                },
                &repository
            )
            .unwrap_err(),
        GetCurrentProjectionFreshnessError::CurrentVersionNotFound
    );
}

struct Pointer {
    version: Option<VersionId>,
    error: Option<CurrentDocumentVersionPointerError>,
}

impl Pointer {
    fn current(value: &str) -> Self {
        Self {
            version: Some(VersionId::new(value).unwrap()),
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
    records: Vec<(ProjectionKind, ProjectionChangeKind, ProjectionWorkState)>,
    error: Option<ProjectionWorkRepositoryError>,
    requested_versions: RefCell<Vec<String>>,
}

impl Repository {
    fn states<const N: usize>(states: [(ProjectionKind, Option<ProjectionWorkState>); N]) -> Self {
        Self {
            records: states
                .into_iter()
                .filter_map(|(kind, state)| {
                    state.map(|state| (kind, ProjectionChangeKind::Updated, state))
                })
                .collect(),
            error: None,
            requested_versions: RefCell::new(Vec::new()),
        }
    }

    fn cause_states<const N: usize>(
        states: [(ProjectionKind, ProjectionChangeKind, ProjectionWorkState); N],
    ) -> Self {
        Self {
            records: states.into_iter().collect(),
            error: None,
            requested_versions: RefCell::new(Vec::new()),
        }
    }
}

impl ProjectionWorkRepository for Repository {
    fn enqueue(
        &mut self,
        _: ProjectionWork,
    ) -> Result<ProjectionEnqueueOutcome, ProjectionWorkRepositoryError> {
        unreachable!()
    }
    fn get(
        &self,
        identity: &ProjectionWorkIdentity,
    ) -> Result<Option<ProjectionWork>, ProjectionWorkRepositoryError> {
        self.requested_versions
            .borrow_mut()
            .push(identity.version_id().as_str().to_string());
        if let Some(error) = self.error {
            return Err(error);
        }
        self.records
            .iter()
            .find(|(kind, change, _)| *kind == identity.kind() && *change == identity.change_kind())
            .map(|(_, _, state)| *state)
            .map(|state| {
                let attempt = u32::from(state != ProjectionWorkState::Pending);
                ProjectionWork::restore(identity.clone(), state, attempt).unwrap()
            })
            .map_or(Ok(None), |work| Ok(Some(work)))
    }
    fn replace(
        &mut self,
        _: ProjectionWork,
        _: ProjectionWorkState,
    ) -> Result<(), ProjectionWorkRepositoryError> {
        unreachable!()
    }
    fn list_resumable(
        &self,
        _: usize,
    ) -> Result<Vec<ProjectionWork>, ProjectionWorkRepositoryError> {
        unreachable!()
    }
}

fn input() -> GetCurrentProjectionFreshnessInput {
    GetCurrentProjectionFreshnessInput::new("workspace-1", "doc-1")
}
