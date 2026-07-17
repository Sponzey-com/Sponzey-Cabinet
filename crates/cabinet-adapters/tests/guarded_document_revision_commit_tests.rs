use std::cell::RefCell;
use std::rc::Rc;

use cabinet_adapters::guarded_document_revision_commit::GuardedDocumentRevisionCommit;
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationKind, DocumentOperationId,
    DocumentOperationIdentity,
};
use cabinet_domain::version::{
    DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId,
    VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::document_revision_commit::{
    DocumentRevisionCommitError, DocumentRevisionCommitPort, DocumentRevisionCommitRequest,
};
use cabinet_ports::version_preparation::{
    PreparedVersion, VersionPreparationError, VersionPreparationOutcome, VersionPreparationPort,
};
use cabinet_ports::version_publication::{
    PublishedVersion, VersionPublicationError, VersionPublicationPort,
};
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot};

struct FakeVersionLifecycle {
    calls: Rc<RefCell<Vec<&'static str>>>,
    prepare_error: Option<VersionPreparationError>,
    publish_result: Result<PublishedVersion, VersionPublicationError>,
}

impl VersionPreparationPort for FakeVersionLifecycle {
    fn prepare_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        operation_id: &DocumentOperationId,
        record: VersionRecord,
    ) -> Result<VersionPreparationOutcome, VersionPreparationError> {
        self.calls.borrow_mut().push("prepare");
        match self.prepare_error {
            Some(error) => Err(error),
            None => Ok(VersionPreparationOutcome::Prepared(PreparedVersion::new(
                operation_id.clone(),
                record,
            ))),
        }
    }

    fn load_prepared(
        &self,
        _workspace_id: &WorkspaceId,
        _operation_id: &DocumentOperationId,
    ) -> Result<Option<PreparedVersion>, VersionPreparationError> {
        unreachable!("guarded commit prepares before pointer work")
    }

    fn discard_prepared(
        &mut self,
        _workspace_id: &WorkspaceId,
        _operation_id: &DocumentOperationId,
    ) -> Result<(), VersionPreparationError> {
        self.calls.borrow_mut().push("discard");
        Ok(())
    }
}

impl VersionPublicationPort for FakeVersionLifecycle {
    fn publish_prepared(
        &mut self,
        _workspace_id: &WorkspaceId,
        _operation_id: &DocumentOperationId,
    ) -> Result<PublishedVersion, VersionPublicationError> {
        self.calls.borrow_mut().push("publish");
        self.publish_result.clone()
    }
}

struct FakePointer {
    calls: Rc<RefCell<Vec<&'static str>>>,
    expected_values: Rc<RefCell<Vec<Option<String>>>>,
    current: Option<VersionId>,
    cas_error: Option<CurrentDocumentVersionPointerError>,
}

impl CurrentDocumentVersionPointerPort for FakePointer {
    fn load_current_version(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<Option<VersionId>, CurrentDocumentVersionPointerError> {
        self.calls.borrow_mut().push("load");
        Ok(self.current.clone())
    }

    fn compare_and_set_current_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        expected: Option<&VersionId>,
        next: VersionId,
    ) -> Result<(), CurrentDocumentVersionPointerError> {
        self.calls.borrow_mut().push("cas");
        self.expected_values
            .borrow_mut()
            .push(expected.map(|value| value.as_str().to_string()));
        match self.cas_error {
            Some(error) => Err(error),
            None => {
                self.current = Some(next);
                Ok(())
            }
        }
    }
}

#[test]
fn successful_update_prepares_compares_and_publishes_in_order() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let expected_values = Rc::new(RefCell::new(Vec::new()));
    let mut versions = lifecycle(Rc::clone(&calls), None, Ok(published("version-2", 2)));
    let mut pointer = pointer(
        Rc::clone(&calls),
        Rc::clone(&expected_values),
        Some("version-1"),
        None,
    );
    let mut commit = GuardedDocumentRevisionCommit::new(&mut versions, &mut pointer);

    let result = commit
        .commit_revision(request(false))
        .expect("guarded commit");

    assert_eq!(result.version_id().as_str(), "version-2");
    assert_eq!(result.revision_number().value(), 2);
    assert_eq!(*calls.borrow(), vec!["prepare", "cas", "publish"]);
    assert_eq!(
        *expected_values.borrow(),
        vec![Some("version-1".to_string())]
    );
}

#[test]
fn create_maps_must_not_exist_to_none_expected_pointer() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let expected_values = Rc::new(RefCell::new(Vec::new()));
    let mut versions = lifecycle(Rc::clone(&calls), None, Ok(published("version-2", 2)));
    let mut pointer = pointer(Rc::clone(&calls), Rc::clone(&expected_values), None, None);
    let mut commit = GuardedDocumentRevisionCommit::new(&mut versions, &mut pointer);

    commit
        .commit_revision(request(true))
        .expect("create commit");

    assert_eq!(*expected_values.borrow(), vec![None]);
}

#[test]
fn prepare_failure_stops_before_pointer_and_publication() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let expected_values = Rc::new(RefCell::new(Vec::new()));
    let mut versions = lifecycle(
        Rc::clone(&calls),
        Some(VersionPreparationError::Conflict),
        Ok(published("version-2", 2)),
    );
    let mut pointer = pointer(Rc::clone(&calls), expected_values, Some("version-1"), None);
    let mut commit = GuardedDocumentRevisionCommit::new(&mut versions, &mut pointer);

    let error = commit
        .commit_revision(request(false))
        .expect_err("prepare conflict");

    assert_eq!(error, DocumentRevisionCommitError::Conflict);
    assert_eq!(*calls.borrow(), vec!["prepare"]);
}

#[test]
fn stale_pointer_conflict_discards_prepared_without_publication() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let expected_values = Rc::new(RefCell::new(Vec::new()));
    let mut versions = lifecycle(Rc::clone(&calls), None, Ok(published("version-2", 2)));
    let mut pointer = pointer(
        Rc::clone(&calls),
        expected_values,
        Some("version-other"),
        Some(CurrentDocumentVersionPointerError::Conflict),
    );
    let mut commit = GuardedDocumentRevisionCommit::new(&mut versions, &mut pointer);

    let error = commit
        .commit_revision(request(false))
        .expect_err("stale conflict");

    assert_eq!(error, DocumentRevisionCommitError::Conflict);
    assert_eq!(*calls.borrow(), vec!["prepare", "cas", "load", "discard"]);
}

#[test]
fn already_next_pointer_resumes_publication_after_cas_conflict() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let expected_values = Rc::new(RefCell::new(Vec::new()));
    let mut versions = lifecycle(Rc::clone(&calls), None, Ok(published("version-2", 2)));
    let mut pointer = pointer(
        Rc::clone(&calls),
        expected_values,
        Some("version-2"),
        Some(CurrentDocumentVersionPointerError::Conflict),
    );
    let mut commit = GuardedDocumentRevisionCommit::new(&mut versions, &mut pointer);

    let result = commit
        .commit_revision(request(false))
        .expect("resume publication");

    assert_eq!(result.version_id().as_str(), "version-2");
    assert_eq!(*calls.borrow(), vec!["prepare", "cas", "load", "publish"]);
}

#[test]
fn pointer_storage_error_is_typed_when_current_remains_expected() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let expected_values = Rc::new(RefCell::new(Vec::new()));
    let mut versions = lifecycle(Rc::clone(&calls), None, Ok(published("version-2", 2)));
    let mut pointer = pointer(
        Rc::clone(&calls),
        expected_values,
        Some("version-1"),
        Some(CurrentDocumentVersionPointerError::StorageUnavailable),
    );
    let mut commit = GuardedDocumentRevisionCommit::new(&mut versions, &mut pointer);

    let error = commit
        .commit_revision(request(false))
        .expect_err("pointer unavailable");

    assert_eq!(error, DocumentRevisionCommitError::StorageUnavailable);
    assert_eq!(*calls.borrow(), vec!["prepare", "cas", "load"]);
}

#[test]
fn pointer_error_with_unexpected_current_requires_recovery() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let expected_values = Rc::new(RefCell::new(Vec::new()));
    let mut versions = lifecycle(Rc::clone(&calls), None, Ok(published("version-2", 2)));
    let mut pointer = pointer(
        Rc::clone(&calls),
        expected_values,
        Some("unexpected-version"),
        Some(CurrentDocumentVersionPointerError::StorageUnavailable),
    );
    let mut commit = GuardedDocumentRevisionCommit::new(&mut versions, &mut pointer);

    let error = commit
        .commit_revision(request(false))
        .expect_err("pointer outcome cannot be determined");

    assert_eq!(error, DocumentRevisionCommitError::RecoveryRequired);
    assert_eq!(*calls.borrow(), vec!["prepare", "cas", "load"]);
}

#[test]
fn publication_failure_or_mismatched_result_requires_recovery() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let expected_values = Rc::new(RefCell::new(Vec::new()));
    let mut failing_versions = lifecycle(
        Rc::clone(&calls),
        None,
        Err(VersionPublicationError::StorageUnavailable),
    );
    let mut first_pointer = pointer(
        Rc::clone(&calls),
        Rc::clone(&expected_values),
        Some("version-1"),
        None,
    );
    let mut commit = GuardedDocumentRevisionCommit::new(&mut failing_versions, &mut first_pointer);
    assert_eq!(
        commit
            .commit_revision(request(false))
            .expect_err("publication failed"),
        DocumentRevisionCommitError::RecoveryRequired
    );

    calls.borrow_mut().clear();
    let mut mismatching_versions = lifecycle(
        Rc::clone(&calls),
        None,
        Ok(published("different-version", 2)),
    );
    let mut second_pointer = pointer(calls.clone(), expected_values, Some("version-1"), None);
    let mut commit =
        GuardedDocumentRevisionCommit::new(&mut mismatching_versions, &mut second_pointer);
    assert_eq!(
        commit
            .commit_revision(request(false))
            .expect_err("publication result mismatch"),
        DocumentRevisionCommitError::RecoveryRequired
    );
}

fn lifecycle(
    calls: Rc<RefCell<Vec<&'static str>>>,
    prepare_error: Option<VersionPreparationError>,
    publish_result: Result<PublishedVersion, VersionPublicationError>,
) -> FakeVersionLifecycle {
    FakeVersionLifecycle {
        calls,
        prepare_error,
        publish_result,
    }
}

fn pointer(
    calls: Rc<RefCell<Vec<&'static str>>>,
    expected_values: Rc<RefCell<Vec<Option<String>>>>,
    current: Option<&str>,
    cas_error: Option<CurrentDocumentVersionPointerError>,
) -> FakePointer {
    FakePointer {
        calls,
        expected_values,
        current: current.map(|value| VersionId::new(value).expect("current")),
        cas_error,
    }
}

fn request(create: bool) -> DocumentRevisionCommitRequest {
    let expected = if create {
        DocumentExpectedCurrentVersion::MustNotExist
    } else {
        DocumentExpectedCurrentVersion::MustMatch(VersionId::new("version-1").expect("expected"))
    };
    let kind = if create {
        DocumentMutationKind::Create
    } else {
        DocumentMutationKind::Update
    };
    DocumentRevisionCommitRequest::new(
        DocumentOperationIdentity::new(
            DocumentOperationId::new("operation-1").expect("operation"),
            WorkspaceId::new("workspace-1").expect("workspace"),
            DocumentId::new("doc-1").expect("document"),
            kind,
            expected,
        )
        .expect("identity"),
        record(),
    )
    .expect("request")
}

fn record() -> VersionRecord {
    let document = DocumentId::new("doc-1").expect("document");
    let snapshot_ref = DocumentSnapshotRef::new("snapshot-2").expect("snapshot");
    let entry = VersionEntry::new(
        VersionId::new("version-2").expect("version"),
        document.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("writer").expect("author"),
        VersionSummary::new("Updated").expect("summary"),
    )
    .expect("entry")
    .with_created_at_epoch_ms(200)
    .expect("timestamp")
    .with_revision_number(DocumentRevisionNumber::new(2).expect("revision"))
    .expect("assigned revision");
    VersionRecord::new(
        entry,
        VersionSnapshot::new(
            document,
            snapshot_ref,
            DocumentBody::new("Body", DocumentBodyPolicy::new(1024).expect("policy"))
                .expect("body"),
        ),
    )
    .expect("record")
}

fn published(version: &str, revision: u64) -> PublishedVersion {
    PublishedVersion::new(
        VersionId::new(version).expect("version"),
        DocumentRevisionNumber::new(revision).expect("revision"),
    )
}
