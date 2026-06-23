use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use cabinet_core::migration::{MigrationErrorCode, MigrationStore, MigrationVersion};

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
        let versions_path = self.versions_path();
        if !versions_path.exists() {
            return Ok(Vec::new());
        }

        let content =
            fs::read_to_string(versions_path).map_err(|_| MigrationErrorCode::VersionReadFailed)?;
        content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                line.trim()
                    .parse::<u32>()
                    .map(MigrationVersion::new)
                    .map_err(|_| MigrationErrorCode::VersionReadFailed)
            })
            .collect()
    }

    fn record_version(&mut self, version: MigrationVersion) -> Result<(), MigrationErrorCode> {
        if !self.metadata_dir.is_dir() {
            return Err(MigrationErrorCode::VersionRecordFailed);
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.versions_path())
            .map_err(|_| MigrationErrorCode::VersionRecordFailed)?;
        writeln!(file, "{}", version.value()).map_err(|_| MigrationErrorCode::VersionRecordFailed)
    }

    fn release_lock(&mut self) -> Result<(), MigrationErrorCode> {
        let lock_path = self.lock_path();
        if lock_path.exists() {
            fs::remove_file(lock_path).map_err(|_| MigrationErrorCode::LockAcquireFailed)?;
        }
        Ok(())
    }
}
