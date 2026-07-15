use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_import_operation_repository::DurableAssetImportOperationRepository;
use cabinet_domain::asset_import_operation::{
    AssetImportEvent, AssetImportOperation, AssetImportOperationId, AssetImportState,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_import_operation_repository::{
    AssetImportOperationCreateOutcome, AssetImportOperationRepository,
    AssetImportOperationRepositoryError,
};

#[test]
fn repository_survives_restart_and_supports_idempotent_create_and_cas_replace() {
    let temp = TempRoot::new("restart");
    let mut repository = DurableAssetImportOperationRepository::new(temp.path().to_path_buf());
    let original = operation("import-1", "workspace-1");

    assert_eq!(
        repository.create(original.clone()).expect("create"),
        AssetImportOperationCreateOutcome::Created
    );
    assert_eq!(
        repository.create(original.clone()).expect("duplicate"),
        AssetImportOperationCreateOutcome::AlreadyExists
    );

    let restarted = DurableAssetImportOperationRepository::new(temp.path().to_path_buf());
    assert_eq!(
        restarted.get(original.operation_id()).expect("get"),
        Some(original.clone())
    );

    let mut running = original;
    running.apply(AssetImportEvent::Begin, 0).expect("begin");
    repository
        .replace(running.clone(), AssetImportState::Selected)
        .expect("replace");
    assert_eq!(
        repository.get(running.operation_id()).expect("readback"),
        Some(running)
    );
}

#[test]
fn repository_enforces_expected_state_workspace_scope_and_active_limit() {
    let temp = TempRoot::new("cas");
    let mut repository = DurableAssetImportOperationRepository::new(temp.path().to_path_buf());
    let first = operation("import-1", "workspace-1");
    let second = operation("import-2", "workspace-1");
    repository.create(first.clone()).expect("first");
    repository.create(second).expect("second");

    assert_eq!(
        repository
            .replace(first, AssetImportState::Staging)
            .expect_err("conflict"),
        AssetImportOperationRepositoryError::Conflict
    );
    assert_eq!(
        repository
            .list_active(&workspace("workspace-1"), 0)
            .expect_err("limit"),
        AssetImportOperationRepositoryError::InvalidLimit
    );
    assert_eq!(
        repository
            .list_active(&workspace("workspace-1"), 1)
            .expect("active")
            .len(),
        1
    );
    assert!(
        repository
            .list_active(&workspace("other"), 10)
            .expect("other")
            .is_empty()
    );
}

#[test]
fn repository_reports_unknown_schema_and_checksum_corruption() {
    let temp = TempRoot::new("corrupt");
    let mut repository = DurableAssetImportOperationRepository::new(temp.path().to_path_buf());
    let operation = operation("import-1", "workspace-1");
    repository.create(operation.clone()).expect("create");
    let path = only_record(temp.path());

    fs::write(&path, "schema\t2\n").expect("future schema");
    assert_eq!(
        repository
            .get(operation.operation_id())
            .expect_err("schema"),
        AssetImportOperationRepositoryError::UnsupportedSchema
    );

    fs::write(
        &path,
        "schema\t1\nchecksum\t0000000000000000\nstate\tselected\n",
    )
    .expect("corrupt");
    assert_eq!(
        repository
            .get(operation.operation_id())
            .expect_err("corrupt"),
        AssetImportOperationRepositoryError::CorruptedRecord
    );
}

fn operation(id: &str, workspace_id: &str) -> AssetImportOperation {
    AssetImportOperation::new(
        AssetImportOperationId::new(id).expect("id"),
        workspace(workspace_id),
        DocumentId::new("doc-1").expect("document"),
        8,
    )
    .expect("operation")
}

fn workspace(value: &str) -> WorkspaceId {
    WorkspaceId::new(value).expect("workspace")
}

fn only_record(root: &Path) -> PathBuf {
    fs::read_dir(root.join("operations/asset-import"))
        .expect("records")
        .next()
        .expect("record")
        .expect("entry")
        .path()
}

struct TempRoot(PathBuf);
impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-asset-operation-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp root");
        Self(path)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
