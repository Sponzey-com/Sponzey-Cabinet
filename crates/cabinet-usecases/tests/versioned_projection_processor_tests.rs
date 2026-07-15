use std::cell::{Cell, RefCell};

use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::projection_work::{
    ProjectionChangeKind, ProjectionKind, ProjectionWorkIdentity,
};
use cabinet_domain::version::{DocumentSnapshotRef, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::markdown_parser::{MarkdownParser, MarkdownParserError, ParsedMarkdown};
use cabinet_ports::projection_worker::{ProjectionProcessorResult, ProjectionWorkProcessor};
use cabinet_ports::projection_writer::{ProjectionWriteError, VersionedProjectionWriter};
use cabinet_ports::version_store::{
    HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};
use cabinet_usecases::versioned_projection_processor::VersionedMarkdownProjectionProcessor;

#[test]
fn processor_reads_exact_version_parses_and_writes_kind_without_body_leak() {
    let versions = FakeVersions {
        snapshot: Some(snapshot("expected body")),
        error: None,
        read_count: Cell::new(0),
    };
    let parser = FakeParser::default();
    let mut writer = FakeWriter::default();
    let mut processor = VersionedMarkdownProjectionProcessor::new(&versions, &parser, &mut writer);

    assert_eq!(
        processor.process(&identity()),
        ProjectionProcessorResult::Succeeded
    );
    assert_eq!(parser.received.borrow().as_deref(), Some("expected body"));
    assert_eq!(writer.kind, Some(ProjectionKind::Graph));
    assert_eq!(writer.version.as_deref(), Some("version-1"));
    assert_eq!(writer.body.as_deref(), Some("expected body"));
}

#[test]
fn processor_maps_missing_parse_and_writer_failures_to_stable_categories() {
    let missing = FakeVersions {
        snapshot: None,
        error: None,
        read_count: Cell::new(0),
    };
    let parser = FakeParser::default();
    let mut writer = FakeWriter::default();
    assert_eq!(
        VersionedMarkdownProjectionProcessor::new(&missing, &parser, &mut writer)
            .process(&identity()),
        ProjectionProcessorResult::PermanentFailure
    );

    let versions = FakeVersions {
        snapshot: Some(snapshot("body")),
        error: None,
        read_count: Cell::new(0),
    };
    let failing_parser = FakeParser {
        fail: true,
        ..FakeParser::default()
    };
    assert_eq!(
        VersionedMarkdownProjectionProcessor::new(&versions, &failing_parser, &mut writer)
            .process(&identity()),
        ProjectionProcessorResult::PermanentFailure
    );

    writer.error = Some(ProjectionWriteError::Retryable);
    assert_eq!(
        VersionedMarkdownProjectionProcessor::new(&versions, &parser, &mut writer)
            .process(&identity()),
        ProjectionProcessorResult::RetryableFailure
    );
}

#[test]
fn processor_routes_deleted_work_without_reading_or_parsing_document_body() {
    let versions = FakeVersions {
        snapshot: Some(snapshot("must not be read")),
        error: None,
        read_count: Cell::new(0),
    };
    let parser = FakeParser::default();
    let mut writer = FakeWriter::default();

    assert_eq!(
        VersionedMarkdownProjectionProcessor::new(&versions, &parser, &mut writer)
            .process(&deleted_identity()),
        ProjectionProcessorResult::Succeeded
    );
    assert_eq!(versions.read_count.get(), 0);
    assert_eq!(parser.received.borrow().as_deref(), None);
    assert_eq!(writer.delete_calls, 1);
    assert_eq!(writer.body, None);
}

struct FakeVersions {
    snapshot: Option<VersionSnapshot>,
    error: Option<VersionStoreError>,
    read_count: Cell<usize>,
}
impl VersionStore for FakeVersions {
    fn append_version(
        &mut self,
        _: &WorkspaceId,
        _: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        Ok(())
    }
    fn get_version_snapshot(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
        _: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        self.read_count.set(self.read_count.get() + 1);
        self.error.map_or_else(|| Ok(self.snapshot.clone()), Err)
    }
    fn list_history(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
        _: HistoryPageRequest,
    ) -> Result<HistoryPage, VersionStoreError> {
        unreachable!()
    }
}
#[derive(Default)]
struct FakeParser {
    received: RefCell<Option<String>>,
    fail: bool,
}
impl MarkdownParser for FakeParser {
    fn parse(&self, body: &DocumentBody) -> Result<ParsedMarkdown, MarkdownParserError> {
        *self.received.borrow_mut() = Some(body.as_str().to_string());
        if self.fail {
            Err(MarkdownParserError::EmptyWikilinkTarget)
        } else {
            Ok(ParsedMarkdown::new(vec![], vec![], vec![]))
        }
    }
}
#[derive(Default)]
struct FakeWriter {
    kind: Option<ProjectionKind>,
    version: Option<String>,
    body: Option<String>,
    error: Option<ProjectionWriteError>,
    delete_calls: usize,
}
impl VersionedProjectionWriter for FakeWriter {
    fn write(
        &mut self,
        id: &ProjectionWorkIdentity,
        body: &DocumentBody,
        _: &ParsedMarkdown,
    ) -> Result<(), ProjectionWriteError> {
        self.kind = Some(id.kind());
        self.version = Some(id.version_id().as_str().to_string());
        self.body = Some(body.as_str().to_string());
        self.error.map_or(Ok(()), Err)
    }

    fn delete(&mut self, _: &ProjectionWorkIdentity) -> Result<(), ProjectionWriteError> {
        self.delete_calls += 1;
        self.error.map_or(Ok(()), Err)
    }
}
fn identity() -> ProjectionWorkIdentity {
    ProjectionWorkIdentity::new(
        WorkspaceId::new("workspace-1").unwrap(),
        DocumentId::new("doc-1").unwrap(),
        VersionId::new("version-1").unwrap(),
        ProjectionKind::Graph,
    )
}
fn deleted_identity() -> ProjectionWorkIdentity {
    ProjectionWorkIdentity::for_change(
        WorkspaceId::new("workspace-1").unwrap(),
        DocumentId::new("doc-1").unwrap(),
        VersionId::new("version-1").unwrap(),
        ProjectionKind::Graph,
        ProjectionChangeKind::Deleted,
    )
}
fn snapshot(body: &str) -> VersionSnapshot {
    VersionSnapshot::new(
        DocumentId::new("doc-1").unwrap(),
        DocumentSnapshotRef::new("snapshot-1").unwrap(),
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).unwrap()).unwrap(),
    )
}
