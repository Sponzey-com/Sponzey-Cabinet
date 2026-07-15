use cabinet_domain::projection_work::{ProjectionChangeKind, ProjectionWorkIdentity};
use cabinet_ports::markdown_parser::MarkdownParser;
use cabinet_ports::projection_worker::{ProjectionProcessorResult, ProjectionWorkProcessor};
use cabinet_ports::projection_writer::{ProjectionWriteError, VersionedProjectionWriter};
use cabinet_ports::version_store::{VersionStore, VersionStoreError};

pub struct VersionedMarkdownProjectionProcessor<'a, V, P, W> {
    versions: &'a V,
    parser: &'a P,
    writer: &'a mut W,
}

impl<'a, V, P, W> VersionedMarkdownProjectionProcessor<'a, V, P, W> {
    pub fn new(versions: &'a V, parser: &'a P, writer: &'a mut W) -> Self {
        Self {
            versions,
            parser,
            writer,
        }
    }
}

impl<V: VersionStore, P: MarkdownParser, W: VersionedProjectionWriter> ProjectionWorkProcessor
    for VersionedMarkdownProjectionProcessor<'_, V, P, W>
{
    fn process(&mut self, identity: &ProjectionWorkIdentity) -> ProjectionProcessorResult {
        if identity.change_kind() == ProjectionChangeKind::Deleted {
            return map_write_result(self.writer.delete(identity));
        }
        let snapshot = match self.versions.get_version_snapshot(
            identity.workspace_id(),
            identity.document_id(),
            identity.version_id(),
        ) {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => return ProjectionProcessorResult::PermanentFailure,
            Err(error) => return map_version_error(error),
        };
        let parsed = match self.parser.parse(snapshot.body()) {
            Ok(parsed) => parsed,
            Err(_) => return ProjectionProcessorResult::PermanentFailure,
        };
        map_write_result(self.writer.write(identity, snapshot.body(), &parsed))
    }
}

const fn map_write_result(result: Result<(), ProjectionWriteError>) -> ProjectionProcessorResult {
    match result {
        Ok(()) => ProjectionProcessorResult::Succeeded,
        Err(ProjectionWriteError::Retryable) => ProjectionProcessorResult::RetryableFailure,
        Err(ProjectionWriteError::Permanent) => ProjectionProcessorResult::PermanentFailure,
    }
}

const fn map_version_error(error: VersionStoreError) -> ProjectionProcessorResult {
    match error {
        VersionStoreError::StorageUnavailable | VersionStoreError::Conflict => {
            ProjectionProcessorResult::RetryableFailure
        }
        _ => ProjectionProcessorResult::PermanentFailure,
    }
}
