use std::time::Instant;

use cabinet_adapters::fake_ai_provider::FakeAiProvider;
use cabinet_adapters::local_ai_answer_store::LocalAiAnswerStore;
use cabinet_domain::ai::{
    AiAnswerJobId, AiAnswerJobState, AiAnswerReference, AiAnswerResult, AiCitation,
    AiFreshnessStatus, AiQuestion, AiRefusal,
};
use cabinet_domain::retrieval::{CitationSpan, RetrievalFreshness, RetrievalSourceId};
use cabinet_ports::ai::{
    AiAnswerResultStorePort, AiPromptReference, AiProviderError, AiProviderPolicy, AiProviderPort,
    AiProviderRequest, AiProviderResponse,
};

#[test]
fn fake_ai_provider_returns_configured_response_and_counts_calls() {
    let provider = FakeAiProvider::new(Ok(AiProviderResponse::answered(
        AiAnswerReference::new("answer:job-1:result").expect("answer ref"),
        vec![source_id("doc-1")],
        AiFreshnessStatus::Fresh,
        128,
    )));
    let request = provider_request("job-1");
    let policy = policy();

    let response = provider
        .generate_answer(&request, &policy)
        .expect("provider response");

    assert_eq!(provider.call_count(), 1);
    assert_eq!(response.cited_source_ids().len(), 1);
    assert_eq!(response.answer_length_bucket(), Some(128));
}

#[test]
fn fake_ai_provider_can_return_retryable_timeout_without_network() {
    let provider = FakeAiProvider::new(Err(AiProviderError::Timeout));
    let request = provider_request("job-timeout");

    let error = provider
        .generate_answer(&request, &policy())
        .expect_err("timeout");

    assert_eq!(error, AiProviderError::Timeout);
    assert!(error.is_retryable());
    assert_eq!(provider.call_count(), 1);
}

#[test]
fn prompt_reference_rejects_secret_like_fixture_text() {
    assert_eq!(
        AiPromptReference::new("prompt:provider_api_key_fixture"),
        Err(AiProviderError::SensitivePromptReference),
    );
    assert_eq!(
        AiPromptReference::new("prompt:connector_access_token_fixture"),
        Err(AiProviderError::SensitivePromptReference),
    );
}

#[test]
fn local_ai_answer_store_persists_status_and_refusal_result_by_job_id() {
    let mut store = LocalAiAnswerStore::default();
    let job_id = AiAnswerJobId::new("job-store-1").expect("job id");
    let result = cabinet_domain::ai::AiAnswerResult::refused(
        AiRefusal::new("ai.refusal.no_valid_citation").expect("refusal"),
    );

    store
        .save_status(&job_id, AiAnswerJobState::Refused)
        .expect("status");
    store.save_result(&job_id, result.clone()).expect("result");

    assert_eq!(
        store.get_status(&job_id).expect("status"),
        Some(AiAnswerJobState::Refused),
    );
    assert_eq!(store.get_result(&job_id).expect("result"), Some(result));
}

#[test]
fn local_ai_answer_store_cached_status_and_result_lookup_stays_under_300ms() {
    let mut store = LocalAiAnswerStore::default();
    let job_id = AiAnswerJobId::new("job-cache-1").expect("job id");
    let result =
        AiAnswerResult::refused(AiRefusal::new("ai.refusal.no_valid_citation").expect("refusal"));

    store
        .save_status(&job_id, AiAnswerJobState::Refused)
        .expect("status");
    store.save_result(&job_id, result).expect("result");

    let started_at = Instant::now();
    for _ in 0..1_000 {
        assert_eq!(
            store.get_status(&job_id).expect("status"),
            Some(AiAnswerJobState::Refused),
        );
        assert!(store.get_result(&job_id).expect("result").is_some());
    }
    let elapsed = started_at.elapsed();

    assert!(
        elapsed.as_millis() < 300,
        "cached answer status/result lookup took {elapsed:?}",
    );
}

fn provider_request(job_id: &str) -> AiProviderRequest {
    AiProviderRequest::new(
        AiQuestion::new("summarize the policy").expect("question"),
        AiPromptReference::new(&format!("prompt:{job_id}:retrieval-context")).expect("prompt ref"),
        vec![citation("doc-1")],
    )
    .expect("request")
}

fn citation(source_id: &str) -> AiCitation {
    let source_id = source_id_from(source_id);
    AiCitation::new(
        CitationSpan::new(source_id, "paragraph:1", 0, 12).expect("citation"),
        RetrievalFreshness::Current,
    )
}

fn source_id(value: &str) -> RetrievalSourceId {
    source_id_from(value)
}

fn source_id_from(value: &str) -> RetrievalSourceId {
    RetrievalSourceId::new(value).expect("source id")
}

fn policy() -> AiProviderPolicy {
    AiProviderPolicy::new("fake-provider", "fake-model", 3_000, 512, 2).expect("policy")
}
