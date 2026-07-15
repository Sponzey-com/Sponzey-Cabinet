use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_projection_repair_repository::DurableProjectionRepairRepository;
use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_repair::{
    ProjectionRepairEvent, ProjectionRepairOperation, ProjectionRepairOperationId,
    ProjectionRepairState,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::projection_repair::{
    ProjectionRepairCreateOutcome, ProjectionRepairRepository, ProjectionRepairRepositoryError,
};

#[test]
fn repair_operation_survives_restart_and_expected_state_blocks_stale_writer() {
    let temp = TempRoot::new("restart");
    let queued = operation("repair-1", "workspace-1");
    let mut writer = DurableProjectionRepairRepository::new(temp.path.clone());
    assert_eq!(
        writer.create(queued.clone()).unwrap(),
        ProjectionRepairCreateOutcome::Created
    );
    assert_eq!(
        writer.create(queued.clone()).unwrap(),
        ProjectionRepairCreateOutcome::AlreadyExists
    );
    let running = queued
        .transition(ProjectionRepairEvent::Start)
        .unwrap()
        .into_operation();
    writer
        .replace(running.clone(), ProjectionRepairState::Queued)
        .unwrap();
    assert_eq!(
        writer.replace(running.clone(), ProjectionRepairState::Queued),
        Err(ProjectionRepairRepositoryError::Conflict)
    );
    drop(writer);

    let reader = DurableProjectionRepairRepository::new(temp.path.clone());
    assert_eq!(
        reader.get(running.operation_id()).unwrap(),
        Some(running.clone())
    );
    assert_eq!(
        reader
            .list_active(&WorkspaceId::new("workspace-1").unwrap(), 10)
            .unwrap(),
        vec![running]
    );
}

#[test]
fn repair_repository_lists_only_workspace_active_operations_in_stable_bounded_order() {
    let temp = TempRoot::new("active");
    let mut repository = DurableProjectionRepairRepository::new(temp.path.clone());
    let active_a = operation("repair-a", "workspace-1");
    let active_b = operation("repair-b", "workspace-1")
        .transition(ProjectionRepairEvent::Start)
        .unwrap()
        .into_operation();
    let terminal = operation("repair-c", "workspace-1")
        .transition(ProjectionRepairEvent::Start)
        .unwrap()
        .into_operation()
        .transition(ProjectionRepairEvent::Succeeded)
        .unwrap()
        .into_operation();
    let other = operation("repair-d", "workspace-2");
    for value in [other, terminal, active_b.clone(), active_a.clone()] {
        repository.create(value).unwrap();
    }

    let workspace = WorkspaceId::new("workspace-1").unwrap();
    assert_eq!(
        repository.list_active(&workspace, 1).unwrap(),
        vec![active_a.clone()]
    );
    assert_eq!(
        repository.list_active(&workspace, 10).unwrap(),
        vec![active_a, active_b]
    );
    assert_eq!(
        repository.list_active(&workspace, 0),
        Err(ProjectionRepairRepositoryError::InvalidLimit)
    );
}

#[test]
fn repair_repository_exposes_corruption_schema_and_missing_without_raw_data() {
    let temp = TempRoot::new("corrupt");
    let operation = operation("repair-private", "workspace-1");
    let mut repository = DurableProjectionRepairRepository::new(temp.path.clone());
    repository.create(operation.clone()).unwrap();
    let snapshot = find_snapshot(&temp.path);

    fs::write(
        &snapshot,
        "schema\t1\nchecksum\t0000000000000000\n/Users/private/raw\n",
    )
    .unwrap();
    assert_eq!(
        repository.get(operation.operation_id()),
        Err(ProjectionRepairRepositoryError::CorruptedRecord)
    );
    fs::write(&snapshot, "schema\t999\nchecksum\t0000000000000000\n").unwrap();
    assert_eq!(
        repository.get(operation.operation_id()),
        Err(ProjectionRepairRepositoryError::UnsupportedSchema)
    );
    assert_eq!(
        repository
            .get(&ProjectionRepairOperationId::new("missing").unwrap())
            .unwrap(),
        None
    );
}

fn operation(id: &str, workspace: &str) -> ProjectionRepairOperation {
    ProjectionRepairOperation::queued(
        ProjectionRepairOperationId::new(id).unwrap(),
        WorkspaceId::new(workspace).unwrap(),
        DocumentId::new("doc-1").unwrap(),
    )
}

fn find_snapshot(root: &PathBuf) -> PathBuf {
    fs::read_dir(root.join("operations").join("projection-repair"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
}

struct TempRoot {
    path: PathBuf,
}
impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-phase012-repair-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}
impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
