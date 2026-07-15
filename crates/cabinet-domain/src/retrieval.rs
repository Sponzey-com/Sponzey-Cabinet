#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalQuery(String);

impl RetrievalQuery {
    pub fn new(value: &str) -> Result<Self, RetrievalError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(RetrievalError::EmptyQuery);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(RetrievalError::InvalidQuery);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalScope {
    workspace_id: String,
    actor_id: String,
    source_kinds: Vec<RetrievalSourceKind>,
}

impl RetrievalScope {
    pub fn new(
        workspace_id: &str,
        actor_id: &str,
        source_kinds: Vec<RetrievalSourceKind>,
    ) -> Result<Self, RetrievalError> {
        let workspace_id = normalize_scope_value(workspace_id)?;
        let actor_id = normalize_scope_value(actor_id)?;
        if source_kinds.is_empty() {
            return Err(RetrievalError::EmptySourceKinds);
        }
        Ok(Self {
            workspace_id,
            actor_id,
            source_kinds,
        })
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    pub fn source_kinds(&self) -> &[RetrievalSourceKind] {
        &self.source_kinds
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalSourceKind {
    Document,
    AssetMetadata,
    Comment,
    ReviewState,
    GraphRelation,
    CanvasNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalSourceId(String);

impl RetrievalSourceId {
    pub fn new(value: &str) -> Result<Self, RetrievalError> {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
            return Err(RetrievalError::InvalidSourceId);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalSnippetReference(String);

impl RetrievalSnippetReference {
    pub fn new(value: &str) -> Result<Self, RetrievalError> {
        let trimmed = value.trim();
        if !trimmed.starts_with("snippet:")
            || trimmed.chars().any(char::is_control)
            || trimmed.len() <= "snippet:".len()
        {
            return Err(RetrievalError::InvalidSnippetReference);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationSpan {
    source_id: RetrievalSourceId,
    reference: String,
    start_offset: u32,
    end_offset: u32,
}

impl CitationSpan {
    pub fn new(
        source_id: RetrievalSourceId,
        reference: &str,
        start_offset: u32,
        end_offset: u32,
    ) -> Result<Self, RetrievalError> {
        let reference = reference.trim();
        if reference.is_empty() {
            return Err(RetrievalError::MissingCitationReference);
        }
        if reference.chars().any(char::is_control) {
            return Err(RetrievalError::InvalidCitationReference);
        }
        if start_offset >= end_offset {
            return Err(RetrievalError::InvalidCitationRange);
        }
        Ok(Self {
            source_id,
            reference: reference.to_string(),
            start_offset,
            end_offset,
        })
    }

    pub fn source_id(&self) -> &RetrievalSourceId {
        &self.source_id
    }

    pub fn reference(&self) -> &str {
        &self.reference
    }

    pub const fn start_offset(&self) -> u32 {
        self.start_offset
    }

    pub const fn end_offset(&self) -> u32 {
        self.end_offset
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalFreshness {
    Current,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalDecision {
    Allowed,
    Denied { reason_code: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalCandidate {
    source_id: RetrievalSourceId,
    source_kind: RetrievalSourceKind,
    decision: RetrievalDecision,
    citation: CitationSpan,
    snippet_reference: RetrievalSnippetReference,
    freshness: RetrievalFreshness,
    estimated_tokens: u32,
}

impl RetrievalCandidate {
    pub fn new(
        source_id: RetrievalSourceId,
        source_kind: RetrievalSourceKind,
        decision: RetrievalDecision,
        citation: CitationSpan,
        snippet_reference: RetrievalSnippetReference,
        freshness: RetrievalFreshness,
        estimated_tokens: u32,
    ) -> Result<Self, RetrievalError> {
        if !matches!(decision, RetrievalDecision::Allowed) {
            return Err(RetrievalError::CandidateNotAllowed);
        }
        if estimated_tokens == 0 {
            return Err(RetrievalError::InvalidTokenEstimate);
        }
        Ok(Self {
            source_id,
            source_kind,
            decision,
            citation,
            snippet_reference,
            freshness,
            estimated_tokens,
        })
    }

    pub fn source_id(&self) -> &RetrievalSourceId {
        &self.source_id
    }

    pub const fn source_kind(&self) -> RetrievalSourceKind {
        self.source_kind
    }

    pub const fn decision(&self) -> RetrievalDecision {
        self.decision
    }

    pub fn citation(&self) -> &CitationSpan {
        &self.citation
    }

    pub fn snippet_reference(&self) -> &RetrievalSnippetReference {
        &self.snippet_reference
    }

    pub const fn freshness(&self) -> RetrievalFreshness {
        self.freshness
    }

    pub const fn estimated_tokens(&self) -> u32 {
        self.estimated_tokens
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextBudget {
    max_tokens: u32,
}

impl ContextBudget {
    pub const fn new(max_tokens: u32) -> Result<Self, RetrievalError> {
        if max_tokens == 0 {
            return Err(RetrievalError::InvalidContextBudget);
        }
        Ok(Self { max_tokens })
    }

    pub const fn max_tokens(self) -> u32 {
        self.max_tokens
    }

    pub const fn allows(self, estimated_tokens: u32) -> bool {
        estimated_tokens <= self.max_tokens
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalPipelineState {
    Received,
    Normalizing,
    SourceQuerying,
    PermissionFiltering,
    Ranking,
    ContextAssembled,
    SourceDegraded,
    PermissionUnavailable,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalPipelineEvent {
    Normalize,
    QuerySources,
    ApplyPermissions,
    Rank,
    AssembleContext,
    MarkSourceDegraded,
    MarkPermissionUnavailable,
    Fail,
}

pub fn transition_retrieval_pipeline(
    state: RetrievalPipelineState,
    event: RetrievalPipelineEvent,
) -> Result<RetrievalPipelineState, RetrievalError> {
    use RetrievalPipelineEvent as Event;
    use RetrievalPipelineState as State;

    match (state, event) {
        (State::Received, Event::Normalize) => Ok(State::Normalizing),
        (State::Normalizing, Event::QuerySources) => Ok(State::SourceQuerying),
        (State::SourceQuerying, Event::ApplyPermissions) => Ok(State::PermissionFiltering),
        (State::PermissionFiltering, Event::Rank) => Ok(State::Ranking),
        (State::Ranking, Event::AssembleContext) => Ok(State::ContextAssembled),
        (State::SourceQuerying, Event::MarkSourceDegraded) => Ok(State::SourceDegraded),
        (State::PermissionFiltering, Event::MarkPermissionUnavailable) => {
            Ok(State::PermissionUnavailable)
        }
        (
            State::Received
            | State::Normalizing
            | State::SourceQuerying
            | State::PermissionFiltering
            | State::Ranking
            | State::SourceDegraded,
            Event::Fail,
        ) => Ok(State::Failed),
        _ => Err(RetrievalError::InvalidPipelineTransition),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalError {
    EmptyQuery,
    InvalidQuery,
    EmptyScopeValue,
    EmptySourceKinds,
    InvalidSourceId,
    InvalidSnippetReference,
    MissingCitationReference,
    InvalidCitationReference,
    InvalidCitationRange,
    CandidateNotAllowed,
    InvalidTokenEstimate,
    InvalidContextBudget,
    InvalidPipelineTransition,
}

impl RetrievalError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyQuery => "retrieval.empty_query",
            Self::InvalidQuery => "retrieval.invalid_query",
            Self::EmptyScopeValue => "retrieval.empty_scope_value",
            Self::EmptySourceKinds => "retrieval.empty_source_kinds",
            Self::InvalidSourceId => "retrieval.invalid_source_id",
            Self::InvalidSnippetReference => "retrieval.invalid_snippet_reference",
            Self::MissingCitationReference => "retrieval.missing_citation_reference",
            Self::InvalidCitationReference => "retrieval.invalid_citation_reference",
            Self::InvalidCitationRange => "retrieval.invalid_citation_range",
            Self::CandidateNotAllowed => "retrieval.candidate_not_allowed",
            Self::InvalidTokenEstimate => "retrieval.invalid_token_estimate",
            Self::InvalidContextBudget => "retrieval.invalid_context_budget",
            Self::InvalidPipelineTransition => "retrieval.invalid_pipeline_transition",
        }
    }
}

fn normalize_scope_value(value: &str) -> Result<String, RetrievalError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(RetrievalError::EmptyScopeValue);
    }
    Ok(trimmed.to_string())
}
