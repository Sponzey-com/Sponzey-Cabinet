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
    DocumentOperationJournalPort, DocumentOperationJournalState,
};
use cabinet_ports::version_preparation::VersionPreparationPort;
use cabinet_ports::version_store::{
    HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
};
use cabinet_usecases::document_revision_recovery::{
    RecoverDocumentRevisionOperationUsecase, RecoverDocumentRevisionOutcomeKind,
};

#[test]
fn local_recovery_completes_pre_primary_and_post_primary_claimed_operations_after_restart() {
    for primary_already_committed in [false, true] {
        let temp = TempRoot::new(if primary_already_committed {
            "post-primary"
        } else {
            "pre-primary"
        });
        let workspace = WorkspaceId::new("workspace-1").expect("workspace");
        let document = DocumentId::new("doc-1").expect("document");
        let operation = DocumentOperationId::new("operation-1").expect("operation");
        let identity = DocumentOperationIdentity::new(
            operation.clone(),
            workspace.clone(),
            document.clone(),
            DocumentMutationKind::Create,
            DocumentExpectedCurrentVersion::MustNotExist,
        )
        .expect("identity");
        let mut versions = LocalVersionStore::new(temp.versions());
        let mut pointer = LocalCurrentDocumentVersionPointer::new(temp.pointers());
        let mut journal = LocalDocumentOperationJournal::new(temp.journal());
        journal.claim_operation(identity).expect("claim");
        versions
            .prepare_version(&workspace, &operation, record())
            .expect("prepare");
        if primary_already_committed {
            pointer
                .compare_and_set_current_version(
                    &workspace,
                    &document,
                    None,
                    VersionId::new("version-1").expect("version"),
                )
                .expect("primary commit");
        }
        drop((versions, pointer, journal));

        let mut versions = LocalVersionStore::new(temp.versions());
        let mut pointer = LocalCurrentDocumentVersionPointer::new(temp.pointers());
        let mut journal = LocalDocumentOperationJournal::new(temp.journal());
        let mut recovery = GuardedDocumentRevisionCommit::new(&mut versions, &mut pointer);
        let output = RecoverDocumentRevisionOperationUsecase::new()
            .execute(operation.clone(), &mut recovery, &mut journal)
            .expect("recover");
        assert_eq!(output.kind(), RecoverDocumentRevisionOutcomeKind::Recovered);
        drop(recovery);

        assert_eq!(
            pointer
                .load_current_version(&workspace, &document)
                .expect("pointer")
                .expect("version")
                .as_str(),
            "version-1"
        );
        assert_eq!(
            versions
                .list_history(
                    &workspace,
                    &document,
                    HistoryPageRequest::first(10).expect("request")
                )
                .expect("history")
                .entries()
                .len(),
            1
        );
        assert_eq!(
            journal
                .load_operation(&operation)
                .expect("journal")
                .expect("record")
                .state(),
            DocumentOperationJournalState::Committed
        );

        let mut recovery = GuardedDocumentRevisionCommit::new(&mut versions, &mut pointer);
        let replay = RecoverDocumentRevisionOperationUsecase::new()
            .execute(operation, &mut recovery, &mut journal)
            .expect("already committed");
        assert_eq!(
            replay.kind(),
            RecoverDocumentRevisionOutcomeKind::AlreadyCommitted
        );
    }
}

fn record() -> VersionRecord {
    let document = DocumentId::new("doc-1").expect("document");
    let snapshot_ref = DocumentSnapshotRef::new("snapshot-1").expect("snapshot");
    let entry = VersionEntry::new(
        VersionId::new("version-1").expect("version"),
        document.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("writer").expect("author"),
        VersionSummary::new("Created").expect("summary"),
    )
    .expect("entry")
    .with_created_at_epoch_ms(100)
    .expect("timestamp")
    .with_revision_number(DocumentRevisionNumber::new(1).expect("revision"))
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

struct TempRoot(PathBuf);

impl TempRoot {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-revision-recovery-{name}-{}-{nanos}",
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
