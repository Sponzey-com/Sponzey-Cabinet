use std::cell::Cell;

use cabinet_domain::retrieval::{
    CitationSpan, RetrievalCandidate, RetrievalDecision, RetrievalFreshness,
    RetrievalSnippetReference, RetrievalSourceId, RetrievalSourceKind,
};
use cabinet_ports::retrieval::{RetrievalPermissionPort, RetrievalPortError, RetrievalSourcePort};
use cabinet_usecases::retrieval::{
    BuildRetrievalContextError, BuildRetrievalContextInput, BuildRetrievalContextUsecase,
};

#[derive(Default)]
struct FakeRetrievalSource {
    candidates: Vec<RetrievalCandidate>,
    fail: bool,
    call_count: Cell<usize>,
}

impl RetrievalSourcePort for FakeRetrievalSource {
    fn query_candidates(
        &self,
        _query: &cabinet_domain::retrieval::RetrievalQuery,
        _scope: &cabinet_domain::retrieval::RetrievalScope,
    ) -> Result<Vec<RetrievalCandidate>, RetrievalPortError> {
        self.call_count.set(self.call_count.get() + 1);
        if self.fail {
            return Err(RetrievalPortError::SourceUnavailable);
        }
        Ok(self.candidates.clone())
    }
}

#[derive(Default)]
struct FakePermissionPort {
    denied_source_id: Option<String>,
    fail: bool,
    call_count: Cell<usize>,
}

impl RetrievalPermissionPort for FakePermissionPort {
    fn allows_candidate(
        &self,
        _scope: &cabinet_domain::retrieval::RetrievalScope,
        candidate: &RetrievalCandidate,
    ) -> Result<bool, RetrievalPortError> {
        self.call_count.set(self.call_count.get() + 1);
        if self.fail {
            return Err(RetrievalPortError::PermissionUnavailable);
        }
        Ok(self.denied_source_id.as_deref() != Some(candidate.source_id().as_str()))
    }
}

#[test]
fn build_retrieval_context_filters_permission_denied_candidates() {
    let source = FakeRetrievalSource {
        candidates: vec![candidate("doc-1", 20), candidate("doc-2", 20)],
        ..FakeRetrievalSource::default()
    };
    let permission = FakePermissionPort {
        denied_source_id: Some("doc-2".to_string()),
        ..FakePermissionPort::default()
    };

    let output = BuildRetrievalContextUsecase::new()
        .execute(
            BuildRetrievalContextInput::new(
                "workspace-1",
                "actor-1",
                "policy",
                vec![RetrievalSourceKind::Document],
                100,
            ),
            &source,
            &permission,
        )
        .expect("context");

    assert_eq!(output.candidates().len(), 1);
    assert_eq!(output.candidates()[0].source_id().as_str(), "doc-1");
    assert_eq!(output.stats().candidate_count(), 2);
    assert_eq!(output.stats().filtered_count(), 1);
    assert_eq!(output.stats().truncated_count(), 0);
    assert_eq!(output.stats().selected_token_count(), 20);
    assert_eq!(source.call_count.get(), 1);
    assert_eq!(permission.call_count.get(), 2);
}

#[test]
fn build_retrieval_context_truncates_candidates_over_context_budget() {
    let source = FakeRetrievalSource {
        candidates: vec![candidate("doc-1", 40), candidate("doc-2", 20)],
        ..FakeRetrievalSource::default()
    };
    let permission = FakePermissionPort::default();

    let output = BuildRetrievalContextUsecase::new()
        .execute(
            BuildRetrievalContextInput::new(
                "workspace-1",
                "actor-1",
                "policy",
                vec![RetrievalSourceKind::Document],
                50,
            ),
            &source,
            &permission,
        )
        .expect("context");

    assert_eq!(output.candidates().len(), 1);
    assert_eq!(output.candidates()[0].source_id().as_str(), "doc-1");
    assert_eq!(output.stats().candidate_count(), 2);
    assert_eq!(output.stats().filtered_count(), 0);
    assert_eq!(output.stats().truncated_count(), 1);
    assert_eq!(output.stats().selected_token_count(), 40);
}

#[test]
fn build_retrieval_context_rejects_invalid_input_before_calling_ports() {
    let source = FakeRetrievalSource::default();
    let permission = FakePermissionPort::default();

    let error = BuildRetrievalContextUsecase::new()
        .execute(
            BuildRetrievalContextInput::new(
                "workspace-1",
                "actor-1",
                "  ",
                vec![RetrievalSourceKind::Document],
                100,
            ),
            &source,
            &permission,
        )
        .expect_err("invalid input");

    assert_eq!(error, BuildRetrievalContextError::InvalidInput);
    assert_eq!(error.code(), "retrieval_context.invalid_input");
    assert_eq!(source.call_count.get(), 0);
    assert_eq!(permission.call_count.get(), 0);
}

#[test]
fn build_retrieval_context_maps_source_and_permission_failures() {
    let source_failure = FakeRetrievalSource {
        fail: true,
        ..FakeRetrievalSource::default()
    };
    let permission = FakePermissionPort::default();
    let input = BuildRetrievalContextInput::new(
        "workspace-1",
        "actor-1",
        "policy",
        vec![RetrievalSourceKind::Document],
        100,
    );

    let source_error = BuildRetrievalContextUsecase::new()
        .execute(input.clone(), &source_failure, &permission)
        .expect_err("source failure");

    assert_eq!(source_error, BuildRetrievalContextError::SourceUnavailable);
    assert_eq!(source_error.code(), "retrieval_context.source_unavailable");

    let source = FakeRetrievalSource {
        candidates: vec![candidate("doc-1", 10)],
        ..FakeRetrievalSource::default()
    };
    let permission_failure = FakePermissionPort {
        fail: true,
        ..FakePermissionPort::default()
    };

    let permission_error = BuildRetrievalContextUsecase::new()
        .execute(input, &source, &permission_failure)
        .expect_err("permission failure");

    assert_eq!(
        permission_error,
        BuildRetrievalContextError::PermissionUnavailable,
    );
    assert_eq!(
        permission_error.code(),
        "retrieval_context.permission_unavailable",
    );
}

fn candidate(source_id: &str, estimated_tokens: u32) -> RetrievalCandidate {
    let source_id = RetrievalSourceId::new(source_id).expect("source id");
    RetrievalCandidate::new(
        source_id.clone(),
        RetrievalSourceKind::Document,
        RetrievalDecision::Allowed,
        CitationSpan::new(source_id.clone(), "paragraph:1", 0, 12).expect("citation"),
        RetrievalSnippetReference::new(&format!("snippet:{}:paragraph:1", source_id.as_str()))
            .expect("snippet"),
        RetrievalFreshness::Current,
        estimated_tokens,
    )
    .expect("candidate")
}
