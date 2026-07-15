use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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

    let (temp_path, mut file) = create_unique_temp_file(path)?;
    if file.write_all(bytes).is_err() {
        drop(file);
        let _ = fs::remove_file(&temp_path);
        return Err(AtomicWriteError::WriteFailed);
    }
    if file.sync_all().is_err() {
        drop(file);
        let _ = fs::remove_file(&temp_path);
        return Err(AtomicWriteError::SyncFailed);
    }
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
    let mut removed_temp = false;
    if temp_path.exists() {
        fs::remove_file(&temp_path).map_err(|_| AtomicWriteError::RecoveryFailed)?;
        removed_temp = true;
    }
    let parent = path.parent().ok_or(AtomicWriteError::RecoveryFailed)?;
    let prefix = unique_temp_prefix(path).ok_or(AtomicWriteError::RecoveryFailed)?;
    match fs::read_dir(parent) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.map_err(|_| AtomicWriteError::RecoveryFailed)?;
                if entry.file_name().to_string_lossy().starts_with(&prefix) {
                    fs::remove_file(entry.path()).map_err(|_| AtomicWriteError::RecoveryFailed)?;
                    removed_temp = true;
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(AtomicWriteError::RecoveryFailed),
    }

    Ok(AtomicRecoveryOutcome {
        final_state: AtomicWriteState::Completed,
        removed_temp,
    })
}

pub fn atomic_temp_path(path: &Path) -> Option<PathBuf> {
    let mut file_name: OsString = path.file_name()?.to_os_string();
    file_name.push(".tmp");
    Some(path.with_file_name(file_name))
}

fn create_unique_temp_file(path: &Path) -> Result<(PathBuf, File), AtomicWriteError> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AtomicWriteError::PrepareFailed)?
        .as_nanos();
    let prefix = unique_temp_prefix(path).ok_or(AtomicWriteError::PrepareFailed)?;
    for attempt in 0..8 {
        let candidate = path.with_file_name(format!(
            "{prefix}{}.{}.{}",
            std::process::id(),
            nonce,
            attempt
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(AtomicWriteError::WriteFailed),
        }
    }
    Err(AtomicWriteError::WriteFailed)
}

fn unique_temp_prefix(path: &Path) -> Option<String> {
    Some(format!("{}.tmp.", path.file_name()?.to_string_lossy()))
}
