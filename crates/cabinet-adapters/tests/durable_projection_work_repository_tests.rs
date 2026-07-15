use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_projection_work_repository::DurableProjectionWorkRepository;
use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_work::{
    ProjectionChangeKind, ProjectionKind, ProjectionWork, ProjectionWorkEvent,
    ProjectionWorkIdentity, ProjectionWorkState,
};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::projection_work::{
    ProjectionEnqueueOutcome, ProjectionWorkRepository, ProjectionWorkRepositoryError,
};

#[test]
fn durable_projection_work_survives_restart_and_preserves_guarded_state() {
    let temp = TempRoot::new("restart");
    let pending = ProjectionWork::pending(identity("version-1", ProjectionKind::Graph));
    let mut writer = DurableProjectionWorkRepository::new(temp.path.clone());
    assert_eq!(
        writer.enqueue(pending.clone()).expect("enqueue"),
        ProjectionEnqueueOutcome::Enqueued
    );
    assert_eq!(
        writer.enqueue(pending.clone()).expect("duplicate"),
        ProjectionEnqueueOutcome::AlreadyExists
    );
    let indexing = pending
        .transition(ProjectionWorkEvent::Start)
        .expect("start");
    writer
        .replace(indexing.clone(), ProjectionWorkState::Pending)
        .expect("replace");
    assert_eq!(
        writer.replace(indexing.clone(), ProjectionWorkState::Pending),
        Err(ProjectionWorkRepositoryError::Conflict)
    );
    drop(writer);

    let reader = DurableProjectionWorkRepository::new(temp.path.clone());
    assert_eq!(
        reader.get(indexing.identity()).expect("read"),
        Some(indexing.clone())
    );
    assert_eq!(reader.list_resumable(10).expect("resume"), vec![indexing]);
}

#[test]
fn durable_projection_work_preserves_change_kind_after_restart() {
    let temp = TempRoot::new("change-kind");
    let identity = ProjectionWorkIdentity::for_change(
        WorkspaceId::new("workspace-1").expect("workspace"),
        DocumentId::new("doc-1").expect("document"),
        VersionId::new("version-1").expect("version"),
        ProjectionKind::Graph,
        ProjectionChangeKind::Deleted,
    );
    let work = ProjectionWork::pending(identity.clone());
    DurableProjectionWorkRepository::new(temp.path.clone())
        .enqueue(work)
        .expect("enqueue");

    let loaded = DurableProjectionWorkRepository::new(temp.path.clone())
        .get(&identity)
        .expect("get")
        .expect("work");
    assert_eq!(
        loaded.identity().change_kind(),
        ProjectionChangeKind::Deleted
    );
}

#[test]
fn durable_projection_work_supports_runtime_sized_document_and_version_ids() {
    let temp = TempRoot::new("runtime-sized-ids");
    let identity = ProjectionWorkIdentity::for_change(
        WorkspaceId::new("workspace-1").expect("workspace"),
        DocumentId::new("doc-4953cdb4-ef07-4bf2-a491-948a05335b14").expect("document"),
        VersionId::new("version-1784039246720321000-4953cdb4-ef07-4bf2-a491-948a05335b14")
            .expect("version"),
        ProjectionKind::Graph,
        ProjectionChangeKind::Created,
    );
    let pending = ProjectionWork::pending(identity.clone());
    let mut repository = DurableProjectionWorkRepository::new(temp.path.clone());

    assert_eq!(
        repository.enqueue(pending.clone()).expect("enqueue"),
        ProjectionEnqueueOutcome::Enqueued
    );
    let indexing = pending
        .transition(ProjectionWorkEvent::Start)
        .expect("start");
    repository
        .replace(indexing.clone(), ProjectionWorkState::Pending)
        .expect("replace");
    drop(repository);

    assert_eq!(
        DurableProjectionWorkRepository::new(temp.path.clone())
            .get(&identity)
            .expect("restart read"),
        Some(indexing)
    );
    let file_name = find_snapshot(&temp.path)
        .file_name()
        .and_then(|value| value.to_str())
        .expect("utf-8 file name")
        .to_string();
    assert!(file_name.len() <= 255, "file name must fit platform limits");
}

#[test]
fn durable_projection_work_reads_and_replaces_legacy_short_key_records() {
    let temp = TempRoot::new("legacy-short-key");
    let identity = identity("version-legacy", ProjectionKind::Links);
    let pending = ProjectionWork::pending(identity.clone());
    let mut repository = DurableProjectionWorkRepository::new(temp.path.clone());
    repository.enqueue(pending.clone()).expect("enqueue");
    let current_path = find_snapshot(&temp.path);
    let legacy_path = temp
        .path
        .join("operations")
        .join("projection")
        .join(format!("{}.work", hex_encode(&identity.idempotency_key())));
    fs::rename(current_path, &legacy_path).expect("move to legacy path");

    assert_eq!(
        repository.get(&identity).expect("legacy read"),
        Some(pending.clone())
    );
    let ready = pending
        .transition(ProjectionWorkEvent::Start)
        .expect("start")
        .transition(ProjectionWorkEvent::Succeeded)
        .expect("ready");
    repository
        .replace(ready.clone(), ProjectionWorkState::Pending)
        .expect("legacy replace");
    assert_eq!(
        repository.get(&identity).expect("updated read"),
        Some(ready)
    );
}

#[test]
fn durable_projection_work_lists_only_resumable_records_in_stable_bounded_order() {
    let temp = TempRoot::new("resume");
    let mut repository = DurableProjectionWorkRepository::new(temp.path.clone());
    let pending = ProjectionWork::pending(identity("version-a", ProjectionKind::Search));
    let ready = ProjectionWork::pending(identity("version-b", ProjectionKind::Links))
        .transition(ProjectionWorkEvent::Start)
        .expect("start")
        .transition(ProjectionWorkEvent::Succeeded)
        .expect("ready");
    let retry = ProjectionWork::pending(identity("version-c", ProjectionKind::Graph))
        .transition(ProjectionWorkEvent::Start)
        .expect("start")
        .transition(ProjectionWorkEvent::RetryScheduled)
        .expect("retry");
    repository.enqueue(retry).expect("retry enqueue");
    repository.enqueue(ready).expect("ready enqueue");
    repository.enqueue(pending).expect("pending enqueue");

    let first = repository.list_resumable(1).expect("bounded");
    let all = repository.list_resumable(10).expect("all");

    assert_eq!(first.len(), 1);
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|work| work.state().is_resumable()));
    assert_eq!(
        repository.list_resumable(0),
        Err(ProjectionWorkRepositoryError::InvalidLimit)
    );
}

#[test]
fn durable_projection_work_distinguishes_corruption_unsupported_schema_and_missing() {
    let temp = TempRoot::new("corruption");
    let work = ProjectionWork::pending(identity("version-1", ProjectionKind::Graph));
    let mut repository = DurableProjectionWorkRepository::new(temp.path.clone());
    repository.enqueue(work.clone()).expect("enqueue");
    let snapshot = find_snapshot(&temp.path);

    fs::write(
        &snapshot,
        "schema\t1\nchecksum\t0000000000000000\nprivate-body\n",
    )
    .expect("corrupt");
    assert_eq!(
        repository.get(work.identity()),
        Err(ProjectionWorkRepositoryError::CorruptedRecord)
    );
    fs::write(&snapshot, "schema\t999\nchecksum\t0000000000000000\n").expect("schema");
    assert_eq!(
        repository.get(work.identity()),
        Err(ProjectionWorkRepositoryError::UnsupportedSchema)
    );
    assert_eq!(
        repository
            .get(&identity("missing", ProjectionKind::Graph))
            .expect("missing"),
        None
    );
}

fn identity(version: &str, kind: ProjectionKind) -> ProjectionWorkIdentity {
    ProjectionWorkIdentity::new(
        WorkspaceId::new("workspace-1").expect("workspace"),
        DocumentId::new("doc-1").expect("document"),
        VersionId::new(version).expect("version"),
        kind,
    )
}

fn find_snapshot(root: &PathBuf) -> PathBuf {
    fs::read_dir(root.join("operations").join("projection"))
        .expect("operation root")
        .next()
        .expect("snapshot")
        .expect("entry")
        .path()
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
            "sponzey-cabinet-phase012-work-{label}-{}-{nonce}",
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
