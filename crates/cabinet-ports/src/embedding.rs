use cabinet_domain::embedding::{EmbeddingInput, EmbeddingVectorReference};
use cabinet_domain::retrieval::{RetrievalSourceId, RetrievalSourceKind};

pub trait EmbeddingProviderPort {
    fn embed(
        &self,
        inputs: &[EmbeddingInput],
    ) -> Result<Vec<EmbeddingVector>, EmbeddingProviderError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingProviderError {
    EmptyInputSet,
    InvalidDimension,
    ProviderUnavailable,
}

impl EmbeddingProviderError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyInputSet => "embedding_provider.empty_input_set",
            Self::InvalidDimension => "embedding_provider.invalid_dimension",
            Self::ProviderUnavailable => "embedding_provider.provider_unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingVector {
    source_id: RetrievalSourceId,
    source_kind: RetrievalSourceKind,
    vector_reference: EmbeddingVectorReference,
    values: Vec<i32>,
}

impl EmbeddingVector {
    pub fn new(
        source_id: RetrievalSourceId,
        source_kind: RetrievalSourceKind,
        vector_reference: EmbeddingVectorReference,
        values: Vec<i32>,
    ) -> Result<Self, VectorIndexError> {
        if values.is_empty() {
            return Err(VectorIndexError::EmptyVector);
        }
        Ok(Self {
            source_id,
            source_kind,
            vector_reference,
            values,
        })
    }

    pub fn source_id(&self) -> &RetrievalSourceId {
        &self.source_id
    }

    pub const fn source_kind(&self) -> RetrievalSourceKind {
        self.source_kind
    }

    pub fn vector_reference(&self) -> &EmbeddingVectorReference {
        &self.vector_reference
    }

    pub fn values(&self) -> &[i32] {
        &self.values
    }
}

pub trait VectorIndexPort {
    fn upsert_vector(&mut self, entry: VectorIndexEntry) -> Result<(), VectorIndexError>;

    fn search_similar(
        &self,
        query: VectorSearchQuery,
    ) -> Result<Vec<VectorSearchResult>, VectorIndexError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorIndexEntry {
    vector: EmbeddingVector,
}

impl VectorIndexEntry {
    pub fn new(vector: EmbeddingVector) -> Self {
        Self { vector }
    }

    pub fn vector(&self) -> &EmbeddingVector {
        &self.vector
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorSearchQuery {
    query_vector: EmbeddingVector,
    source_kinds: Vec<RetrievalSourceKind>,
    limit: usize,
}

impl VectorSearchQuery {
    pub fn new(
        query_vector: EmbeddingVector,
        source_kinds: Vec<RetrievalSourceKind>,
        limit: usize,
    ) -> Result<Self, VectorIndexError> {
        if source_kinds.is_empty() {
            return Err(VectorIndexError::EmptySourceKinds);
        }
        if limit == 0 {
            return Err(VectorIndexError::InvalidLimit);
        }
        Ok(Self {
            query_vector,
            source_kinds,
            limit,
        })
    }

    pub fn query_vector(&self) -> &EmbeddingVector {
        &self.query_vector
    }

    pub fn source_kinds(&self) -> &[RetrievalSourceKind] {
        &self.source_kinds
    }

    pub const fn limit(&self) -> usize {
        self.limit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorSearchResult {
    source_id: RetrievalSourceId,
    source_kind: RetrievalSourceKind,
    vector_reference: EmbeddingVectorReference,
    score: i64,
}

impl VectorSearchResult {
    pub fn new(
        source_id: RetrievalSourceId,
        source_kind: RetrievalSourceKind,
        vector_reference: EmbeddingVectorReference,
        score: i64,
    ) -> Self {
        Self {
            source_id,
            source_kind,
            vector_reference,
            score,
        }
    }

    pub fn source_id(&self) -> &RetrievalSourceId {
        &self.source_id
    }

    pub const fn source_kind(&self) -> RetrievalSourceKind {
        self.source_kind
    }

    pub fn vector_reference(&self) -> &EmbeddingVectorReference {
        &self.vector_reference
    }

    pub const fn score(&self) -> i64 {
        self.score
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorIndexError {
    EmptyVector,
    EmptySourceKinds,
    InvalidLimit,
    IndexUnavailable,
}

impl VectorIndexError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyVector => "vector_index.empty_vector",
            Self::EmptySourceKinds => "vector_index.empty_source_kinds",
            Self::InvalidLimit => "vector_index.invalid_limit",
            Self::IndexUnavailable => "vector_index.index_unavailable",
        }
    }
}
