use cabinet_domain::document::DocumentBodyPolicy;
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor,
    VersionEntry, VersionId, VersionSummary,
};
use cabinet_ports::current_document_revision_projection::{
    CurrentDocumentRevisionProjection, CurrentDocumentRevisionProjectionError,
    CurrentDocumentRevisionProjectionOutcome, CurrentDocumentRevisionProjectionWriter,
};
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot};
use cabinet_usecases::project_current_document_revision::{
    ProjectCurrentDocumentRevisionError, ProjectCurrentDocumentRevisionInput,
    ProjectCurrentDocumentRevisionUsecase,
};

#[derive(Default)]
struct FakeProjectionWriter {
    result: Option<
        Result<CurrentDocumentRevisionProjectionOutcome, CurrentDocumentRevisionProjectionError>,
    >,
    writes: Vec<(String, CurrentDocumentRevisionProjection)>,
}

impl FakeProjectionWriter {
    fn returning(outcome: CurrentDocumentRevisionProjectionOutcome) -> Self {
        Self {
            result: Some(Ok(outcome)),
            writes: Vec::new(),
        }
    }

    fn failing(error: CurrentDocumentRevisionProjectionError) -> Self {
        Self {
            result: Some(Err(error)),
            writes: Vec::new(),
        }
    }
}

impl CurrentDocumentRevisionProjectionWriter for FakeProjectionWriter {
    fn write_current_projection(
        &mut self,
        workspace_id: &cabinet_domain::workspace::WorkspaceId,
        projection: CurrentDocumentRevisionProjection,
    ) -> Result<CurrentDocumentRevisionProjectionOutcome, CurrentDocumentRevisionProjectionError>
    {
        self.writes
            .push((workspace_id.as_str().to_string(), projection));
        self.result.expect("configured fake result")
    }
}

#[test]
fn projects_committed_revision_with_title_derived_from_first_markdown_line() {
    let record = version_record("version-2", Some(2), "# 변경된 제목\n\n본문");
    let mut writer =
        FakeProjectionWriter::returning(CurrentDocumentRevisionProjectionOutcome::Applied);

    let output = ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            ProjectCurrentDocumentRevisionInput::new("workspace-1", "notes/document.md", record),
            &mut writer,
        )
        .expect("projection succeeds");

    assert_eq!(
        output.outcome(),
        CurrentDocumentRevisionProjectionOutcome::Applied
    );
    assert_eq!(output.version_id().as_str(), "version-2");
    assert_eq!(output.revision_number().value(), 2);
    assert_eq!(writer.writes.len(), 1);
    let (workspace_id, projection) = &writer.writes[0];
    assert_eq!(workspace_id, "workspace-1");
    assert_eq!(projection.version_id().as_str(), "version-2");
    assert_eq!(projection.revision_number().value(), 2);
    assert_eq!(
        projection.record().metadata().title().as_str(),
        "변경된 제목"
    );
    assert_eq!(projection.record().path().as_str(), "notes/document.md");
    assert_eq!(projection.record().body().as_str(), "# 변경된 제목\n\n본문");
}

#[test]
fn preserves_idempotent_already_current_outcome() {
    let record = version_record("version-2", Some(2), "제목\n본문");
    let mut writer =
        FakeProjectionWriter::returning(CurrentDocumentRevisionProjectionOutcome::AlreadyCurrent);

    let output = ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            ProjectCurrentDocumentRevisionInput::new("workspace-1", "document.md", record),
            &mut writer,
        )
        .expect("idempotent projection succeeds");

    assert_eq!(
        output.outcome(),
        CurrentDocumentRevisionProjectionOutcome::AlreadyCurrent
    );
    assert_eq!(writer.writes.len(), 1);
}

#[test]
fn rejects_legacy_revision_and_invalid_path_before_writing() {
    let mut writer =
        FakeProjectionWriter::returning(CurrentDocumentRevisionProjectionOutcome::Applied);
    let legacy_error = ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            ProjectCurrentDocumentRevisionInput::new(
                "workspace-1",
                "document.md",
                version_record("legacy-version", None, "제목"),
            ),
            &mut writer,
        )
        .expect_err("legacy revision must be rejected");
    let invalid_path_error = ProjectCurrentDocumentRevisionUsecase::new()
        .execute(
            ProjectCurrentDocumentRevisionInput::new(
                "workspace-1",
                "../document.md",
                version_record("version-2", Some(2), "제목"),
            ),
            &mut writer,
        )
        .expect_err("invalid path must be rejected");

    assert_eq!(
        legacy_error,
        ProjectCurrentDocumentRevisionError::InvalidInput
    );
    assert_eq!(
        invalid_path_error,
        ProjectCurrentDocumentRevisionError::InvalidInput
    );
    assert!(writer.writes.is_empty());
}

#[test]
fn maps_projection_writer_failures_to_stable_errors() {
    let cases = [
        (
            CurrentDocumentRevisionProjectionError::StaleRevision,
            ProjectCurrentDocumentRevisionError::StaleRevision,
            "current_document_projection.stale_revision",
        ),
        (
            CurrentDocumentRevisionProjectionError::RevisionConflict,
            ProjectCurrentDocumentRevisionError::RevisionConflict,
            "current_document_projection.revision_conflict",
        ),
        (
            CurrentDocumentRevisionProjectionError::StorageUnavailable,
            ProjectCurrentDocumentRevisionError::StorageUnavailable,
            "current_document_projection.storage_unavailable",
        ),
        (
            CurrentDocumentRevisionProjectionError::CorruptedProjection,
            ProjectCurrentDocumentRevisionError::CorruptedProjection,
            "current_document_projection.corrupted_projection",
        ),
    ];

    for (port_error, expected_error, expected_code) in cases {
        let mut writer = FakeProjectionWriter::failing(port_error);
        let error = ProjectCurrentDocumentRevisionUsecase::new()
            .execute(
                ProjectCurrentDocumentRevisionInput::new(
                    "workspace-1",
                    "document.md",
                    version_record("version-2", Some(2), "제목"),
                ),
                &mut writer,
            )
            .expect_err("writer failure must surface");

        assert_eq!(error, expected_error);
        assert_eq!(error.code(), expected_code);
        assert_eq!(writer.writes.len(), 1);
    }
}

fn version_record(version_id: &str, revision: Option<u64>, body: &str) -> VersionRecord {
    let document_id = cabinet_domain::document::DocumentId::new("document-1").unwrap();
    let snapshot_ref = DocumentSnapshotRef::new(&format!("snapshot:{version_id}")).unwrap();
    let mut entry = VersionEntry::new(
        VersionId::new(version_id).unwrap(),
        document_id.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("local-user").unwrap(),
        VersionSummary::new("save").unwrap(),
    )
    .unwrap();
    if let Some(revision) = revision {
        entry = entry
            .with_revision_number(DocumentRevisionNumber::new(revision).unwrap())
            .unwrap();
    }
    let snapshot = VersionSnapshot::with_attachment_state(
        document_id,
        snapshot_ref,
        cabinet_domain::document::DocumentBody::new(body, DocumentBodyPolicy::new(4096).unwrap())
            .unwrap(),
        AttachmentSnapshotState::known(Vec::new()).unwrap(),
    );
    VersionRecord::new(entry, snapshot).unwrap()
}
