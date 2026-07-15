use cabinet_domain::ai::{AiFreshnessStatus, AiSummaryReference, AiSummaryTarget};
use cabinet_domain::retrieval::{
    CitationSpan, RetrievalCandidate, RetrievalDecision, RetrievalFreshness,
    RetrievalSnippetReference, RetrievalSourceId, RetrievalSourceKind,
};
use cabinet_usecases::ai::{
    AiSummaryUsecaseError, SuggestRelatedDocumentsInput, SuggestRelatedDocumentsUsecase,
    SummarizeRetrievalContextInput, SummarizeRetrievalContextUsecase,
};

#[test]
fn summarize_retrieval_context_reflects_stale_source_freshness() {
    let output = SummarizeRetrievalContextUsecase::new()
        .execute(SummarizeRetrievalContextInput::new(
            AiSummaryTarget::Document,
            AiSummaryReference::new("summary:doc-1:current").expect("summary ref"),
            vec![
                candidate(
                    "doc-1",
                    RetrievalSourceKind::Document,
                    RetrievalFreshness::Current,
                ),
                candidate(
                    "doc-2",
                    RetrievalSourceKind::Document,
                    RetrievalFreshness::Stale,
                ),
            ],
        ))
        .expect("summary");

    assert_eq!(output.target(), AiSummaryTarget::Document);
    assert_eq!(output.citations().len(), 2);
    assert_eq!(output.freshness(), AiFreshnessStatus::Stale);
}

#[test]
fn summarize_retrieval_context_rejects_empty_context() {
    let error = SummarizeRetrievalContextUsecase::new()
        .execute(SummarizeRetrievalContextInput::new(
            AiSummaryTarget::Section,
            AiSummaryReference::new("summary:section-1:current").expect("summary ref"),
            vec![],
        ))
        .expect_err("empty context");

    assert_eq!(error, AiSummaryUsecaseError::NoRetrievalContext);
    assert_eq!(error.code(), "ai_summary.no_retrieval_context");
}

#[test]
fn suggest_related_documents_returns_document_candidates_only_and_applies_limit() {
    let output = SuggestRelatedDocumentsUsecase::new()
        .execute(
            SuggestRelatedDocumentsInput::new(
                vec![
                    candidate(
                        "doc-1",
                        RetrievalSourceKind::Document,
                        RetrievalFreshness::Current,
                    ),
                    candidate(
                        "canvas-1",
                        RetrievalSourceKind::CanvasNode,
                        RetrievalFreshness::Current,
                    ),
                    candidate(
                        "doc-2",
                        RetrievalSourceKind::Document,
                        RetrievalFreshness::Stale,
                    ),
                ],
                1,
            )
            .expect("input"),
        )
        .expect("recommendations");

    assert_eq!(output.recommendations().len(), 1);
    assert_eq!(output.recommendations()[0].source_id().as_str(), "doc-1");
    assert_eq!(
        output.recommendations()[0].reason_code(),
        "ai.recommendation.shared_context",
    );
}

#[test]
fn suggest_related_documents_rejects_invalid_limit_and_missing_document_candidates() {
    let invalid_limit = SuggestRelatedDocumentsInput::new(
        vec![candidate(
            "doc-1",
            RetrievalSourceKind::Document,
            RetrievalFreshness::Current,
        )],
        0,
    )
    .expect_err("invalid limit");

    assert_eq!(invalid_limit, AiSummaryUsecaseError::InvalidLimit);
    assert_eq!(invalid_limit.code(), "ai_summary.invalid_limit");

    let no_documents = SuggestRelatedDocumentsUsecase::new()
        .execute(
            SuggestRelatedDocumentsInput::new(
                vec![candidate(
                    "canvas-1",
                    RetrievalSourceKind::CanvasNode,
                    RetrievalFreshness::Current,
                )],
                3,
            )
            .expect("input"),
        )
        .expect_err("no documents");

    assert_eq!(no_documents, AiSummaryUsecaseError::NoRecommendations);
    assert_eq!(no_documents.code(), "ai_summary.no_recommendations");
}

fn candidate(
    source_id: &str,
    source_kind: RetrievalSourceKind,
    freshness: RetrievalFreshness,
) -> RetrievalCandidate {
    let source_id = RetrievalSourceId::new(source_id).expect("source id");
    RetrievalCandidate::new(
        source_id.clone(),
        source_kind,
        RetrievalDecision::Allowed,
        CitationSpan::new(source_id.clone(), "paragraph:1", 0, 12).expect("citation"),
        RetrievalSnippetReference::new(&format!("snippet:{}:paragraph:1", source_id.as_str()))
            .expect("snippet"),
        freshness,
        20,
    )
    .expect("candidate")
}
