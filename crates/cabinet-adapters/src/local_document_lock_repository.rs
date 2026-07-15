use std::fmt;
use std::fs;
use std::path::PathBuf;

use cabinet_domain::document::DocumentId;
use cabinet_domain::document_lock::{DocumentLock, DocumentLockId, DocumentLockTimestamp};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_lock::{DocumentLockRepository, DocumentLockRepositoryError};

use crate::local_atomic_file::write_text_atomically;

pub const LOCAL_DOCUMENT_LOCKS_DIR: &str = "document-locks";
pub const LOCAL_DOCUMENT_LOCKS_BY_DOCUMENT_DIR: &str = "by-document";

#[derive(Clone, PartialEq, Eq)]
pub struct LocalDocumentLockRepository {
    root: PathBuf,
}

impl fmt::Debug for LocalDocumentLockRepository {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalDocumentLockRepository")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl LocalDocumentLockRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn lock_path(&self, workspace_id: &WorkspaceId, document_id: &DocumentId) -> PathBuf {
        self.root
            .join(LOCAL_DOCUMENT_LOCKS_DIR)
            .join(hex_encode(workspace_id.as_str()))
            .join(LOCAL_DOCUMENT_LOCKS_BY_DOCUMENT_DIR)
            .join(format!("{}.lock", hex_encode(document_id.as_str())))
    }
}

impl DocumentLockRepository for LocalDocumentLockRepository {
    fn get_document_lock(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentLock>, DocumentLockRepositoryError> {
        let path = self.lock_path(workspace_id, document_id);
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(DocumentLockRepositoryError::StorageUnavailable),
        };
        let lock = decode_lock(&content)?;
        if lock.document_id() != document_id {
            return Err(DocumentLockRepositoryError::CorruptedState);
        }
        Ok(Some(lock))
    }

    fn save_document_lock(
        &mut self,
        workspace_id: &WorkspaceId,
        lock: DocumentLock,
    ) -> Result<(), DocumentLockRepositoryError> {
        write_text_atomically(
            &self.lock_path(workspace_id, lock.document_id()),
            encode_lock(&lock),
        )
        .map(|_| ())
        .map_err(|_| DocumentLockRepositoryError::StorageUnavailable)
    }

    fn delete_document_lock(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentLock>, DocumentLockRepositoryError> {
        let path = self.lock_path(workspace_id, document_id);
        let Some(lock) = self.get_document_lock(workspace_id, document_id)? else {
            return Ok(None);
        };
        fs::remove_file(path)
            .map(|_| Some(lock))
            .map_err(|_| DocumentLockRepositoryError::StorageUnavailable)
    }
}

fn encode_lock(lock: &DocumentLock) -> String {
    format!(
        "lock_id={}\ndocument_id={}\nowner_user_id={}\nacquired_at={}\nexpires_at={}\n",
        hex_encode(lock.lock_id().as_str()),
        hex_encode(lock.document_id().as_str()),
        hex_encode(lock.owner_user_id().as_str()),
        lock.acquired_at().as_millis(),
        lock.expires_at().as_millis()
    )
}

fn decode_lock(content: &str) -> Result<DocumentLock, DocumentLockRepositoryError> {
    let mut lock_id = None;
    let mut document_id = None;
    let mut owner_user_id = None;
    let mut acquired_at = None;
    let mut expires_at = None;
    for line in content.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(DocumentLockRepositoryError::CorruptedState)?;
        match key {
            "lock_id" => lock_id = Some(hex_decode(value)?),
            "document_id" => document_id = Some(hex_decode(value)?),
            "owner_user_id" => owner_user_id = Some(hex_decode(value)?),
            "acquired_at" => {
                acquired_at = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| DocumentLockRepositoryError::CorruptedState)?,
                );
            }
            "expires_at" => {
                expires_at = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| DocumentLockRepositoryError::CorruptedState)?,
                );
            }
            _ => return Err(DocumentLockRepositoryError::CorruptedState),
        }
    }
    DocumentLock::new(
        DocumentLockId::new(&lock_id.ok_or(DocumentLockRepositoryError::CorruptedState)?)
            .map_err(|_| DocumentLockRepositoryError::CorruptedState)?,
        DocumentId::new(&document_id.ok_or(DocumentLockRepositoryError::CorruptedState)?)
            .map_err(|_| DocumentLockRepositoryError::CorruptedState)?,
        UserId::new(&owner_user_id.ok_or(DocumentLockRepositoryError::CorruptedState)?)
            .map_err(|_| DocumentLockRepositoryError::CorruptedState)?,
        DocumentLockTimestamp::from_millis(
            acquired_at.ok_or(DocumentLockRepositoryError::CorruptedState)?,
        ),
        DocumentLockTimestamp::from_millis(
            expires_at.ok_or(DocumentLockRepositoryError::CorruptedState)?,
        ),
    )
    .map_err(|_| DocumentLockRepositoryError::CorruptedState)
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, DocumentLockRepositoryError> {
    if !value.len().is_multiple_of(2) {
        return Err(DocumentLockRepositoryError::CorruptedState);
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| DocumentLockRepositoryError::CorruptedState)?;
    String::from_utf8(bytes).map_err(|_| DocumentLockRepositoryError::CorruptedState)
}
