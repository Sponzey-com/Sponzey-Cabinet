use cabinet_domain::retrieval::{
    CitationSpan, ContextBudget, RetrievalCandidate, RetrievalDecision, RetrievalError,
    RetrievalFreshness, RetrievalPipelineEvent, RetrievalPipelineState, RetrievalQuery,
    RetrievalScope, RetrievalSnippetReference, RetrievalSourceId, RetrievalSourceKind,
    transition_retrieval_pipeline,
};

#[test]
fn retrieval_query_trims_and_rejects_empty_or_control_text() {
    let query = RetrievalQuery::new("  policy update  ").expect("query");

    assert_eq!(query.as_str(), "policy update");
    assert_eq!(RetrievalQuery::new("   "), Err(RetrievalError::EmptyQuery));
    assert_eq!(
        RetrievalQuery::new("policy\nupdate"),
        Err(RetrievalError::InvalidQuery),
    );
}

#[test]
fn retrieval_scope_requires_workspace_actor_and_source_kinds() {
    let scope = RetrievalScope::new(
        "workspace-1",
        "actor-1",
        vec![
            RetrievalSourceKind::Document,
            RetrievalSourceKind::GraphRelation,
            RetrievalSourceKind::CanvasNode,
        ],
    )
    .expect("scope");

    assert_eq!(scope.workspace_id(), "workspace-1");
    assert_eq!(scope.actor_id(), "actor-1");
    assert_eq!(scope.source_kinds().len(), 3);
    assert_eq!(
        RetrievalScope::new("", "actor-1", vec![RetrievalSourceKind::Document]),
        Err(RetrievalError::EmptyScopeValue),
    );
    assert_eq!(
        RetrievalScope::new("workspace-1", "actor-1", vec![]),
        Err(RetrievalError::EmptySourceKinds),
    );
}

#[test]
fn retrieval_candidate_uses_references_without_raw_body_or_denied_source() {
    let source_id = RetrievalSourceId::new("doc-1").expect("source id");
    let citation = CitationSpan::new(source_id.clone(), "paragraph:2", 10, 30).expect("citation");
    let snippet = RetrievalSnippetReference::new("snippet:doc-1:paragraph:2").expect("snippet");
    let candidate = RetrievalCandidate::new(
        source_id.clone(),
        RetrievalSourceKind::Document,
        RetrievalDecision::Allowed,
        citation,
        snippet,
        RetrievalFreshness::Current,
        42,
    )
    .expect("candidate");

    assert_eq!(candidate.source_id(), &source_id);
    assert_eq!(candidate.source_kind(), RetrievalSourceKind::Document);
    assert_eq!(candidate.estimated_tokens(), 42);
    assert_eq!(
        RetrievalSnippetReference::new("raw document body should not be accepted"),
        Err(RetrievalError::InvalidSnippetReference),
    );

    let denied = RetrievalCandidate::new(
        source_id.clone(),
        RetrievalSourceKind::Document,
        RetrievalDecision::Denied {
            reason_code: "permission.denied",
        },
        CitationSpan::new(source_id, "paragraph:2", 10, 30).expect("citation"),
        RetrievalSnippetReference::new("snippet:doc-1:paragraph:2").expect("snippet"),
        RetrievalFreshness::Current,
        12,
    );

    assert_eq!(denied, Err(RetrievalError::CandidateNotAllowed));
}

#[test]
fn citation_and_context_budget_enforce_stable_bounds() {
    let source_id = RetrievalSourceId::new("doc-1").expect("source id");

    assert_eq!(
        CitationSpan::new(source_id.clone(), "", 1, 2),
        Err(RetrievalError::MissingCitationReference),
    );
    assert_eq!(
        CitationSpan::new(source_id, "paragraph:1", 5, 5),
        Err(RetrievalError::InvalidCitationRange),
    );

    let budget = ContextBudget::new(1).expect("budget");
    assert_eq!(budget.max_tokens(), 1);
    assert!(budget.allows(1));
    assert!(!budget.allows(2));
    assert_eq!(
        ContextBudget::new(0),
        Err(RetrievalError::InvalidContextBudget)
    );
}

#[test]
fn retrieval_pipeline_uses_explicit_transitions() {
    let normalizing = transition_retrieval_pipeline(
        RetrievalPipelineState::Received,
        RetrievalPipelineEvent::Normalize,
    )
    .expect("normalize");
    let source_querying =
        transition_retrieval_pipeline(normalizing, RetrievalPipelineEvent::QuerySources)
            .expect("query sources");
    let filtering =
        transition_retrieval_pipeline(source_querying, RetrievalPipelineEvent::ApplyPermissions)
            .expect("filter");
    let ranking =
        transition_retrieval_pipeline(filtering, RetrievalPipelineEvent::Rank).expect("rank");
    let assembled = transition_retrieval_pipeline(ranking, RetrievalPipelineEvent::AssembleContext)
        .expect("assemble");

    assert_eq!(normalizing, RetrievalPipelineState::Normalizing);
    assert_eq!(source_querying, RetrievalPipelineState::SourceQuerying);
    assert_eq!(filtering, RetrievalPipelineState::PermissionFiltering);
    assert_eq!(ranking, RetrievalPipelineState::Ranking);
    assert_eq!(assembled, RetrievalPipelineState::ContextAssembled);
    assert_eq!(
        transition_retrieval_pipeline(assembled, RetrievalPipelineEvent::Rank),
        Err(RetrievalError::InvalidPipelineTransition),
    );
}

#[test]
fn retrieval_pipeline_models_degraded_permission_unavailable_and_failed_states() {
    assert_eq!(
        transition_retrieval_pipeline(
            RetrievalPipelineState::SourceQuerying,
            RetrievalPipelineEvent::MarkSourceDegraded,
        )
        .expect("degraded"),
        RetrievalPipelineState::SourceDegraded,
    );
    assert_eq!(
        transition_retrieval_pipeline(
            RetrievalPipelineState::PermissionFiltering,
            RetrievalPipelineEvent::MarkPermissionUnavailable,
        )
        .expect("permission unavailable"),
        RetrievalPipelineState::PermissionUnavailable,
    );
    assert_eq!(
        transition_retrieval_pipeline(
            RetrievalPipelineState::Ranking,
            RetrievalPipelineEvent::Fail
        )
        .expect("failed"),
        RetrievalPipelineState::Failed,
    );
    assert_eq!(
        transition_retrieval_pipeline(RetrievalPipelineState::Failed, RetrievalPipelineEvent::Rank),
        Err(RetrievalError::InvalidPipelineTransition),
    );
}
