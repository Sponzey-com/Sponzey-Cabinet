use std::collections::HashMap;

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
use cabinet_ports::document_revision_commit::{
    DocumentOperationJournalClaim, DocumentOperationJournalError, DocumentOperationJournalPort,
    DocumentOperationJournalRecord, DocumentOperationJournalState,
    DocumentOperationTerminalFailure, DocumentRevisionCommitError, DocumentRevisionCommitPort,
    DocumentRevisionCommitRequest, DocumentRevisionCommitResult,
};
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot};

#[derive(Default)]
struct FakeCommitPort {
    requests: Vec<DocumentRevisionCommitRequest>,
    failure: Option<DocumentRevisionCommitError>,
}

impl DocumentRevisionCommitPort for FakeCommitPort {
    fn commit_revision(
        &mut self,
        request: DocumentRevisionCommitRequest,
    ) -> Result<DocumentRevisionCommitResult, DocumentRevisionCommitError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        let result = DocumentRevisionCommitResult::new(
            request.record().version_id().clone(),
            DocumentRevisionNumber::new(1).expect("revision"),
        );
        self.requests.push(request);
        Ok(result)
    }
}

#[derive(Default)]
struct FakeJournalPort {
    records: HashMap<String, DocumentOperationJournalRecord>,
}

impl DocumentOperationJournalPort for FakeJournalPort {
    fn load_operation(
        &self,
        operation_id: &DocumentOperationId,
    ) -> Result<Option<DocumentOperationJournalRecord>, DocumentOperationJournalError> {
        Ok(self.records.get(operation_id.as_str()).cloned())
    }

    fn claim_operation(
        &mut self,
        identity: DocumentOperationIdentity,
    ) -> Result<DocumentOperationJournalClaim, DocumentOperationJournalError> {
        if let Some(existing) = self.records.get(identity.operation_id().as_str()) {
            if existing.identity() != &identity {
                return Err(DocumentOperationJournalError::IdentityConflict);
            }
            return Ok(DocumentOperationJournalClaim::Existing(existing.clone()));
        }
        let record = DocumentOperationJournalRecord::claimed(identity);
        self.records.insert(
            record.identity().operation_id().as_str().to_string(),
            record,
        );
        Ok(DocumentOperationJournalClaim::Claimed)
    }

    fn complete_operation(
        &mut self,
        operation_id: &DocumentOperationId,
        result: DocumentRevisionCommitResult,
    ) -> Result<(), DocumentOperationJournalError> {
        let record = self
            .records
            .get(operation_id.as_str())
            .cloned()
            .ok_or(DocumentOperationJournalError::NotClaimed)?;
        let completed = record.complete(result)?;
        self.records
            .insert(operation_id.as_str().to_string(), completed);
        Ok(())
    }

    fn fail_operation(
        &mut self,
        operation_id: &DocumentOperationId,
        failure: DocumentOperationTerminalFailure,
    ) -> Result<(), DocumentOperationJournalError> {
        let record = self
            .records
            .get(operation_id.as_str())
            .cloned()
            .ok_or(DocumentOperationJournalError::NotClaimed)?;
        let failed = record.fail(failure)?;
        self.records
            .insert(operation_id.as_str().to_string(), failed);
        Ok(())
    }
}

#[test]
fn commit_request_rejects_record_for_another_document() {
    let error = DocumentRevisionCommitRequest::new(
        identity("operation-1", "doc-1"),
        version_record("doc-2", "version-1", "snapshot-1"),
    )
    .expect_err("document mismatch must fail");

    assert_eq!(error, DocumentRevisionCommitError::IdentityMismatch);
    assert_eq!(error.code(), "document_revision_commit.identity_mismatch");
}

#[test]
fn commit_port_is_replaceable_and_returns_version_and_revision() {
    let request = DocumentRevisionCommitRequest::new(
        identity("operation-1", "doc-1"),
        version_record("doc-1", "version-1", "snapshot-1"),
    )
    .expect("request");
    let mut port = FakeCommitPort::default();

    let result = port.commit_revision(request).expect("commit");

    assert_eq!(result.version_id().as_str(), "version-1");
    assert_eq!(result.revision_number().value(), 1);
    assert_eq!(port.requests.len(), 1);
}

#[test]
fn journal_distinguishes_existing_identity_from_identity_conflict() {
    let original_identity = identity("operation-1", "doc-1");
    let mut journal = FakeJournalPort::default();

    assert_eq!(
        journal
            .claim_operation(original_identity.clone())
            .expect("claim"),
        DocumentOperationJournalClaim::Claimed
    );
    assert!(matches!(
        journal
            .claim_operation(original_identity)
            .expect("existing"),
        DocumentOperationJournalClaim::Existing(_)
    ));
    assert_eq!(
        journal
            .claim_operation(identity("operation-1", "doc-2"))
            .expect_err("identity conflict"),
        DocumentOperationJournalError::IdentityConflict
    );
}

#[test]
fn journal_requires_claim_before_complete_and_keeps_terminal_result() {
    let operation_id = DocumentOperationId::new("operation-1").expect("operation id");
    let result = DocumentRevisionCommitResult::new(
        VersionId::new("version-1").expect("version"),
        DocumentRevisionNumber::new(1).expect("revision"),
    );
    let mut journal = FakeJournalPort::default();

    assert_eq!(
        journal
            .complete_operation(&operation_id, result.clone())
            .expect_err("claim required"),
        DocumentOperationJournalError::NotClaimed
    );
    journal
        .claim_operation(identity("operation-1", "doc-1"))
        .expect("claim");
    journal
        .complete_operation(&operation_id, result.clone())
        .expect("complete");
    let record = journal
        .load_operation(&operation_id)
        .expect("load")
        .expect("record");

    assert_eq!(record.state(), DocumentOperationJournalState::Committed);
    assert_eq!(record.result(), Some(&result));
    assert_eq!(
        journal
            .complete_operation(&operation_id, result)
            .expect_err("terminal record cannot complete twice"),
        DocumentOperationJournalError::AlreadyCompleted
    );
}

#[test]
fn journal_records_terminal_failure_and_rejects_terminal_overwrite() {
    let operation_id = DocumentOperationId::new("operation-1").expect("operation id");
    let result = DocumentRevisionCommitResult::new(
        VersionId::new("version-1").expect("version"),
        DocumentRevisionNumber::new(1).expect("revision"),
    );
    let mut journal = FakeJournalPort::default();

    assert_eq!(
        journal
            .fail_operation(&operation_id, DocumentOperationTerminalFailure::Conflict)
            .expect_err("claim required"),
        DocumentOperationJournalError::NotClaimed
    );
    journal
        .claim_operation(identity("operation-1", "doc-1"))
        .expect("claim");
    journal
        .fail_operation(&operation_id, DocumentOperationTerminalFailure::Conflict)
        .expect("fail terminal");
    let record = journal
        .load_operation(&operation_id)
        .expect("load")
        .expect("record");

    assert_eq!(record.state(), DocumentOperationJournalState::Failed);
    assert_eq!(
        record.failure(),
        Some(DocumentOperationTerminalFailure::Conflict)
    );
    assert_eq!(
        DocumentOperationTerminalFailure::Conflict.code(),
        "document_operation.conflict"
    );
    assert_eq!(
        DocumentOperationTerminalFailure::InvalidRequest.code(),
        "document_operation.invalid_request"
    );
    assert!(record.result().is_none());
    assert_eq!(
        journal
            .complete_operation(&operation_id, result)
            .expect_err("failed is terminal"),
        DocumentOperationJournalError::AlreadyCompleted
    );
    assert_eq!(
        journal
            .fail_operation(
                &operation_id,
                DocumentOperationTerminalFailure::InvalidRequest
            )
            .expect_err("failure cannot change"),
        DocumentOperationJournalError::AlreadyCompleted
    );
}

fn identity(operation_id: &str, document_id: &str) -> DocumentOperationIdentity {
    DocumentOperationIdentity::new(
        DocumentOperationId::new(operation_id).expect("operation id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        DocumentId::new(document_id).expect("document id"),
        DocumentMutationKind::Update,
        DocumentExpectedCurrentVersion::MustMatch(
            VersionId::new("current-version").expect("current version"),
        ),
    )
    .expect("identity")
}

fn version_record(document_id: &str, version_id: &str, snapshot_ref: &str) -> VersionRecord {
    VersionRecord::new(
        VersionEntry::new(
            VersionId::new(version_id).expect("version id"),
            DocumentId::new(document_id).expect("document id"),
            DocumentSnapshotRef::new(snapshot_ref).expect("snapshot ref"),
            VersionAuthor::new("writer").expect("author"),
            VersionSummary::new("Saved").expect("summary"),
        )
        .expect("entry"),
        VersionSnapshot::new(
            DocumentId::new(document_id).expect("document id"),
            DocumentSnapshotRef::new(snapshot_ref).expect("snapshot ref"),
            DocumentBody::new("Body", DocumentBodyPolicy::new(1024).expect("policy"))
                .expect("body"),
        ),
    )
    .expect("record")
}
