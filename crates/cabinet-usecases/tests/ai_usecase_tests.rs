use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::ai::{
    AiAnswerJobId, AiAnswerJobState, AiAnswerReference, AiAnswerResult, AiFreshnessStatus,
    AiQuestion, AiRefusal,
};
use cabinet_domain::retrieval::{
    CitationSpan, RetrievalCandidate, RetrievalDecision, RetrievalFreshness,
    RetrievalSnippetReference, RetrievalSourceId, RetrievalSourceKind,
};
use cabinet_ports::ai::{
    AiAnswerResultStorePort, AiAnswerStoreError, AiPromptReference, AiProviderError,
    AiProviderPolicy, AiProviderPort, AiProviderRequest, AiProviderResponse,
};
use cabinet_usecases::ai::{AskKnowledgeBaseError, AskKnowledgeBaseInput, AskKnowledgeBaseUsecase};

#[derive(Debug)]
struct FakeAiProvider {
    response: Result<AiProviderResponse, AiProviderError>,
    call_count: Cell<usize>,
}

impl FakeAiProvider {
    fn new(response: Result<AiProviderResponse, AiProviderError>) -> Self {
        Self {
            response,
            call_count: Cell::new(0),
        }
    }

    fn call_count(&self) -> usize {
        self.call_count.get()
    }
}

impl AiProviderPort for FakeAiProvider {
    fn generate_answer(
        &self,
        _request: &AiProviderRequest,
        _policy: &AiProviderPolicy,
    ) -> Result<AiProviderResponse, AiProviderError> {
        self.call_count.set(self.call_count.get() + 1);
        self.response.clone()
    }
}

#[derive(Default)]
struct FakeAnswerStore {
    statuses: HashMap<String, AiAnswerJobState>,
    results: HashMap<String, AiAnswerResult>,
}

impl AiAnswerResultStorePort for FakeAnswerStore {
    fn save_status(
        &mut self,
        job_id: &AiAnswerJobId,
        state: AiAnswerJobState,
    ) -> Result<(), AiAnswerStoreError> {
        self.statuses.insert(job_id.as_str().to_string(), state);
        Ok(())
    }

    fn save_result(
        &mut self,
        job_id: &AiAnswerJobId,
        result: AiAnswerResult,
    ) -> Result<(), AiAnswerStoreError> {
        self.results.insert(job_id.as_str().to_string(), result);
        Ok(())
    }

    fn get_status(
        &self,
        job_id: &AiAnswerJobId,
    ) -> Result<Option<AiAnswerJobState>, AiAnswerStoreError> {
        Ok(self.statuses.get(job_id.as_str()).copied())
    }

    fn get_result(
        &self,
        job_id: &AiAnswerJobId,
    ) -> Result<Option<AiAnswerResult>, AiAnswerStoreError> {
        Ok(self.results.get(job_id.as_str()).cloned())
    }
}

#[test]
fn ask_knowledge_base_stores_completed_answer_with_valid_citation() {
    let provider = FakeAiProvider::new(Ok(AiProviderResponse::answered(
        AiAnswerReference::new("answer:job-1:result").expect("answer ref"),
        vec![source_id("doc-1")],
        AiFreshnessStatus::Fresh,
        120,
    )));
    let mut store = FakeAnswerStore::default();
    let input = input("job-1", vec![candidate("doc-1")]);

    let output = AskKnowledgeBaseUsecase::new()
        .execute(input, &provider, &mut store)
        .expect("answer");

    assert_eq!(output.state(), AiAnswerJobState::Completed);
    assert_eq!(output.citation_count(), 1);
    assert_eq!(provider.call_count(), 1);
    let job_id = AiAnswerJobId::new("job-1").expect("job id");
    assert_eq!(
        store.get_status(&job_id).expect("status"),
        Some(AiAnswerJobState::Completed),
    );
    let result = store
        .get_result(&job_id)
        .expect("result")
        .expect("stored result");
    assert_eq!(result.answer_reference().as_str(), "answer:job-1:result");
    assert_eq!(result.citations().len(), 1);
}

#[test]
fn ask_knowledge_base_converts_answer_without_context_citation_to_refusal() {
    let provider = FakeAiProvider::new(Ok(AiProviderResponse::answered(
        AiAnswerReference::new("answer:job-2:result").expect("answer ref"),
        vec![source_id("doc-outside-context")],
        AiFreshnessStatus::Fresh,
        80,
    )));
    let mut store = FakeAnswerStore::default();
    let input = input("job-2", vec![candidate("doc-1")]);

    let output = AskKnowledgeBaseUsecase::new()
        .execute(input, &provider, &mut store)
        .expect("refusal");

    assert_eq!(output.state(), AiAnswerJobState::Refused);
    assert_eq!(output.citation_count(), 0);
    let result = store
        .get_result(&AiAnswerJobId::new("job-2").expect("job id"))
        .expect("result")
        .expect("stored result");
    assert_eq!(
        result.refusal().expect("refusal").reason_code(),
        "ai.refusal.no_valid_citation",
    );
}

#[test]
fn ask_knowledge_base_schedules_retry_when_provider_times_out() {
    let provider = FakeAiProvider::new(Err(AiProviderError::Timeout));
    let mut store = FakeAnswerStore::default();
    let input = input("job-3", vec![candidate("doc-1")]);

    let output = AskKnowledgeBaseUsecase::new()
        .execute(input, &provider, &mut store)
        .expect("retry");

    assert_eq!(output.state(), AiAnswerJobState::RetryScheduled);
    assert_eq!(output.citation_count(), 0);
    assert_eq!(
        store
            .get_status(&AiAnswerJobId::new("job-3").expect("job id"))
            .expect("status"),
        Some(AiAnswerJobState::RetryScheduled),
    );
}

#[test]
fn ask_knowledge_base_rejects_empty_context_before_calling_provider() {
    let provider = FakeAiProvider::new(Ok(AiProviderResponse::refused(
        AiRefusal::new("ai.refusal.no_context").expect("refusal"),
    )));
    let mut store = FakeAnswerStore::default();
    let input = input("job-4", vec![]);

    let error = AskKnowledgeBaseUsecase::new()
        .execute(input, &provider, &mut store)
        .expect_err("invalid input");

    assert_eq!(error, AskKnowledgeBaseError::NoRetrievalContext);
    assert_eq!(error.code(), "ai_answer.no_retrieval_context");
    assert_eq!(provider.call_count(), 0);
}

fn input(job_id: &str, candidates: Vec<RetrievalCandidate>) -> AskKnowledgeBaseInput {
    AskKnowledgeBaseInput::new(
        AiAnswerJobId::new(job_id).expect("job id"),
        AiQuestion::new("summarize the policy").expect("question"),
        AiPromptReference::new(&format!("prompt:{job_id}:retrieval-context")).expect("prompt ref"),
        candidates,
        AiProviderPolicy::new("fake-provider", "fake-model", 3_000, 512, 2).expect("policy"),
    )
    .expect("input")
}

fn candidate(source_id: &str) -> RetrievalCandidate {
    let source_id = source_id_from(source_id);
    RetrievalCandidate::new(
        source_id.clone(),
        RetrievalSourceKind::Document,
        RetrievalDecision::Allowed,
        CitationSpan::new(source_id.clone(), "paragraph:1", 0, 12).expect("citation"),
        RetrievalSnippetReference::new(&format!("snippet:{}:paragraph:1", source_id.as_str()))
            .expect("snippet"),
        RetrievalFreshness::Current,
        20,
    )
    .expect("candidate")
}

fn source_id(value: &str) -> RetrievalSourceId {
    source_id_from(value)
}

fn source_id_from(value: &str) -> RetrievalSourceId {
    RetrievalSourceId::new(value).expect("source id")
}
