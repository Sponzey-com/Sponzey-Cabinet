use cabinet_domain::projection_work::ProjectionWorkIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionProcessorResult {
    Succeeded,
    RetryableFailure,
    PermanentFailure,
}

pub trait ProjectionWorkProcessor {
    fn process(&mut self, identity: &ProjectionWorkIdentity) -> ProjectionProcessorResult;
}
