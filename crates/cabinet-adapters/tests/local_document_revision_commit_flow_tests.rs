use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::guarded_document_revision_commit::GuardedDocumentRevisionCommit;
use cabinet_adapters::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use cabinet_adapters::local_document_operation_journal::LocalDocumentOperationJournal;
use cabinet_adapters::local_version_store::LocalVersionStore;
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
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::document_revision_commit::{
    DocumentOperationJournalPort, DocumentOperationJournalState, DocumentOperationTerminalFailure,
    DocumentRevisionCommitRequest,
};
use cabinet_ports::version_preparation::VersionPreparationPort;
use cabinet_ports::version_store::{
    HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
};
use cabinet_usecases::document_revision_commit::{
    CommitDocumentRevisionError, CommitDocumentRevisionOutcomeKind, CommitDocumentRevisionOutput,
    CommitDocumentRevisionUsecase,
};

struct TempRoot(PathBuf);

impl TempRoot {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-document-revision-flow-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp root");
        Self(path)
    }

    fn versions(&self) -> PathBuf {
        self.0.join("versions")
    }

    fn pointers(&self) -> PathBuf {
        self.0.join("pointers")
    }

    fn journal(&self) -> PathBuf {
        self.0.join("journal")
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn local_create_is_replayed_without_duplicate_revision_before_and_after_restart() {
    let temp = TempRoot::new("create-replay");
    let workspace = workspace();
    let document = document();
    let operation = operation("create-operation");
    let request = request(
        operation.clone(),
        DocumentMutationKind::Create,
        DocumentExpectedCurrentVersion::MustNotExist,
        record("version-1", "Created body", 1),
    );
    let mut versions = LocalVersionStore::new(temp.versions());
    let mut pointer = LocalCurrentDocumentVersionPointer::new(temp.pointers());
    let mut journal = LocalDocumentOperationJournal::new(temp.journal());

    let fresh =
        execute(request.clone(), &mut versions, &mut pointer, &mut journal).expect("fresh create");
    let replayed = execute(request.clone(), &mut versions, &mut pointer, &mut journal)
        .expect("same-process replay");
    drop((versions, pointer, journal));

    let mut versions = LocalVersionStore::new(temp.versions());
    let mut pointer = LocalCurrentDocumentVersionPointer::new(temp.pointers());
    let mut journal = LocalDocumentOperationJournal::new(temp.journal());
    let restart_replay =
        execute(request, &mut versions, &mut pointer, &mut journal).expect("restart replay");

    assert_eq!(fresh.kind(), CommitDocumentRevisionOutcomeKind::Fresh);
    assert_eq!(replayed.kind(), CommitDocumentRevisionOutcomeKind::Replayed);
    assert_eq!(
        restart_replay.kind(),
        CommitDocumentRevisionOutcomeKind::Replayed
    );
    assert_eq!(history_revisions(&versions, &workspace, &document), vec![1]);
    assert_eq!(current(&pointer, &workspace, &document), "version-1");
    let terminal = journal
        .load_operation(&operation)
        .expect("journal")
        .expect("record");
    assert_eq!(terminal.state(), DocumentOperationJournalState::Committed);
    assert_eq!(
        terminal.result().expect("result").revision_number().value(),
        1
    );
}

#[test]
fn local_update_advances_revision_and_stale_retry_is_terminal_without_mutation() {
    let temp = TempRoot::new("update-stale");
    let workspace = workspace();
    let document = document();
    let mut versions = LocalVersionStore::new(temp.versions());
    let mut pointer = LocalCurrentDocumentVersionPointer::new(temp.pointers());
    let mut journal = LocalDocumentOperationJournal::new(temp.journal());
    execute(
        request(
            operation("create-operation"),
            DocumentMutationKind::Create,
            DocumentExpectedCurrentVersion::MustNotExist,
            record("version-1", "First", 1),
        ),
        &mut versions,
        &mut pointer,
        &mut journal,
    )
    .expect("create");

    execute(
        request(
            operation("update-operation"),
            DocumentMutationKind::Update,
            DocumentExpectedCurrentVersion::MustMatch(version("version-1")),
            record("version-2", "Second", 2),
        ),
        &mut versions,
        &mut pointer,
        &mut journal,
    )
    .expect("update");
    assert_eq!(
        history_revisions(&versions, &workspace, &document),
        vec![1, 2]
    );
    assert_eq!(current(&pointer, &workspace, &document), "version-2");
    assert_eq!(
        versions
            .get_version_snapshot(&workspace, &document, &version("version-2"))
            .expect("snapshot")
            .expect("version 2")
            .body()
            .as_str(),
        "Second"
    );

    let stale_operation = operation("stale-operation");
    let stale_request = request(
        stale_operation.clone(),
        DocumentMutationKind::Update,
        DocumentExpectedCurrentVersion::MustMatch(version("version-1")),
        record("version-3", "Stale", 3),
    );
    let first_error = execute(
        stale_request.clone(),
        &mut versions,
        &mut pointer,
        &mut journal,
    )
    .expect_err("stale conflict");
    let replayed_error = execute(stale_request, &mut versions, &mut pointer, &mut journal)
        .expect_err("terminal conflict replay");

    assert_eq!(first_error, CommitDocumentRevisionError::CommitConflict);
    assert_eq!(replayed_error, CommitDocumentRevisionError::CommitConflict);
    assert_eq!(
        history_revisions(&versions, &workspace, &document),
        vec![1, 2]
    );
    assert_eq!(current(&pointer, &workspace, &document), "version-2");
    assert!(
        versions
            .get_version_snapshot(&workspace, &document, &version("version-3"))
            .expect("version 3 query")
            .is_none()
    );
    assert!(
        versions
            .load_prepared(&workspace, &stale_operation)
            .expect("prepared cleanup")
            .is_none()
    );
    let failed = journal
        .load_operation(&stale_operation)
        .expect("journal")
        .expect("failed record");
    assert_eq!(failed.state(), DocumentOperationJournalState::Failed);
    assert_eq!(
        failed.failure(),
        Some(DocumentOperationTerminalFailure::Conflict)
    );

    drop((versions, pointer, journal));
    let restarted_versions = LocalVersionStore::new(temp.versions());
    let restarted_pointer = LocalCurrentDocumentVersionPointer::new(temp.pointers());
    let restarted_journal = LocalDocumentOperationJournal::new(temp.journal());
    assert_eq!(
        history_revisions(&restarted_versions, &workspace, &document),
        vec![1, 2]
    );
    assert_eq!(
        current(&restarted_pointer, &workspace, &document),
        "version-2"
    );
    let restarted_failed = restarted_journal
        .load_operation(&stale_operation)
        .expect("restart journal")
        .expect("failed record");
    assert_eq!(
        restarted_failed.state(),
        DocumentOperationJournalState::Failed
    );
    assert_eq!(
        restarted_failed.failure(),
        Some(DocumentOperationTerminalFailure::Conflict)
    );
}

fn execute(
    request: DocumentRevisionCommitRequest,
    versions: &mut LocalVersionStore,
    pointer: &mut LocalCurrentDocumentVersionPointer,
    journal: &mut LocalDocumentOperationJournal,
) -> Result<CommitDocumentRevisionOutput, CommitDocumentRevisionError> {
    let mut commit = GuardedDocumentRevisionCommit::new(versions, pointer);
    CommitDocumentRevisionUsecase::new().execute(request, &mut commit, journal)
}

fn request(
    operation: DocumentOperationId,
    kind: DocumentMutationKind,
    expected: DocumentExpectedCurrentVersion,
    record: VersionRecord,
) -> DocumentRevisionCommitRequest {
    DocumentRevisionCommitRequest::new(
        DocumentOperationIdentity::new(operation, workspace(), document(), kind, expected)
            .expect("identity"),
        record,
    )
    .expect("request")
}

fn record(version_id: &str, body: &str, revision: u64) -> VersionRecord {
    let document = document();
    let snapshot_ref =
        DocumentSnapshotRef::new(&format!("snapshot-{version_id}")).expect("snapshot");
    let entry = VersionEntry::new(
        version(version_id),
        document.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("writer").expect("author"),
        VersionSummary::new("Saved").expect("summary"),
    )
    .expect("entry")
    .with_created_at_epoch_ms(1_000 + revision)
    .expect("timestamp")
    .with_revision_number(DocumentRevisionNumber::new(revision).expect("revision"))
    .expect("assigned revision");
    VersionRecord::new(
        entry,
        VersionSnapshot::new(
            document,
            snapshot_ref,
            DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
        ),
    )
    .expect("record")
}

fn history_revisions(
    versions: &LocalVersionStore,
    workspace: &WorkspaceId,
    document: &DocumentId,
) -> Vec<u64> {
    versions
        .list_history(
            workspace,
            document,
            HistoryPageRequest::first(10).expect("request"),
        )
        .expect("history")
        .entries()
        .iter()
        .map(|entry| entry.revision_number().expect("revision").value())
        .collect()
}

fn current(
    pointer: &LocalCurrentDocumentVersionPointer,
    workspace: &WorkspaceId,
    document: &DocumentId,
) -> String {
    pointer
        .load_current_version(workspace, document)
        .expect("current")
        .expect("version")
        .as_str()
        .to_string()
}

fn workspace() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace")
}

fn document() -> DocumentId {
    DocumentId::new("doc-1").expect("document")
}

fn operation(value: &str) -> DocumentOperationId {
    DocumentOperationId::new(value).expect("operation")
}

fn version(value: &str) -> VersionId {
    VersionId::new(value).expect("version")
}
