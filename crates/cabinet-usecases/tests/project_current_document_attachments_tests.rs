use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor,
    VersionEntry, VersionId, VersionSummary,
};
use cabinet_ports::current_document_attachment_projection::{
    CurrentDocumentAttachmentProjectionError, CurrentDocumentAttachmentProjectionOutcome,
    CurrentDocumentAttachmentProjectionRequest, CurrentDocumentAttachmentProjectionWriter,
};
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot};
use cabinet_usecases::project_current_document_attachments::{
    ProjectCurrentDocumentAttachmentsError, ProjectCurrentDocumentAttachmentsInput,
    ProjectCurrentDocumentAttachmentsOutcomeKind, ProjectCurrentDocumentAttachmentsUsecase,
};

#[derive(Default)]
struct Writer {
    requests: Vec<CurrentDocumentAttachmentProjectionRequest>,
    outcome: Option<CurrentDocumentAttachmentProjectionOutcome>,
    error: Option<CurrentDocumentAttachmentProjectionError>,
}

impl CurrentDocumentAttachmentProjectionWriter for Writer {
    fn replace_current_document_attachments(
        &mut self,
        request: CurrentDocumentAttachmentProjectionRequest,
    ) -> Result<CurrentDocumentAttachmentProjectionOutcome, CurrentDocumentAttachmentProjectionError>
    {
        self.requests.push(request);
        if let Some(error) = self.error {
            return Err(error);
        }
        Ok(self
            .outcome
            .unwrap_or(CurrentDocumentAttachmentProjectionOutcome::Applied))
    }
}

#[test]
fn known_snapshot_maps_full_sorted_set_and_revision_to_writer() {
    let record = record(
        "doc-1",
        Some(7),
        AttachmentSnapshotState::known(vec![reference('b', "B"), reference('a', "A")]).unwrap(),
    );
    let mut writer = Writer::default();

    let output = ProjectCurrentDocumentAttachmentsUsecase::new()
        .execute(
            ProjectCurrentDocumentAttachmentsInput::new("workspace-1", "doc-1", record),
            &mut writer,
        )
        .expect("project");

    assert_eq!(
        output.kind(),
        ProjectCurrentDocumentAttachmentsOutcomeKind::Applied
    );
    assert_eq!(writer.requests.len(), 1);
    let request = &writer.requests[0];
    assert_eq!(request.workspace_id().as_str(), "workspace-1");
    assert_eq!(request.document_id().as_str(), "doc-1");
    assert_eq!(request.revision_number().value(), 7);
    assert_eq!(
        request.references(),
        &[reference('a', "A"), reference('b', "B")]
    );
}

#[test]
fn known_empty_is_explicit_replacement_and_already_current_is_preserved() {
    let mut writer = Writer {
        outcome: Some(CurrentDocumentAttachmentProjectionOutcome::AlreadyCurrent),
        ..Writer::default()
    };

    let output = ProjectCurrentDocumentAttachmentsUsecase::new()
        .execute(
            ProjectCurrentDocumentAttachmentsInput::new(
                "workspace-1",
                "doc-1",
                record(
                    "doc-1",
                    Some(2),
                    AttachmentSnapshotState::known(Vec::new()).unwrap(),
                ),
            ),
            &mut writer,
        )
        .expect("already current");

    assert_eq!(
        output.kind(),
        ProjectCurrentDocumentAttachmentsOutcomeKind::AlreadyCurrent
    );
    assert!(writer.requests[0].references().is_empty());
}

#[test]
fn legacy_unknown_is_preserved_without_writer_call() {
    let mut writer = Writer::default();

    let output = ProjectCurrentDocumentAttachmentsUsecase::new()
        .execute(
            ProjectCurrentDocumentAttachmentsInput::new(
                "workspace-1",
                "doc-1",
                record("doc-1", Some(1), AttachmentSnapshotState::legacy_unknown()),
            ),
            &mut writer,
        )
        .expect("legacy preserved");

    assert_eq!(
        output.kind(),
        ProjectCurrentDocumentAttachmentsOutcomeKind::LegacyPreserved
    );
    assert!(writer.requests.is_empty());
}

#[test]
fn identity_mismatch_and_missing_revision_are_rejected_before_writer() {
    let mut writer = Writer::default();
    let mismatch = ProjectCurrentDocumentAttachmentsUsecase::new()
        .execute(
            ProjectCurrentDocumentAttachmentsInput::new(
                "workspace-1",
                "doc-expected",
                record(
                    "doc-actual",
                    Some(1),
                    AttachmentSnapshotState::known(Vec::new()).unwrap(),
                ),
            ),
            &mut writer,
        )
        .unwrap_err();
    assert_eq!(
        mismatch,
        ProjectCurrentDocumentAttachmentsError::CorruptedRecord
    );

    let missing_revision = ProjectCurrentDocumentAttachmentsUsecase::new()
        .execute(
            ProjectCurrentDocumentAttachmentsInput::new(
                "workspace-1",
                "doc-1",
                record(
                    "doc-1",
                    None,
                    AttachmentSnapshotState::known(Vec::new()).unwrap(),
                ),
            ),
            &mut writer,
        )
        .unwrap_err();
    assert_eq!(
        missing_revision,
        ProjectCurrentDocumentAttachmentsError::CorruptedRecord
    );
    assert!(writer.requests.is_empty());
}

#[test]
fn writer_failures_keep_stable_error_categories() {
    let cases = [
        (
            CurrentDocumentAttachmentProjectionError::Conflict,
            ProjectCurrentDocumentAttachmentsError::Conflict,
        ),
        (
            CurrentDocumentAttachmentProjectionError::StorageUnavailable,
            ProjectCurrentDocumentAttachmentsError::StorageUnavailable,
        ),
        (
            CurrentDocumentAttachmentProjectionError::CorruptedProjection,
            ProjectCurrentDocumentAttachmentsError::CorruptedProjection,
        ),
    ];
    for (source, expected) in cases {
        let mut writer = Writer {
            error: Some(source),
            ..Writer::default()
        };
        let error = ProjectCurrentDocumentAttachmentsUsecase::new()
            .execute(
                ProjectCurrentDocumentAttachmentsInput::new(
                    "workspace-1",
                    "doc-1",
                    record(
                        "doc-1",
                        Some(1),
                        AttachmentSnapshotState::known(Vec::new()).unwrap(),
                    ),
                ),
                &mut writer,
            )
            .unwrap_err();
        assert_eq!(error, expected);
    }
}

fn record(document: &str, revision: Option<u64>, state: AttachmentSnapshotState) -> VersionRecord {
    let document_id = DocumentId::new(document).unwrap();
    let snapshot_ref = DocumentSnapshotRef::new("snapshot-1").unwrap();
    let mut entry = VersionEntry::new(
        VersionId::new("version-1").unwrap(),
        document_id.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("local-user").unwrap(),
        VersionSummary::new("Snapshot").unwrap(),
    )
    .unwrap()
    .with_created_at_epoch_ms(1)
    .unwrap();
    if let Some(revision) = revision {
        entry = entry
            .with_revision_number(DocumentRevisionNumber::new(revision).unwrap())
            .unwrap();
    }
    VersionRecord::new(
        entry,
        VersionSnapshot::with_attachment_state(
            document_id,
            snapshot_ref,
            DocumentBody::new("문서\n", DocumentBodyPolicy::new(1024).unwrap()).unwrap(),
            state,
        ),
    )
    .unwrap()
}

fn reference(character: char, label: &str) -> AssetReference {
    AssetReference::new(
        AssetId::from_sha256_hex(&std::iter::repeat_n(character, 64).collect::<String>()).unwrap(),
        label,
    )
    .unwrap()
}
