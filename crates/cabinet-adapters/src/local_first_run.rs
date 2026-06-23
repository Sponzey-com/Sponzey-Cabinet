use std::fs;
use std::path::Path;

use cabinet_core::first_run::{
    FirstRunDirectoryRole, FirstRunErrorCode, FirstRunStore, FirstRunStoreStatus,
};

pub const FIRST_RUN_MARKER_FILE: &str = "first-run.marker";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LocalFirstRunStore;

impl LocalFirstRunStore {
    pub const fn new() -> Self {
        Self
    }
}

impl FirstRunStore for LocalFirstRunStore {
    fn ensure_directory(
        &mut self,
        _role: FirstRunDirectoryRole,
        path: &Path,
    ) -> Result<FirstRunStoreStatus, FirstRunErrorCode> {
        if path.is_dir() {
            return Ok(FirstRunStoreStatus::AlreadyPresent);
        }
        if path.exists() {
            return Err(FirstRunErrorCode::StoreCreationFailed);
        }

        fs::create_dir_all(path).map_err(|_| FirstRunErrorCode::StoreCreationFailed)?;
        Ok(FirstRunStoreStatus::Created)
    }

    fn write_metadata_marker(
        &mut self,
        metadata_dir: &Path,
    ) -> Result<FirstRunStoreStatus, FirstRunErrorCode> {
        if !metadata_dir.is_dir() {
            return Err(FirstRunErrorCode::MetadataWriteFailed);
        }

        let marker_path = metadata_dir.join(FIRST_RUN_MARKER_FILE);
        if marker_path.exists() {
            return Ok(FirstRunStoreStatus::AlreadyPresent);
        }

        fs::write(marker_path, b"initialized\n")
            .map_err(|_| FirstRunErrorCode::MetadataWriteFailed)?;
        Ok(FirstRunStoreStatus::Created)
    }
}
