use cabinet_domain::backup::{BackupJobId, BackupPackageManifest};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupCatalogRecord {
    package_id: BackupJobId,
    manifest: BackupPackageManifest,
}

impl BackupCatalogRecord {
    pub fn new(package_id: BackupJobId, manifest: BackupPackageManifest) -> Self {
        Self {
            package_id,
            manifest,
        }
    }

    pub fn package_id(&self) -> &BackupJobId {
        &self.package_id
    }

    pub fn manifest(&self) -> &BackupPackageManifest {
        &self.manifest
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupCatalogPage {
    records: Vec<BackupCatalogRecord>,
    next_cursor: Option<String>,
}

impl BackupCatalogPage {
    pub fn new(records: Vec<BackupCatalogRecord>, next_cursor: Option<String>) -> Self {
        Self {
            records,
            next_cursor,
        }
    }

    pub fn records(&self) -> &[BackupCatalogRecord] {
        &self.records
    }

    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}

pub trait BackupCatalogPort {
    fn list_backup_packages(
        &self,
        workspace_id: &WorkspaceId,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<BackupCatalogPage, BackupCatalogError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupCatalogError {
    InvalidLimit,
    InvalidCursor,
    StorageUnavailable,
    CorruptedCatalog,
}

impl BackupCatalogError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimit => "backup_catalog.invalid_limit",
            Self::InvalidCursor => "backup_catalog.invalid_cursor",
            Self::StorageUnavailable => "backup_catalog.storage_unavailable",
            Self::CorruptedCatalog => "backup_catalog.corrupted",
        }
    }
}
