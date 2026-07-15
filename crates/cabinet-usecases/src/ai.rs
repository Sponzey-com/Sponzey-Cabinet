use cabinet_domain::ai::{
    AiAnswerJob, AiAnswerJobEvent, AiAnswerJobId, AiAnswerJobState, AiAnswerResult, AiCitation,
    AiError, AiFreshnessStatus, AiQuestion, AiRefusal, AiRelatedDocumentRecommendation,
    AiSummaryReference, AiSummaryResult, AiSummaryTarget, transition_ai_answer_job,
};
use cabinet_domain::retrieval::{RetrievalCandidate, RetrievalFreshness, RetrievalSourceKind};
use cabinet_ports::ai::{
    AiAnswerResultStorePort, AiAnswerStoreError, AiPromptReference, AiProviderError,
    AiProviderPolicy, AiProviderPort, AiProviderRequest, AiProviderResponse,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskKnowledgeBaseInput {
    job_id: AiAnswerJobId,
    question: AiQuestion,
    prompt_reference: AiPromptReference,
    candidates: Vec<RetrievalCandidate>,
    provider_policy: AiProviderPolicy,
}

impl AskKnowledgeBaseInput {
    pub fn new(
        job_id: AiAnswerJobId,
        question: AiQuestion,
        prompt_reference: AiPromptReference,
        candidates: Vec<RetrievalCandidate>,
        provider_policy: AiProviderPolicy,
    ) -> Result<Self, AskKnowledgeBaseError> {
        Ok(Self {
            job_id,
            question,
            prompt_reference,
            candidates,
            provider_policy,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildAiPromptReferenceInput {
    job_id: AiAnswerJobId,
    candidates: Vec<RetrievalCandidate>,
}

impl BuildAiPromptReferenceInput {
    pub fn new(job_id: AiAnswerJobId, candidates: Vec<RetrievalCandidate>) -> Self {
        Self { job_id, candidates }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildAiPromptReferenceUsecase;

impl BuildAiPromptReferenceUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: BuildAiPromptReferenceInput,
    ) -> Result<AiPromptReference, AskKnowledgeBaseError> {
        if input.candidates.is_empty() {
            return Err(AskKnowledgeBaseError::NoRetrievalContext);
        }
        AiPromptReference::new(&format!(
            "prompt:{}:retrieval-citations:{}",
            input.job_id.as_str(),
            input.candidates.len()
        ))
        .map_err(AskKnowledgeBaseError::from_provider_error)
    }
}

impl Default for BuildAiPromptReferenceUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummarizeRetrievalContextInput {
    target: AiSummaryTarget,
    summary_reference: AiSummaryReference,
    candidates: Vec<RetrievalCandidate>,
}

impl SummarizeRetrievalContextInput {
    pub fn new(
        target: AiSummaryTarget,
        summary_reference: AiSummaryReference,
        candidates: Vec<RetrievalCandidate>,
    ) -> Self {
        Self {
            target,
            summary_reference,
            candidates,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummarizeRetrievalContextUsecase;

impl SummarizeRetrievalContextUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: SummarizeRetrievalContextInput,
    ) -> Result<AiSummaryResult, AiSummaryUsecaseError> {
        if input.candidates.is_empty() {
            return Err(AiSummaryUsecaseError::NoRetrievalContext);
        }
        let citations = input
            .candidates
            .iter()
            .map(|candidate| AiCitation::new(candidate.citation().clone(), candidate.freshness()))
            .collect::<Vec<_>>();
        let freshness = summary_freshness(&input.candidates);
        AiSummaryResult::new(input.summary_reference, input.target, citations, freshness)
            .map_err(AiSummaryUsecaseError::from_domain_error)
    }
}

impl Default for SummarizeRetrievalContextUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestRelatedDocumentsInput {
    candidates: Vec<RetrievalCandidate>,
    limit: usize,
}

impl SuggestRelatedDocumentsInput {
    pub fn new(
        candidates: Vec<RetrievalCandidate>,
        limit: usize,
    ) -> Result<Self, AiSummaryUsecaseError> {
        if limit == 0 {
            return Err(AiSummaryUsecaseError::InvalidLimit);
        }
        Ok(Self { candidates, limit })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestRelatedDocumentsOutput {
    recommendations: Vec<AiRelatedDocumentRecommendation>,
}

impl SuggestRelatedDocumentsOutput {
    pub fn recommendations(&self) -> &[AiRelatedDocumentRecommendation] {
        &self.recommendations
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuggestRelatedDocumentsUsecase;

impl SuggestRelatedDocumentsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: SuggestRelatedDocumentsInput,
    ) -> Result<SuggestRelatedDocumentsOutput, AiSummaryUsecaseError> {
        let recommendations = input
            .candidates
            .into_iter()
            .filter(|candidate| candidate.source_kind() == RetrievalSourceKind::Document)
            .take(input.limit)
            .map(|candidate| {
                AiRelatedDocumentRecommendation::new(
                    candidate.source_id().clone(),
                    candidate.source_kind(),
                    AiCitation::new(candidate.citation().clone(), candidate.freshness()),
                    candidate.freshness(),
                    "ai.recommendation.shared_context",
                )
                .map_err(AiSummaryUsecaseError::from_domain_error)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if recommendations.is_empty() {
            return Err(AiSummaryUsecaseError::NoRecommendations);
        }
        Ok(SuggestRelatedDocumentsOutput { recommendations })
    }
}

impl Default for SuggestRelatedDocumentsUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiSummaryUsecaseError {
    InvalidInput,
    NoRetrievalContext,
    InvalidLimit,
    NoRecommendations,
}

impl AiSummaryUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "ai_summary.invalid_input",
            Self::NoRetrievalContext => "ai_summary.no_retrieval_context",
            Self::InvalidLimit => "ai_summary.invalid_limit",
            Self::NoRecommendations => "ai_summary.no_recommendations",
        }
    }

    fn from_domain_error(_error: AiError) -> Self {
        Self::InvalidInput
    }
}

fn summary_freshness(candidates: &[RetrievalCandidate]) -> AiFreshnessStatus {
    if candidates
        .iter()
        .any(|candidate| candidate.freshness() == RetrievalFreshness::Stale)
    {
        AiFreshnessStatus::Stale
    } else if candidates
        .iter()
        .any(|candidate| candidate.freshness() == RetrievalFreshness::Unknown)
    {
        AiFreshnessStatus::Unknown
    } else {
        AiFreshnessStatus::Fresh
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskKnowledgeBaseOutput {
    job_id: AiAnswerJobId,
    state: AiAnswerJobState,
    result: Option<AiAnswerResult>,
    citation_count: usize,
}

impl AskKnowledgeBaseOutput {
    pub fn job_id(&self) -> &AiAnswerJobId {
        &self.job_id
    }

    pub const fn state(&self) -> AiAnswerJobState {
        self.state
    }

    pub fn result(&self) -> Option<&AiAnswerResult> {
        self.result.as_ref()
    }

    pub const fn citation_count(&self) -> usize {
        self.citation_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AskKnowledgeBaseUsecase;

impl AskKnowledgeBaseUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: AskKnowledgeBaseInput,
        provider: &impl AiProviderPort,
        store: &mut impl AiAnswerResultStorePort,
    ) -> Result<AskKnowledgeBaseOutput, AskKnowledgeBaseError> {
        if input.candidates.is_empty() {
            return Err(AskKnowledgeBaseError::NoRetrievalContext);
        }

        let job = AiAnswerJob::new(input.job_id.clone(), input.question.clone())
            .map_err(AskKnowledgeBaseError::from_domain_error)?;
        store
            .save_status(job.id(), job.state())
            .map_err(AskKnowledgeBaseError::from_store_error)?;

        let retrieval_state =
            transition_ai_answer_job(job.state(), AiAnswerJobEvent::PrepareRetrieval)
                .map_err(AskKnowledgeBaseError::from_domain_error)?;
        store
            .save_status(job.id(), retrieval_state)
            .map_err(AskKnowledgeBaseError::from_store_error)?;

        let provider_state =
            transition_ai_answer_job(retrieval_state, AiAnswerJobEvent::RequestProvider)
                .map_err(AskKnowledgeBaseError::from_domain_error)?;
        store
            .save_status(job.id(), provider_state)
            .map_err(AskKnowledgeBaseError::from_store_error)?;

        let request = AiProviderRequest::new(
            input.question,
            input.prompt_reference,
            input
                .candidates
                .iter()
                .map(|candidate| {
                    AiCitation::new(candidate.citation().clone(), candidate.freshness())
                })
                .collect(),
        )
        .map_err(AskKnowledgeBaseError::from_provider_error)?;

        let response = match provider.generate_answer(&request, &input.provider_policy) {
            Ok(response) => response,
            Err(error) if error.is_retryable() => {
                let retry_state =
                    transition_ai_answer_job(provider_state, AiAnswerJobEvent::ScheduleRetry)
                        .map_err(AskKnowledgeBaseError::from_domain_error)?;
                store
                    .save_status(job.id(), retry_state)
                    .map_err(AskKnowledgeBaseError::from_store_error)?;
                return Ok(AskKnowledgeBaseOutput {
                    job_id: input.job_id,
                    state: retry_state,
                    result: None,
                    citation_count: 0,
                });
            }
            Err(error) => {
                let failed_state = transition_ai_answer_job(provider_state, AiAnswerJobEvent::Fail)
                    .map_err(AskKnowledgeBaseError::from_domain_error)?;
                store
                    .save_status(job.id(), failed_state)
                    .map_err(AskKnowledgeBaseError::from_store_error)?;
                return Err(AskKnowledgeBaseError::from_provider_error(error));
            }
        };

        let validating_state =
            transition_ai_answer_job(provider_state, AiAnswerJobEvent::ValidateCitations)
                .map_err(AskKnowledgeBaseError::from_domain_error)?;
        store
            .save_status(job.id(), validating_state)
            .map_err(AskKnowledgeBaseError::from_store_error)?;

        let (final_state, result, citation_count) =
            result_from_provider_response(response, &input.candidates, validating_state)?;
        store
            .save_result(job.id(), result.clone())
            .map_err(AskKnowledgeBaseError::from_store_error)?;
        store
            .save_status(job.id(), final_state)
            .map_err(AskKnowledgeBaseError::from_store_error)?;

        Ok(AskKnowledgeBaseOutput {
            job_id: input.job_id,
            state: final_state,
            result: Some(result),
            citation_count,
        })
    }
}

impl Default for AskKnowledgeBaseUsecase {
    fn default() -> Self {
        Self::new()
    }
}

fn result_from_provider_response(
    response: AiProviderResponse,
    candidates: &[RetrievalCandidate],
    validating_state: AiAnswerJobState,
) -> Result<(AiAnswerJobState, AiAnswerResult, usize), AskKnowledgeBaseError> {
    match response {
        AiProviderResponse::Answered {
            answer_reference,
            cited_source_ids,
            freshness,
            ..
        } => {
            let citations = cited_source_ids
                .iter()
                .filter_map(|source_id| {
                    candidates
                        .iter()
                        .find(|candidate| candidate.source_id() == source_id)
                        .map(|candidate| {
                            AiCitation::new(candidate.citation().clone(), candidate.freshness())
                        })
                })
                .collect::<Vec<_>>();
            if citations.is_empty() {
                return refusal_result(validating_state, "ai.refusal.no_valid_citation");
            }
            let citation_count = citations.len();
            let result = AiAnswerResult::completed(answer_reference, citations, freshness)
                .map_err(AskKnowledgeBaseError::from_domain_error)?;
            let state = transition_ai_answer_job(validating_state, AiAnswerJobEvent::Complete)
                .map_err(AskKnowledgeBaseError::from_domain_error)?;
            Ok((state, result, citation_count))
        }
        AiProviderResponse::Refused { refusal } => {
            let result = AiAnswerResult::refused(refusal);
            let state = transition_ai_answer_job(validating_state, AiAnswerJobEvent::Refuse)
                .map_err(AskKnowledgeBaseError::from_domain_error)?;
            Ok((state, result, 0))
        }
    }
}

fn refusal_result(
    validating_state: AiAnswerJobState,
    reason_code: &str,
) -> Result<(AiAnswerJobState, AiAnswerResult, usize), AskKnowledgeBaseError> {
    let refusal = AiRefusal::new(reason_code).map_err(AskKnowledgeBaseError::from_domain_error)?;
    let result = AiAnswerResult::refused(refusal);
    let state = transition_ai_answer_job(validating_state, AiAnswerJobEvent::Refuse)
        .map_err(AskKnowledgeBaseError::from_domain_error)?;
    Ok((state, result, 0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskKnowledgeBaseError {
    InvalidInput,
    NoRetrievalContext,
    ProviderTimeout,
    ProviderUnavailable,
    StoreUnavailable,
    InvalidTransition,
}

impl AskKnowledgeBaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "ai_answer.invalid_input",
            Self::NoRetrievalContext => "ai_answer.no_retrieval_context",
            Self::ProviderTimeout => "ai_answer.provider_timeout",
            Self::ProviderUnavailable => "ai_answer.provider_unavailable",
            Self::StoreUnavailable => "ai_answer.store_unavailable",
            Self::InvalidTransition => "ai_answer.invalid_transition",
        }
    }

    fn from_domain_error(error: AiError) -> Self {
        match error {
            AiError::EmptyQuestion
            | AiError::InvalidQuestion
            | AiError::InvalidAnswerReference
            | AiError::CitationRequired
            | AiError::InvalidSummaryReference
            | AiError::SummaryCitationRequired
            | AiError::InvalidRecommendationReason
            | AiError::InvalidRefusalReason
            | AiError::InvalidAnswerJobId => Self::InvalidInput,
            AiError::InvalidAnswerJobTransition => Self::InvalidTransition,
        }
    }

    fn from_provider_error(error: AiProviderError) -> Self {
        match error {
            AiProviderError::Timeout => Self::ProviderTimeout,
            AiProviderError::ProviderUnavailable => Self::ProviderUnavailable,
            AiProviderError::InvalidPromptReference
            | AiProviderError::SensitivePromptReference
            | AiProviderError::MissingCitationContext
            | AiProviderError::InvalidPolicy => Self::InvalidInput,
        }
    }

    fn from_store_error(error: AiAnswerStoreError) -> Self {
        match error {
            AiAnswerStoreError::StoreUnavailable => Self::StoreUnavailable,
        }
    }
}
