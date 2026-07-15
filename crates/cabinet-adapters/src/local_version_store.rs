use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::{
    DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::version_store::{
    HistoryCursor, HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};

use crate::local_atomic_file::write_text_atomically;

pub const VERSION_DOCUMENTS_DIR: &str = "documents";
pub const VERSION_HISTORY_FILE: &str = "history.txt";
pub const VERSION_SNAPSHOTS_DIR: &str = "snapshots";
pub const VERSION_ENTRY_FILE: &str = "entry.txt";
pub const VERSION_BODY_FILE: &str = "body.md";
const DEFAULT_VERSION_BODY_MAX_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct LocalVersionStore {
    version_store_root: PathBuf,
    body_policy: DocumentBodyPolicy,
    clock: fn() -> u64,
}

impl LocalVersionStore {
    pub fn new(version_store_root: PathBuf) -> Self {
        Self {
            version_store_root,
            body_policy: DocumentBodyPolicy::new(DEFAULT_VERSION_BODY_MAX_BYTES)
                .expect("default version body policy must be valid"),
            clock: system_epoch_ms,
        }
    }

    pub fn with_body_policy(version_store_root: PathBuf, body_policy: DocumentBodyPolicy) -> Self {
        Self {
            version_store_root,
            body_policy,
            clock: system_epoch_ms,
        }
    }

    pub fn with_body_policy_and_clock(
        version_store_root: PathBuf,
        body_policy: DocumentBodyPolicy,
        clock: fn() -> u64,
    ) -> Self {
        Self {
            version_store_root,
            body_policy,
            clock,
        }
    }

    fn document_dir(&self, workspace_id: &WorkspaceId, document_id: &DocumentId) -> PathBuf {
        self.version_store_root
            .join(encode_path_segment(workspace_id.as_str()))
            .join(VERSION_DOCUMENTS_DIR)
            .join(encode_path_segment(document_id.as_str()))
    }

    fn history_path(&self, workspace_id: &WorkspaceId, document_id: &DocumentId) -> PathBuf {
        self.document_dir(workspace_id, document_id)
            .join(VERSION_HISTORY_FILE)
    }

    fn version_dir(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> PathBuf {
        self.document_dir(workspace_id, document_id)
            .join(VERSION_SNAPSHOTS_DIR)
            .join(encode_path_segment(version_id.as_str()))
    }

    fn entry_path(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> PathBuf {
        self.version_dir(workspace_id, document_id, version_id)
            .join(VERSION_ENTRY_FILE)
    }

    fn body_path(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> PathBuf {
        self.version_dir(workspace_id, document_id, version_id)
            .join(VERSION_BODY_FILE)
    }

    fn read_entry_by_version(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<VersionEntry, VersionStoreError> {
        let entry = read_entry(&self.entry_path(workspace_id, document_id, version_id))?;
        if entry.document_id() != document_id || entry.version_id() != version_id {
            return Err(VersionStoreError::CorruptedHistory);
        }
        Ok(entry)
    }
}

impl VersionStore for LocalVersionStore {
    fn append_version(
        &mut self,
        workspace_id: &WorkspaceId,
        record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        let version_dir = self.version_dir(workspace_id, record.document_id(), record.version_id());
        if version_dir.exists() {
            return Err(VersionStoreError::Conflict);
        }

        write_file_atomically(
            self.entry_path(workspace_id, record.document_id(), record.version_id()),
            entry_content(record.entry(), (self.clock)()),
        )?;
        write_file_atomically(
            self.body_path(workspace_id, record.document_id(), record.version_id()),
            record.snapshot().body().as_str(),
        )?;
        append_history(
            self.history_path(workspace_id, record.document_id()),
            record.version_id(),
        )
    }

    fn get_version_snapshot(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        let version_dir = self.version_dir(workspace_id, document_id, version_id);
        if !version_dir.exists() {
            return Ok(None);
        }

        let entry = self.read_entry_by_version(workspace_id, document_id, version_id)?;
        let body = read_body(
            &self.body_path(workspace_id, document_id, version_id),
            self.body_policy,
        )?;
        Ok(Some(VersionSnapshot::new(
            entry.document_id().clone(),
            entry.snapshot_ref().clone(),
            body,
        )))
    }

    fn list_history(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        request: HistoryPageRequest,
    ) -> Result<HistoryPage, VersionStoreError> {
        let history_path = self.history_path(workspace_id, document_id);
        let file = match fs::File::open(history_path) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(HistoryPage::new(Vec::new(), None));
            }
            Err(_) => return Err(VersionStoreError::StorageUnavailable),
        };

        let start = request
            .cursor()
            .map(|cursor| cursor.as_str().parse::<usize>())
            .transpose()
            .map_err(|_| VersionStoreError::CorruptedHistory)?
            .unwrap_or(0);
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        let mut next_cursor = None;

        for (index, line) in reader.lines().enumerate() {
            if index < start {
                continue;
            }
            if entries.len() == request.limit() {
                next_cursor = Some(
                    HistoryCursor::new(&index.to_string())
                        .map_err(|_| VersionStoreError::CorruptedHistory)?,
                );
                break;
            }

            let line = line.map_err(|_| VersionStoreError::StorageUnavailable)?;
            let version_id =
                VersionId::new(line.trim()).map_err(|_| VersionStoreError::CorruptedHistory)?;
            entries.push(self.read_entry_by_version(workspace_id, document_id, &version_id)?);
        }

        Ok(HistoryPage::new(entries, next_cursor))
    }
}

fn entry_content(entry: &VersionEntry, created_at_epoch_ms: u64) -> String {
    format!(
        "version_id={}\ndocument_id={}\nsnapshot_ref={}\nauthor={}\nsummary={}\ncreated_at_epoch_ms={}\n",
        entry.version_id().as_str(),
        entry.document_id().as_str(),
        entry.snapshot_ref().as_str(),
        entry.author().as_str(),
        entry.summary().as_str(),
        created_at_epoch_ms,
    )
}

fn read_entry(path: &Path) -> Result<VersionEntry, VersionStoreError> {
    let content = fs::read_to_string(path).map_err(|_| VersionStoreError::CorruptedHistory)?;
    let mut version_id = None;
    let mut document_id = None;
    let mut snapshot_ref = None;
    let mut author = None;
    let mut summary = None;
    let mut created_at_epoch_ms = None;

    for line in content.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(VersionStoreError::CorruptedHistory)?;
        match key {
            "version_id" => version_id = Some(value),
            "document_id" => document_id = Some(value),
            "snapshot_ref" => snapshot_ref = Some(value),
            "author" => author = Some(value),
            "summary" => summary = Some(value),
            "created_at_epoch_ms" => {
                created_at_epoch_ms = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| VersionStoreError::CorruptedHistory)?,
                )
            }
            _ => return Err(VersionStoreError::CorruptedHistory),
        }
    }

    let entry = VersionEntry::new(
        VersionId::new(version_id.ok_or(VersionStoreError::CorruptedHistory)?)
            .map_err(|_| VersionStoreError::CorruptedHistory)?,
        DocumentId::new(document_id.ok_or(VersionStoreError::CorruptedHistory)?)
            .map_err(|_| VersionStoreError::CorruptedHistory)?,
        DocumentSnapshotRef::new(snapshot_ref.ok_or(VersionStoreError::CorruptedHistory)?)
            .map_err(|_| VersionStoreError::CorruptedHistory)?,
        VersionAuthor::new(author.ok_or(VersionStoreError::CorruptedHistory)?)
            .map_err(|_| VersionStoreError::CorruptedHistory)?,
        VersionSummary::new(summary.ok_or(VersionStoreError::CorruptedHistory)?)
            .map_err(|_| VersionStoreError::CorruptedHistory)?,
    )
    .map_err(|_| VersionStoreError::CorruptedHistory)?;
    match created_at_epoch_ms {
        Some(value) => entry
            .with_created_at_epoch_ms(value)
            .map_err(|_| VersionStoreError::CorruptedHistory),
        None => Ok(entry),
    }
}

fn system_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(1)
        .max(1)
}

fn read_body(path: &Path, policy: DocumentBodyPolicy) -> Result<DocumentBody, VersionStoreError> {
    let content = fs::read_to_string(path).map_err(|_| VersionStoreError::CorruptedHistory)?;
    DocumentBody::new(&content, policy).map_err(|_| VersionStoreError::CorruptedHistory)
}

fn append_history(path: PathBuf, version_id: &VersionId) -> Result<(), VersionStoreError> {
    let parent = path.parent().ok_or(VersionStoreError::StorageUnavailable)?;
    fs::create_dir_all(parent).map_err(|_| VersionStoreError::StorageUnavailable)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| VersionStoreError::StorageUnavailable)?;
    writeln!(file, "{}", version_id.as_str()).map_err(|_| VersionStoreError::StorageUnavailable)
}

fn write_file_atomically(path: PathBuf, content: impl AsRef<str>) -> Result<(), VersionStoreError> {
    write_text_atomically(&path, content)
        .map(|_| ())
        .map_err(|_| VersionStoreError::StorageUnavailable)
}

fn encode_path_segment(value: &str) -> String {
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
