use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicWriteState {
    Prepared,
    WritingTemp,
    Syncing,
    Replacing,
    Completed,
    Failed,
    Recovering,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtomicWriteOutcome {
    final_state: AtomicWriteState,
}

impl AtomicWriteOutcome {
    pub fn final_state(self) -> AtomicWriteState {
        self.final_state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtomicRecoveryOutcome {
    final_state: AtomicWriteState,
    removed_temp: bool,
}

impl AtomicRecoveryOutcome {
    pub fn final_state(self) -> AtomicWriteState {
        self.final_state
    }

    pub fn removed_temp(self) -> bool {
        self.removed_temp
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicWriteError {
    PrepareFailed,
    WriteFailed,
    SyncFailed,
    ReplaceFailed,
    RecoveryFailed,
}

impl AtomicWriteError {
    pub fn failed_state(self) -> AtomicWriteState {
        AtomicWriteState::Failed
    }
}

pub fn write_text_atomically(
    path: &Path,
    content: impl AsRef<str>,
) -> Result<AtomicWriteOutcome, AtomicWriteError> {
    write_bytes_atomically(path, content.as_ref().as_bytes())
}

pub fn write_bytes_atomically(
    path: &Path,
    bytes: &[u8],
) -> Result<AtomicWriteOutcome, AtomicWriteError> {
    let parent = path.parent().ok_or(AtomicWriteError::PrepareFailed)?;
    fs::create_dir_all(parent).map_err(|_| AtomicWriteError::PrepareFailed)?;

    let temp_path = atomic_temp_path(path).ok_or(AtomicWriteError::PrepareFailed)?;
    let mut file = File::create(&temp_path).map_err(|_| AtomicWriteError::WriteFailed)?;
    file.write_all(bytes)
        .map_err(|_| AtomicWriteError::WriteFailed)?;
    file.sync_all().map_err(|_| AtomicWriteError::SyncFailed)?;
    drop(file);

    fs::rename(&temp_path, path).map_err(|_| {
        let _ = fs::remove_file(&temp_path);
        AtomicWriteError::ReplaceFailed
    })?;
    Ok(AtomicWriteOutcome {
        final_state: AtomicWriteState::Completed,
    })
}

pub fn recover_stale_temp(path: &Path) -> Result<AtomicRecoveryOutcome, AtomicWriteError> {
    let temp_path = atomic_temp_path(path).ok_or(AtomicWriteError::RecoveryFailed)?;
    if temp_path.exists() {
        fs::remove_file(temp_path).map_err(|_| AtomicWriteError::RecoveryFailed)?;
        return Ok(AtomicRecoveryOutcome {
            final_state: AtomicWriteState::Completed,
            removed_temp: true,
        });
    }

    Ok(AtomicRecoveryOutcome {
        final_state: AtomicWriteState::Completed,
        removed_temp: false,
    })
}

pub fn atomic_temp_path(path: &Path) -> Option<PathBuf> {
    let mut file_name: OsString = path.file_name()?.to_os_string();
    file_name.push(".tmp");
    Some(path.with_file_name(file_name))
}
