use std::collections::HashMap;

use cabinet_domain::workspace::{Workspace, WorkspaceId};
use cabinet_ports::workspace_repository::{WorkspaceRepository, WorkspaceRepositoryError};
use cabinet_usecases::workspace::{
    CreateWorkspaceError, CreateWorkspaceInput, CreateWorkspaceProductEvent,
    CreateWorkspaceProductLogger, CreateWorkspaceUsecase,
};

#[derive(Default)]
struct FakeWorkspaceRepository {
    workspaces: HashMap<String, Workspace>,
}

impl WorkspaceRepository for FakeWorkspaceRepository {
    fn put_workspace(&mut self, workspace: Workspace) -> Result<(), WorkspaceRepositoryError> {
        let key = workspace.id().as_str().to_string();
        if self.workspaces.contains_key(&key) {
            return Err(WorkspaceRepositoryError::Conflict);
        }
        self.workspaces.insert(key, workspace);
        Ok(())
    }

    fn get_workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Option<Workspace>, WorkspaceRepositoryError> {
        Ok(self.workspaces.get(workspace_id.as_str()).cloned())
    }
}

#[derive(Default)]
struct FakeProductLogger {
    events: Vec<CreateWorkspaceProductEvent>,
}

impl CreateWorkspaceProductLogger for FakeProductLogger {
    fn write_product(&mut self, event: CreateWorkspaceProductEvent) {
        self.events.push(event);
    }
}

#[test]
fn create_workspace_persists_workspace_and_returns_explicit_output() {
    let mut repository = FakeWorkspaceRepository::default();
    let mut logger = FakeProductLogger::default();
    let usecase = CreateWorkspaceUsecase::new();

    let output = usecase
        .execute(
            CreateWorkspaceInput::new("workspace-1", "Research", "research"),
            &mut repository,
            &mut logger,
        )
        .expect("workspace should be created");

    assert_eq!(output.workspace().id().as_str(), "workspace-1");
    assert_eq!(output.workspace().name().as_str(), "Research");
    assert_eq!(
        repository
            .get_workspace(output.workspace().id())
            .expect("get workspace")
            .expect("workspace")
            .path()
            .as_str(),
        "research"
    );
    assert_eq!(
        logger.events,
        vec![CreateWorkspaceProductEvent::WorkspaceCreated {
            workspace_id: "workspace-1".to_string(),
        }]
    );
}

#[test]
fn create_workspace_reports_conflict_for_duplicate_workspace() {
    let mut repository = FakeWorkspaceRepository::default();
    let mut logger = FakeProductLogger::default();
    let usecase = CreateWorkspaceUsecase::new();

    usecase
        .execute(
            CreateWorkspaceInput::new("workspace-1", "Research", "research"),
            &mut repository,
            &mut logger,
        )
        .expect("first create");
    let error = usecase
        .execute(
            CreateWorkspaceInput::new("workspace-1", "Research", "research"),
            &mut repository,
            &mut logger,
        )
        .expect_err("duplicate must fail");

    assert_eq!(error, CreateWorkspaceError::WorkspaceAlreadyExists);
    assert_eq!(
        logger.events.last(),
        Some(&CreateWorkspaceProductEvent::UsecaseFailed {
            error_code: "workspace.already_exists",
        })
    );
}

#[test]
fn create_workspace_rejects_invalid_input_before_repository_write_and_logs_failure() {
    let mut repository = FakeWorkspaceRepository::default();
    let mut logger = FakeProductLogger::default();
    let usecase = CreateWorkspaceUsecase::new();

    let error = usecase
        .execute(
            CreateWorkspaceInput::new("workspace-1", " ", "research"),
            &mut repository,
            &mut logger,
        )
        .expect_err("invalid input must fail");

    assert_eq!(error, CreateWorkspaceError::InvalidWorkspaceInput);
    assert!(repository.workspaces.is_empty());
    assert_eq!(
        logger.events,
        vec![CreateWorkspaceProductEvent::UsecaseFailed {
            error_code: "workspace.invalid_input",
        }]
    );
}

#[test]
fn create_workspace_product_log_excludes_workspace_path() {
    let mut repository = FakeWorkspaceRepository::default();
    let mut logger = FakeProductLogger::default();
    let usecase = CreateWorkspaceUsecase::new();

    usecase
        .execute(
            CreateWorkspaceInput::new("workspace-1", "Research", "private/customer-path"),
            &mut repository,
            &mut logger,
        )
        .expect("workspace should be created");

    let rendered_log = format!("{:?}", logger.events);
    assert!(!rendered_log.contains("private/customer-path"));
}
