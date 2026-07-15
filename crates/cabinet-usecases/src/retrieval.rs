use cabinet_domain::retrieval::{
    ContextBudget, RetrievalCandidate, RetrievalQuery, RetrievalScope, RetrievalSourceKind,
};
use cabinet_ports::retrieval::{RetrievalPermissionPort, RetrievalPortError, RetrievalSourcePort};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRetrievalContextInput {
    workspace_id: String,
    actor_id: String,
    query: String,
    source_kinds: Vec<RetrievalSourceKind>,
    max_context_tokens: u32,
}

impl BuildRetrievalContextInput {
    pub fn new(
        workspace_id: &str,
        actor_id: &str,
        query: &str,
        source_kinds: Vec<RetrievalSourceKind>,
        max_context_tokens: u32,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            actor_id: actor_id.to_string(),
            query: query.to_string(),
            source_kinds,
            max_context_tokens,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRetrievalContextOutput {
    candidates: Vec<RetrievalCandidate>,
    stats: RetrievalContextStats,
}

impl BuildRetrievalContextOutput {
    pub fn candidates(&self) -> &[RetrievalCandidate] {
        &self.candidates
    }

    pub const fn stats(&self) -> RetrievalContextStats {
        self.stats
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetrievalContextStats {
    candidate_count: usize,
    filtered_count: usize,
    truncated_count: usize,
    selected_token_count: u32,
}

impl RetrievalContextStats {
    pub const fn new(
        candidate_count: usize,
        filtered_count: usize,
        truncated_count: usize,
        selected_token_count: u32,
    ) -> Self {
        Self {
            candidate_count,
            filtered_count,
            truncated_count,
            selected_token_count,
        }
    }

    pub const fn candidate_count(self) -> usize {
        self.candidate_count
    }

    pub const fn filtered_count(self) -> usize {
        self.filtered_count
    }

    pub const fn truncated_count(self) -> usize {
        self.truncated_count
    }

    pub const fn selected_token_count(self) -> u32 {
        self.selected_token_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildRetrievalContextUsecase;

impl BuildRetrievalContextUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: BuildRetrievalContextInput,
        source: &impl RetrievalSourcePort,
        permission: &impl RetrievalPermissionPort,
    ) -> Result<BuildRetrievalContextOutput, BuildRetrievalContextError> {
        let query = RetrievalQuery::new(&input.query)
            .map_err(|_| BuildRetrievalContextError::InvalidInput)?;
        let scope = RetrievalScope::new(&input.workspace_id, &input.actor_id, input.source_kinds)
            .map_err(|_| BuildRetrievalContextError::InvalidInput)?;
        let budget = ContextBudget::new(input.max_context_tokens)
            .map_err(|_| BuildRetrievalContextError::InvalidInput)?;
        let candidates = source
            .query_candidates(&query, &scope)
            .map_err(BuildRetrievalContextError::from_port_error)?;

        let candidate_count = candidates.len();
        let mut filtered_count = 0;
        let mut truncated_count = 0;
        let mut selected_token_count: u32 = 0;
        let mut selected = Vec::new();

        for candidate in candidates {
            let allowed = permission
                .allows_candidate(&scope, &candidate)
                .map_err(BuildRetrievalContextError::from_port_error)?;
            if !allowed {
                filtered_count += 1;
                continue;
            }

            let next_token_count =
                selected_token_count.saturating_add(candidate.estimated_tokens());
            if !budget.allows(next_token_count) {
                truncated_count += 1;
                continue;
            }
            selected_token_count = next_token_count;
            selected.push(candidate);
        }

        Ok(BuildRetrievalContextOutput {
            candidates: selected,
            stats: RetrievalContextStats::new(
                candidate_count,
                filtered_count,
                truncated_count,
                selected_token_count,
            ),
        })
    }
}

impl Default for BuildRetrievalContextUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildRetrievalContextError {
    InvalidInput,
    SourceUnavailable,
    PermissionUnavailable,
}

impl BuildRetrievalContextError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "retrieval_context.invalid_input",
            Self::SourceUnavailable => "retrieval_context.source_unavailable",
            Self::PermissionUnavailable => "retrieval_context.permission_unavailable",
        }
    }

    fn from_port_error(error: RetrievalPortError) -> Self {
        match error {
            RetrievalPortError::SourceUnavailable => Self::SourceUnavailable,
            RetrievalPortError::PermissionUnavailable => Self::PermissionUnavailable,
        }
    }
}
