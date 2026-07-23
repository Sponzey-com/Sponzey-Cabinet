use std::collections::HashSet;

use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_catalog::{BackupCatalogError, BackupCatalogPort};

use crate::backup_package::BackupPackageSummary;

pub const BACKUP_CATALOG_MAX_PAGE_SIZE: usize = 50;
const BACKUP_CATALOG_MAX_CURSOR_LENGTH: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListBackupCatalogInput {
    workspace_id: String,
    cursor: Option<String>,
    limit: usize,
}

impl ListBackupCatalogInput {
    pub fn new(workspace_id: &str, cursor: Option<&str>, limit: usize) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            cursor: cursor.map(str::to_string),
            limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupCatalogItem {
    package_id: String,
    created_at_epoch_ms: Option<u64>,
    summary: BackupPackageSummary,
}

impl BackupCatalogItem {
    pub fn package_id(&self) -> &str {
        &self.package_id
    }

    pub const fn created_at_epoch_ms(&self) -> Option<u64> {
        self.created_at_epoch_ms
    }

    pub const fn summary(&self) -> &BackupPackageSummary {
        &self.summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListBackupCatalogOutput {
    records: Vec<BackupCatalogItem>,
    next_cursor: Option<String>,
}

impl ListBackupCatalogOutput {
    pub fn records(&self) -> &[BackupCatalogItem] {
        &self.records
    }

    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListBackupCatalogError {
    InvalidInput,
    CatalogUnavailable,
    CorruptedCatalog,
}

impl ListBackupCatalogError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "backup_catalog.invalid_input",
            Self::CatalogUnavailable => "backup_catalog.unavailable",
            Self::CorruptedCatalog => "backup_catalog.corrupted",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(self, Self::CatalogUnavailable)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ListBackupCatalogUsecase;

impl ListBackupCatalogUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ListBackupCatalogInput,
        catalog: &impl BackupCatalogPort,
    ) -> Result<ListBackupCatalogOutput, ListBackupCatalogError> {
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ListBackupCatalogError::InvalidInput)?;
        if input.limit == 0 || input.limit > BACKUP_CATALOG_MAX_PAGE_SIZE {
            return Err(ListBackupCatalogError::InvalidInput);
        }
        if input
            .cursor
            .as_deref()
            .is_some_and(|cursor| !valid_cursor(cursor))
        {
            return Err(ListBackupCatalogError::InvalidInput);
        }

        let page = catalog
            .list_backup_packages(&workspace, input.cursor.as_deref(), input.limit)
            .map_err(map_catalog_error)?;
        if page.records().len() > input.limit
            || page
                .next_cursor()
                .is_some_and(|cursor| !valid_cursor(cursor))
            || !is_newest_first(page.records())
            || has_duplicate_identity(page.records())
        {
            return Err(ListBackupCatalogError::CorruptedCatalog);
        }

        let records = page
            .records()
            .iter()
            .map(|record| BackupCatalogItem {
                package_id: record.package_id().as_str().to_string(),
                created_at_epoch_ms: record.manifest().created_at_epoch_ms(),
                summary: BackupPackageSummary::from_manifest(record.manifest()),
            })
            .collect();
        Ok(ListBackupCatalogOutput {
            records,
            next_cursor: page.next_cursor().map(str::to_string),
        })
    }
}

fn valid_cursor(cursor: &str) -> bool {
    !cursor.is_empty()
        && cursor.len() <= BACKUP_CATALOG_MAX_CURSOR_LENGTH
        && !cursor.chars().any(char::is_control)
}

fn is_newest_first(records: &[cabinet_ports::backup_catalog::BackupCatalogRecord]) -> bool {
    records.windows(2).all(|pair| {
        pair[0].manifest().created_at_epoch_ms() >= pair[1].manifest().created_at_epoch_ms()
    })
}

fn has_duplicate_identity(records: &[cabinet_ports::backup_catalog::BackupCatalogRecord]) -> bool {
    let mut identities = HashSet::with_capacity(records.len());
    records
        .iter()
        .any(|record| !identities.insert(record.package_id().as_str()))
}

const fn map_catalog_error(error: BackupCatalogError) -> ListBackupCatalogError {
    match error {
        BackupCatalogError::InvalidLimit | BackupCatalogError::InvalidCursor => {
            ListBackupCatalogError::InvalidInput
        }
        BackupCatalogError::StorageUnavailable => ListBackupCatalogError::CatalogUnavailable,
        BackupCatalogError::CorruptedCatalog => ListBackupCatalogError::CorruptedCatalog,
    }
}
