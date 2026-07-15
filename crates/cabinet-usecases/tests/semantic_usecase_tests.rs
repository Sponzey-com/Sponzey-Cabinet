use std::time::Instant;

use cabinet_domain::retrieval::{RetrievalSourceId, RetrievalSourceKind};
use cabinet_usecases::semantic::{
    HybridSearchError, HybridSearchInput, MergeHybridSearchUsecase, SearchMatch,
};

#[test]
fn hybrid_search_merge_dedupes_keyword_and_semantic_matches() {
    let output = MergeHybridSearchUsecase::new()
        .execute(
            HybridSearchInput::new(
                vec![search_match("doc-1", RetrievalSourceKind::Document, 30)],
                vec![
                    search_match("doc-1", RetrievalSourceKind::Document, 70),
                    search_match("canvas-1", RetrievalSourceKind::CanvasNode, 50),
                ],
                10,
            )
            .expect("input"),
        )
        .expect("merge");

    assert_eq!(output.results().len(), 2);
    assert_eq!(output.results()[0].source_id().as_str(), "doc-1");
    assert_eq!(output.results()[0].combined_score(), 100);
    assert!(output.results()[0].keyword_hit());
    assert!(output.results()[0].semantic_hit());
    assert_eq!(output.results()[1].source_id().as_str(), "canvas-1");
}

#[test]
fn hybrid_search_merge_applies_limit_and_score_order() {
    let output = MergeHybridSearchUsecase::new()
        .execute(
            HybridSearchInput::new(
                vec![
                    search_match("doc-1", RetrievalSourceKind::Document, 10),
                    search_match("doc-2", RetrievalSourceKind::Document, 30),
                ],
                vec![search_match("doc-3", RetrievalSourceKind::Document, 20)],
                2,
            )
            .expect("input"),
        )
        .expect("merge");

    assert_eq!(output.results().len(), 2);
    assert_eq!(output.results()[0].source_id().as_str(), "doc-2");
    assert_eq!(output.results()[1].source_id().as_str(), "doc-3");
}

#[test]
fn hybrid_search_merge_rejects_empty_input_and_invalid_limit() {
    assert_eq!(
        HybridSearchInput::new(vec![], vec![], 10),
        Err(HybridSearchError::EmptyInput),
    );
    assert_eq!(
        HybridSearchInput::new(
            vec![search_match("doc-1", RetrievalSourceKind::Document, 10)],
            vec![],
            0,
        ),
        Err(HybridSearchError::InvalidLimit),
    );
    assert_eq!(
        SearchMatch::new(
            RetrievalSourceId::new("doc-1").expect("source id"),
            RetrievalSourceKind::Document,
            0,
        ),
        Err(HybridSearchError::InvalidScore),
    );
}

#[test]
fn hybrid_merge_completes_under_300ms_fixture() {
    let keyword = (0..1000)
        .map(|index| search_match(&format!("doc-{index}"), RetrievalSourceKind::Document, 10))
        .collect::<Vec<_>>();
    let semantic = (500..1500)
        .map(|index| search_match(&format!("doc-{index}"), RetrievalSourceKind::Document, 20))
        .collect::<Vec<_>>();
    let input = HybridSearchInput::new(keyword, semantic, 50).expect("input");

    let started = Instant::now();
    let output = MergeHybridSearchUsecase::new()
        .execute(input)
        .expect("merge");
    let elapsed = started.elapsed();

    assert_eq!(output.results().len(), 50);
    assert!(
        elapsed.as_millis() < 300,
        "hybrid merge should stay under 300ms fixture, observed {elapsed:?}",
    );
}

fn search_match(source_id: &str, source_kind: RetrievalSourceKind, score: u32) -> SearchMatch {
    SearchMatch::new(
        RetrievalSourceId::new(source_id).expect("source id"),
        source_kind,
        score,
    )
    .expect("match")
}
