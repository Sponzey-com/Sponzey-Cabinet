use crate::retrieval::{RetrievalSourceId, RetrievalSourceKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingInput {
    source_id: RetrievalSourceId,
    source_kind: RetrievalSourceKind,
    reference: String,
}

impl EmbeddingInput {
    pub fn new(
        source_id: RetrievalSourceId,
        source_kind: RetrievalSourceKind,
        reference: &str,
    ) -> Result<Self, EmbeddingError> {
        let reference = reference.trim();
        if !reference.starts_with("embedding-input:")
            || reference.len() <= "embedding-input:".len()
            || reference.chars().any(char::is_control)
        {
            return Err(EmbeddingError::InvalidInputReference);
        }
        Ok(Self {
            source_id,
            source_kind,
            reference: reference.to_string(),
        })
    }

    pub fn source_id(&self) -> &RetrievalSourceId {
        &self.source_id
    }

    pub const fn source_kind(&self) -> RetrievalSourceKind {
        self.source_kind
    }

    pub fn reference(&self) -> &str {
        &self.reference
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingVectorReference(String);

impl EmbeddingVectorReference {
    pub fn new(value: &str) -> Result<Self, EmbeddingError> {
        let trimmed = value.trim();
        if !trimmed.starts_with("vector:")
            || trimmed.len() <= "vector:".len()
            || trimmed.chars().any(char::is_control)
        {
            return Err(EmbeddingError::InvalidVectorReference);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingJobId(String);

impl EmbeddingJobId {
    pub fn new(value: &str) -> Result<Self, EmbeddingError> {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
            return Err(EmbeddingError::InvalidJobId);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingJob {
    id: EmbeddingJobId,
    inputs: Vec<EmbeddingInput>,
    state: EmbeddingJobState,
}

impl EmbeddingJob {
    pub fn new(id: EmbeddingJobId, inputs: Vec<EmbeddingInput>) -> Result<Self, EmbeddingError> {
        if inputs.is_empty() {
            return Err(EmbeddingError::EmptyInputSet);
        }
        Ok(Self {
            id,
            inputs,
            state: EmbeddingJobState::Queued,
        })
    }

    pub fn id(&self) -> &EmbeddingJobId {
        &self.id
    }

    pub fn inputs(&self) -> &[EmbeddingInput] {
        &self.inputs
    }

    pub fn input_count(&self) -> usize {
        self.inputs.len()
    }

    pub const fn state(&self) -> EmbeddingJobState {
        self.state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingJobState {
    Queued,
    PreparingInput,
    ProviderRequested,
    VectorStored,
    Completed,
    RetryScheduled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingJobEvent {
    PrepareInput,
    RequestProvider,
    StoreVector,
    Complete,
    ScheduleRetry,
    RetryProvider,
    Fail,
}

pub fn transition_embedding_job(
    state: EmbeddingJobState,
    event: EmbeddingJobEvent,
) -> Result<EmbeddingJobState, EmbeddingError> {
    use EmbeddingJobEvent as Event;
    use EmbeddingJobState as State;

    match (state, event) {
        (State::Queued, Event::PrepareInput) => Ok(State::PreparingInput),
        (State::PreparingInput, Event::RequestProvider) => Ok(State::ProviderRequested),
        (State::ProviderRequested, Event::StoreVector) => Ok(State::VectorStored),
        (State::VectorStored, Event::Complete) => Ok(State::Completed),
        (State::ProviderRequested, Event::ScheduleRetry) => Ok(State::RetryScheduled),
        (State::RetryScheduled, Event::RetryProvider) => Ok(State::ProviderRequested),
        (
            State::Queued
            | State::PreparingInput
            | State::ProviderRequested
            | State::RetryScheduled,
            Event::Fail,
        ) => Ok(State::Failed),
        _ => Err(EmbeddingError::InvalidJobTransition),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingError {
    InvalidInputReference,
    InvalidVectorReference,
    InvalidJobId,
    EmptyInputSet,
    InvalidJobTransition,
}

impl EmbeddingError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInputReference => "embedding.invalid_input_reference",
            Self::InvalidVectorReference => "embedding.invalid_vector_reference",
            Self::InvalidJobId => "embedding.invalid_job_id",
            Self::EmptyInputSet => "embedding.empty_input_set",
            Self::InvalidJobTransition => "embedding.invalid_job_transition",
        }
    }
}
