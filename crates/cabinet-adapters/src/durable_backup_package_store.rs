use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_domain::backup::{
    BackupDataClass, BackupDataOwnership, BackupJobId, BackupManifestEntry, BackupPackageManifest,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_catalog::{
    BackupCatalogError, BackupCatalogPage, BackupCatalogPort, BackupCatalogRecord,
};
use cabinet_ports::backup_package::{
    BackupPackageStore, BackupPackageStoreError, BackupPackageValidation,
};
use sha2::{Digest, Sha256};

use crate::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT,
};

const MANIFEST_SCHEMA: u16 = 1;
pub(crate) const CURRENT_VERSION_POINTERS_PAYLOAD_DIR: &str = ".version-pointers";
const MANIFEST_FILE: &str = "manifest.tsv";
const PAYLOAD_DIR: &str = "data";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalBackupPackagePolicy {
    max_file_count: u64,
    max_total_bytes: u64,
}

impl LocalBackupPackagePolicy {
    pub const fn new(
        max_file_count: u64,
        max_total_bytes: u64,
    ) -> Result<Self, LocalBackupPackagePolicyError> {
        if max_file_count == 0 || max_total_bytes == 0 {
            return Err(LocalBackupPackagePolicyError::InvalidLimit);
        }
        Ok(Self {
            max_file_count,
            max_total_bytes,
        })
    }

    pub(crate) const fn max_file_count(self) -> u64 {
        self.max_file_count
    }

    pub(crate) const fn max_total_bytes(self) -> u64 {
        self.max_total_bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalBackupPackagePolicyError {
    InvalidLimit,
}

#[derive(Debug, Clone)]
pub struct LocalBackupPackageStore {
    app_data_root: PathBuf,
    policy: LocalBackupPackagePolicy,
    clock: fn() -> u64,
}

impl LocalBackupPackageStore {
    pub fn new(app_data_root: PathBuf, policy: LocalBackupPackagePolicy) -> Self {
        Self {
            app_data_root,
            policy,
            clock: system_epoch_ms,
        }
    }

    pub fn with_clock(
        app_data_root: PathBuf,
        policy: LocalBackupPackagePolicy,
        clock: fn() -> u64,
    ) -> Self {
        Self {
            app_data_root,
            policy,
            clock,
        }
    }

    fn packages_root(&self, workspace: &WorkspaceId) -> PathBuf {
        self.app_data_root
            .join("backup-packages")
            .join(hex(workspace.as_str()))
    }

    pub(crate) fn package_root(&self, workspace: &WorkspaceId, package: &BackupJobId) -> PathBuf {
        self.packages_root(workspace).join(hex(package.as_str()))
    }

    fn staging_root(&self, workspace: &WorkspaceId, package: &BackupJobId) -> PathBuf {
        self.packages_root(workspace)
            .join(format!(".{}.staging", hex(package.as_str())))
    }

    fn source_root(&self, workspace: &WorkspaceId, class: BackupDataClass) -> Option<PathBuf> {
        let document_workspace = encode_document_segment(workspace.as_str());
        let workspace = hex(workspace.as_str());
        match class {
            BackupDataClass::CurrentDocuments => Some(
                self.app_data_root
                    .join("authoring-current")
                    .join(document_workspace.clone()),
            ),
            BackupDataClass::VersionHistory => Some(
                self.app_data_root
                    .join(LOCAL_DOCUMENT_VERSION_ROOT)
                    .join(document_workspace),
            ),
            BackupDataClass::CanvasRecords => {
                Some(self.app_data_root.join("canvases").join(workspace))
            }
            BackupDataClass::AssetMetadata => {
                Some(self.app_data_root.join("assets/metadata").join(workspace))
            }
            BackupDataClass::AssetObjects => {
                Some(self.app_data_root.join("assets/objects").join(workspace))
            }
            BackupDataClass::AssetAssociations => Some(
                self.app_data_root
                    .join("assets/associations")
                    .join(workspace),
            ),
            BackupDataClass::GraphRebuildMetadata | BackupDataClass::SearchRebuildMetadata => None,
        }
    }

    fn build_in_staging(
        &self,
        workspace: &WorkspaceId,
        staging: &Path,
    ) -> Result<BackupPackageManifest, BackupPackageStoreError> {
        let payload = staging.join(PAYLOAD_DIR);
        let mut usage = PackageUsage::default();
        for class in BackupDataClass::ALL {
            let destination = payload.join(class_name(class));
            fs::create_dir_all(&destination)
                .map_err(|_| BackupPackageStoreError::StorageUnavailable)?;
            if let Some(source) = self.source_root(workspace, class) {
                copy_tree(&source, &destination, self.policy, &mut usage)?;
                if class == BackupDataClass::CurrentDocuments {
                    let pointer_destination =
                        destination.join(CURRENT_VERSION_POINTERS_PAYLOAD_DIR);
                    if pointer_destination.exists() {
                        return Err(BackupPackageStoreError::CorruptedPackage);
                    }
                    let pointer_source = self
                        .app_data_root
                        .join(LOCAL_DOCUMENT_POINTER_ROOT)
                        .join(hex(workspace.as_str()));
                    fs::create_dir_all(&pointer_destination)
                        .map_err(|_| BackupPackageStoreError::StorageUnavailable)?;
                    copy_tree(
                        &pointer_source,
                        &pointer_destination,
                        self.policy,
                        &mut usage,
                    )?;
                }
            } else {
                let metadata = format!(
                    "schema\t1\nkind\t{}\nsource\tcurrent_documents\n",
                    class_name(class)
                );
                usage.add(1, metadata.len() as u64, self.policy)?;
                write_synced(&destination.join("rebuild.meta"), metadata.as_bytes())?;
            }
        }
        let entries = BackupDataClass::ALL
            .into_iter()
            .map(|class| {
                let digest = digest_tree(&payload.join(class_name(class)))?;
                BackupManifestEntry::new(
                    class,
                    class.expected_ownership(),
                    digest.file_count,
                    digest.byte_count,
                    &digest.checksum,
                )
                .map_err(|_| BackupPackageStoreError::CorruptedPackage)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let manifest = BackupPackageManifest::new(MANIFEST_SCHEMA, entries)
            .and_then(|manifest| manifest.with_created_at_epoch_ms((self.clock)()))
            .map_err(|_| BackupPackageStoreError::CorruptedPackage)?;
        write_synced(
            &staging.join(MANIFEST_FILE),
            encode_manifest(&manifest).as_bytes(),
        )?;
        Ok(manifest)
    }
}

impl BackupPackageStore for LocalBackupPackageStore {
    fn build_package(
        &mut self,
        workspace_id: &WorkspaceId,
        package_id: &BackupJobId,
    ) -> Result<BackupPackageManifest, BackupPackageStoreError> {
        let final_root = self.package_root(workspace_id, package_id);
        let staging = self.staging_root(workspace_id, package_id);
        if final_root.exists() || staging.exists() {
            return Err(BackupPackageStoreError::Conflict);
        }
        fs::create_dir_all(&staging).map_err(|_| BackupPackageStoreError::StorageUnavailable)?;
        let result = self.build_in_staging(workspace_id, &staging);
        let manifest = match result {
            Ok(manifest) => manifest,
            Err(error) => {
                let _ = fs::remove_dir_all(&staging);
                return Err(error);
            }
        };
        fs::rename(&staging, &final_root).map_err(|_| {
            let _ = fs::remove_dir_all(&staging);
            BackupPackageStoreError::StorageUnavailable
        })?;
        sync_directory(
            final_root
                .parent()
                .ok_or(BackupPackageStoreError::StorageUnavailable)?,
        )?;
        Ok(manifest)
    }

    fn inspect_manifest(
        &mut self,
        workspace_id: &WorkspaceId,
        package_id: &BackupJobId,
    ) -> Result<BackupPackageManifest, BackupPackageStoreError> {
        let root = self.package_root(workspace_id, package_id);
        let metadata = match fs::symlink_metadata(&root) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(BackupPackageStoreError::PackageNotFound);
            }
            Err(_) => return Err(BackupPackageStoreError::StorageUnavailable),
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(BackupPackageStoreError::CorruptedPackage);
        }
        let text = fs::read_to_string(root.join(MANIFEST_FILE)).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                BackupPackageStoreError::CorruptedPackage
            } else {
                BackupPackageStoreError::StorageUnavailable
            }
        })?;
        decode_manifest(&text)
    }

    fn discard_package(
        &mut self,
        workspace_id: &WorkspaceId,
        package_id: &BackupJobId,
    ) -> Result<(), BackupPackageStoreError> {
        let root = self.package_root(workspace_id, package_id);
        let metadata = match fs::symlink_metadata(&root) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err(BackupPackageStoreError::StorageUnavailable),
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(BackupPackageStoreError::CorruptedPackage);
        }
        fs::remove_dir_all(&root).map_err(|_| BackupPackageStoreError::StorageUnavailable)?;
        sync_directory(
            root.parent()
                .ok_or(BackupPackageStoreError::StorageUnavailable)?,
        )
    }

    fn validate_package(
        &mut self,
        workspace_id: &WorkspaceId,
        package_id: &BackupJobId,
        manifest: &BackupPackageManifest,
    ) -> Result<BackupPackageValidation, BackupPackageStoreError> {
        let inspected = self.inspect_manifest(workspace_id, package_id)?;
        if inspected != *manifest {
            return Ok(BackupPackageValidation::failed(
                "BACKUP_PACKAGE_MANIFEST_MISMATCH",
            ));
        }
        validate_payload_against_manifest(
            &self
                .package_root(workspace_id, package_id)
                .join(PAYLOAD_DIR),
            manifest,
        )
    }
}

impl BackupCatalogPort for LocalBackupPackageStore {
    fn list_backup_packages(
        &self,
        workspace_id: &WorkspaceId,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<BackupCatalogPage, BackupCatalogError> {
        if limit == 0 || limit > 50 {
            return Err(BackupCatalogError::InvalidLimit);
        }
        let offset = parse_catalog_cursor(cursor)?;
        let root = self.packages_root(workspace_id);
        let entries = match fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return if offset == 0 {
                    Ok(BackupCatalogPage::new(vec![], None))
                } else {
                    Err(BackupCatalogError::InvalidCursor)
                };
            }
            Err(_) => return Err(BackupCatalogError::StorageUnavailable),
        };

        let mut records = Vec::new();
        let mut reader = self.clone();
        for entry in entries {
            let entry = entry.map_err(|_| BackupCatalogError::StorageUnavailable)?;
            let encoded = entry
                .file_name()
                .into_string()
                .map_err(|_| BackupCatalogError::CorruptedCatalog)?;
            if encoded.starts_with('.') {
                continue;
            }
            let file_type = entry
                .file_type()
                .map_err(|_| BackupCatalogError::StorageUnavailable)?;
            if file_type.is_symlink() || !file_type.is_dir() {
                return Err(BackupCatalogError::CorruptedCatalog);
            }
            let decoded = decode_catalog_hex(&encoded)?;
            if hex(&decoded) != encoded {
                return Err(BackupCatalogError::CorruptedCatalog);
            }
            let package_id =
                BackupJobId::new(&decoded).map_err(|_| BackupCatalogError::CorruptedCatalog)?;
            let manifest = reader
                .inspect_manifest(workspace_id, &package_id)
                .map_err(map_catalog_manifest_error)?;
            records.push(BackupCatalogRecord::new(package_id, manifest));
        }
        records.sort_by(|left, right| {
            right
                .manifest()
                .created_at_epoch_ms()
                .cmp(&left.manifest().created_at_epoch_ms())
                .then_with(|| left.package_id().cmp(right.package_id()))
        });
        if offset > records.len() {
            return Err(BackupCatalogError::InvalidCursor);
        }
        let end = offset.saturating_add(limit).min(records.len());
        let page = records[offset..end].to_vec();
        let next_cursor = (end < records.len()).then(|| end.to_string());
        Ok(BackupCatalogPage::new(page, next_cursor))
    }
}

fn parse_catalog_cursor(cursor: Option<&str>) -> Result<usize, BackupCatalogError> {
    let Some(cursor) = cursor else { return Ok(0) };
    let offset = cursor
        .parse::<usize>()
        .map_err(|_| BackupCatalogError::InvalidCursor)?;
    if offset == 0 || offset.to_string() != cursor {
        return Err(BackupCatalogError::InvalidCursor);
    }
    Ok(offset)
}

fn decode_catalog_hex(value: &str) -> Result<String, BackupCatalogError> {
    if value.is_empty() || value.len() % 2 != 0 {
        return Err(BackupCatalogError::CorruptedCatalog);
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks_exact(2) {
        let pair = std::str::from_utf8(pair).map_err(|_| BackupCatalogError::CorruptedCatalog)?;
        bytes.push(u8::from_str_radix(pair, 16).map_err(|_| BackupCatalogError::CorruptedCatalog)?);
    }
    String::from_utf8(bytes).map_err(|_| BackupCatalogError::CorruptedCatalog)
}

const fn map_catalog_manifest_error(error: BackupPackageStoreError) -> BackupCatalogError {
    match error {
        BackupPackageStoreError::StorageUnavailable => BackupCatalogError::StorageUnavailable,
        BackupPackageStoreError::PackageNotFound
        | BackupPackageStoreError::CorruptedPackage
        | BackupPackageStoreError::Conflict => BackupCatalogError::CorruptedCatalog,
    }
}

#[derive(Default)]
struct PackageUsage {
    file_count: u64,
    byte_count: u64,
}

impl PackageUsage {
    fn add(
        &mut self,
        file_count: u64,
        byte_count: u64,
        policy: LocalBackupPackagePolicy,
    ) -> Result<(), BackupPackageStoreError> {
        self.file_count = self
            .file_count
            .checked_add(file_count)
            .ok_or(BackupPackageStoreError::CorruptedPackage)?;
        self.byte_count = self
            .byte_count
            .checked_add(byte_count)
            .ok_or(BackupPackageStoreError::CorruptedPackage)?;
        if self.file_count > policy.max_file_count || self.byte_count > policy.max_total_bytes {
            return Err(BackupPackageStoreError::Conflict);
        }
        Ok(())
    }
}

struct TreeDigest {
    file_count: u64,
    byte_count: u64,
    checksum: String,
}

fn copy_tree(
    source: &Path,
    destination: &Path,
    policy: LocalBackupPackagePolicy,
    usage: &mut PackageUsage,
) -> Result<(), BackupPackageStoreError> {
    for (relative, source_file, length) in list_regular_files(source)? {
        usage.add(1, length, policy)?;
        let target = destination.join(&relative);
        fs::create_dir_all(
            target
                .parent()
                .ok_or(BackupPackageStoreError::StorageUnavailable)?,
        )
        .map_err(|_| BackupPackageStoreError::StorageUnavailable)?;
        fs::copy(&source_file, &target).map_err(|_| BackupPackageStoreError::StorageUnavailable)?;
        File::open(&target)
            .and_then(|file| file.sync_all())
            .map_err(|_| BackupPackageStoreError::StorageUnavailable)?;
    }
    Ok(())
}

fn digest_tree(root: &Path) -> Result<TreeDigest, BackupPackageStoreError> {
    let files = list_regular_files(root)?;
    let mut hasher = Sha256::new();
    let mut byte_count = 0_u64;
    for (relative, path, length) in &files {
        let normalized = normalize_relative(relative)?;
        hasher.update((normalized.len() as u64).to_be_bytes());
        hasher.update(normalized.as_bytes());
        hasher.update(length.to_be_bytes());
        let file = File::open(path).map_err(|_| BackupPackageStoreError::StorageUnavailable)?;
        let mut reader = BufReader::new(file);
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|_| BackupPackageStoreError::StorageUnavailable)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        byte_count = byte_count
            .checked_add(*length)
            .ok_or(BackupPackageStoreError::CorruptedPackage)?;
    }
    Ok(TreeDigest {
        file_count: files.len() as u64,
        byte_count,
        checksum: format!("{:x}", hasher.finalize()),
    })
}

fn list_regular_files(
    root: &Path,
) -> Result<Vec<(PathBuf, PathBuf, u64)>, BackupPackageStoreError> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(BackupPackageStoreError::StorageUnavailable),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(BackupPackageStoreError::CorruptedPackage);
    }
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let entries =
            fs::read_dir(&directory).map_err(|_| BackupPackageStoreError::StorageUnavailable)?;
        for entry in entries {
            let path = entry
                .map_err(|_| BackupPackageStoreError::StorageUnavailable)?
                .path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|_| BackupPackageStoreError::StorageUnavailable)?;
            if metadata.file_type().is_symlink() {
                return Err(BackupPackageStoreError::CorruptedPackage);
            }
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                files.push((
                    path.strip_prefix(root)
                        .map_err(|_| BackupPackageStoreError::CorruptedPackage)?
                        .to_path_buf(),
                    path,
                    metadata.len(),
                ));
            } else {
                return Err(BackupPackageStoreError::CorruptedPackage);
            }
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn normalize_relative(path: &Path) -> Result<String, BackupPackageStoreError> {
    let values = path
        .components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .ok_or(BackupPackageStoreError::CorruptedPackage)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.is_empty() || values.iter().any(|value| *value == "." || *value == "..") {
        return Err(BackupPackageStoreError::CorruptedPackage);
    }
    Ok(values.join("/"))
}

fn encode_document_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' {
            encoded.push(byte as char);
        } else {
            encoded.push('~');
            encoded.push_str(&format!("{byte:02x}"));
        }
    }
    encoded
}

fn validate_payload_class_names(payload: &Path) -> Result<(), BackupPackageStoreError> {
    let mut actual = fs::read_dir(payload)
        .map_err(|_| BackupPackageStoreError::CorruptedPackage)?
        .map(|entry| {
            let path = entry
                .map_err(|_| BackupPackageStoreError::StorageUnavailable)?
                .path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|_| BackupPackageStoreError::StorageUnavailable)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(BackupPackageStoreError::CorruptedPackage);
            }
            path.file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string)
                .ok_or(BackupPackageStoreError::CorruptedPackage)
        })
        .collect::<Result<Vec<_>, _>>()?;
    actual.sort();
    let mut expected = BackupDataClass::ALL
        .into_iter()
        .map(class_name)
        .map(str::to_string)
        .collect::<Vec<_>>();
    expected.sort();
    if actual != expected {
        return Err(BackupPackageStoreError::CorruptedPackage);
    }
    Ok(())
}

pub(crate) fn validate_payload_against_manifest(
    payload: &Path,
    manifest: &BackupPackageManifest,
) -> Result<BackupPackageValidation, BackupPackageStoreError> {
    validate_payload_class_names(payload)?;
    for entry in manifest.entries() {
        let digest = digest_tree(&payload.join(class_name(entry.data_class())))?;
        if digest.file_count != entry.record_count()
            || digest.byte_count != entry.byte_count()
            || digest.checksum != entry.checksum_sha256()
        {
            return Ok(BackupPackageValidation::failed(
                "BACKUP_PACKAGE_CHECKSUM_MISMATCH",
            ));
        }
    }
    Ok(BackupPackageValidation::valid())
}

fn encode_manifest(manifest: &BackupPackageManifest) -> String {
    let mut text = format!("schema\t{}\n", manifest.schema_version());
    if let Some(created_at_epoch_ms) = manifest.created_at_epoch_ms() {
        text.push_str(&format!("created_at_epoch_ms\t{created_at_epoch_ms}\n"));
    }
    for entry in manifest.entries() {
        text.push_str(&format!(
            "entry\t{}\t{}\t{}\t{}\t{}\n",
            class_name(entry.data_class()),
            ownership_name(entry.ownership()),
            entry.record_count(),
            entry.byte_count(),
            entry.checksum_sha256(),
        ));
    }
    text
}

fn decode_manifest(text: &str) -> Result<BackupPackageManifest, BackupPackageStoreError> {
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or(BackupPackageStoreError::CorruptedPackage)?;
    let (key, version) = header
        .split_once('\t')
        .ok_or(BackupPackageStoreError::CorruptedPackage)?;
    if key != "schema" {
        return Err(BackupPackageStoreError::CorruptedPackage);
    }
    let version = version
        .parse::<u16>()
        .map_err(|_| BackupPackageStoreError::CorruptedPackage)?;
    let mut created_at_epoch_ms = None;
    let mut entries = Vec::new();
    for line in lines {
        if let Some(value) = line.strip_prefix("created_at_epoch_ms\t") {
            if created_at_epoch_ms.is_some() {
                return Err(BackupPackageStoreError::CorruptedPackage);
            }
            let value = value
                .parse::<u64>()
                .map_err(|_| BackupPackageStoreError::CorruptedPackage)?;
            if value == 0 {
                return Err(BackupPackageStoreError::CorruptedPackage);
            }
            created_at_epoch_ms = Some(value);
            continue;
        }
        entries.push({
            let values = line.split('\t').collect::<Vec<_>>();
            if values.len() != 6 || values[0] != "entry" {
                return Err(BackupPackageStoreError::CorruptedPackage);
            }
            let class = parse_class(values[1])?;
            let ownership = parse_ownership(values[2])?;
            let record_count = values[3]
                .parse::<u64>()
                .map_err(|_| BackupPackageStoreError::CorruptedPackage)?;
            let byte_count = values[4]
                .parse::<u64>()
                .map_err(|_| BackupPackageStoreError::CorruptedPackage)?;
            BackupManifestEntry::new(class, ownership, record_count, byte_count, values[5])
                .map_err(|_| BackupPackageStoreError::CorruptedPackage)
        }?);
    }
    let manifest = BackupPackageManifest::new(version, entries)
        .map_err(|_| BackupPackageStoreError::CorruptedPackage)?;
    match created_at_epoch_ms {
        Some(value) => manifest
            .with_created_at_epoch_ms(value)
            .map_err(|_| BackupPackageStoreError::CorruptedPackage),
        None => Ok(manifest),
    }
}

fn system_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

pub(crate) fn class_name(class: BackupDataClass) -> &'static str {
    match class {
        BackupDataClass::CurrentDocuments => "current_documents",
        BackupDataClass::VersionHistory => "version_history",
        BackupDataClass::CanvasRecords => "canvas_records",
        BackupDataClass::AssetMetadata => "asset_metadata",
        BackupDataClass::AssetObjects => "asset_objects",
        BackupDataClass::AssetAssociations => "asset_associations",
        BackupDataClass::GraphRebuildMetadata => "graph_rebuild_metadata",
        BackupDataClass::SearchRebuildMetadata => "search_rebuild_metadata",
    }
}

fn parse_class(value: &str) -> Result<BackupDataClass, BackupPackageStoreError> {
    BackupDataClass::ALL
        .into_iter()
        .find(|class| class_name(*class) == value)
        .ok_or(BackupPackageStoreError::CorruptedPackage)
}

fn ownership_name(ownership: BackupDataOwnership) -> &'static str {
    match ownership {
        BackupDataOwnership::Authoritative => "authoritative",
        BackupDataOwnership::Rebuildable => "rebuildable",
    }
}

fn parse_ownership(value: &str) -> Result<BackupDataOwnership, BackupPackageStoreError> {
    match value {
        "authoritative" => Ok(BackupDataOwnership::Authoritative),
        "rebuildable" => Ok(BackupDataOwnership::Rebuildable),
        _ => Err(BackupPackageStoreError::CorruptedPackage),
    }
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), BackupPackageStoreError> {
    let mut file = File::create(path).map_err(|_| BackupPackageStoreError::StorageUnavailable)?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| BackupPackageStoreError::StorageUnavailable)
}

fn sync_directory(path: &Path) -> Result<(), BackupPackageStoreError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| BackupPackageStoreError::StorageUnavailable)
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
