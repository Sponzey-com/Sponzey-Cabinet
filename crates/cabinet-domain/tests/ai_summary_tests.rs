use cabinet_domain::ai::{
    AiCitation, AiError, AiFreshnessStatus, AiRelatedDocumentRecommendation, AiSummaryReference,
    AiSummaryResult, AiSummaryTarget,
};
use cabinet_domain::retrieval::{
    CitationSpan, RetrievalFreshness, RetrievalSourceId, RetrievalSourceKind,
};

#[test]
fn ai_summary_result_requires_summary_reference_citation_and_freshness() {
    let result = AiSummaryResult::new(
        AiSummaryReference::new("summary:doc-1:current").expect("summary ref"),
        AiSummaryTarget::Document,
        vec![citation("doc-1", RetrievalFreshness::Current)],
        AiFreshnessStatus::Fresh,
    )
    .expect("summary");

    assert_eq!(result.summary_reference().as_str(), "summary:doc-1:current");
    assert_eq!(result.target(), AiSummaryTarget::Document);
    assert_eq!(result.citations().len(), 1);
    assert_eq!(result.freshness(), AiFreshnessStatus::Fresh);
    assert_eq!(
        AiSummaryReference::new("raw summary text"),
        Err(AiError::InvalidSummaryReference),
    );
    assert_eq!(
        AiSummaryResult::new(
            AiSummaryReference::new("summary:doc-1:current").expect("summary ref"),
            AiSummaryTarget::Document,
            vec![],
            AiFreshnessStatus::Fresh,
        ),
        Err(AiError::SummaryCitationRequired),
    );
}

#[test]
fn related_document_recommendation_is_reference_only() {
    let recommendation = AiRelatedDocumentRecommendation::new(
        source_id("doc-2"),
        RetrievalSourceKind::Document,
        citation("doc-2", RetrievalFreshness::Stale),
        RetrievalFreshness::Stale,
        "ai.recommendation.shared_context",
    )
    .expect("recommendation");

    assert_eq!(recommendation.source_id().as_str(), "doc-2");
    assert_eq!(recommendation.source_kind(), RetrievalSourceKind::Document);
    assert_eq!(
        recommendation.citation().freshness(),
        RetrievalFreshness::Stale
    );
    assert_eq!(recommendation.freshness(), RetrievalFreshness::Stale);
    assert_eq!(
        recommendation.reason_code(),
        "ai.recommendation.shared_context",
    );
    assert_eq!(
        AiRelatedDocumentRecommendation::new(
            source_id("doc-3"),
            RetrievalSourceKind::Document,
            citation("doc-3", RetrievalFreshness::Current),
            RetrievalFreshness::Current,
            "",
        ),
        Err(AiError::InvalidRecommendationReason),
    );
}

fn citation(source_id: &str, freshness: RetrievalFreshness) -> AiCitation {
    let source_id = source_id_from(source_id);
    AiCitation::new(
        CitationSpan::new(source_id, "paragraph:1", 0, 12).expect("citation"),
        freshness,
    )
}

fn source_id(value: &str) -> RetrievalSourceId {
    source_id_from(value)
}

fn source_id_from(value: &str) -> RetrievalSourceId {
    RetrievalSourceId::new(value).expect("source id")
}
