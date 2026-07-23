use std::fs::{self, File};
use std::path::{Path, PathBuf};

use cabinet_domain::backup::{BackupDataClass, BackupJobId, BackupPackageManifest, RestoreState};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_package::{BackupPackageStore, BackupPackageStoreError};
use cabinet_ports::backup_restore::{
    BackupRecoveryReport, BackupRecoveryStore, BackupRestoreOperationSnapshot, BackupRestoreStore,
    BackupRestoreStoreError,
};

use crate::durable_backup_package_store::{
    CURRENT_VERSION_POINTERS_PAYLOAD_DIR, LocalBackupPackagePolicy, LocalBackupPackageStore,
    class_name, validate_payload_against_manifest,
};
use crate::local_atomic_file::write_text_atomically;
use crate::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT,
};

const STATUS_FILE: &str = "status.tsv";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalBackupRestoreStore {
    app_data_root: PathBuf,
    package_policy: LocalBackupPackagePolicy,
}

impl LocalBackupRestoreStore {
    pub fn new(app_data_root: PathBuf, package_policy: LocalBackupPackagePolicy) -> Self {
        Self {
            app_data_root,
            package_policy,
        }
    }

    fn operations_root(&self, workspace: &WorkspaceId) -> PathBuf {
        self.app_data_root
            .join("restore-operations")
            .join(hex(workspace.as_str()))
    }

    fn operation_root(&self, workspace: &WorkspaceId, operation: &BackupJobId) -> PathBuf {
        self.operations_root(workspace)
            .join(hex(operation.as_str()))
    }

    fn preparing_root(&self, workspace: &WorkspaceId, operation: &BackupJobId) -> PathBuf {
        self.operations_root(workspace)
            .join(format!(".{}.preparing", hex(operation.as_str())))
    }

    fn write_status(
        &self,
        root: &Path,
        snapshot: &BackupRestoreOperationSnapshot,
    ) -> Result<(), BackupRestoreStoreError> {
        write_text_atomically(
            &root.join(STATUS_FILE),
            format!(
                "schema\t1\nworkspace\t{}\npackage\t{}\noperation\t{}\nstate\t{}\n",
                hex(snapshot.workspace_id().as_str()),
                hex(snapshot.package_id().as_str()),
                hex(snapshot.operation_id().as_str()),
                state_name(snapshot.state()),
            ),
        )
        .map(|_| ())
        .map_err(|_| BackupRestoreStoreError::StorageUnavailable)
    }

    fn target_slots(&self, workspace: &WorkspaceId) -> Vec<TargetSlot> {
        let document_workspace = encode_document_segment(workspace.as_str());
        let workspace = hex(workspace.as_str());
        vec![
            TargetSlot::replacement_from(
                "current_version_pointers",
                format!(
                    "{}/{}",
                    class_name(BackupDataClass::CurrentDocuments),
                    CURRENT_VERSION_POINTERS_PAYLOAD_DIR
                ),
                self.app_data_root
                    .join(LOCAL_DOCUMENT_POINTER_ROOT)
                    .join(&workspace),
            ),
            TargetSlot::replacement(
                class_name(BackupDataClass::CurrentDocuments),
                self.app_data_root
                    .join("authoring-current")
                    .join(&document_workspace),
            ),
            TargetSlot::replacement(
                class_name(BackupDataClass::VersionHistory),
                self.app_data_root
                    .join(LOCAL_DOCUMENT_VERSION_ROOT)
                    .join(&document_workspace),
            ),
            TargetSlot::replacement(
                class_name(BackupDataClass::CanvasRecords),
                self.app_data_root.join("canvases").join(&workspace),
            ),
            TargetSlot::replacement(
                class_name(BackupDataClass::AssetMetadata),
                self.app_data_root.join("assets/metadata").join(&workspace),
            ),
            TargetSlot::replacement(
                class_name(BackupDataClass::AssetObjects),
                self.app_data_root.join("assets/objects").join(&workspace),
            ),
            TargetSlot::replacement(
                class_name(BackupDataClass::AssetAssociations),
                self.app_data_root
                    .join("assets/associations")
                    .join(&workspace),
            ),
            TargetSlot::remove_only(
                "graph_projection",
                self.app_data_root
                    .join("graph-projections")
                    .join(&workspace),
            ),
            TargetSlot::remove_only(
                "search_projection",
                self.app_data_root
                    .join("search-projections")
                    .join(format!("{workspace}.snapshot")),
            ),
        ]
    }

    fn load_required(
        &self,
        workspace: &WorkspaceId,
        operation: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        self.get_restore_status(workspace, operation)?
            .ok_or(BackupRestoreStoreError::OperationNotFound)
    }
}

impl BackupRestoreStore for LocalBackupRestoreStore {
    fn request_restore(
        &mut self,
        workspace_id: &WorkspaceId,
        package_id: &BackupJobId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        let root = self.operation_root(workspace_id, operation_id);
        let requesting = self
            .operations_root(workspace_id)
            .join(format!(".{}.requesting", hex(operation_id.as_str())));
        if root.exists() || requesting.exists() {
            return Err(BackupRestoreStoreError::Conflict);
        }
        fs::create_dir_all(&requesting).map_err(|_| BackupRestoreStoreError::StorageUnavailable)?;
        let snapshot = BackupRestoreOperationSnapshot::new(
            workspace_id.clone(),
            package_id.clone(),
            operation_id.clone(),
            RestoreState::Staging,
        );
        if let Err(error) = self.write_status(&requesting, &snapshot) {
            let _ = fs::remove_dir_all(&requesting);
            return Err(error);
        }
        fs::rename(&requesting, &root).map_err(|_| BackupRestoreStoreError::StorageUnavailable)?;
        sync_directory(
            root.parent()
                .ok_or(BackupRestoreStoreError::StorageUnavailable)?,
        )?;
        Ok(snapshot)
    }

    fn prepare_restore(
        &mut self,
        workspace_id: &WorkspaceId,
        package_id: &BackupJobId,
        operation_id: &BackupJobId,
        manifest: &BackupPackageManifest,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        let root = self.operation_root(workspace_id, operation_id);
        let preparing = self.preparing_root(workspace_id, operation_id);
        if root.exists() {
            let snapshot = self.load_required(workspace_id, operation_id)?;
            if snapshot.state() != RestoreState::Staging
                || snapshot.package_id() != package_id
                || root.join("staged").exists()
            {
                return Err(BackupRestoreStoreError::Conflict);
            }
            let staged_payload = root.join("staged/data");
            fs::create_dir_all(&staged_payload)
                .map_err(|_| BackupRestoreStoreError::StorageUnavailable)?;
            let mut packages =
                LocalBackupPackageStore::new(self.app_data_root.clone(), self.package_policy);
            let inspected = packages
                .inspect_manifest(workspace_id, package_id)
                .map_err(map_package_error)?;
            if inspected != *manifest
                || !packages
                    .validate_package(workspace_id, package_id, manifest)
                    .map_err(map_package_error)?
                    .is_valid()
            {
                let _ = fs::remove_dir_all(root.join("staged"));
                return Err(BackupRestoreStoreError::PackageInvalid);
            }
            if let Err(error) = copy_directory(
                &packages.package_root(workspace_id, package_id).join("data"),
                &staged_payload,
                self.package_policy.max_file_count(),
                self.package_policy.max_total_bytes(),
            ) {
                let _ = fs::remove_dir_all(root.join("staged"));
                return Err(error);
            }
            if !validate_payload_against_manifest(&staged_payload, manifest)
                .map_err(map_package_error)?
                .is_valid()
            {
                let _ = fs::remove_dir_all(root.join("staged"));
                return Err(BackupRestoreStoreError::PackageInvalid);
            }
            return Ok(snapshot);
        }
        if preparing.exists() {
            return Err(BackupRestoreStoreError::Conflict);
        }
        let mut packages =
            LocalBackupPackageStore::new(self.app_data_root.clone(), self.package_policy);
        let inspected = packages
            .inspect_manifest(workspace_id, package_id)
            .map_err(map_package_error)?;
        if inspected != *manifest
            || !packages
                .validate_package(workspace_id, package_id, manifest)
                .map_err(map_package_error)?
                .is_valid()
        {
            return Err(BackupRestoreStoreError::PackageInvalid);
        }
        fs::create_dir_all(preparing.join("staged/data"))
            .map_err(|_| BackupRestoreStoreError::StorageUnavailable)?;
        let package_payload = packages.package_root(workspace_id, package_id).join("data");
        let staged_payload = preparing.join("staged/data");
        if let Err(error) = copy_directory(
            &package_payload,
            &staged_payload,
            self.package_policy.max_file_count(),
            self.package_policy.max_total_bytes(),
        ) {
            let _ = fs::remove_dir_all(&preparing);
            return Err(error);
        }
        if !validate_payload_against_manifest(&staged_payload, manifest)
            .map_err(map_package_error)?
            .is_valid()
        {
            let _ = fs::remove_dir_all(&preparing);
            return Err(BackupRestoreStoreError::PackageInvalid);
        }
        let snapshot = BackupRestoreOperationSnapshot::new(
            workspace_id.clone(),
            package_id.clone(),
            operation_id.clone(),
            RestoreState::Staging,
        );
        self.write_status(&preparing, &snapshot)?;
        fs::rename(&preparing, &root).map_err(|_| BackupRestoreStoreError::StorageUnavailable)?;
        sync_directory(
            root.parent()
                .ok_or(BackupRestoreStoreError::StorageUnavailable)?,
        )?;
        Ok(snapshot)
    }

    fn apply_restore(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        let snapshot = self.load_required(workspace_id, operation_id)?;
        if snapshot.state() != RestoreState::Staging {
            return Err(BackupRestoreStoreError::Conflict);
        }
        let root = self.operation_root(workspace_id, operation_id);
        let applying = snapshot.with_state(RestoreState::Applying);
        self.write_status(&root, &applying)?;
        let slots = self.target_slots(workspace_id);
        for slot in &slots {
            if slot.replacement && !root.join("staged/data").join(&slot.staged_key).is_dir() {
                return Err(BackupRestoreStoreError::CorruptedOperation);
            }
        }
        for slot in &slots {
            let rollback = root.join("rollback").join(slot.key);
            let rollback_expected = slot.target.exists();
            write_slot_marker(&root, slot, rollback_expected, SlotMarkerPhase::Intent)?;
            if rollback_expected {
                create_parent(&rollback)?;
                fs::rename(&slot.target, &rollback)
                    .map_err(|_| BackupRestoreStoreError::StorageUnavailable)?;
            }
            if slot.replacement {
                let staged = root.join("staged/data").join(&slot.staged_key);
                create_parent(&slot.target)?;
                if fs::rename(&staged, &slot.target).is_err() {
                    let _ = restore_slots(&root, &slots);
                    let rolled_back = snapshot.with_state(RestoreState::RolledBack);
                    let _ = self.write_status(&root, &rolled_back);
                    return Err(BackupRestoreStoreError::StorageUnavailable);
                }
            }
            write_slot_marker(&root, slot, rollback_expected, SlotMarkerPhase::Applied)?;
        }
        let applied = snapshot.with_state(RestoreState::Reopening);
        self.write_status(&root, &applied)?;
        Ok(applied)
    }

    fn rollback_restore(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        let snapshot = self.load_required(workspace_id, operation_id)?;
        if snapshot.state() != RestoreState::Reopening {
            return Err(BackupRestoreStoreError::Conflict);
        }
        let root = self.operation_root(workspace_id, operation_id);
        self.write_status(&root, &snapshot.with_state(RestoreState::RollbackRequired))?;
        restore_slots(&root, &self.target_slots(workspace_id))?;
        let rolled_back = snapshot.with_state(RestoreState::RolledBack);
        self.write_status(&root, &rolled_back)?;
        Ok(rolled_back)
    }

    fn finalize_restore(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        let snapshot = self.load_required(workspace_id, operation_id)?;
        if snapshot.state() != RestoreState::Reopening {
            return Err(BackupRestoreStoreError::Conflict);
        }
        let root = self.operation_root(workspace_id, operation_id);
        remove_path(&root.join("rollback"))?;
        remove_path(&root.join("staged"))?;
        let completed = snapshot.with_state(RestoreState::Completed);
        self.write_status(&root, &completed)?;
        Ok(completed)
    }

    fn cancel_restore(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        let snapshot = self.load_required(workspace_id, operation_id)?;
        if snapshot.state() != RestoreState::Staging {
            return Err(BackupRestoreStoreError::Conflict);
        }
        let root = self.operation_root(workspace_id, operation_id);
        remove_path(&root.join("staged"))?;
        let cancelled = snapshot.with_state(RestoreState::Cancelled);
        self.write_status(&root, &cancelled)?;
        Ok(cancelled)
    }

    fn get_restore_status(
        &self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<Option<BackupRestoreOperationSnapshot>, BackupRestoreStoreError> {
        let path = self
            .operation_root(workspace_id, operation_id)
            .join(STATUS_FILE);
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(BackupRestoreStoreError::StorageUnavailable),
        };
        let snapshot = decode_status(&text)?;
        if snapshot.workspace_id() != workspace_id || snapshot.operation_id() != operation_id {
            return Err(BackupRestoreStoreError::CorruptedOperation);
        }
        Ok(Some(snapshot))
    }

    fn mark_cleanup_required(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        let snapshot = self.load_required(workspace_id, operation_id)?;
        if snapshot.state() != RestoreState::Completed {
            return Err(BackupRestoreStoreError::Conflict);
        }
        let required = snapshot.with_state(RestoreState::CleanupRequired);
        self.write_status(&self.operation_root(workspace_id, operation_id), &required)?;
        Ok(required)
    }

    fn mark_recovery_required(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &BackupJobId,
    ) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
        let snapshot = self.load_required(workspace_id, operation_id)?;
        if !matches!(
            snapshot.state(),
            RestoreState::Reopening
                | RestoreState::RollbackRequired
                | RestoreState::RecoveryRequired
        ) {
            return Err(BackupRestoreStoreError::Conflict);
        }
        let required = snapshot.with_state(RestoreState::RecoveryRequired);
        self.write_status(&self.operation_root(workspace_id, operation_id), &required)?;
        Ok(required)
    }
}

impl BackupRecoveryStore for LocalBackupRestoreStore {
    fn recover_startup(
        &mut self,
        workspace_id: &WorkspaceId,
    ) -> Result<BackupRecoveryReport, BackupRestoreStoreError> {
        let mut cleaned = clean_suffix_directories(
            &self
                .app_data_root
                .join("backup-packages")
                .join(hex(workspace_id.as_str())),
            ".staging",
        )?;
        let operations_root = self.operations_root(workspace_id);
        cleaned += clean_suffix_directories(&operations_root, ".preparing")?;
        cleaned += clean_suffix_directories(&operations_root, ".requesting")?;
        let mut rolled_back = Vec::new();
        let mut cleanup_required = Vec::new();
        let entries = match fs::read_dir(&operations_root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(BackupRecoveryReport::new(cleaned, vec![], vec![]));
            }
            Err(_) => return Err(BackupRestoreStoreError::StorageUnavailable),
        };
        let mut operation_ids = entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let name = entry.file_name().to_str()?.to_string();
                (!name.starts_with('.')).then_some(name)
            })
            .collect::<Vec<_>>();
        operation_ids.sort();
        for encoded in operation_ids {
            let operation_value = match decode_hex(&encoded) {
                Ok(value) => value,
                Err(_) => {
                    cleanup_required.push("invalid-operation-id".to_string());
                    continue;
                }
            };
            let operation = match BackupJobId::new(&operation_value) {
                Ok(value) => value,
                Err(_) => {
                    cleanup_required.push("invalid-operation-id".to_string());
                    continue;
                }
            };
            let snapshot = match self.get_restore_status(workspace_id, &operation) {
                Ok(Some(value)) => value,
                Ok(None) | Err(_) => {
                    cleanup_required.push(operation_value);
                    continue;
                }
            };
            if matches!(
                snapshot.state(),
                RestoreState::Applying
                    | RestoreState::Reopening
                    | RestoreState::RollbackRequired
                    | RestoreState::RecoveryRequired
            ) {
                let root = self.operation_root(workspace_id, &operation);
                if restore_slots(&root, &self.target_slots(workspace_id)).is_err() {
                    cleanup_required.push(operation_value);
                    continue;
                }
                let recovered = snapshot.with_state(RestoreState::RolledBack);
                if self.write_status(&root, &recovered).is_err() {
                    cleanup_required.push(operation_value);
                    continue;
                }
                rolled_back.push(operation_value);
            }
        }
        Ok(BackupRecoveryReport::new(
            cleaned,
            rolled_back,
            cleanup_required,
        ))
    }
}

struct TargetSlot {
    key: &'static str,
    staged_key: PathBuf,
    target: PathBuf,
    replacement: bool,
}

impl TargetSlot {
    fn replacement(key: &'static str, target: PathBuf) -> Self {
        Self {
            key,
            staged_key: PathBuf::from(key),
            target,
            replacement: true,
        }
    }

    fn replacement_from(key: &'static str, staged_key: String, target: PathBuf) -> Self {
        Self {
            key,
            staged_key: PathBuf::from(staged_key),
            target,
            replacement: true,
        }
    }

    fn remove_only(key: &'static str, target: PathBuf) -> Self {
        Self {
            key,
            staged_key: PathBuf::from(key),
            target,
            replacement: false,
        }
    }
}

fn restore_slots(root: &Path, slots: &[TargetSlot]) -> Result<(), BackupRestoreStoreError> {
    let markers = validate_restore_slots(root, slots)?;
    for (slot, marker) in slots.iter().zip(markers).rev() {
        let Some(marker) = marker else {
            continue;
        };
        let rollback = root.join("rollback").join(slot.key);
        remove_path(&slot.target)?;
        if marker.rollback_expected {
            create_parent(&slot.target)?;
            fs::rename(rollback, &slot.target)
                .map_err(|_| BackupRestoreStoreError::StorageUnavailable)?;
        }
        remove_path(&slot_marker(root, slot))?;
    }
    Ok(())
}

fn validate_restore_slots(
    root: &Path,
    slots: &[TargetSlot],
) -> Result<Vec<Option<SlotMarker>>, BackupRestoreStoreError> {
    let mut markers = Vec::with_capacity(slots.len());
    for slot in slots {
        let marker_path = slot_marker(root, slot);
        let marker = match fs::symlink_metadata(&marker_path) {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                let text = fs::read_to_string(&marker_path)
                    .map_err(|_| BackupRestoreStoreError::CorruptedOperation)?;
                Some(decode_slot_marker(&text)?)
            }
            Ok(_) => return Err(BackupRestoreStoreError::CorruptedOperation),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(BackupRestoreStoreError::StorageUnavailable),
        };
        let rollback = root.join("rollback").join(slot.key);
        if let Some(marker) = marker {
            match fs::symlink_metadata(&rollback) {
                Ok(metadata)
                    if marker.rollback_expected
                        && !metadata.file_type().is_symlink()
                        && (metadata.is_dir() || metadata.is_file()) => {}
                Ok(_) => return Err(BackupRestoreStoreError::CorruptedOperation),
                Err(error)
                    if error.kind() == std::io::ErrorKind::NotFound
                        && !marker.rollback_expected => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Err(BackupRestoreStoreError::CorruptedOperation);
                }
                Err(_) => return Err(BackupRestoreStoreError::StorageUnavailable),
            }
        } else {
            match fs::symlink_metadata(&rollback) {
                Ok(_) => return Err(BackupRestoreStoreError::CorruptedOperation),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(BackupRestoreStoreError::StorageUnavailable),
            }
        }
        markers.push(marker);
    }
    Ok(markers)
}

#[derive(Clone, Copy)]
struct SlotMarker {
    rollback_expected: bool,
    _phase: SlotMarkerPhase,
}

#[derive(Clone, Copy)]
enum SlotMarkerPhase {
    Intent,
    Applied,
}

fn write_slot_marker(
    root: &Path,
    slot: &TargetSlot,
    rollback_expected: bool,
    phase: SlotMarkerPhase,
) -> Result<(), BackupRestoreStoreError> {
    let phase = match phase {
        SlotMarkerPhase::Intent => "intent",
        SlotMarkerPhase::Applied => "applied",
    };
    write_text_atomically(
        &slot_marker(root, slot),
        format!(
            "schema\t1\nrollback_expected\t{}\nphase\t{phase}\n",
            u8::from(rollback_expected)
        ),
    )
    .map(|_| ())
    .map_err(|_| BackupRestoreStoreError::StorageUnavailable)
}

fn decode_slot_marker(text: &str) -> Result<SlotMarker, BackupRestoreStoreError> {
    let mut schema = None;
    let mut rollback_expected = None;
    let mut phase = None;
    for line in text.lines() {
        let (key, value) = line
            .split_once('\t')
            .ok_or(BackupRestoreStoreError::CorruptedOperation)?;
        match key {
            "schema" if schema.replace(value).is_none() => {}
            "rollback_expected" if rollback_expected.replace(value).is_none() => {}
            "phase" if phase.replace(value).is_none() => {}
            _ => return Err(BackupRestoreStoreError::CorruptedOperation),
        }
    }
    if schema != Some("1") {
        return Err(BackupRestoreStoreError::CorruptedOperation);
    }
    let rollback_expected = match rollback_expected {
        Some("0") => false,
        Some("1") => true,
        _ => return Err(BackupRestoreStoreError::CorruptedOperation),
    };
    let phase = match phase {
        Some("intent") => SlotMarkerPhase::Intent,
        Some("applied") => SlotMarkerPhase::Applied,
        _ => return Err(BackupRestoreStoreError::CorruptedOperation),
    };
    Ok(SlotMarker {
        rollback_expected,
        _phase: phase,
    })
}

fn slot_marker(root: &Path, slot: &TargetSlot) -> PathBuf {
    root.join("journal").join(format!("{}.applied", slot.key))
}

fn clean_suffix_directories(root: &Path, suffix: &str) -> Result<u64, BackupRestoreStoreError> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(_) => return Err(BackupRestoreStoreError::StorageUnavailable),
    };
    let mut count = 0;
    for entry in entries {
        let path = entry
            .map_err(|_| BackupRestoreStoreError::StorageUnavailable)?
            .path();
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or(BackupRestoreStoreError::CorruptedOperation)?;
        if name.ends_with(suffix) {
            let metadata = fs::symlink_metadata(&path)
                .map_err(|_| BackupRestoreStoreError::StorageUnavailable)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(BackupRestoreStoreError::CorruptedOperation);
            }
            fs::remove_dir_all(path).map_err(|_| BackupRestoreStoreError::StorageUnavailable)?;
            count += 1;
        }
    }
    Ok(count)
}

fn copy_directory(
    source: &Path,
    destination: &Path,
    max_files: u64,
    max_bytes: u64,
) -> Result<(), BackupRestoreStoreError> {
    let mut pending = vec![(source.to_path_buf(), destination.to_path_buf())];
    let mut files = 0_u64;
    let mut bytes = 0_u64;
    while let Some((from, to)) = pending.pop() {
        let metadata =
            fs::symlink_metadata(&from).map_err(|_| BackupRestoreStoreError::PackageInvalid)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(BackupRestoreStoreError::PackageInvalid);
        }
        fs::create_dir_all(&to).map_err(|_| BackupRestoreStoreError::StorageUnavailable)?;
        for entry in fs::read_dir(from).map_err(|_| BackupRestoreStoreError::StorageUnavailable)? {
            let source_path = entry
                .map_err(|_| BackupRestoreStoreError::StorageUnavailable)?
                .path();
            let metadata = fs::symlink_metadata(&source_path)
                .map_err(|_| BackupRestoreStoreError::StorageUnavailable)?;
            if metadata.file_type().is_symlink() {
                return Err(BackupRestoreStoreError::PackageInvalid);
            }
            let target = to.join(
                source_path
                    .file_name()
                    .ok_or(BackupRestoreStoreError::PackageInvalid)?,
            );
            if metadata.is_dir() {
                pending.push((source_path, target));
            } else if metadata.is_file() {
                files = files
                    .checked_add(1)
                    .ok_or(BackupRestoreStoreError::PackageInvalid)?;
                bytes = bytes
                    .checked_add(metadata.len())
                    .ok_or(BackupRestoreStoreError::PackageInvalid)?;
                if files > max_files || bytes > max_bytes {
                    return Err(BackupRestoreStoreError::PackageInvalid);
                }
                fs::copy(source_path, &target)
                    .map_err(|_| BackupRestoreStoreError::StorageUnavailable)?;
                File::open(target)
                    .and_then(|file| file.sync_all())
                    .map_err(|_| BackupRestoreStoreError::StorageUnavailable)?;
            } else {
                return Err(BackupRestoreStoreError::PackageInvalid);
            }
        }
    }
    Ok(())
}

fn decode_status(text: &str) -> Result<BackupRestoreOperationSnapshot, BackupRestoreStoreError> {
    let mut workspace = None;
    let mut package = None;
    let mut operation = None;
    let mut state = None;
    for line in text.lines() {
        let (key, value) = line
            .split_once('\t')
            .ok_or(BackupRestoreStoreError::CorruptedOperation)?;
        match key {
            "schema" if value == "1" => {}
            "workspace" => workspace = Some(decode_hex(value)?),
            "package" => package = Some(decode_hex(value)?),
            "operation" => operation = Some(decode_hex(value)?),
            "state" => state = Some(parse_state(value)?),
            _ => return Err(BackupRestoreStoreError::CorruptedOperation),
        }
    }
    Ok(BackupRestoreOperationSnapshot::new(
        WorkspaceId::new(&workspace.ok_or(BackupRestoreStoreError::CorruptedOperation)?)
            .map_err(|_| BackupRestoreStoreError::CorruptedOperation)?,
        BackupJobId::new(&package.ok_or(BackupRestoreStoreError::CorruptedOperation)?)
            .map_err(|_| BackupRestoreStoreError::CorruptedOperation)?,
        BackupJobId::new(&operation.ok_or(BackupRestoreStoreError::CorruptedOperation)?)
            .map_err(|_| BackupRestoreStoreError::CorruptedOperation)?,
        state.ok_or(BackupRestoreStoreError::CorruptedOperation)?,
    ))
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

fn state_name(state: RestoreState) -> &'static str {
    match state {
        RestoreState::Staging => "staging",
        RestoreState::Applying => "applying",
        RestoreState::Reopening => "reopening",
        RestoreState::RollbackRequired => "rollback_required",
        RestoreState::RecoveryRequired => "recovery_required",
        RestoreState::RolledBack => "rolled_back",
        RestoreState::Completed => "completed",
        RestoreState::CleanupRequired => "cleanup_required",
        RestoreState::Cancelled => "cancelled",
        _ => "invalid",
    }
}

fn parse_state(value: &str) -> Result<RestoreState, BackupRestoreStoreError> {
    match value {
        "staging" => Ok(RestoreState::Staging),
        "applying" => Ok(RestoreState::Applying),
        "reopening" => Ok(RestoreState::Reopening),
        "rollback_required" => Ok(RestoreState::RollbackRequired),
        "recovery_required" => Ok(RestoreState::RecoveryRequired),
        "rolled_back" => Ok(RestoreState::RolledBack),
        "completed" => Ok(RestoreState::Completed),
        "cleanup_required" => Ok(RestoreState::CleanupRequired),
        "cancelled" => Ok(RestoreState::Cancelled),
        _ => Err(BackupRestoreStoreError::CorruptedOperation),
    }
}

fn map_package_error(error: BackupPackageStoreError) -> BackupRestoreStoreError {
    match error {
        BackupPackageStoreError::PackageNotFound | BackupPackageStoreError::CorruptedPackage => {
            BackupRestoreStoreError::PackageInvalid
        }
        BackupPackageStoreError::StorageUnavailable => BackupRestoreStoreError::StorageUnavailable,
        BackupPackageStoreError::Conflict => BackupRestoreStoreError::Conflict,
    }
}

fn create_parent(path: &Path) -> Result<(), BackupRestoreStoreError> {
    fs::create_dir_all(
        path.parent()
            .ok_or(BackupRestoreStoreError::StorageUnavailable)?,
    )
    .map_err(|_| BackupRestoreStoreError::StorageUnavailable)
}

fn remove_path(path: &Path) -> Result<(), BackupRestoreStoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path).map_err(|_| BackupRestoreStoreError::StorageUnavailable)
        }
        Ok(_) => fs::remove_file(path).map_err(|_| BackupRestoreStoreError::StorageUnavailable),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(BackupRestoreStoreError::StorageUnavailable),
    }
}

fn sync_directory(path: &Path) -> Result<(), BackupRestoreStoreError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| BackupRestoreStoreError::StorageUnavailable)
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode_hex(value: &str) -> Result<String, BackupRestoreStoreError> {
    if !value.len().is_multiple_of(2) {
        return Err(BackupRestoreStoreError::CorruptedOperation);
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| BackupRestoreStoreError::CorruptedOperation)?;
    String::from_utf8(bytes).map_err(|_| BackupRestoreStoreError::CorruptedOperation)
}
