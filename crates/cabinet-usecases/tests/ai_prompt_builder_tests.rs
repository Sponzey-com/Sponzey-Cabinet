use cabinet_domain::ai::AiAnswerJobId;
use cabinet_domain::retrieval::{
    CitationSpan, RetrievalCandidate, RetrievalDecision, RetrievalFreshness,
    RetrievalSnippetReference, RetrievalSourceId, RetrievalSourceKind,
};
use cabinet_usecases::ai::{
    AskKnowledgeBaseError, BuildAiPromptReferenceInput, BuildAiPromptReferenceUsecase,
};

#[test]
fn prompt_reference_builder_does_not_expose_raw_prompt_or_secret_fixture() {
    let reference = BuildAiPromptReferenceUsecase::new()
        .execute(BuildAiPromptReferenceInput::new(
            AiAnswerJobId::new("job-safe-1").expect("job id"),
            vec![candidate("doc-1"), candidate("doc-2")],
        ))
        .expect("prompt reference");

    assert_eq!(
        reference.as_str(),
        "prompt:job-safe-1:retrieval-citations:2"
    );
    assert!(!reference.as_str().contains("provider_api_key_fixture"));
    assert!(
        !reference
            .as_str()
            .contains("connector_access_token_fixture")
    );
    assert!(!reference.as_str().contains("raw prompt"));
    assert!(!reference.as_str().contains("document body"));
}

#[test]
fn prompt_reference_builder_rejects_empty_retrieval_context() {
    let error = BuildAiPromptReferenceUsecase::new()
        .execute(BuildAiPromptReferenceInput::new(
            AiAnswerJobId::new("job-empty-context").expect("job id"),
            vec![],
        ))
        .expect_err("empty context");

    assert_eq!(error, AskKnowledgeBaseError::NoRetrievalContext);
    assert_eq!(error.code(), "ai_answer.no_retrieval_context");
}

#[test]
fn prompt_reference_builder_rejects_secret_like_job_id() {
    let error = BuildAiPromptReferenceUsecase::new()
        .execute(BuildAiPromptReferenceInput::new(
            AiAnswerJobId::new("provider_api_key_fixture").expect("job id"),
            vec![candidate("doc-1")],
        ))
        .expect_err("sensitive prompt reference");

    assert_eq!(error, AskKnowledgeBaseError::InvalidInput);
    assert_eq!(error.code(), "ai_answer.invalid_input");
}

fn candidate(source_id: &str) -> RetrievalCandidate {
    let source_id = RetrievalSourceId::new(source_id).expect("source id");
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
