use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::{
    CurrentDocumentSnapshot, DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionError,
    VersionId, VersionSummary,
};

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
    assert_eq!(
        VersionId::new(" ").expect_err("empty version id must fail"),
        VersionError::EmptyVersionId
    );
    assert_eq!(
        VersionAuthor::new(" ").expect_err("empty author must fail"),
        VersionError::EmptyAuthor
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
