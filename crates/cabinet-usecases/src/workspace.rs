use cabinet_domain::workspace::{Workspace, WorkspaceId, WorkspaceName, WorkspacePath};
use cabinet_ports::workspace_repository::{WorkspaceRepository, WorkspaceRepositoryError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateWorkspaceInput {
    workspace_id: String,
    name: String,
    path: String,
}

impl CreateWorkspaceInput {
    pub fn new(workspace_id: &str, name: &str, path: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            name: name.to_string(),
            path: path.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateWorkspaceOutput {
    workspace: Workspace,
}

impl CreateWorkspaceOutput {
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateWorkspaceProductEvent {
    WorkspaceCreated { workspace_id: String },
    UsecaseFailed { error_code: &'static str },
}

pub trait CreateWorkspaceProductLogger {
    fn write_product(&mut self, event: CreateWorkspaceProductEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateWorkspaceUsecase;

impl CreateWorkspaceUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: CreateWorkspaceInput,
        repository: &mut impl WorkspaceRepository,
        product_logger: &mut impl CreateWorkspaceProductLogger,
    ) -> Result<CreateWorkspaceOutput, CreateWorkspaceError> {
        let workspace = match build_workspace(input) {
            Ok(workspace) => workspace,
            Err(error) => {
                product_logger.write_product(CreateWorkspaceProductEvent::UsecaseFailed {
                    error_code: error.code(),
                });
                return Err(error);
            }
        };

        if let Err(error) = repository.put_workspace(workspace.clone()) {
            let usecase_error = CreateWorkspaceError::from_repository_error(error);
            product_logger.write_product(CreateWorkspaceProductEvent::UsecaseFailed {
                error_code: usecase_error.code(),
            });
            return Err(usecase_error);
        }

        product_logger.write_product(CreateWorkspaceProductEvent::WorkspaceCreated {
            workspace_id: workspace.id().as_str().to_string(),
        });
        Ok(CreateWorkspaceOutput { workspace })
    }
}

impl Default for CreateWorkspaceUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateWorkspaceError {
    InvalidWorkspaceInput,
    WorkspaceAlreadyExists,
    StorageUnavailable,
}

impl CreateWorkspaceError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidWorkspaceInput => "workspace.invalid_input",
            Self::WorkspaceAlreadyExists => "workspace.already_exists",
            Self::StorageUnavailable => "workspace.storage_unavailable",
        }
    }

    fn from_repository_error(error: WorkspaceRepositoryError) -> Self {
        match error {
            WorkspaceRepositoryError::Conflict => Self::WorkspaceAlreadyExists,
            WorkspaceRepositoryError::StorageUnavailable => Self::StorageUnavailable,
        }
    }
}

fn build_workspace(input: CreateWorkspaceInput) -> Result<Workspace, CreateWorkspaceError> {
    let workspace_id = WorkspaceId::new(&input.workspace_id)
        .map_err(|_| CreateWorkspaceError::InvalidWorkspaceInput)?;
    let name =
        WorkspaceName::new(&input.name).map_err(|_| CreateWorkspaceError::InvalidWorkspaceInput)?;
    let path =
        WorkspacePath::new(&input.path).map_err(|_| CreateWorkspaceError::InvalidWorkspaceInput)?;
    Ok(Workspace::new(workspace_id, name, path))
}
