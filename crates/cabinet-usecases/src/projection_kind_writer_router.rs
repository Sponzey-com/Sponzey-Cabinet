use cabinet_domain::document::DocumentBody;
use cabinet_domain::projection_work::{ProjectionKind, ProjectionWorkIdentity};
use cabinet_ports::markdown_parser::ParsedMarkdown;
use cabinet_ports::projection_writer::{ProjectionWriteError, VersionedProjectionWriter};

pub struct ProjectionKindWriterRouter<'a, S, R> {
    search: &'a mut S,
    relations: &'a mut R,
}

impl<'a, S, R> ProjectionKindWriterRouter<'a, S, R> {
    pub fn new(search: &'a mut S, relations: &'a mut R) -> Self {
        Self { search, relations }
    }
}

impl<S: VersionedProjectionWriter, R: VersionedProjectionWriter> VersionedProjectionWriter
    for ProjectionKindWriterRouter<'_, S, R>
{
    fn write(
        &mut self,
        identity: &ProjectionWorkIdentity,
        body: &DocumentBody,
        parsed: &ParsedMarkdown,
    ) -> Result<(), ProjectionWriteError> {
        match identity.kind() {
            ProjectionKind::Search => self.search.write(identity, body, parsed),
            ProjectionKind::Links | ProjectionKind::Graph => {
                self.relations.write(identity, body, parsed)
            }
        }
    }

    fn delete(&mut self, identity: &ProjectionWorkIdentity) -> Result<(), ProjectionWriteError> {
        match identity.kind() {
            ProjectionKind::Search => self.search.delete(identity),
            ProjectionKind::Links | ProjectionKind::Graph => self.relations.delete(identity),
        }
    }
}
