use cabinet_adapters::local_vector_index::LocalVectorIndex;
use cabinet_domain::embedding::EmbeddingVectorReference;
use cabinet_domain::retrieval::{RetrievalSourceId, RetrievalSourceKind};
use cabinet_ports::embedding::{
    EmbeddingVector, VectorIndexEntry, VectorIndexError, VectorIndexPort, VectorSearchQuery,
};

#[test]
fn local_vector_index_returns_results_ordered_by_similarity_score() {
    let mut index = LocalVectorIndex::default();
    index
        .upsert_vector(entry("doc-1", RetrievalSourceKind::Document, vec![10, 0]))
        .expect("upsert doc");
    index
        .upsert_vector(entry("doc-2", RetrievalSourceKind::Document, vec![1, 0]))
        .expect("upsert doc2");

    let results = index
        .search_similar(
            VectorSearchQuery::new(
                EmbeddingVector::new(
                    RetrievalSourceId::new("query").expect("query id"),
                    RetrievalSourceKind::Document,
                    EmbeddingVectorReference::new("vector:query:deterministic").expect("vector"),
                    vec![9, 0],
                )
                .expect("query vector"),
                vec![RetrievalSourceKind::Document],
                2,
            )
            .expect("query"),
        )
        .expect("search");

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].source_id().as_str(), "doc-1");
    assert!(results[0].score() > results[1].score());
}

#[test]
fn local_vector_index_applies_source_kind_filter_and_limit() {
    let mut index = LocalVectorIndex::default();
    index
        .upsert_vector(entry("doc-1", RetrievalSourceKind::Document, vec![10, 0]))
        .expect("upsert doc");
    index
        .upsert_vector(entry(
            "canvas-1",
            RetrievalSourceKind::CanvasNode,
            vec![10, 0],
        ))
        .expect("upsert canvas");

    let results = index
        .search_similar(
            VectorSearchQuery::new(
                EmbeddingVector::new(
                    RetrievalSourceId::new("query").expect("query id"),
                    RetrievalSourceKind::Document,
                    EmbeddingVectorReference::new("vector:query:deterministic").expect("vector"),
                    vec![10, 0],
                )
                .expect("query vector"),
                vec![RetrievalSourceKind::CanvasNode],
                1,
            )
            .expect("query"),
        )
        .expect("search");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source_kind(), RetrievalSourceKind::CanvasNode);
}

#[test]
fn vector_index_contract_rejects_empty_vector_and_invalid_limit() {
    assert_eq!(
        EmbeddingVector::new(
            RetrievalSourceId::new("doc-1").expect("source id"),
            RetrievalSourceKind::Document,
            EmbeddingVectorReference::new("vector:doc-1:deterministic").expect("vector"),
            vec![],
        ),
        Err(VectorIndexError::EmptyVector),
    );

    let query_vector = EmbeddingVector::new(
        RetrievalSourceId::new("query").expect("source id"),
        RetrievalSourceKind::Document,
        EmbeddingVectorReference::new("vector:query:deterministic").expect("vector"),
        vec![1],
    )
    .expect("query vector");

    assert_eq!(
        VectorSearchQuery::new(query_vector, vec![RetrievalSourceKind::Document], 0),
        Err(VectorIndexError::InvalidLimit),
    );
}

fn entry(source_id: &str, source_kind: RetrievalSourceKind, values: Vec<i32>) -> VectorIndexEntry {
    let source_id = RetrievalSourceId::new(source_id).expect("source id");
    let reference = format!("vector:{}:deterministic", source_id.as_str());
    VectorIndexEntry::new(
        EmbeddingVector::new(
            source_id,
            source_kind,
            EmbeddingVectorReference::new(&reference).expect("vector reference"),
            values,
        )
        .expect("vector"),
    )
}
