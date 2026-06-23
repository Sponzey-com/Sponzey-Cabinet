const WORKSPACE_NAME_MAX_LEN: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    id: WorkspaceId,
    name: WorkspaceName,
    path: WorkspacePath,
}

impl Workspace {
    pub fn new(id: WorkspaceId, name: WorkspaceName, path: WorkspacePath) -> Self {
        Self { id, name, path }
    }

    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }

    pub fn name(&self) -> &WorkspaceName {
        &self.name
    }

    pub fn path(&self) -> &WorkspacePath {
        &self.path
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceId {
    value: String,
}

impl WorkspaceId {
    pub fn new(value: &str) -> Result<Self, WorkspaceError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(WorkspaceError::EmptyId);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceName {
    value: String,
}

impl WorkspaceName {
    pub fn new(value: &str) -> Result<Self, WorkspaceError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(WorkspaceError::EmptyName);
        }
        if trimmed.chars().count() > WORKSPACE_NAME_MAX_LEN {
            return Err(WorkspaceError::NameTooLong {
                max: WORKSPACE_NAME_MAX_LEN,
            });
        }
        if trimmed.chars().any(char::is_control) {
            return Err(WorkspaceError::InvalidNameCharacter);
        }

        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePath {
    value: String,
}

impl WorkspacePath {
    pub fn new(value: &str) -> Result<Self, WorkspaceError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(WorkspaceError::EmptyPathSegment);
        }
        if trimmed.starts_with('/') || trimmed.contains('\\') || trimmed.contains(':') {
            return Err(WorkspaceError::AbsoluteWorkspacePath);
        }

        for segment in trimmed.split('/') {
            if segment.is_empty() {
                return Err(WorkspaceError::EmptyPathSegment);
            }
            if segment == "." || segment == ".." {
                return Err(WorkspaceError::TraversalPathSegment);
            }
            if segment.chars().any(char::is_control) {
                return Err(WorkspaceError::InvalidPathCharacter);
            }
        }

        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceError {
    EmptyId,
    EmptyName,
    NameTooLong { max: usize },
    InvalidNameCharacter,
    AbsoluteWorkspacePath,
    EmptyPathSegment,
    TraversalPathSegment,
    InvalidPathCharacter,
}
