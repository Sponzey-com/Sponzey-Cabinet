use cabinet_domain::asset_import_operation::{
    AssetImportEvent, AssetImportOperation, AssetImportOperationError, AssetImportOperationId,
    AssetImportState,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;

#[test]
fn operation_tracks_identity_state_attempt_and_bounded_progress() {
    let mut operation = operation();
    operation.apply(AssetImportEvent::Begin, 0).expect("begin");
    operation
        .apply(AssetImportEvent::ValidationSucceeded, 0)
        .expect("validate");
    operation
        .apply(AssetImportEvent::StagingSucceeded, 4)
        .expect("stage");
    assert_eq!(operation.state(), AssetImportState::Hashing);
    assert_eq!(operation.attempt(), 1);
    assert_eq!(operation.completed_bytes(), 4);
    assert_eq!(operation.total_bytes(), 8);
}

#[test]
fn operation_rejects_invalid_identity_and_progress() {
    assert_eq!(
        AssetImportOperationId::new(" ").expect_err("id"),
        AssetImportOperationError::InvalidOperationId
    );
    let mut operation = operation();
    assert_eq!(
        operation
            .apply(AssetImportEvent::Begin, 9)
            .expect_err("overflow"),
        AssetImportOperationError::InvalidProgress
    );
    assert_eq!(
        AssetImportOperation::restore(
            AssetImportOperationId::new("import-2").expect("id"),
            WorkspaceId::new("workspace-1").expect("workspace"),
            DocumentId::new("doc-1").expect("document"),
            AssetImportState::Staging,
            1,
            9,
            8,
        )
        .expect_err("invalid restore"),
        AssetImportOperationError::InvalidProgress
    );
}

fn operation() -> AssetImportOperation {
    AssetImportOperation::new(
        AssetImportOperationId::new("import-1").expect("id"),
        WorkspaceId::new("workspace-1").expect("workspace"),
        DocumentId::new("doc-1").expect("document"),
        8,
    )
    .expect("operation")
}
