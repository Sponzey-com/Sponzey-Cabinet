use cabinet_domain::ai::{
    AiAnswerJob, AiAnswerJobEvent, AiAnswerJobId, AiAnswerJobState, AiAnswerReference,
    AiAnswerResult, AiCitation, AiError, AiFreshnessStatus, AiQuestion, AiRefusal,
    transition_ai_answer_job,
};
use cabinet_domain::retrieval::{CitationSpan, RetrievalFreshness, RetrievalSourceId};

#[test]
fn ai_question_rejects_empty_or_control_text() {
    let question = AiQuestion::new("  summarize policy  ").expect("question");

    assert_eq!(question.as_str(), "summarize policy");
    assert_eq!(AiQuestion::new("  "), Err(AiError::EmptyQuestion));
    assert_eq!(
        AiQuestion::new("summarize\npolicy"),
        Err(AiError::InvalidQuestion),
    );
}

#[test]
fn completed_ai_answer_requires_answer_reference_and_citation() {
    let result = AiAnswerResult::completed(
        AiAnswerReference::new("answer:job-1:result").expect("answer ref"),
        vec![citation("doc-1")],
        AiFreshnessStatus::Fresh,
    )
    .expect("answer");

    assert_eq!(result.answer_reference().as_str(), "answer:job-1:result");
    assert_eq!(result.citations().len(), 1);
    assert_eq!(result.freshness(), AiFreshnessStatus::Fresh);
    assert_eq!(
        AiAnswerReference::new("raw answer text should not be accepted"),
        Err(AiError::InvalidAnswerReference),
    );
    assert_eq!(
        AiAnswerResult::completed(
            AiAnswerReference::new("answer:job-1:result").expect("answer ref"),
            vec![],
            AiFreshnessStatus::Fresh,
        ),
        Err(AiError::CitationRequired),
    );
}

#[test]
fn ai_refusal_uses_stable_reason_code() {
    let refusal =
        AiAnswerResult::refused(AiRefusal::new("ai.refusal.no_citation").expect("refusal"));

    assert_eq!(
        refusal.refusal().expect("refusal").reason_code(),
        "ai.refusal.no_citation",
    );
    assert_eq!(AiRefusal::new(""), Err(AiError::InvalidRefusalReason));
}

#[test]
fn ai_answer_job_requires_id_and_question() {
    let job = AiAnswerJob::new(
        AiAnswerJobId::new("answer-job-1").expect("job id"),
        AiQuestion::new("summarize policy").expect("question"),
    )
    .expect("job");

    assert_eq!(job.id().as_str(), "answer-job-1");
    assert_eq!(job.question().as_str(), "summarize policy");
    assert_eq!(job.state(), AiAnswerJobState::Queued);
    assert_eq!(AiAnswerJobId::new(""), Err(AiError::InvalidAnswerJobId));
}

#[test]
fn ai_answer_job_uses_success_refusal_retry_and_failure_transitions() {
    let retrieval =
        transition_ai_answer_job(AiAnswerJobState::Queued, AiAnswerJobEvent::PrepareRetrieval)
            .expect("retrieval");
    let provider =
        transition_ai_answer_job(retrieval, AiAnswerJobEvent::RequestProvider).expect("provider");
    let validating =
        transition_ai_answer_job(provider, AiAnswerJobEvent::ValidateCitations).expect("validate");
    let completed =
        transition_ai_answer_job(validating, AiAnswerJobEvent::Complete).expect("complete");

    assert_eq!(retrieval, AiAnswerJobState::RetrievalPreparing);
    assert_eq!(provider, AiAnswerJobState::ProviderRequested);
    assert_eq!(validating, AiAnswerJobState::CitationValidating);
    assert_eq!(completed, AiAnswerJobState::Completed);
    assert_eq!(
        transition_ai_answer_job(
            AiAnswerJobState::CitationValidating,
            AiAnswerJobEvent::Refuse
        )
        .expect("refuse"),
        AiAnswerJobState::Refused,
    );
    assert_eq!(
        transition_ai_answer_job(
            AiAnswerJobState::ProviderRequested,
            AiAnswerJobEvent::ScheduleRetry
        )
        .expect("retry"),
        AiAnswerJobState::RetryScheduled,
    );
    assert_eq!(
        transition_ai_answer_job(
            AiAnswerJobState::RetryScheduled,
            AiAnswerJobEvent::RetryProvider
        )
        .expect("retry provider"),
        AiAnswerJobState::ProviderRequested,
    );
    assert_eq!(
        transition_ai_answer_job(AiAnswerJobState::ProviderRequested, AiAnswerJobEvent::Fail)
            .expect("fail"),
        AiAnswerJobState::Failed,
    );
}

#[test]
fn ai_answer_job_rejects_completed_without_citation_validation_or_retry_from_terminal() {
    assert_eq!(
        transition_ai_answer_job(
            AiAnswerJobState::ProviderRequested,
            AiAnswerJobEvent::Complete
        ),
        Err(AiError::InvalidAnswerJobTransition),
    );
    assert_eq!(
        transition_ai_answer_job(AiAnswerJobState::Completed, AiAnswerJobEvent::ScheduleRetry),
        Err(AiError::InvalidAnswerJobTransition),
    );
}

fn citation(source_id: &str) -> AiCitation {
    let source_id = RetrievalSourceId::new(source_id).expect("source id");
    AiCitation::new(
        CitationSpan::new(source_id, "paragraph:1", 0, 12).expect("citation"),
        RetrievalFreshness::Current,
    )
}
