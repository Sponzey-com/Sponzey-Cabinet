use cabinet_domain::backup::{BackupJobId, BackupPackageManifest};
use cabinet_domain::workspace::WorkspaceId;

pub trait BackupPackageStore {
    fn build_package(
        &mut self,
        workspace_id: &WorkspaceId,
        package_id: &BackupJobId,
    ) -> Result<BackupPackageManifest, BackupPackageStoreError>;

    fn inspect_manifest(
        &mut self,
        workspace_id: &WorkspaceId,
        package_id: &BackupJobId,
    ) -> Result<BackupPackageManifest, BackupPackageStoreError>;

    fn discard_package(
        &mut self,
        workspace_id: &WorkspaceId,
        package_id: &BackupJobId,
    ) -> Result<(), BackupPackageStoreError>;

    fn validate_package(
        &mut self,
        workspace_id: &WorkspaceId,
        package_id: &BackupJobId,
        manifest: &BackupPackageManifest,
    ) -> Result<BackupPackageValidation, BackupPackageStoreError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupPackageStoreError {
    StorageUnavailable,
    PackageNotFound,
    CorruptedPackage,
    Conflict,
}

impl BackupPackageStoreError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "backup_package.storage_unavailable",
            Self::PackageNotFound => "backup_package.not_found",
            Self::CorruptedPackage => "backup_package.corrupted",
            Self::Conflict => "backup_package.conflict",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPackageValidation {
    valid: bool,
    error_code: Option<&'static str>,
}

impl BackupPackageValidation {
    pub const fn valid() -> Self {
        Self {
            valid: true,
            error_code: None,
        }
    }

    pub const fn failed(error_code: &'static str) -> Self {
        Self {
            valid: false,
            error_code: Some(error_code),
        }
    }

    pub const fn is_valid(&self) -> bool {
        self.valid
    }

    pub const fn error_code(&self) -> Option<&'static str> {
        self.error_code
    }
}
