use cabinet_domain::projection_work::{ProjectionWorkEvent, ProjectionWorkState};
use cabinet_ports::projection_work::{ProjectionWorkRepository, ProjectionWorkRepositoryError};
use cabinet_ports::projection_worker::{ProjectionProcessorResult, ProjectionWorkProcessor};

#[derive(Debug, Clone, Copy)]
pub struct ProjectionWorkerPolicy {
    batch_limit: usize,
    max_attempts: u32,
}
impl ProjectionWorkerPolicy {
    pub fn new(batch_limit: usize, max_attempts: u32) -> Result<Self, ProjectionWorkerError> {
        if batch_limit == 0 || max_attempts == 0 {
            return Err(ProjectionWorkerError::InvalidPolicy);
        }
        Ok(Self {
            batch_limit,
            max_attempts,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionWorkerError {
    InvalidPolicy,
    RepositoryFailure,
    InvalidTransition,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ProjectionWorkerOutput {
    ready: usize,
    retry: usize,
    failed: usize,
}
impl ProjectionWorkerOutput {
    pub const fn ready_count(self) -> usize {
        self.ready
    }
    pub const fn retry_scheduled_count(self) -> usize {
        self.retry
    }
    pub const fn failed_count(self) -> usize {
        self.failed
    }
}
pub struct RunProjectionWorkerUsecase {
    policy: ProjectionWorkerPolicy,
}
impl RunProjectionWorkerUsecase {
    pub const fn new(policy: ProjectionWorkerPolicy) -> Self {
        Self { policy }
    }
    pub fn execute(
        &self,
        repository: &mut impl ProjectionWorkRepository,
        processor: &mut impl ProjectionWorkProcessor,
    ) -> Result<ProjectionWorkerOutput, ProjectionWorkerError> {
        let works = repository
            .list_resumable(self.policy.batch_limit)
            .map_err(map_repo)?;
        let mut out = ProjectionWorkerOutput::default();
        for mut work in works {
            if work.state() == ProjectionWorkState::Indexing {
                let recovered = work
                    .transition(ProjectionWorkEvent::Interrupted)
                    .map_err(|_| ProjectionWorkerError::InvalidTransition)?;
                repository
                    .replace(recovered.clone(), ProjectionWorkState::Indexing)
                    .map_err(map_repo)?;
                work = recovered;
            }
            let expected = work.state();
            let indexing = work
                .transition(ProjectionWorkEvent::Start)
                .map_err(|_| ProjectionWorkerError::InvalidTransition)?;
            repository
                .replace(indexing.clone(), expected)
                .map_err(map_repo)?;
            let event = match processor.process(indexing.identity()) {
                ProjectionProcessorResult::Succeeded => {
                    out.ready += 1;
                    ProjectionWorkEvent::Succeeded
                }
                ProjectionProcessorResult::PermanentFailure => {
                    out.failed += 1;
                    ProjectionWorkEvent::Failed
                }
                ProjectionProcessorResult::RetryableFailure
                    if indexing.attempt() >= self.policy.max_attempts =>
                {
                    out.failed += 1;
                    ProjectionWorkEvent::Failed
                }
                ProjectionProcessorResult::RetryableFailure => {
                    out.retry += 1;
                    ProjectionWorkEvent::RetryScheduled
                }
            };
            let final_work = indexing
                .transition(event)
                .map_err(|_| ProjectionWorkerError::InvalidTransition)?;
            repository
                .replace(final_work, ProjectionWorkState::Indexing)
                .map_err(map_repo)?;
        }
        Ok(out)
    }
}
fn map_repo(_: ProjectionWorkRepositoryError) -> ProjectionWorkerError {
    ProjectionWorkerError::RepositoryFailure
}
