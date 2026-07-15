use std::fs::{self, File};
use std::path::{Path, PathBuf};

const MARKER: &str = "phase-upgrade.tsv";
const MARKER_CONTENT: &str = "schema\t1\nsource_phase\t11\ntarget_phase\t12\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phase011UpgradePolicy {
    max_file_count: u64,
    max_total_bytes: u64,
}

impl Phase011UpgradePolicy {
    pub const fn new(
        max_file_count: u64,
        max_total_bytes: u64,
    ) -> Result<Self, Phase011UpgradeError> {
        if max_file_count == 0 || max_total_bytes == 0 {
            return Err(Phase011UpgradeError::InvalidPolicy);
        }
        Ok(Self {
            max_file_count,
            max_total_bytes,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase011UpgradeOutcome {
    Migrated,
    AlreadyCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase011UpgradeError {
    InvalidPolicy,
    UnsafeSource,
    SourceMissing,
    DestinationConflict,
    CorruptedDestination,
    UnsupportedFutureSchema,
    StorageUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phase011UpgradeMigrator {
    policy: Phase011UpgradePolicy,
}

impl Phase011UpgradeMigrator {
    pub const fn new(policy: Phase011UpgradePolicy) -> Self {
        Self { policy }
    }

    pub fn migrate(
        &self,
        source: &Path,
        destination: &Path,
    ) -> Result<Phase011UpgradeOutcome, Phase011UpgradeError> {
        if source == destination {
            return Err(Phase011UpgradeError::DestinationConflict);
        }
        let source_metadata = fs::symlink_metadata(source).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Phase011UpgradeError::SourceMissing
            } else {
                Phase011UpgradeError::StorageUnavailable
            }
        })?;
        if source_metadata.file_type().is_symlink() || !source_metadata.is_dir() {
            return Err(Phase011UpgradeError::UnsafeSource);
        }
        if destination.exists() {
            return match fs::read_to_string(destination.join(MARKER)) {
                Ok(value) if value == MARKER_CONTENT => Ok(Phase011UpgradeOutcome::AlreadyCurrent),
                _ => Err(Phase011UpgradeError::CorruptedDestination),
            };
        }
        let preparing = preparing_path(destination)?;
        if preparing.exists() {
            fs::remove_dir_all(&preparing).map_err(|_| Phase011UpgradeError::StorageUnavailable)?;
        }
        fs::create_dir_all(&preparing).map_err(|_| Phase011UpgradeError::StorageUnavailable)?;
        let result = self.copy_and_validate(source, &preparing);
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&preparing);
            return Err(error);
        }
        write_synced(&preparing.join(MARKER), MARKER_CONTENT.as_bytes())?;
        fs::rename(&preparing, destination).map_err(|_| {
            let _ = fs::remove_dir_all(&preparing);
            Phase011UpgradeError::StorageUnavailable
        })?;
        sync_directory(
            destination
                .parent()
                .ok_or(Phase011UpgradeError::DestinationConflict)?,
        )?;
        Ok(Phase011UpgradeOutcome::Migrated)
    }

    fn copy_and_validate(
        &self,
        source: &Path,
        destination: &Path,
    ) -> Result<(), Phase011UpgradeError> {
        for required in [
            "authoring-current",
            "authoring-versions",
            "canvases",
            "assets",
        ] {
            if !source.join(required).is_dir() {
                return Err(Phase011UpgradeError::UnsafeSource);
            }
        }
        let mut pending = vec![(source.to_path_buf(), destination.to_path_buf())];
        let mut files = 0_u64;
        let mut bytes = 0_u64;
        while let Some((from, to)) = pending.pop() {
            fs::create_dir_all(&to).map_err(|_| Phase011UpgradeError::StorageUnavailable)?;
            for entry in
                fs::read_dir(&from).map_err(|_| Phase011UpgradeError::StorageUnavailable)?
            {
                let source_path = entry
                    .map_err(|_| Phase011UpgradeError::StorageUnavailable)?
                    .path();
                let metadata = fs::symlink_metadata(&source_path)
                    .map_err(|_| Phase011UpgradeError::StorageUnavailable)?;
                if metadata.file_type().is_symlink() {
                    return Err(Phase011UpgradeError::UnsafeSource);
                }
                let target = to.join(
                    source_path
                        .file_name()
                        .ok_or(Phase011UpgradeError::UnsafeSource)?,
                );
                if metadata.is_dir() {
                    pending.push((source_path, target));
                } else if metadata.is_file() {
                    files = files
                        .checked_add(1)
                        .ok_or(Phase011UpgradeError::UnsafeSource)?;
                    bytes = bytes
                        .checked_add(metadata.len())
                        .ok_or(Phase011UpgradeError::UnsafeSource)?;
                    if files > self.policy.max_file_count || bytes > self.policy.max_total_bytes {
                        return Err(Phase011UpgradeError::UnsafeSource);
                    }
                    validate_schema(&source_path)?;
                    fs::copy(source_path, &target)
                        .map_err(|_| Phase011UpgradeError::StorageUnavailable)?;
                    File::open(target)
                        .and_then(|file| file.sync_all())
                        .map_err(|_| Phase011UpgradeError::StorageUnavailable)?;
                } else {
                    return Err(Phase011UpgradeError::UnsafeSource);
                }
            }
        }
        Ok(())
    }
}

fn validate_schema(path: &Path) -> Result<(), Phase011UpgradeError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if !matches!(extension, "canvas" | "asset" | "link" | "snapshot") {
        return Ok(());
    }
    let bytes = fs::read(path).map_err(|_| Phase011UpgradeError::StorageUnavailable)?;
    let first = bytes
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    if !first.starts_with(b"schema\t") {
        return Err(Phase011UpgradeError::UnsupportedFutureSchema);
    }
    let version = std::str::from_utf8(&first[7..])
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or(Phase011UpgradeError::UnsupportedFutureSchema)?;
    if version > 2 {
        return Err(Phase011UpgradeError::UnsupportedFutureSchema);
    }
    Ok(())
}

fn preparing_path(destination: &Path) -> Result<PathBuf, Phase011UpgradeError> {
    let parent = destination
        .parent()
        .ok_or(Phase011UpgradeError::DestinationConflict)?;
    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(Phase011UpgradeError::DestinationConflict)?;
    Ok(parent.join(format!(".{name}.preparing")))
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), Phase011UpgradeError> {
    fs::write(path, bytes).map_err(|_| Phase011UpgradeError::StorageUnavailable)?;
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| Phase011UpgradeError::StorageUnavailable)
}

fn sync_directory(path: &Path) -> Result<(), Phase011UpgradeError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| Phase011UpgradeError::StorageUnavailable)
}
