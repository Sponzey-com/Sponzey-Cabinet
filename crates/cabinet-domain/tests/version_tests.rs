use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::{
    CurrentDocumentSnapshot, DocumentRevisionNumber, DocumentRevisionNumberState,
    DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionError, VersionId, VersionSummary,
};

#[test]
fn document_revision_number_requires_a_non_zero_value() {
    assert_eq!(
        DocumentRevisionNumber::new(0).expect_err("zero must fail"),
        VersionError::InvalidRevisionNumber
    );
    assert_eq!(
        VersionError::InvalidRevisionNumber.code(),
        "version.invalid_revision_number"
    );
    assert_eq!(
        DocumentRevisionNumber::new(1)
            .expect("first revision")
            .value(),
        1
    );
    assert_eq!(
        DocumentRevisionNumber::new(u64::MAX)
            .expect("maximum revision")
            .value(),
        u64::MAX
    );
}

#[test]
fn version_entry_distinguishes_legacy_from_assigned_revision_number() {
    let legacy = version_entry();
    assert_eq!(
        legacy.revision_number_state(),
        &DocumentRevisionNumberState::LegacyUnassigned
    );

    let assigned = legacy
        .with_revision_number(DocumentRevisionNumber::new(7).expect("revision"))
        .expect("first assignment");
    assert_eq!(
        assigned.revision_number().map(|value| value.value()),
        Some(7)
    );
    assert_eq!(
        assigned.revision_number_state(),
        &DocumentRevisionNumberState::Assigned(DocumentRevisionNumber::new(7).expect("revision"))
    );
}

#[test]
fn version_entry_rejects_revision_number_reassignment() {
    let assigned = version_entry()
        .with_created_at_epoch_ms(1_721_000_000_123)
        .expect("created at")
        .with_revision_number(DocumentRevisionNumber::new(1).expect("revision"))
        .expect("first assignment");

    let error = assigned
        .with_revision_number(DocumentRevisionNumber::new(2).expect("revision"))
        .expect_err("reassignment must fail");

    assert_eq!(error, VersionError::RevisionNumberAlreadyAssigned);
    assert_eq!(error.code(), "version.revision_number_already_assigned");
}

#[test]
fn current_document_snapshot_keeps_current_body_without_history_identity() {
    let body = DocumentBody::new(
        "Current content",
        DocumentBodyPolicy::new(128).expect("policy"),
    )
    .expect("body");
    let snapshot = CurrentDocumentSnapshot::new(DocumentId::new("doc-1").expect("id"), body);

    assert_eq!(snapshot.document_id().as_str(), "doc-1");
    assert_eq!(snapshot.body().as_str(), "Current content");
}

#[test]
fn version_entry_validates_identity_author_summary_and_snapshot_ref() {
    let entry = VersionEntry::new(
        VersionId::new("version-1").expect("version id"),
        DocumentId::new("doc-1").expect("document id"),
        DocumentSnapshotRef::new("snapshot-1").expect("snapshot ref"),
        VersionAuthor::new("writer").expect("author"),
        VersionSummary::new("Initial save").expect("summary"),
    )
    .expect("version entry");

    assert_eq!(entry.version_id().as_str(), "version-1");
    assert_eq!(entry.document_id().as_str(), "doc-1");
    assert_eq!(entry.snapshot_ref().as_str(), "snapshot-1");
    assert_eq!(entry.author().as_str(), "writer");
    assert_eq!(entry.summary().as_str(), "Initial save");
    assert_eq!(entry.created_at_epoch_ms(), None);
    let timestamped = entry
        .with_created_at_epoch_ms(1_721_000_000_123)
        .expect("created at");
    assert_eq!(timestamped.created_at_epoch_ms(), Some(1_721_000_000_123));
    assert_eq!(
        VersionId::new(" ").expect_err("empty version id must fail"),
        VersionError::EmptyVersionId
    );
    assert_eq!(
        VersionAuthor::new(" ").expect_err("empty author must fail"),
        VersionError::EmptyAuthor
    );
    assert_eq!(
        timestamped
            .with_created_at_epoch_ms(0)
            .expect_err("zero timestamp must fail"),
        VersionError::InvalidCreatedAt
    );
}

#[test]
fn current_snapshot_and_history_entry_are_distinct_domain_types() {
    let current = CurrentDocumentSnapshot::new(
        DocumentId::new("doc-1").expect("document id"),
        DocumentBody::new("Current", DocumentBodyPolicy::new(128).expect("policy")).expect("body"),
    );
    let history = VersionEntry::new(
        VersionId::new("version-1").expect("version id"),
        DocumentId::new("doc-1").expect("document id"),
        DocumentSnapshotRef::new("snapshot-1").expect("snapshot ref"),
        VersionAuthor::new("writer").expect("author"),
        VersionSummary::new("Past save").expect("summary"),
    )
    .expect("version entry");

    assert_eq!(current.document_id(), history.document_id());
    assert_eq!(current.body().as_str(), "Current");
    assert_eq!(history.snapshot_ref().as_str(), "snapshot-1");
}

fn version_entry() -> VersionEntry {
    VersionEntry::new(
        VersionId::new("version-1").expect("version id"),
        DocumentId::new("doc-1").expect("document id"),
        DocumentSnapshotRef::new("snapshot-1").expect("snapshot ref"),
        VersionAuthor::new("writer").expect("author"),
        VersionSummary::new("Saved document").expect("summary"),
    )
    .expect("version entry")
}
