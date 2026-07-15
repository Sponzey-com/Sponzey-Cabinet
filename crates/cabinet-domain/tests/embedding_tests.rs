use cabinet_domain::embedding::{
    EmbeddingError, EmbeddingInput, EmbeddingJob, EmbeddingJobEvent, EmbeddingJobId,
    EmbeddingJobState, EmbeddingVectorReference, transition_embedding_job,
};
use cabinet_domain::retrieval::{RetrievalSourceId, RetrievalSourceKind};

#[test]
fn embedding_input_uses_reference_without_raw_text() {
    let input = EmbeddingInput::new(
        RetrievalSourceId::new("doc-1").expect("source id"),
        RetrievalSourceKind::Document,
        "embedding-input:doc-1:paragraph:1",
    )
    .expect("input");

    assert_eq!(input.source_id().as_str(), "doc-1");
    assert_eq!(input.source_kind(), RetrievalSourceKind::Document);
    assert_eq!(input.reference(), "embedding-input:doc-1:paragraph:1");
    assert_eq!(
        EmbeddingInput::new(
            RetrievalSourceId::new("doc-1").expect("source id"),
            RetrievalSourceKind::Document,
            "raw document body should not be accepted",
        ),
        Err(EmbeddingError::InvalidInputReference),
    );
}

#[test]
fn embedding_vector_reference_rejects_empty_or_control_reference() {
    let reference = EmbeddingVectorReference::new("vector:doc-1:default").expect("vector");

    assert_eq!(reference.as_str(), "vector:doc-1:default");
    assert_eq!(
        EmbeddingVectorReference::new(""),
        Err(EmbeddingError::InvalidVectorReference),
    );
    assert_eq!(
        EmbeddingVectorReference::new("vector:doc-1\nsecret"),
        Err(EmbeddingError::InvalidVectorReference),
    );
}

#[test]
fn embedding_job_requires_id_and_inputs() {
    let job = EmbeddingJob::new(EmbeddingJobId::new("job-1").expect("job id"), vec![input()])
        .expect("job");

    assert_eq!(job.id().as_str(), "job-1");
    assert_eq!(job.input_count(), 1);
    assert_eq!(job.state(), EmbeddingJobState::Queued);
    assert_eq!(EmbeddingJobId::new(""), Err(EmbeddingError::InvalidJobId));
    assert_eq!(
        EmbeddingJob::new(EmbeddingJobId::new("job-2").expect("job id"), vec![]),
        Err(EmbeddingError::EmptyInputSet),
    );
}

#[test]
fn embedding_job_uses_explicit_success_transitions() {
    let preparing =
        transition_embedding_job(EmbeddingJobState::Queued, EmbeddingJobEvent::PrepareInput)
            .expect("prepare");
    let provider =
        transition_embedding_job(preparing, EmbeddingJobEvent::RequestProvider).expect("provider");
    let stored = transition_embedding_job(provider, EmbeddingJobEvent::StoreVector).expect("store");
    let completed =
        transition_embedding_job(stored, EmbeddingJobEvent::Complete).expect("complete");

    assert_eq!(preparing, EmbeddingJobState::PreparingInput);
    assert_eq!(provider, EmbeddingJobState::ProviderRequested);
    assert_eq!(stored, EmbeddingJobState::VectorStored);
    assert_eq!(completed, EmbeddingJobState::Completed);
}

#[test]
fn embedding_job_uses_explicit_retry_and_failure_transitions() {
    let retry = transition_embedding_job(
        EmbeddingJobState::ProviderRequested,
        EmbeddingJobEvent::ScheduleRetry,
    )
    .expect("schedule retry");
    let provider =
        transition_embedding_job(retry, EmbeddingJobEvent::RetryProvider).expect("retry provider");
    let failed =
        transition_embedding_job(provider, EmbeddingJobEvent::Fail).expect("fail provider");

    assert_eq!(retry, EmbeddingJobState::RetryScheduled);
    assert_eq!(provider, EmbeddingJobState::ProviderRequested);
    assert_eq!(failed, EmbeddingJobState::Failed);
    assert_eq!(
        transition_embedding_job(
            EmbeddingJobState::Completed,
            EmbeddingJobEvent::ScheduleRetry
        ),
        Err(EmbeddingError::InvalidJobTransition),
    );
}

fn input() -> EmbeddingInput {
    EmbeddingInput::new(
        RetrievalSourceId::new("doc-1").expect("source id"),
        RetrievalSourceKind::Document,
        "embedding-input:doc-1:paragraph:1",
    )
    .expect("input")
}
