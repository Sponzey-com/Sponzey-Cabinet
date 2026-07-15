use cabinet_adapters::local_retrieval_source::{
    LocalRetrievalSource, LocalRetrievalSourceError, LocalRetrievalSourceRecord,
};
use cabinet_domain::retrieval::{
    CitationSpan, RetrievalCandidate, RetrievalDecision, RetrievalFreshness, RetrievalQuery,
    RetrievalScope, RetrievalSnippetReference, RetrievalSourceId, RetrievalSourceKind,
};
use cabinet_ports::retrieval::RetrievalSourcePort;

#[test]
fn local_retrieval_source_returns_matching_candidates_by_query_and_source_kind() {
    let source = LocalRetrievalSource::new(vec![
        record(
            "doc-1",
            RetrievalSourceKind::Document,
            "cabinet architecture",
        ),
        record(
            "canvas-1",
            RetrievalSourceKind::CanvasNode,
            "cabinet canvas",
        ),
        record(
            "comment-1",
            RetrievalSourceKind::Comment,
            "comment without term",
        ),
    ]);
    let scope = RetrievalScope::new(
        "workspace-1",
        "actor-1",
        vec![
            RetrievalSourceKind::Document,
            RetrievalSourceKind::CanvasNode,
        ],
    )
    .expect("scope");

    let candidates = source
        .query_candidates(&RetrievalQuery::new("cabinet").expect("query"), &scope)
        .expect("candidates");

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].source_id().as_str(), "doc-1");
    assert_eq!(candidates[1].source_id().as_str(), "canvas-1");
}

#[test]
fn local_retrieval_source_excludes_source_kinds_outside_scope() {
    let source = LocalRetrievalSource::new(vec![
        record(
            "doc-1",
            RetrievalSourceKind::Document,
            "cabinet architecture",
        ),
        record(
            "canvas-1",
            RetrievalSourceKind::CanvasNode,
            "cabinet canvas",
        ),
    ]);
    let scope = RetrievalScope::new(
        "workspace-1",
        "actor-1",
        vec![RetrievalSourceKind::Document],
    )
    .expect("scope");

    let candidates = source
        .query_candidates(&RetrievalQuery::new("cabinet").expect("query"), &scope)
        .expect("candidates");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].source_kind(), RetrievalSourceKind::Document);
}

#[test]
fn local_retrieval_source_record_rejects_empty_or_control_search_text() {
    let candidate = candidate("doc-1", RetrievalSourceKind::Document);

    assert_eq!(
        LocalRetrievalSourceRecord::new("", candidate.clone()),
        Err(LocalRetrievalSourceError::InvalidSearchText),
    );
    assert_eq!(
        LocalRetrievalSourceRecord::new("cabinet\nsecret", candidate),
        Err(LocalRetrievalSourceError::InvalidSearchText),
    );
}

fn record(
    source_id: &str,
    source_kind: RetrievalSourceKind,
    searchable_text: &str,
) -> LocalRetrievalSourceRecord {
    LocalRetrievalSourceRecord::new(searchable_text, candidate(source_id, source_kind))
        .expect("record")
}

fn candidate(source_id: &str, source_kind: RetrievalSourceKind) -> RetrievalCandidate {
    let source_id = RetrievalSourceId::new(source_id).expect("source id");
    RetrievalCandidate::new(
        source_id.clone(),
        source_kind,
        RetrievalDecision::Allowed,
        CitationSpan::new(source_id.clone(), "paragraph:1", 0, 8).expect("citation"),
        RetrievalSnippetReference::new(&format!("snippet:{}:paragraph:1", source_id.as_str()))
            .expect("snippet"),
        RetrievalFreshness::Current,
        10,
    )
    .expect("candidate")
}
