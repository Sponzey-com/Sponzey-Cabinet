use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_JOURNAL_ROOT, LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT,
    LocalCreateDocumentRevisionRuntime,
};
use cabinet_adapters::local_current_document_revision_projection::{
    LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT, LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT,
};
use cabinet_adapters::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use cabinet_adapters::local_document_operation_journal::LocalDocumentOperationJournal;
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_domain::document::{DocumentBodyPolicy, DocumentId, DocumentTitle};
use cabinet_domain::document_revision::DocumentOperationId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::document_repository::DocumentRepository;
use cabinet_ports::document_revision_commit::{
    DocumentOperationJournalPort, DocumentOperationJournalState,
};
use cabinet_ports::version_store::{HistoryPageRequest, VersionStore};
use cabinet_usecases::create_document_revision::{
    CreateDocumentRevisionError, CreateDocumentRevisionInput,
};
use cabinet_usecases::document_revision_commit::CommitDocumentRevisionOutcomeKind;

#[test]
fn local_runtime_creates_and_replays_same_revision_after_restart() {
    let temp = TempRoot::new("create-restart");
    let command = input("# First\r\nbody");
    let mut runtime = build_runtime(&temp);

    let created = runtime.execute(command.clone()).expect("create");
    assert_eq!(created.kind(), CommitDocumentRevisionOutcomeKind::Fresh);
    assert_eq!(created.revision_number().value(), 1);
    let created_version = created.version_id().clone();
    drop(runtime);

    assert_durable_revision(&temp, &created_version, "# First\nbody");

    let mut restarted = build_runtime(&temp);
    let replayed = restarted.execute(command).expect("restart replay");
    assert_eq!(replayed.kind(), CommitDocumentRevisionOutcomeKind::Replayed);
    assert_eq!(replayed.version_id(), &created_version);
    assert_eq!(replayed.revision_number().value(), 1);
    drop(restarted);

    assert_durable_revision(&temp, &created_version, "# First\nbody");
}

#[test]
fn changed_payload_conflict_and_invalid_input_leave_durable_state_unchanged() {
    let temp = TempRoot::new("conflict-no-write");
    let mut runtime = build_runtime(&temp);
    let created = runtime.execute(input("original")).expect("create");
    let version = created.version_id().clone();

    let error = runtime
        .execute(input("changed"))
        .expect_err("same operation changed payload");
    assert_eq!(error, CreateDocumentRevisionError::OperationConflict);
    drop(runtime);
    assert_durable_revision(&temp, &version, "original");

    let empty = TempRoot::new("invalid-no-write");
    let mut runtime = LocalCreateDocumentRevisionRuntime::new(
        empty.path.clone(),
        DocumentBodyPolicy::new(4).expect("body policy"),
    );
    let error = runtime.execute(input("too long")).expect_err("body policy");
    assert_eq!(error, CreateDocumentRevisionError::InvalidInput);
    assert!(fs::read_dir(&empty.path).expect("root").next().is_none());
}

#[test]
fn projection_failure_is_recovery_required_and_same_operation_repairs_without_new_revision() {
    let temp = TempRoot::new("projection-recovery");
    let blocker = temp
        .path
        .join(LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT);
    fs::write(&blocker, "blocks projection directory").expect("create blocker");
    let command = input("복구 제목\n본문");
    let mut runtime = build_runtime(&temp);

    let error = runtime
        .execute(command.clone())
        .expect_err("post-primary projection failure");
    assert_eq!(error, CreateDocumentRevisionError::RecoveryRequired);
    let committed_version = current_pointer(&temp).expect("primary commit pointer");
    assert_history_count(&temp, 1);

    fs::remove_file(blocker).expect("remove blocker");
    drop(runtime);
    let repaired = build_runtime(&temp)
        .execute(command)
        .expect("same operation repairs projection");

    assert_eq!(repaired.kind(), CommitDocumentRevisionOutcomeKind::Replayed);
    assert_eq!(repaired.version_id(), &committed_version);
    assert_history_count(&temp, 1);
    let current = projected_current(&temp).expect("repaired current projection");
    assert_eq!(current.metadata().title().as_str(), "복구 제목");
    assert_eq!(current.body().as_str(), "복구 제목\n본문");
}

fn assert_durable_revision(
    temp: &TempRoot,
    expected_version: &cabinet_domain::version::VersionId,
    expected_body: &str,
) {
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let document = DocumentId::new("doc-1").expect("document");
    let pointer =
        LocalCurrentDocumentVersionPointer::new(temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT));
    assert_eq!(
        pointer
            .load_current_version(&workspace, &document)
            .expect("pointer")
            .as_ref(),
        Some(expected_version)
    );

    let versions = LocalVersionStore::new(temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT));
    let history = versions
        .list_history(
            &workspace,
            &document,
            HistoryPageRequest::first(10).expect("request"),
        )
        .expect("history");
    assert_eq!(history.entries().len(), 1);
    assert_eq!(history.entries()[0].version_id(), expected_version);
    assert_eq!(
        history.entries()[0]
            .revision_number()
            .map(|value| value.value()),
        Some(1)
    );
    let snapshot = versions
        .get_version_snapshot(&workspace, &document, expected_version)
        .expect("snapshot")
        .expect("stored snapshot");
    assert_eq!(snapshot.body().as_str(), expected_body);
    assert_eq!(snapshot.attachment_state().references(), Some(&[][..]));

    let journal = LocalDocumentOperationJournal::new(temp.path.join(LOCAL_DOCUMENT_JOURNAL_ROOT));
    let record = journal
        .load_operation(&DocumentOperationId::new("operation-1").expect("operation"))
        .expect("journal")
        .expect("record");
    assert_eq!(record.state(), DocumentOperationJournalState::Committed);
    assert_eq!(
        record.result().expect("result").version_id(),
        expected_version
    );

    let current = projected_current(temp).expect("current projection");
    assert_eq!(
        current.metadata().title(),
        &DocumentTitle::from_markdown_text(expected_body)
    );
    assert_eq!(current.body().as_str(), expected_body);
    let path = current.path().as_str();
    assert!(path.starts_with("notes/"));
    assert!(path.ends_with(".md"));
    assert_eq!(path.len(), 73);
    assert!(!path.contains("doc-1"));
}

fn projected_current(
    temp: &TempRoot,
) -> Option<cabinet_ports::document_repository::CurrentDocumentRecord> {
    LocalDocumentRepository::new(temp.path.join(LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT))
        .get_current_by_id(
            &WorkspaceId::new("workspace-1").expect("workspace"),
            &DocumentId::new("doc-1").expect("document"),
        )
        .expect("current projection read")
}

fn current_pointer(temp: &TempRoot) -> Option<cabinet_domain::version::VersionId> {
    LocalCurrentDocumentVersionPointer::new(temp.path.join(LOCAL_DOCUMENT_POINTER_ROOT))
        .load_current_version(
            &WorkspaceId::new("workspace-1").unwrap(),
            &DocumentId::new("doc-1").unwrap(),
        )
        .unwrap()
}

fn assert_history_count(temp: &TempRoot, expected: usize) {
    let history = LocalVersionStore::new(temp.path.join(LOCAL_DOCUMENT_VERSION_ROOT))
        .list_history(
            &WorkspaceId::new("workspace-1").unwrap(),
            &DocumentId::new("doc-1").unwrap(),
            HistoryPageRequest::first(10).unwrap(),
        )
        .unwrap();
    assert_eq!(history.entries().len(), expected);
}

fn build_runtime(temp: &TempRoot) -> LocalCreateDocumentRevisionRuntime {
    LocalCreateDocumentRevisionRuntime::new(
        temp.path.clone(),
        DocumentBodyPolicy::new(1024).expect("body policy"),
    )
}

fn input(body: &str) -> CreateDocumentRevisionInput {
    CreateDocumentRevisionInput::new(
        "operation-1",
        "workspace-1",
        "doc-1",
        body,
        "local-user",
        "Create document",
    )
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
            "sponzey-local-create-revision-{label}-{}-{nonce}",
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
