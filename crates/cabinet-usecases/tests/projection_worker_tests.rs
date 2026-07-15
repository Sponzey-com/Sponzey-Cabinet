use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_work::{
    ProjectionKind, ProjectionWork, ProjectionWorkEvent, ProjectionWorkIdentity,
    ProjectionWorkState,
};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::projection_work::{
    ProjectionEnqueueOutcome, ProjectionWorkRepository, ProjectionWorkRepositoryError,
};
use cabinet_ports::projection_worker::{ProjectionProcessorResult, ProjectionWorkProcessor};
use cabinet_usecases::projection_worker::{ProjectionWorkerPolicy, RunProjectionWorkerUsecase};
use std::collections::BTreeMap;

#[test]
fn worker_recovers_interrupted_and_maps_success_retry_and_exhaustion() {
    let pending = ProjectionWork::pending(identity("v1", ProjectionKind::Search));
    let interrupted = ProjectionWork::pending(identity("v1", ProjectionKind::Links))
        .transition(ProjectionWorkEvent::Start)
        .unwrap();
    let retry = ProjectionWork::pending(identity("v1", ProjectionKind::Graph))
        .transition(ProjectionWorkEvent::Start)
        .unwrap()
        .transition(ProjectionWorkEvent::RetryScheduled)
        .unwrap();
    let mut repository = MemoryRepository::new([pending, interrupted, retry]);
    let mut processor = FakeProcessor {
        results: vec![
            ProjectionProcessorResult::Succeeded,
            ProjectionProcessorResult::RetryableFailure,
            ProjectionProcessorResult::RetryableFailure,
        ],
    };
    let output = RunProjectionWorkerUsecase::new(ProjectionWorkerPolicy::new(10, 2).unwrap())
        .execute(&mut repository, &mut processor)
        .unwrap();
    assert_eq!(
        (
            output.ready_count(),
            output.retry_scheduled_count(),
            output.failed_count()
        ),
        (1, 1, 1)
    );
    let states = repository.states();
    assert_eq!(
        states
            .iter()
            .filter(|state| **state == ProjectionWorkState::Ready)
            .count(),
        1
    );
    assert_eq!(
        states
            .iter()
            .filter(|state| **state == ProjectionWorkState::RetryScheduled)
            .count(),
        1
    );
    assert_eq!(
        states
            .iter()
            .filter(|state| **state == ProjectionWorkState::Failed)
            .count(),
        1
    );
}

struct FakeProcessor {
    results: Vec<ProjectionProcessorResult>,
}
impl ProjectionWorkProcessor for FakeProcessor {
    fn process(&mut self, _: &ProjectionWorkIdentity) -> ProjectionProcessorResult {
        self.results.remove(0)
    }
}
struct MemoryRepository {
    records: BTreeMap<String, ProjectionWork>,
}
impl MemoryRepository {
    fn new<const N: usize>(works: [ProjectionWork; N]) -> Self {
        Self {
            records: works
                .into_iter()
                .map(|w| (w.identity().idempotency_key(), w))
                .collect(),
        }
    }
    fn states(&self) -> Vec<ProjectionWorkState> {
        self.records.values().map(|w| w.state()).collect()
    }
}
impl ProjectionWorkRepository for MemoryRepository {
    fn enqueue(
        &mut self,
        w: ProjectionWork,
    ) -> Result<ProjectionEnqueueOutcome, ProjectionWorkRepositoryError> {
        self.records.insert(w.identity().idempotency_key(), w);
        Ok(ProjectionEnqueueOutcome::Enqueued)
    }
    fn get(
        &self,
        i: &ProjectionWorkIdentity,
    ) -> Result<Option<ProjectionWork>, ProjectionWorkRepositoryError> {
        Ok(self.records.get(&i.idempotency_key()).cloned())
    }
    fn replace(
        &mut self,
        w: ProjectionWork,
        e: ProjectionWorkState,
    ) -> Result<(), ProjectionWorkRepositoryError> {
        let k = w.identity().idempotency_key();
        if self.records.get(&k).map(|x| x.state()) != Some(e) {
            return Err(ProjectionWorkRepositoryError::Conflict);
        }
        self.records.insert(k, w);
        Ok(())
    }
    fn list_resumable(
        &self,
        l: usize,
    ) -> Result<Vec<ProjectionWork>, ProjectionWorkRepositoryError> {
        Ok(self
            .records
            .values()
            .filter(|w| w.state().is_resumable())
            .take(l)
            .cloned()
            .collect())
    }
}
fn identity(v: &str, k: ProjectionKind) -> ProjectionWorkIdentity {
    ProjectionWorkIdentity::new(
        WorkspaceId::new("w").unwrap(),
        DocumentId::new("d").unwrap(),
        VersionId::new(v).unwrap(),
        k,
    )
}
