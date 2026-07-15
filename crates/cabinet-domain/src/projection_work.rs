use crate::document::DocumentId;
use crate::version::VersionId;
use crate::workspace::WorkspaceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionKind {
    Search,
    Links,
    Graph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionChangeKind {
    Created,
    Updated,
    Restored,
    Renamed,
    Deleted,
    AssetAttached,
    AssetDetached,
}

impl ProjectionChangeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Restored => "restored",
            Self::Renamed => "renamed",
            Self::Deleted => "deleted",
            Self::AssetAttached => "asset_attached",
            Self::AssetDetached => "asset_detached",
        }
    }
}

impl ProjectionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Links => "links",
            Self::Graph => "graph",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionWorkIdentity {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    version_id: VersionId,
    kind: ProjectionKind,
    change_kind: ProjectionChangeKind,
}

impl ProjectionWorkIdentity {
    pub fn new(
        workspace_id: WorkspaceId,
        document_id: DocumentId,
        version_id: VersionId,
        kind: ProjectionKind,
    ) -> Self {
        Self::for_change(
            workspace_id,
            document_id,
            version_id,
            kind,
            ProjectionChangeKind::Updated,
        )
    }

    pub fn for_change(
        workspace_id: WorkspaceId,
        document_id: DocumentId,
        version_id: VersionId,
        kind: ProjectionKind,
        change_kind: ProjectionChangeKind,
    ) -> Self {
        Self {
            workspace_id,
            document_id,
            version_id,
            kind,
            change_kind,
        }
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn version_id(&self) -> &VersionId {
        &self.version_id
    }

    pub const fn kind(&self) -> ProjectionKind {
        self.kind
    }

    pub const fn change_kind(&self) -> ProjectionChangeKind {
        self.change_kind
    }

    pub fn idempotency_key(&self) -> String {
        let workspace = self.workspace_id.as_str();
        let document = self.document_id.as_str();
        let version = self.version_id.as_str();
        let kind = self.kind.as_str();
        let base = format!(
            "{}:{workspace}{}:{document}{}:{version}{}:{kind}",
            workspace.len(),
            document.len(),
            version.len(),
            kind.len(),
        );
        if self.change_kind == ProjectionChangeKind::Updated {
            return base;
        }
        let change = self.change_kind.as_str();
        format!("{base}{}:{change}", change.len())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionWorkState {
    Pending,
    Indexing,
    Ready,
    RetryScheduled,
    Failed,
}

impl ProjectionWorkState {
    pub const fn is_resumable(self) -> bool {
        matches!(self, Self::Pending | Self::Indexing | Self::RetryScheduled)
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Ready | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionWorkEvent {
    Start,
    Succeeded,
    RetryScheduled,
    Failed,
    Interrupted,
    ReindexRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionWorkTransitionError {
    InvalidTransition,
    TerminalState,
}

impl ProjectionWorkTransitionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidTransition => "projection_work.invalid_transition",
            Self::TerminalState => "projection_work.terminal_state",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionWork {
    identity: ProjectionWorkIdentity,
    state: ProjectionWorkState,
    attempt: u32,
}

impl ProjectionWork {
    pub fn pending(identity: ProjectionWorkIdentity) -> Self {
        Self {
            identity,
            state: ProjectionWorkState::Pending,
            attempt: 0,
        }
    }

    pub fn restore(
        identity: ProjectionWorkIdentity,
        state: ProjectionWorkState,
        attempt: u32,
    ) -> Result<Self, ProjectionWorkTransitionError> {
        if (state == ProjectionWorkState::Pending && attempt != 0)
            || (state != ProjectionWorkState::Pending && attempt == 0)
        {
            return Err(ProjectionWorkTransitionError::InvalidTransition);
        }
        Ok(Self {
            identity,
            state,
            attempt,
        })
    }

    pub fn identity(&self) -> &ProjectionWorkIdentity {
        &self.identity
    }

    pub const fn state(&self) -> ProjectionWorkState {
        self.state
    }

    pub const fn attempt(&self) -> u32 {
        self.attempt
    }

    pub fn transition(
        &self,
        event: ProjectionWorkEvent,
    ) -> Result<Self, ProjectionWorkTransitionError> {
        if self.state.is_terminal() && event != ProjectionWorkEvent::ReindexRequested {
            return Err(ProjectionWorkTransitionError::TerminalState);
        }
        let (state, attempt) = match (self.state, event) {
            (
                ProjectionWorkState::Ready | ProjectionWorkState::Failed,
                ProjectionWorkEvent::ReindexRequested,
            ) => (ProjectionWorkState::Pending, 0),
            (
                ProjectionWorkState::Pending | ProjectionWorkState::RetryScheduled,
                ProjectionWorkEvent::Start,
            ) => (ProjectionWorkState::Indexing, self.attempt + 1),
            (ProjectionWorkState::Indexing, ProjectionWorkEvent::Succeeded) => {
                (ProjectionWorkState::Ready, self.attempt)
            }
            (ProjectionWorkState::Indexing, ProjectionWorkEvent::RetryScheduled) => {
                (ProjectionWorkState::RetryScheduled, self.attempt)
            }
            (ProjectionWorkState::Indexing, ProjectionWorkEvent::Failed) => {
                (ProjectionWorkState::Failed, self.attempt)
            }
            (ProjectionWorkState::Indexing, ProjectionWorkEvent::Interrupted) => {
                (ProjectionWorkState::RetryScheduled, self.attempt)
            }
            _ => return Err(ProjectionWorkTransitionError::InvalidTransition),
        };
        Ok(Self {
            identity: self.identity.clone(),
            state,
            attempt,
        })
    }
}
