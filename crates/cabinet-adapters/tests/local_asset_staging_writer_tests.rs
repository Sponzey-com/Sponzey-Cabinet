use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_asset_staging_writer::LocalAssetStagingWriter;
use cabinet_domain::asset_import_operation::AssetImportOperationId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_staging::{AssetStagingError, AssetStagingWriter};

#[test]
fn staging_writer_supports_restart_finalize_offset_guard_and_cleanup() {
    let root = std::env::temp_dir().join(format!(
        "sponzey-staging-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let operation = AssetImportOperationId::new("import-1").expect("operation");
    let mut writer = LocalAssetStagingWriter::new(root.clone());
    writer.begin(&workspace, &operation).expect("begin");
    writer
        .append(&workspace, &operation, 0, b"abc")
        .expect("append");
    assert_eq!(
        writer
            .append(&workspace, &operation, 1, b"x")
            .expect_err("offset"),
        AssetStagingError::OffsetConflict
    );
    let mut restarted = LocalAssetStagingWriter::new(root.clone());
    assert_eq!(
        restarted
            .finalize(&workspace, &operation, 3)
            .expect("finalize")
            .byte_size(),
        3
    );
    assert_eq!(
        restarted
            .finalize(&workspace, &operation, 4)
            .expect_err("size"),
        AssetStagingError::SizeMismatch
    );
    restarted.cleanup(&workspace, &operation).expect("cleanup");
    assert_eq!(
        restarted
            .finalize(&workspace, &operation, 3)
            .expect_err("missing"),
        AssetStagingError::NotFound
    );
    let _ = fs::remove_dir_all(root);
}
