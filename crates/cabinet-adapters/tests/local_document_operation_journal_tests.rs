use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_document_operation_journal::{
    DOCUMENT_OPERATION_JOURNAL_DIR, LocalDocumentOperationJournal,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationFingerprint, DocumentMutationKind,
    DocumentOperationId, DocumentOperationIdentity,
};
use cabinet_domain::version::{DocumentRevisionNumber, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_revision_commit::{
    DocumentOperationJournalClaim, DocumentOperationJournalError, DocumentOperationJournalPort,
    DocumentOperationJournalState, DocumentOperationTerminalFailure, DocumentRevisionCommitResult,
};

#[test]
fn local_journal_claim_survives_restart_and_distinguishes_identity_conflict() {
    let temp = TempRoot::new("claim-restart");
    let original_identity = identity("operation-1", "doc-1");
    let mut journal = LocalDocumentOperationJournal::new(temp.path.clone());

    assert_eq!(
        journal
            .claim_operation(original_identity.clone())
            .expect("first claim"),
        DocumentOperationJournalClaim::Claimed
    );
    drop(journal);

    let mut restarted = LocalDocumentOperationJournal::new(temp.path.clone());
    assert!(matches!(
        restarted
            .claim_operation(original_identity.clone())
            .expect("same identity"),
        DocumentOperationJournalClaim::Existing(_)
    ));
    assert_eq!(
        restarted
            .claim_operation(identity("operation-1", "doc-2"))
            .expect_err("different identity must conflict"),
        DocumentOperationJournalError::IdentityConflict
    );
    assert_eq!(
        restarted
            .load_operation(original_identity.operation_id())
            .expect("load")
            .expect("record")
            .state(),
        DocumentOperationJournalState::Claimed
    );
}

#[test]
fn local_journal_requires_claim_and_preserves_terminal_result_after_restart() {
    let temp = TempRoot::new("complete-restart");
    let operation_id = DocumentOperationId::new("operation-1").expect("operation id");
    let result = commit_result("version-2", 2);
    let mut journal = LocalDocumentOperationJournal::new(temp.path.clone());

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
    drop(journal);

    let mut restarted = LocalDocumentOperationJournal::new(temp.path.clone());
    let record = restarted
        .load_operation(&operation_id)
        .expect("load")
        .expect("record");
    assert_eq!(record.state(), DocumentOperationJournalState::Committed);
    assert_eq!(record.result(), Some(&result));
    assert_eq!(
        restarted
            .complete_operation(&operation_id, commit_result("version-3", 3))
            .expect_err("terminal result cannot be replaced"),
        DocumentOperationJournalError::AlreadyCompleted
    );
}

#[test]
fn local_journal_preserves_terminal_failure_after_restart() {
    let temp = TempRoot::new("failed-restart");
    let operation_id = DocumentOperationId::new("operation-1").expect("operation id");
    let mut journal = LocalDocumentOperationJournal::new(temp.path.clone());

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
        .expect("terminal failure");
    drop(journal);

    let mut restarted = LocalDocumentOperationJournal::new(temp.path.clone());
    let record = restarted
        .load_operation(&operation_id)
        .expect("load")
        .expect("record");
    assert_eq!(record.state(), DocumentOperationJournalState::Failed);
    assert_eq!(
        record.failure(),
        Some(DocumentOperationTerminalFailure::Conflict)
    );
    assert!(record.result().is_none());
    assert_eq!(
        restarted
            .complete_operation(&operation_id, commit_result("version-2", 2))
            .expect_err("failed is terminal"),
        DocumentOperationJournalError::AlreadyCompleted
    );
}

#[test]
fn local_journal_reads_legacy_record_without_failure_field() {
    let temp = TempRoot::new("legacy-no-failure-field");
    let operation_id = DocumentOperationId::new("operation-1").expect("operation id");
    let mut journal = LocalDocumentOperationJournal::new(temp.path.clone());
    journal
        .claim_operation(identity("operation-1", "doc-1"))
        .expect("claim");
    let record_path = journal_record_path(&temp, "operation-1");
    let legacy = fs::read_to_string(&record_path)
        .expect("record")
        .replace(",\"failure_code\":null", "")
        .replace(",\"request_fingerprint\":null", "");
    fs::write(&record_path, legacy).expect("legacy fixture");

    let record = journal
        .load_operation(&operation_id)
        .expect("legacy load")
        .expect("record");

    assert_eq!(record.state(), DocumentOperationJournalState::Claimed);
    assert!(record.failure().is_none());
    assert!(record.identity().request_fingerprint().is_none());
}

#[test]
fn local_journal_persists_fingerprint_and_rejects_changed_request_after_restart() {
    let temp = TempRoot::new("fingerprint-restart");
    let original = identity_with_fingerprint("operation-1", "sha256:first");
    let mut journal = LocalDocumentOperationJournal::new(temp.path.clone());
    journal
        .claim_operation(original.clone())
        .expect("first claim");
    drop(journal);

    let mut restarted = LocalDocumentOperationJournal::new(temp.path.clone());
    let loaded = restarted
        .load_operation(original.operation_id())
        .expect("load")
        .expect("record");
    assert_eq!(
        loaded
            .identity()
            .request_fingerprint()
            .expect("fingerprint")
            .as_str(),
        "sha256:first"
    );
    assert!(matches!(
        restarted
            .claim_operation(original)
            .expect("same fingerprint"),
        DocumentOperationJournalClaim::Existing(_)
    ));
    assert_eq!(
        restarted
            .claim_operation(identity_with_fingerprint("operation-1", "sha256:changed"))
            .expect_err("changed request must conflict"),
        DocumentOperationJournalError::IdentityConflict
    );

    let content = fs::read_to_string(journal_record_path(&temp, "operation-1")).expect("record");
    assert!(content.contains("\"request_fingerprint\":\"sha256:first\""));
    for forbidden in ["document body", "document title", "asset payload"] {
        assert!(!content.contains(forbidden));
    }
}

#[test]
fn local_journal_rejects_corrupt_records_instead_of_treating_them_as_missing() {
    let temp = TempRoot::new("corrupt");
    let operation_id = DocumentOperationId::new("operation-1").expect("operation id");
    let mut journal = LocalDocumentOperationJournal::new(temp.path.clone());
    journal
        .claim_operation(identity("operation-1", "doc-1"))
        .expect("claim");
    let record_path = journal_record_path(&temp, "operation-1");
    for fixture in [
        "not-json",
        r#"{"schema_version":2,"state":"claimed"}"#,
        r#"{"schema_version":1,"state":"committed","operation_id":"operation-1","workspace_id":"workspace-1","document_id":"doc-1","mutation_kind":"update","expected_current_version":"version-1","result":null}"#,
    ] {
        fs::write(&record_path, fixture).expect("corrupt fixture");
        assert_eq!(
            journal
                .load_operation(&operation_id)
                .expect_err("corruption must fail"),
            DocumentOperationJournalError::CorruptedJournal
        );
    }
}

#[test]
fn local_journal_schema_excludes_document_and_asset_payloads() {
    let temp = TempRoot::new("payload-boundary");
    let mut journal = LocalDocumentOperationJournal::new(temp.path.clone());
    journal
        .claim_operation(identity("operation-1", "doc-1"))
        .expect("claim");
    let content = fs::read_to_string(journal_record_path(&temp, "operation-1")).expect("record");

    for forbidden in ["body", "title", "path", "asset_label", "file_name"] {
        assert!(!content.contains(forbidden), "forbidden field: {forbidden}");
    }
}

#[test]
fn concurrent_local_journal_claim_has_one_claimed_and_one_existing_result() {
    let temp = TempRoot::new("concurrent-claim");
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let root = temp.path.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let mut journal = LocalDocumentOperationJournal::new(root);
            barrier.wait();
            journal.claim_operation(identity("operation-1", "doc-1"))
        }));
    }

    let claims = handles
        .into_iter()
        .map(|handle| handle.join().expect("thread").expect("claim result"))
        .collect::<Vec<_>>();

    assert_eq!(
        claims
            .iter()
            .filter(|claim| matches!(claim, DocumentOperationJournalClaim::Claimed))
            .count(),
        1
    );
    assert_eq!(
        claims
            .iter()
            .filter(|claim| matches!(claim, DocumentOperationJournalClaim::Existing(_)))
            .count(),
        1
    );
}

#[test]
fn local_journal_scans_only_committed_restore_candidates_in_stable_order() {
    let temp = TempRoot::new("restore-scan");
    let mut journal = LocalDocumentOperationJournal::new(temp.path.clone());
    for (operation, document, kind, version, revision) in [
        (
            "restore-b",
            "doc-b",
            DocumentMutationKind::Restore,
            "version-b",
            3,
        ),
        (
            "update-a",
            "doc-a",
            DocumentMutationKind::Update,
            "version-a",
            2,
        ),
        (
            "restore-a",
            "doc-a",
            DocumentMutationKind::Restore,
            "version-c",
            4,
        ),
    ] {
        let identity = identity_with_kind(operation, document, kind);
        journal.claim_operation(identity).expect("claim");
        journal
            .complete_operation(
                &DocumentOperationId::new(operation).unwrap(),
                commit_result(version, revision),
            )
            .expect("complete");
    }
    journal
        .claim_operation(identity_with_kind(
            "restore-claimed",
            "doc-c",
            DocumentMutationKind::Restore,
        ))
        .expect("claimed only");

    let candidates = journal.list_committed_restore_candidates(20).expect("scan");

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].document_id().as_str(), "doc-a");
    assert_eq!(candidates[0].version_id().as_str(), "version-c");
    assert_eq!(candidates[1].document_id().as_str(), "doc-b");
    assert_eq!(candidates[1].version_id().as_str(), "version-b");
    assert!(journal.list_committed_restore_candidates(0).is_err());
}

#[test]
fn local_journal_restore_scan_rejects_corrupt_record() {
    let temp = TempRoot::new("restore-scan-corrupt");
    let journal = LocalDocumentOperationJournal::new(temp.path.clone());
    fs::create_dir_all(temp.path.join(DOCUMENT_OPERATION_JOURNAL_DIR)).unwrap();
    fs::write(
        temp.path
            .join(DOCUMENT_OPERATION_JOURNAL_DIR)
            .join("broken.json"),
        "not-json",
    )
    .unwrap();

    assert!(journal.list_committed_restore_candidates(20).is_err());
}

fn identity(operation_id: &str, document_id: &str) -> DocumentOperationIdentity {
    identity_with_kind(operation_id, document_id, DocumentMutationKind::Update)
}

fn identity_with_kind(
    operation_id: &str,
    document_id: &str,
    kind: DocumentMutationKind,
) -> DocumentOperationIdentity {
    DocumentOperationIdentity::new(
        DocumentOperationId::new(operation_id).expect("operation id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        DocumentId::new(document_id).expect("document id"),
        kind,
        DocumentExpectedCurrentVersion::MustMatch(VersionId::new("version-1").expect("version")),
    )
    .expect("identity")
}

fn identity_with_fingerprint(operation_id: &str, fingerprint: &str) -> DocumentOperationIdentity {
    identity(operation_id, "doc-1").with_request_fingerprint(
        DocumentMutationFingerprint::new(fingerprint).expect("fingerprint"),
    )
}

fn commit_result(version_id: &str, revision_number: u64) -> DocumentRevisionCommitResult {
    DocumentRevisionCommitResult::new(
        VersionId::new(version_id).expect("version id"),
        DocumentRevisionNumber::new(revision_number).expect("revision"),
    )
}

fn journal_record_path(temp: &TempRoot, operation_id: &str) -> PathBuf {
    temp.path
        .join(DOCUMENT_OPERATION_JOURNAL_DIR)
        .join(format!("{operation_id}.json"))
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-document-operation-journal-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp root");
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
