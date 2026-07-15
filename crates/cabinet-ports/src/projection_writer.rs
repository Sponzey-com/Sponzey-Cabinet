use crate::markdown_parser::ParsedMarkdown;
use cabinet_domain::document::DocumentBody;
use cabinet_domain::projection_work::ProjectionWorkIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionWriteError {
    Retryable,
    Permanent,
}

pub trait VersionedProjectionWriter {
    fn write(
        &mut self,
        identity: &ProjectionWorkIdentity,
        body: &DocumentBody,
        parsed: &ParsedMarkdown,
    ) -> Result<(), ProjectionWriteError>;

    fn delete(&mut self, _identity: &ProjectionWorkIdentity) -> Result<(), ProjectionWriteError> {
        Err(ProjectionWriteError::Permanent)
    }
}
