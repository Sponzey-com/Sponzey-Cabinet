use cabinet_domain::asset_import_operation::{
    AssetImportEvent, AssetImportOperation, AssetImportOperationId, AssetImportState,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_import_operation_repository::{
    AssetImportOperationCreateOutcome, AssetImportOperationRepository,
    AssetImportOperationRepositoryError,
};
use cabinet_ports::asset_staging::{AssetStagingError, AssetStagingWriter, StagedAsset};
use cabinet_usecases::asset_import::{
    CancelAssetImportInput, CancelAssetImportUsecase, ImportAssetProductEvent,
    ImportAssetProductLogger, RecoverAssetImportsInput, RecoverAssetImportsUsecase,
};

#[test]
fn startup_recovery_cancels_active_operations_and_cleans_staging() {
    let mut repository = FakeOperations(vec![operation("op-1", AssetImportState::Staging)]);
    let mut staging = FakeStaging {
        fail_cleanup: false,
        cleaned: Vec::new(),
    };
    let mut logger = Logger::default();

    let output = RecoverAssetImportsUsecase::new()
        .execute(
            RecoverAssetImportsInput::new("workspace-1", 10).expect("input"),
            &mut repository,
            &mut staging,
            &mut logger,
        )
        .expect("recover");

    assert_eq!(output.cancelled(), 1);
    assert_eq!(repository.0[0].state(), AssetImportState::Cancelled);
    assert_eq!(staging.cleaned, vec!["op-1"]);
    assert_eq!(logger.events.len(), 1);
}

#[test]
fn startup_recovery_preserves_cleanup_required_when_cleanup_fails() {
    let mut repository =
        FakeOperations(vec![operation("op-2", AssetImportState::PublishingObject)]);
    let mut staging = FakeStaging {
        fail_cleanup: true,
        cleaned: Vec::new(),
    };
    let mut logger = Logger::default();

    let output = RecoverAssetImportsUsecase::new()
        .execute(
            RecoverAssetImportsInput::new("workspace-1", 10).expect("input"),
            &mut repository,
            &mut staging,
            &mut logger,
        )
        .expect("recover");

    assert_eq!(output.cleanup_required(), 1);
    assert_eq!(repository.0[0].state(), AssetImportState::CleanupRequired);
    assert_eq!(logger.events.len(), 1);
}

#[test]
fn explicit_cancel_is_idempotent_after_terminal_transition() {
    let mut repository = FakeOperations(vec![operation("op-3", AssetImportState::Staging)]);
    let mut staging = FakeStaging {
        fail_cleanup: false,
        cleaned: Vec::new(),
    };
    let mut logger = Logger::default();
    let input = || CancelAssetImportInput::new("workspace-1", "op-3").expect("input");

    let first = CancelAssetImportUsecase::new()
        .execute(input(), &mut repository, &mut staging, &mut logger)
        .expect("cancel");
    let second = CancelAssetImportUsecase::new()
        .execute(input(), &mut repository, &mut staging, &mut logger)
        .expect("duplicate cancel");

    assert_eq!(first.state(), AssetImportState::Cancelled);
    assert_eq!(second.state(), AssetImportState::Cancelled);
    assert_eq!(staging.cleaned.len(), 1);
    assert_eq!(logger.events.len(), 1);
}

fn operation(id: &str, target: AssetImportState) -> AssetImportOperation {
    let mut value = AssetImportOperation::new(
        AssetImportOperationId::new(id).expect("id"),
        WorkspaceId::new("workspace-1").expect("workspace"),
        DocumentId::new("doc-1").expect("document"),
        10,
    )
    .expect("operation");
    let events = match target {
        AssetImportState::Staging => vec![
            AssetImportEvent::Begin,
            AssetImportEvent::ValidationSucceeded,
        ],
        AssetImportState::PublishingObject => vec![
            AssetImportEvent::Begin,
            AssetImportEvent::ValidationSucceeded,
            AssetImportEvent::StagingSucceeded,
            AssetImportEvent::HashingSucceeded,
        ],
        _ => unreachable!(),
    };
    for event in events {
        value.apply(event, 0).expect("transition");
    }
    value
}

struct FakeOperations(Vec<AssetImportOperation>);
impl AssetImportOperationRepository for FakeOperations {
    fn create(
        &mut self,
        _: AssetImportOperation,
    ) -> Result<AssetImportOperationCreateOutcome, AssetImportOperationRepositoryError> {
        unreachable!()
    }
    fn get(
        &self,
        id: &AssetImportOperationId,
    ) -> Result<Option<AssetImportOperation>, AssetImportOperationRepositoryError> {
        Ok(self
            .0
            .iter()
            .find(|value| value.operation_id() == id)
            .cloned())
    }
    fn replace(
        &mut self,
        operation: AssetImportOperation,
        expected: AssetImportState,
    ) -> Result<(), AssetImportOperationRepositoryError> {
        let current = self
            .0
            .iter_mut()
            .find(|value| value.operation_id() == operation.operation_id())
            .ok_or(AssetImportOperationRepositoryError::NotFound)?;
        if current.state() != expected {
            return Err(AssetImportOperationRepositoryError::Conflict);
        }
        *current = operation;
        Ok(())
    }
    fn list_active(
        &self,
        workspace: &WorkspaceId,
        limit: usize,
    ) -> Result<Vec<AssetImportOperation>, AssetImportOperationRepositoryError> {
        Ok(self
            .0
            .iter()
            .filter(|value| value.workspace_id() == workspace && !value.state().is_terminal())
            .take(limit)
            .cloned()
            .collect())
    }
}

struct FakeStaging {
    fail_cleanup: bool,
    cleaned: Vec<String>,
}
impl AssetStagingWriter for FakeStaging {
    fn begin(
        &mut self,
        _: &WorkspaceId,
        _: &AssetImportOperationId,
    ) -> Result<(), AssetStagingError> {
        unreachable!()
    }
    fn append(
        &mut self,
        _: &WorkspaceId,
        _: &AssetImportOperationId,
        _: u64,
        _: &[u8],
    ) -> Result<(), AssetStagingError> {
        unreachable!()
    }
    fn finalize(
        &mut self,
        _: &WorkspaceId,
        _: &AssetImportOperationId,
        _: u64,
    ) -> Result<StagedAsset, AssetStagingError> {
        unreachable!()
    }
    fn cleanup(
        &mut self,
        _: &WorkspaceId,
        operation: &AssetImportOperationId,
    ) -> Result<(), AssetStagingError> {
        if self.fail_cleanup {
            return Err(AssetStagingError::StorageUnavailable);
        }
        self.cleaned.push(operation.as_str().to_string());
        Ok(())
    }
}

#[derive(Default)]
struct Logger {
    events: Vec<ImportAssetProductEvent>,
}
impl ImportAssetProductLogger for Logger {
    fn write_product(&mut self, event: ImportAssetProductEvent) {
        self.events.push(event);
    }
}
