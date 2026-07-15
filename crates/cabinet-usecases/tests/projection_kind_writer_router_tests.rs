use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::projection_work::{ProjectionKind, ProjectionWorkIdentity};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::markdown_parser::ParsedMarkdown;
use cabinet_ports::projection_writer::{ProjectionWriteError, VersionedProjectionWriter};
use cabinet_usecases::projection_kind_writer_router::ProjectionKindWriterRouter;

#[test]
fn router_selects_only_writer_owned_by_projection_kind() {
    for (kind, expected_search, expected_relations) in [
        (ProjectionKind::Search, 1, 0),
        (ProjectionKind::Links, 0, 1),
        (ProjectionKind::Graph, 0, 1),
    ] {
        let mut search = Writer::default();
        let mut relations = Writer::default();
        let mut router = ProjectionKindWriterRouter::new(&mut search, &mut relations);
        router
            .write(&identity(kind), &body(), &parsed())
            .expect("route success");
        assert_eq!(search.calls, expected_search);
        assert_eq!(relations.calls, expected_relations);
    }
}

#[test]
fn router_forwards_identity_body_parsed_and_child_result_unchanged() {
    let mut search = Writer {
        result: Err(ProjectionWriteError::Retryable),
        ..Writer::default()
    };
    let mut relations = Writer::default();
    let mut router = ProjectionKindWriterRouter::new(&mut search, &mut relations);
    assert_eq!(
        router.write(&identity(ProjectionKind::Search), &body(), &parsed()),
        Err(ProjectionWriteError::Retryable)
    );
    assert_eq!(search.kind, Some(ProjectionKind::Search));
    assert_eq!(search.version.as_deref(), Some("v1"));
    assert_eq!(search.body.as_deref(), Some("exact body"));
    assert_eq!(search.wikilink_count, Some(0));
}

#[test]
fn router_routes_delete_to_only_the_writer_owned_by_projection_kind() {
    for (kind, expected_search, expected_relations) in [
        (ProjectionKind::Search, 1, 0),
        (ProjectionKind::Links, 0, 1),
        (ProjectionKind::Graph, 0, 1),
    ] {
        let mut search = Writer::default();
        let mut relations = Writer::default();
        ProjectionKindWriterRouter::new(&mut search, &mut relations)
            .delete(&identity(kind))
            .unwrap();
        assert_eq!(search.delete_calls, expected_search);
        assert_eq!(relations.delete_calls, expected_relations);
    }
}

struct Writer {
    calls: usize,
    kind: Option<ProjectionKind>,
    version: Option<String>,
    body: Option<String>,
    wikilink_count: Option<usize>,
    delete_calls: usize,
    result: Result<(), ProjectionWriteError>,
}

impl Default for Writer {
    fn default() -> Self {
        Self {
            calls: 0,
            kind: None,
            version: None,
            body: None,
            wikilink_count: None,
            delete_calls: 0,
            result: Ok(()),
        }
    }
}

impl VersionedProjectionWriter for Writer {
    fn write(
        &mut self,
        identity: &ProjectionWorkIdentity,
        body: &DocumentBody,
        parsed: &ParsedMarkdown,
    ) -> Result<(), ProjectionWriteError> {
        self.calls += 1;
        self.kind = Some(identity.kind());
        self.version = Some(identity.version_id().as_str().to_string());
        self.body = Some(body.as_str().to_string());
        self.wikilink_count = Some(parsed.wikilinks().len());
        self.result
    }

    fn delete(&mut self, _: &ProjectionWorkIdentity) -> Result<(), ProjectionWriteError> {
        self.delete_calls += 1;
        self.result
    }
}

fn identity(kind: ProjectionKind) -> ProjectionWorkIdentity {
    ProjectionWorkIdentity::new(
        WorkspaceId::new("workspace-1").unwrap(),
        DocumentId::new("doc-1").unwrap(),
        VersionId::new("v1").unwrap(),
        kind,
    )
}

fn body() -> DocumentBody {
    DocumentBody::new("exact body", DocumentBodyPolicy::new(1024).expect("policy")).unwrap()
}

fn parsed() -> ParsedMarkdown {
    ParsedMarkdown::new(vec![], vec![], vec![])
}
