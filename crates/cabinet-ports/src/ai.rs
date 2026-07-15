use cabinet_domain::ai::{
    AiAnswerJobId, AiAnswerJobState, AiAnswerReference, AiAnswerResult, AiCitation,
    AiFreshnessStatus, AiQuestion, AiRefusal,
};
use cabinet_domain::retrieval::RetrievalSourceId;

pub trait AiProviderPort {
    fn generate_answer(
        &self,
        request: &AiProviderRequest,
        policy: &AiProviderPolicy,
    ) -> Result<AiProviderResponse, AiProviderError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiProviderRequest {
    question: AiQuestion,
    prompt_reference: AiPromptReference,
    citations: Vec<AiCitation>,
}

impl AiProviderRequest {
    pub fn new(
        question: AiQuestion,
        prompt_reference: AiPromptReference,
        citations: Vec<AiCitation>,
    ) -> Result<Self, AiProviderError> {
        if citations.is_empty() {
            return Err(AiProviderError::MissingCitationContext);
        }
        Ok(Self {
            question,
            prompt_reference,
            citations,
        })
    }

    pub fn question(&self) -> &AiQuestion {
        &self.question
    }

    pub fn prompt_reference(&self) -> &AiPromptReference {
        &self.prompt_reference
    }

    pub fn citations(&self) -> &[AiCitation] {
        &self.citations
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiPromptReference(String);

impl AiPromptReference {
    pub fn new(value: &str) -> Result<Self, AiProviderError> {
        let trimmed = value.trim();
        if !trimmed.starts_with("prompt:")
            || trimmed.len() <= "prompt:".len()
            || trimmed.chars().any(char::is_control)
        {
            return Err(AiProviderError::InvalidPromptReference);
        }
        let lowered = trimmed.to_ascii_lowercase();
        if lowered.contains("api_key")
            || lowered.contains("access_token")
            || lowered.contains("refresh_token")
            || lowered.contains("credential")
            || lowered.contains("secret")
        {
            return Err(AiProviderError::SensitivePromptReference);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiProviderPolicy {
    provider_name: String,
    model_name: String,
    timeout_ms: u32,
    max_answer_tokens: u32,
    retry_limit: u8,
}

impl AiProviderPolicy {
    pub fn new(
        provider_name: &str,
        model_name: &str,
        timeout_ms: u32,
        max_answer_tokens: u32,
        retry_limit: u8,
    ) -> Result<Self, AiProviderError> {
        let provider_name = normalized_policy_value(provider_name)?;
        let model_name = normalized_policy_value(model_name)?;
        if timeout_ms == 0 || max_answer_tokens == 0 {
            return Err(AiProviderError::InvalidPolicy);
        }
        Ok(Self {
            provider_name,
            model_name,
            timeout_ms,
            max_answer_tokens,
            retry_limit,
        })
    }

    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    pub const fn timeout_ms(&self) -> u32 {
        self.timeout_ms
    }

    pub const fn max_answer_tokens(&self) -> u32 {
        self.max_answer_tokens
    }

    pub const fn retry_limit(&self) -> u8 {
        self.retry_limit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiProviderResponse {
    Answered {
        answer_reference: AiAnswerReference,
        cited_source_ids: Vec<RetrievalSourceId>,
        freshness: AiFreshnessStatus,
        answer_length_bucket: u32,
    },
    Refused {
        refusal: AiRefusal,
    },
}

impl AiProviderResponse {
    pub fn answered(
        answer_reference: AiAnswerReference,
        cited_source_ids: Vec<RetrievalSourceId>,
        freshness: AiFreshnessStatus,
        answer_length_bucket: u32,
    ) -> Self {
        Self::Answered {
            answer_reference,
            cited_source_ids,
            freshness,
            answer_length_bucket,
        }
    }

    pub fn refused(refusal: AiRefusal) -> Self {
        Self::Refused { refusal }
    }

    pub fn answer_reference(&self) -> Option<&AiAnswerReference> {
        match self {
            Self::Answered {
                answer_reference, ..
            } => Some(answer_reference),
            Self::Refused { .. } => None,
        }
    }

    pub fn cited_source_ids(&self) -> &[RetrievalSourceId] {
        match self {
            Self::Answered {
                cited_source_ids, ..
            } => cited_source_ids,
            Self::Refused { .. } => &[],
        }
    }

    pub const fn freshness(&self) -> AiFreshnessStatus {
        match self {
            Self::Answered { freshness, .. } => *freshness,
            Self::Refused { .. } => AiFreshnessStatus::Unknown,
        }
    }

    pub const fn answer_length_bucket(&self) -> Option<u32> {
        match self {
            Self::Answered {
                answer_length_bucket,
                ..
            } => Some(*answer_length_bucket),
            Self::Refused { .. } => None,
        }
    }

    pub fn refusal(&self) -> Option<&AiRefusal> {
        match self {
            Self::Answered { .. } => None,
            Self::Refused { refusal } => Some(refusal),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiProviderError {
    InvalidPromptReference,
    SensitivePromptReference,
    MissingCitationContext,
    InvalidPolicy,
    Timeout,
    ProviderUnavailable,
}

impl AiProviderError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidPromptReference => "ai_provider.invalid_prompt_reference",
            Self::SensitivePromptReference => "ai_provider.sensitive_prompt_reference",
            Self::MissingCitationContext => "ai_provider.missing_citation_context",
            Self::InvalidPolicy => "ai_provider.invalid_policy",
            Self::Timeout => "ai_provider.timeout",
            Self::ProviderUnavailable => "ai_provider.provider_unavailable",
        }
    }

    pub const fn is_retryable(self) -> bool {
        matches!(self, Self::Timeout)
    }
}

pub trait AiAnswerResultStorePort {
    fn save_status(
        &mut self,
        job_id: &AiAnswerJobId,
        state: AiAnswerJobState,
    ) -> Result<(), AiAnswerStoreError>;

    fn save_result(
        &mut self,
        job_id: &AiAnswerJobId,
        result: AiAnswerResult,
    ) -> Result<(), AiAnswerStoreError>;

    fn get_status(
        &self,
        job_id: &AiAnswerJobId,
    ) -> Result<Option<AiAnswerJobState>, AiAnswerStoreError>;

    fn get_result(
        &self,
        job_id: &AiAnswerJobId,
    ) -> Result<Option<AiAnswerResult>, AiAnswerStoreError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiAnswerStoreError {
    StoreUnavailable,
}

impl AiAnswerStoreError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StoreUnavailable => "ai_answer_store.store_unavailable",
        }
    }
}

fn normalized_policy_value(value: &str) -> Result<String, AiProviderError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(AiProviderError::InvalidPolicy);
    }
    Ok(trimmed.to_string())
}
