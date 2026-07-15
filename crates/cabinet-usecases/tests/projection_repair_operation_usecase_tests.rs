use std::collections::BTreeMap;

use cabinet_domain::projection_repair::{
    ProjectionRepairEvent, ProjectionRepairOperation, ProjectionRepairOperationId,
    ProjectionRepairState,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::projection_repair::{
    ProjectionRepairCreateOutcome, ProjectionRepairRepository, ProjectionRepairRepositoryError,
};
use cabinet_usecases::projection_repair_operation::{
    CancelProjectionRepairInput, CancelProjectionRepairUsecase, GetProjectionRepairStatusInput,
    GetProjectionRepairStatusUsecase, ProjectionRepairOperationIdGenerator,
    ProjectionRepairUsecaseError, RetryProjectionRepairInput, RetryProjectionRepairUsecase,
    StartProjectionRepairInput, StartProjectionRepairUsecase,
};

#[test]
fn start_creates_queued_operation_with_injected_identity() {
    let mut repository = FakeRepository::default();
    let mut ids = FakeIds::new("repair-1");
    let output = StartProjectionRepairUsecase::new()
        .execute(
            StartProjectionRepairInput::new("workspace-1", "doc-1"),
            &mut ids,
            &mut repository,
        )
        .unwrap();

    assert_eq!(output.operation().operation_id().as_str(), "repair-1");
    assert_eq!(output.operation().state(), ProjectionRepairState::Queued);
    assert_eq!(repository.values.len(), 1);
}

#[test]
fn status_is_workspace_scoped_and_returns_explicit_progress() {
    let mut repository = FakeRepository::default();
    let mut ids = FakeIds::new("repair-1");
    StartProjectionRepairUsecase::new()
        .execute(
            StartProjectionRepairInput::new("workspace-1", "doc-1"),
            &mut ids,
            &mut repository,
        )
        .unwrap();
    let output = GetProjectionRepairStatusUsecase::new()
        .execute(
            GetProjectionRepairStatusInput::new("workspace-1", "repair-1"),
            &repository,
        )
        .unwrap();
    assert_eq!(output.state(), ProjectionRepairState::Queued);
    assert_eq!(output.completed_units(), 0);
    assert_eq!(output.total_units(), 3);
    assert_eq!(
        GetProjectionRepairStatusUsecase::new().execute(
            GetProjectionRepairStatusInput::new("workspace-2", "repair-1"),
            &repository,
        ),
        Err(ProjectionRepairUsecaseError::NotFound)
    );
}

#[test]
fn cancel_and_retry_use_domain_transition_with_expected_state_guard() {
    let mut repository = FakeRepository::default();
    let running = operation("repair-1")
        .transition(ProjectionRepairEvent::Start)
        .unwrap()
        .into_operation();
    repository.create(running.clone()).unwrap();
    let cancelled = CancelProjectionRepairUsecase::new()
        .execute(
            CancelProjectionRepairInput::new("workspace-1", "repair-1"),
            &mut repository,
        )
        .unwrap();
    assert_eq!(
        cancelled.operation().state(),
        ProjectionRepairState::CancelPending
    );
    assert_eq!(cancelled.product_log_event(), None);

    let failed = operation("repair-2")
        .transition(ProjectionRepairEvent::Start)
        .unwrap()
        .into_operation()
        .transition(ProjectionRepairEvent::FailedRetryable)
        .unwrap()
        .into_operation();
    repository.create(failed).unwrap();
    let retried = RetryProjectionRepairUsecase::new()
        .execute(
            RetryProjectionRepairInput::new("workspace-1", "repair-2"),
            &mut repository,
        )
        .unwrap();
    assert_eq!(retried.operation().state(), ProjectionRepairState::Queued);
    assert_eq!(
        retried.product_log_event(),
        Some("projection.reindex.retry_requested")
    );
}

#[test]
fn usecases_map_invalid_identity_duplicate_conflict_and_terminal_safely() {
    let mut repository = FakeRepository::default();
    let mut ids = FakeIds::new("repair-1");
    assert_eq!(
        StartProjectionRepairUsecase::new().execute(
            StartProjectionRepairInput::new("", "doc-1"),
            &mut ids,
            &mut repository,
        ),
        Err(ProjectionRepairUsecaseError::InvalidInput)
    );
    let mut ids = FakeIds::new("repair-1");
    StartProjectionRepairUsecase::new()
        .execute(
            StartProjectionRepairInput::new("workspace-1", "doc-1"),
            &mut ids,
            &mut repository,
        )
        .unwrap();
    let mut ids = FakeIds::new("repair-1");
    assert_eq!(
        StartProjectionRepairUsecase::new().execute(
            StartProjectionRepairInput::new("workspace-1", "doc-1"),
            &mut ids,
            &mut repository
        ),
        Err(ProjectionRepairUsecaseError::AlreadyExists)
    );
}

fn operation(id: &str) -> ProjectionRepairOperation {
    ProjectionRepairOperation::queued(
        ProjectionRepairOperationId::new(id).unwrap(),
        WorkspaceId::new("workspace-1").unwrap(),
        cabinet_domain::document::DocumentId::new("doc-1").unwrap(),
    )
}

struct FakeIds {
    value: String,
}
impl FakeIds {
    fn new(value: &str) -> Self {
        Self {
            value: value.into(),
        }
    }
}
impl ProjectionRepairOperationIdGenerator for FakeIds {
    fn next_id(&mut self) -> Result<String, ()> {
        Ok(self.value.clone())
    }
}

#[derive(Default)]
struct FakeRepository {
    values: BTreeMap<String, ProjectionRepairOperation>,
}
impl ProjectionRepairRepository for FakeRepository {
    fn create(
        &mut self,
        operation: ProjectionRepairOperation,
    ) -> Result<ProjectionRepairCreateOutcome, ProjectionRepairRepositoryError> {
        let key = operation.operation_id().as_str().to_string();
        if self.values.contains_key(&key) {
            return Ok(ProjectionRepairCreateOutcome::AlreadyExists);
        }
        self.values.insert(key, operation);
        Ok(ProjectionRepairCreateOutcome::Created)
    }
    fn get(
        &self,
        id: &ProjectionRepairOperationId,
    ) -> Result<Option<ProjectionRepairOperation>, ProjectionRepairRepositoryError> {
        Ok(self.values.get(id.as_str()).cloned())
    }
    fn replace(
        &mut self,
        operation: ProjectionRepairOperation,
        expected: ProjectionRepairState,
    ) -> Result<(), ProjectionRepairRepositoryError> {
        let current = self
            .values
            .get(operation.operation_id().as_str())
            .ok_or(ProjectionRepairRepositoryError::NotFound)?;
        if current.state() != expected {
            return Err(ProjectionRepairRepositoryError::Conflict);
        }
        self.values
            .insert(operation.operation_id().as_str().to_string(), operation);
        Ok(())
    }
    fn list_active(
        &self,
        _workspace: &WorkspaceId,
        _limit: usize,
    ) -> Result<Vec<ProjectionRepairOperation>, ProjectionRepairRepositoryError> {
        Ok(vec![])
    }
}
