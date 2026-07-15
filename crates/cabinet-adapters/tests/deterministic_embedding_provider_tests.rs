use cabinet_adapters::deterministic_embedding_provider::DeterministicEmbeddingProvider;
use cabinet_domain::embedding::EmbeddingInput;
use cabinet_domain::retrieval::{RetrievalSourceId, RetrievalSourceKind};
use cabinet_ports::embedding::{EmbeddingProviderError, EmbeddingProviderPort};

#[test]
fn deterministic_embedding_provider_returns_stable_vectors_for_same_input() {
    let provider = DeterministicEmbeddingProvider::new(4).expect("provider");
    let input = input("doc-1", "embedding-input:doc-1:paragraph:1");

    let first = provider.embed(&[input.clone()]).expect("first");
    let second = provider.embed(&[input]).expect("second");

    assert_eq!(first, second);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].source_id().as_str(), "doc-1");
    assert_eq!(
        first[0].vector_reference().as_str(),
        "vector:doc-1:deterministic"
    );
    assert_eq!(first[0].values().len(), 4);
}

#[test]
fn deterministic_embedding_provider_rejects_empty_input_and_invalid_dimension() {
    let provider = DeterministicEmbeddingProvider::new(4).expect("provider");

    assert_eq!(
        provider.embed(&[]),
        Err(EmbeddingProviderError::EmptyInputSet)
    );
    assert_eq!(
        DeterministicEmbeddingProvider::new(0),
        Err(EmbeddingProviderError::InvalidDimension),
    );
}

fn input(source_id: &str, reference: &str) -> EmbeddingInput {
    EmbeddingInput::new(
        RetrievalSourceId::new(source_id).expect("source id"),
        RetrievalSourceKind::Document,
        reference,
    )
    .expect("input")
}
