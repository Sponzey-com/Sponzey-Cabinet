use crate::retrieval::{CitationSpan, RetrievalFreshness, RetrievalSourceId, RetrievalSourceKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiQuestion(String);

impl AiQuestion {
    pub fn new(value: &str) -> Result<Self, AiError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(AiError::EmptyQuestion);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(AiError::InvalidQuestion);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiAnswerReference(String);

impl AiAnswerReference {
    pub fn new(value: &str) -> Result<Self, AiError> {
        let trimmed = value.trim();
        if !trimmed.starts_with("answer:")
            || trimmed.len() <= "answer:".len()
            || trimmed.chars().any(char::is_control)
        {
            return Err(AiError::InvalidAnswerReference);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiCitation {
    span: CitationSpan,
    freshness: RetrievalFreshness,
}

impl AiCitation {
    pub const fn new(span: CitationSpan, freshness: RetrievalFreshness) -> Self {
        Self { span, freshness }
    }

    pub fn span(&self) -> &CitationSpan {
        &self.span
    }

    pub const fn freshness(&self) -> RetrievalFreshness {
        self.freshness
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiFreshnessStatus {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiSummaryReference(String);

impl AiSummaryReference {
    pub fn new(value: &str) -> Result<Self, AiError> {
        let trimmed = value.trim();
        if !trimmed.starts_with("summary:")
            || trimmed.len() <= "summary:".len()
            || trimmed.chars().any(char::is_control)
        {
            return Err(AiError::InvalidSummaryReference);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiSummaryTarget {
    Document,
    Section,
    ChangeSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiSummaryResult {
    summary_reference: AiSummaryReference,
    target: AiSummaryTarget,
    citations: Vec<AiCitation>,
    freshness: AiFreshnessStatus,
}

impl AiSummaryResult {
    pub fn new(
        summary_reference: AiSummaryReference,
        target: AiSummaryTarget,
        citations: Vec<AiCitation>,
        freshness: AiFreshnessStatus,
    ) -> Result<Self, AiError> {
        if citations.is_empty() {
            return Err(AiError::SummaryCitationRequired);
        }
        Ok(Self {
            summary_reference,
            target,
            citations,
            freshness,
        })
    }

    pub fn summary_reference(&self) -> &AiSummaryReference {
        &self.summary_reference
    }

    pub const fn target(&self) -> AiSummaryTarget {
        self.target
    }

    pub fn citations(&self) -> &[AiCitation] {
        &self.citations
    }

    pub const fn freshness(&self) -> AiFreshnessStatus {
        self.freshness
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiRelatedDocumentRecommendation {
    source_id: RetrievalSourceId,
    source_kind: RetrievalSourceKind,
    citation: AiCitation,
    freshness: RetrievalFreshness,
    reason_code: String,
}

impl AiRelatedDocumentRecommendation {
    pub fn new(
        source_id: RetrievalSourceId,
        source_kind: RetrievalSourceKind,
        citation: AiCitation,
        freshness: RetrievalFreshness,
        reason_code: &str,
    ) -> Result<Self, AiError> {
        let reason_code = reason_code.trim();
        if reason_code.is_empty() || reason_code.chars().any(char::is_control) {
            return Err(AiError::InvalidRecommendationReason);
        }
        Ok(Self {
            source_id,
            source_kind,
            citation,
            freshness,
            reason_code: reason_code.to_string(),
        })
    }

    pub fn source_id(&self) -> &RetrievalSourceId {
        &self.source_id
    }

    pub const fn source_kind(&self) -> RetrievalSourceKind {
        self.source_kind
    }

    pub fn citation(&self) -> &AiCitation {
        &self.citation
    }

    pub const fn freshness(&self) -> RetrievalFreshness {
        self.freshness
    }

    pub fn reason_code(&self) -> &str {
        &self.reason_code
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiRefusal {
    reason_code: String,
}

impl AiRefusal {
    pub fn new(reason_code: &str) -> Result<Self, AiError> {
        let trimmed = reason_code.trim();
        if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
            return Err(AiError::InvalidRefusalReason);
        }
        Ok(Self {
            reason_code: trimmed.to_string(),
        })
    }

    pub fn reason_code(&self) -> &str {
        &self.reason_code
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiAnswerResult {
    Completed {
        answer_reference: AiAnswerReference,
        citations: Vec<AiCitation>,
        freshness: AiFreshnessStatus,
    },
    Refused {
        refusal: AiRefusal,
    },
}

impl AiAnswerResult {
    pub fn completed(
        answer_reference: AiAnswerReference,
        citations: Vec<AiCitation>,
        freshness: AiFreshnessStatus,
    ) -> Result<Self, AiError> {
        if citations.is_empty() {
            return Err(AiError::CitationRequired);
        }
        Ok(Self::Completed {
            answer_reference,
            citations,
            freshness,
        })
    }

    pub const fn refused(refusal: AiRefusal) -> Self {
        Self::Refused { refusal }
    }

    pub fn answer_reference(&self) -> &AiAnswerReference {
        match self {
            Self::Completed {
                answer_reference, ..
            } => answer_reference,
            Self::Refused { .. } => panic!("refused answer has no answer reference"),
        }
    }

    pub fn citations(&self) -> &[AiCitation] {
        match self {
            Self::Completed { citations, .. } => citations,
            Self::Refused { .. } => &[],
        }
    }

    pub const fn freshness(&self) -> AiFreshnessStatus {
        match self {
            Self::Completed { freshness, .. } => *freshness,
            Self::Refused { .. } => AiFreshnessStatus::Unknown,
        }
    }

    pub const fn refusal(&self) -> Option<&AiRefusal> {
        match self {
            Self::Completed { .. } => None,
            Self::Refused { refusal } => Some(refusal),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiAnswerJobId(String);

impl AiAnswerJobId {
    pub fn new(value: &str) -> Result<Self, AiError> {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
            return Err(AiError::InvalidAnswerJobId);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiAnswerJob {
    id: AiAnswerJobId,
    question: AiQuestion,
    state: AiAnswerJobState,
}

impl AiAnswerJob {
    pub fn new(id: AiAnswerJobId, question: AiQuestion) -> Result<Self, AiError> {
        Ok(Self {
            id,
            question,
            state: AiAnswerJobState::Queued,
        })
    }

    pub fn id(&self) -> &AiAnswerJobId {
        &self.id
    }

    pub fn question(&self) -> &AiQuestion {
        &self.question
    }

    pub const fn state(&self) -> AiAnswerJobState {
        self.state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiAnswerJobState {
    Queued,
    RetrievalPreparing,
    ProviderRequested,
    CitationValidating,
    Completed,
    Refused,
    RetryScheduled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiAnswerJobEvent {
    PrepareRetrieval,
    RequestProvider,
    ValidateCitations,
    Complete,
    Refuse,
    ScheduleRetry,
    RetryProvider,
    Fail,
}

pub fn transition_ai_answer_job(
    state: AiAnswerJobState,
    event: AiAnswerJobEvent,
) -> Result<AiAnswerJobState, AiError> {
    use AiAnswerJobEvent as Event;
    use AiAnswerJobState as State;

    match (state, event) {
        (State::Queued, Event::PrepareRetrieval) => Ok(State::RetrievalPreparing),
        (State::RetrievalPreparing, Event::RequestProvider) => Ok(State::ProviderRequested),
        (State::ProviderRequested, Event::ValidateCitations) => Ok(State::CitationValidating),
        (State::CitationValidating, Event::Complete) => Ok(State::Completed),
        (State::CitationValidating, Event::Refuse) => Ok(State::Refused),
        (State::ProviderRequested, Event::ScheduleRetry) => Ok(State::RetryScheduled),
        (State::RetryScheduled, Event::RetryProvider) => Ok(State::ProviderRequested),
        (
            State::Queued
            | State::RetrievalPreparing
            | State::ProviderRequested
            | State::CitationValidating
            | State::RetryScheduled,
            Event::Fail,
        ) => Ok(State::Failed),
        _ => Err(AiError::InvalidAnswerJobTransition),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiError {
    EmptyQuestion,
    InvalidQuestion,
    InvalidAnswerReference,
    CitationRequired,
    InvalidSummaryReference,
    SummaryCitationRequired,
    InvalidRecommendationReason,
    InvalidRefusalReason,
    InvalidAnswerJobId,
    InvalidAnswerJobTransition,
}

impl AiError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyQuestion => "ai.empty_question",
            Self::InvalidQuestion => "ai.invalid_question",
            Self::InvalidAnswerReference => "ai.invalid_answer_reference",
            Self::CitationRequired => "ai.citation_required",
            Self::InvalidSummaryReference => "ai.invalid_summary_reference",
            Self::SummaryCitationRequired => "ai.summary_citation_required",
            Self::InvalidRecommendationReason => "ai.invalid_recommendation_reason",
            Self::InvalidRefusalReason => "ai.invalid_refusal_reason",
            Self::InvalidAnswerJobId => "ai.invalid_answer_job_id",
            Self::InvalidAnswerJobTransition => "ai.invalid_answer_job_transition",
        }
    }
}
