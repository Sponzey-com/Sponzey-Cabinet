use std::fs::{self, OpenOptions};
use std::path::PathBuf;

use cabinet_core::migration::{MigrationErrorCode, MigrationStore, MigrationVersion};

use crate::local_atomic_file::{recover_stale_temp, write_text_atomically};

pub const MIGRATION_LOCK_FILE: &str = "migration.lock";
pub const MIGRATION_VERSIONS_FILE: &str = "migration-versions.txt";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMigrationStore {
    metadata_dir: PathBuf,
}

impl LocalMigrationStore {
    pub fn new(metadata_dir: PathBuf) -> Self {
        Self { metadata_dir }
    }

    fn lock_path(&self) -> PathBuf {
        self.metadata_dir.join(MIGRATION_LOCK_FILE)
    }

    fn versions_path(&self) -> PathBuf {
        self.metadata_dir.join(MIGRATION_VERSIONS_FILE)
    }

    fn read_valid_versions(&self) -> Result<Vec<MigrationVersion>, MigrationErrorCode> {
        let versions_path = self.versions_path();
        if !versions_path.exists() {
            return Ok(Vec::new());
        }

        let content =
            fs::read_to_string(versions_path).map_err(|_| MigrationErrorCode::VersionReadFailed)?;
        let mut versions = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return Err(MigrationErrorCode::VersionReadFailed);
            }
            let value = trimmed
                .parse::<u32>()
                .map_err(|_| MigrationErrorCode::VersionReadFailed)?;
            if value == 0
                || versions
                    .last()
                    .is_some_and(|previous: &MigrationVersion| previous.value() >= value)
            {
                return Err(MigrationErrorCode::VersionReadFailed);
            }
            versions.push(MigrationVersion::new(value));
        }
        Ok(versions)
    }
}

impl MigrationStore for LocalMigrationStore {
    fn acquire_lock(&mut self) -> Result<(), MigrationErrorCode> {
        if !self.metadata_dir.is_dir() {
            return Err(MigrationErrorCode::LockAcquireFailed);
        }

        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.lock_path())
            .map_err(|_| MigrationErrorCode::LockAcquireFailed)?;
        Ok(())
    }

    fn applied_versions(&mut self) -> Result<Vec<MigrationVersion>, MigrationErrorCode> {
        self.read_valid_versions()
    }

    fn record_version(&mut self, version: MigrationVersion) -> Result<(), MigrationErrorCode> {
        if !self.metadata_dir.is_dir() {
            return Err(MigrationErrorCode::VersionRecordFailed);
        }

        let mut versions = self
            .read_valid_versions()
            .map_err(|_| MigrationErrorCode::VersionRecordFailed)?;
        if version.value() == 0
            || versions
                .last()
                .is_some_and(|previous| previous.value() >= version.value())
        {
            return Err(MigrationErrorCode::VersionRecordFailed);
        }
        versions.push(version);

        let versions_path = self.versions_path();
        recover_stale_temp(&versions_path).map_err(|_| MigrationErrorCode::VersionRecordFailed)?;
        let content = versions
            .iter()
            .map(|version| version.value().to_string())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        write_text_atomically(&versions_path, content)
            .map(|_| ())
            .map_err(|_| MigrationErrorCode::VersionRecordFailed)
    }

    fn release_lock(&mut self) -> Result<(), MigrationErrorCode> {
        let lock_path = self.lock_path();
        if lock_path.exists() {
            fs::remove_file(lock_path).map_err(|_| MigrationErrorCode::LockReleaseFailed)?;
        }
        Ok(())
    }
}
