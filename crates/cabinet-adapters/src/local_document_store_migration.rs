use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use cabinet_domain::document::{DocumentBodyPolicy, DocumentId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;

use crate::local_atomic_file::write_text_atomically;
use crate::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT,
};
use crate::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use crate::local_version_store::LocalVersionStore;

pub const LEGACY_DOCUMENT_VERSION_ROOT: &str = "authoring-versions";
pub const LEGACY_DOCUMENT_POINTER_ROOT: &str = "authoring-current-version";
pub const AUTHORITATIVE_DOCUMENT_MIGRATION_STAGING_ROOT: &str =
    "document-store-migration-v1.staging";
pub const AUTHORITATIVE_DOCUMENT_MIGRATION_MARKER: &str = "document-store-migration-v1.complete";

const STAGED_VERSIONS: &str = "versions";
const STAGED_POINTERS: &str = "pointers";
const COMPLETED_MARKER_CONTENT: &str = "schema=1\nstate=completed\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalDocumentStoreMigrationOutcome {
    NoLegacyData,
    Migrated,
    AlreadyMigrated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalDocumentStoreMigrationError {
    Conflict,
    CorruptedLegacy,
    StorageUnavailable,
}

impl LocalDocumentStoreMigrationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Conflict => "document_store_migration.conflict",
            Self::CorruptedLegacy => "document_store_migration.corrupted_legacy",
            Self::StorageUnavailable => "document_store_migration.storage_unavailable",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalDocumentStoreMigration {
    app_data_root: PathBuf,
    body_policy: DocumentBodyPolicy,
}

impl LocalDocumentStoreMigration {
    pub fn new(app_data_root: PathBuf, body_policy: DocumentBodyPolicy) -> Self {
        Self {
            app_data_root,
            body_policy,
        }
    }

    pub fn execute(
        &self,
    ) -> Result<LocalDocumentStoreMigrationOutcome, LocalDocumentStoreMigrationError> {
        if self.has_completed_marker()? {
            self.validate_authoritative_roots()?;
            return Ok(LocalDocumentStoreMigrationOutcome::AlreadyMigrated);
        }
        let legacy_versions = self.app_data_root.join(LEGACY_DOCUMENT_VERSION_ROOT);
        let legacy_pointers = self.app_data_root.join(LEGACY_DOCUMENT_POINTER_ROOT);
        if !legacy_versions.exists() && !legacy_pointers.exists() {
            self.initialize_authoritative_roots()?;
            return Ok(LocalDocumentStoreMigrationOutcome::NoLegacyData);
        }
        require_plain_directory(&legacy_versions)?;
        require_plain_directory(&legacy_pointers)?;

        let staging = self
            .app_data_root
            .join(AUTHORITATIVE_DOCUMENT_MIGRATION_STAGING_ROOT);
        remove_staging(&staging)?;
        fs::create_dir_all(&staging)
            .map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
        let staged_versions = staging.join(STAGED_VERSIONS);
        let staged_pointers = staging.join(STAGED_POINTERS);

        let result = (|| {
            copy_tree(&legacy_versions, &staged_versions)?;
            copy_tree(&legacy_pointers, &staged_pointers)?;
            LocalVersionStore::with_body_policy(staged_versions.clone(), self.body_policy)
                .migrate_revision_numbers()
                .map_err(|_| LocalDocumentStoreMigrationError::CorruptedLegacy)?;
            validate_pointer_tree(&staged_pointers)?;
            self.publish_staged(&staged_versions, &staged_pointers)
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }

    fn has_completed_marker(&self) -> Result<bool, LocalDocumentStoreMigrationError> {
        let marker = self
            .app_data_root
            .join(AUTHORITATIVE_DOCUMENT_MIGRATION_MARKER);
        let metadata = match fs::symlink_metadata(&marker) {
            Ok(value) => value,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
            Err(_) => return Err(LocalDocumentStoreMigrationError::StorageUnavailable),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(LocalDocumentStoreMigrationError::CorruptedLegacy);
        }
        let content = fs::read_to_string(marker)
            .map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
        if content != COMPLETED_MARKER_CONTENT {
            return Err(LocalDocumentStoreMigrationError::CorruptedLegacy);
        }
        Ok(true)
    }

    fn validate_authoritative_roots(&self) -> Result<(), LocalDocumentStoreMigrationError> {
        for root in [LOCAL_DOCUMENT_VERSION_ROOT, LOCAL_DOCUMENT_POINTER_ROOT] {
            require_plain_directory(&self.app_data_root.join(root))?;
        }
        Ok(())
    }

    fn initialize_authoritative_roots(&self) -> Result<(), LocalDocumentStoreMigrationError> {
        for root in [LOCAL_DOCUMENT_VERSION_ROOT, LOCAL_DOCUMENT_POINTER_ROOT] {
            fs::create_dir_all(self.app_data_root.join(root))
                .map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
        }
        Ok(())
    }

    fn publish_staged(
        &self,
        staged_versions: &Path,
        staged_pointers: &Path,
    ) -> Result<LocalDocumentStoreMigrationOutcome, LocalDocumentStoreMigrationError> {
        let target_versions = self.app_data_root.join(LOCAL_DOCUMENT_VERSION_ROOT);
        let target_pointers = self.app_data_root.join(LOCAL_DOCUMENT_POINTER_ROOT);
        let versions_existed = target_versions.exists();
        let pointers_existed = target_pointers.exists();

        validate_existing_target(&target_versions, staged_versions)?;
        validate_existing_target(&target_pointers, staged_pointers)?;

        publish_tree_if_missing(staged_versions, &target_versions)?;
        publish_tree_if_missing(staged_pointers, &target_pointers)?;
        write_text_atomically(
            &self
                .app_data_root
                .join(AUTHORITATIVE_DOCUMENT_MIGRATION_MARKER),
            COMPLETED_MARKER_CONTENT,
        )
        .map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
        let _ = fs::remove_dir_all(
            self.app_data_root
                .join(AUTHORITATIVE_DOCUMENT_MIGRATION_STAGING_ROOT),
        );

        if versions_existed && pointers_existed {
            Ok(LocalDocumentStoreMigrationOutcome::AlreadyMigrated)
        } else {
            Ok(LocalDocumentStoreMigrationOutcome::Migrated)
        }
    }
}

fn require_plain_directory(path: &Path) -> Result<(), LocalDocumentStoreMigrationError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| LocalDocumentStoreMigrationError::CorruptedLegacy)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LocalDocumentStoreMigrationError::CorruptedLegacy);
    }
    Ok(())
}

fn remove_staging(path: &Path) -> Result<(), LocalDocumentStoreMigrationError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err(LocalDocumentStoreMigrationError::CorruptedLegacy)
        }
        Ok(_) => fs::remove_dir_all(path)
            .map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(LocalDocumentStoreMigrationError::StorageUnavailable),
    }
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), LocalDocumentStoreMigrationError> {
    require_plain_directory(source)?;
    fs::create_dir(target).map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
    for entry in
        fs::read_dir(source).map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?
    {
        let entry = entry.map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)
            .map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
        if metadata.file_type().is_symlink() {
            return Err(LocalDocumentStoreMigrationError::CorruptedLegacy);
        }
        if metadata.is_dir() {
            copy_tree(&source_path, &target_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &target_path)
                .map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
        } else {
            return Err(LocalDocumentStoreMigrationError::CorruptedLegacy);
        }
    }
    Ok(())
}

fn validate_existing_target(
    target: &Path,
    staged: &Path,
) -> Result<(), LocalDocumentStoreMigrationError> {
    if !target.exists() {
        return Ok(());
    }
    require_plain_directory(target).map_err(|_| LocalDocumentStoreMigrationError::Conflict)?;
    if tree_files(target).map_err(|_| LocalDocumentStoreMigrationError::Conflict)?
        == tree_files(staged).map_err(|_| LocalDocumentStoreMigrationError::CorruptedLegacy)?
    {
        Ok(())
    } else {
        Err(LocalDocumentStoreMigrationError::Conflict)
    }
}

fn publish_tree_if_missing(
    staged: &Path,
    target: &Path,
) -> Result<(), LocalDocumentStoreMigrationError> {
    if target.exists() {
        fs::remove_dir_all(staged)
            .map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
        return Ok(());
    }
    fs::rename(staged, target).map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)
}

fn tree_files(root: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, LocalDocumentStoreMigrationError> {
    let mut files = Vec::new();
    collect_tree_files(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn collect_tree_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<(PathBuf, Vec<u8>)>,
) -> Result<(), LocalDocumentStoreMigrationError> {
    for entry in
        fs::read_dir(current).map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?
    {
        let entry = entry.map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
        if metadata.file_type().is_symlink() {
            return Err(LocalDocumentStoreMigrationError::CorruptedLegacy);
        }
        if metadata.is_dir() {
            collect_tree_files(root, &path, files)?;
        } else if metadata.is_file() {
            files.push((
                path.strip_prefix(root)
                    .map_err(|_| LocalDocumentStoreMigrationError::CorruptedLegacy)?
                    .to_path_buf(),
                fs::read(path).map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?,
            ));
        } else {
            return Err(LocalDocumentStoreMigrationError::CorruptedLegacy);
        }
    }
    Ok(())
}

fn validate_pointer_tree(root: &Path) -> Result<(), LocalDocumentStoreMigrationError> {
    let pointer = LocalCurrentDocumentVersionPointer::new(root.to_path_buf());
    for workspace_entry in
        fs::read_dir(root).map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?
    {
        let workspace_entry =
            workspace_entry.map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
        require_plain_directory(&workspace_entry.path())?;
        let workspace = WorkspaceId::new(&decode_hex_name(&workspace_entry.file_name())?)
            .map_err(|_| LocalDocumentStoreMigrationError::CorruptedLegacy)?;
        for document_entry in fs::read_dir(workspace_entry.path())
            .map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?
        {
            let document_entry =
                document_entry.map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
            require_plain_directory(&document_entry.path())?;
            let document = DocumentId::new(&decode_hex_name(&document_entry.file_name())?)
                .map_err(|_| LocalDocumentStoreMigrationError::CorruptedLegacy)?;
            let entries = fs::read_dir(document_entry.path())
                .map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| LocalDocumentStoreMigrationError::StorageUnavailable)?;
            if entries.len() != 1 || entries[0].file_name() != "current.pointer" {
                return Err(LocalDocumentStoreMigrationError::CorruptedLegacy);
            }
            pointer
                .load_current_version(&workspace, &document)
                .map_err(|_| LocalDocumentStoreMigrationError::CorruptedLegacy)?
                .ok_or(LocalDocumentStoreMigrationError::CorruptedLegacy)?;
        }
    }
    Ok(())
}

fn decode_hex_name(value: &std::ffi::OsStr) -> Result<String, LocalDocumentStoreMigrationError> {
    let value = value
        .to_str()
        .ok_or(LocalDocumentStoreMigrationError::CorruptedLegacy)?;
    if value.len() % 2 != 0 {
        return Err(LocalDocumentStoreMigrationError::CorruptedLegacy);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair)
                .map_err(|_| LocalDocumentStoreMigrationError::CorruptedLegacy)?;
            u8::from_str_radix(pair, 16)
                .map_err(|_| LocalDocumentStoreMigrationError::CorruptedLegacy)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| LocalDocumentStoreMigrationError::CorruptedLegacy)
}
