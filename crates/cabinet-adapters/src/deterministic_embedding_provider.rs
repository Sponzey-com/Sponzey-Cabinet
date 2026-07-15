use cabinet_domain::embedding::{EmbeddingInput, EmbeddingVectorReference};
use cabinet_ports::embedding::{EmbeddingProviderError, EmbeddingProviderPort, EmbeddingVector};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeterministicEmbeddingProvider {
    dimension: usize,
}

impl DeterministicEmbeddingProvider {
    pub const fn new(dimension: usize) -> Result<Self, EmbeddingProviderError> {
        if dimension == 0 {
            return Err(EmbeddingProviderError::InvalidDimension);
        }
        Ok(Self { dimension })
    }
}

impl EmbeddingProviderPort for DeterministicEmbeddingProvider {
    fn embed(
        &self,
        inputs: &[EmbeddingInput],
    ) -> Result<Vec<EmbeddingVector>, EmbeddingProviderError> {
        if inputs.is_empty() {
            return Err(EmbeddingProviderError::EmptyInputSet);
        }
        inputs
            .iter()
            .map(|input| {
                let vector_reference = EmbeddingVectorReference::new(&format!(
                    "vector:{}:deterministic",
                    input.source_id().as_str()
                ))
                .map_err(|_| EmbeddingProviderError::ProviderUnavailable)?;
                EmbeddingVector::new(
                    input.source_id().clone(),
                    input.source_kind(),
                    vector_reference,
                    deterministic_values(input.reference(), self.dimension),
                )
                .map_err(|_| EmbeddingProviderError::ProviderUnavailable)
            })
            .collect()
    }
}

fn deterministic_values(reference: &str, dimension: usize) -> Vec<i32> {
    let bytes = reference.as_bytes();
    (0..dimension)
        .map(|index| {
            bytes
                .iter()
                .enumerate()
                .map(|(byte_index, value)| {
                    ((*value as usize + 1) * (index + 1) * (byte_index + 1)) as i32
                })
                .sum::<i32>()
                % 997
        })
        .collect()
}
