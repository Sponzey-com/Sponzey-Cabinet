use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::CurrentDocumentSnapshot;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_existence::{DocumentExistenceError, DocumentExistenceReader};
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_ports::document_title_reader::{
    DocumentTitleLookup, DocumentTitleReader, DocumentTitleReaderError,
};

use crate::local_atomic_file::write_text_atomically;

pub const LOCAL_DOCUMENTS_DIR: &str = "documents";
pub const DOCUMENTS_BY_ID_DIR: &str = "by-id";
pub const DOCUMENTS_BY_PATH_DIR: &str = "by-path";
pub const DOCUMENT_METADATA_FILE: &str = "metadata.txt";
pub const DOCUMENT_BODY_FILE: &str = "body.md";
const DEFAULT_LOCAL_DOCUMENT_BODY_MAX_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDocumentRepository {
    workspace_root: PathBuf,
    body_policy: DocumentBodyPolicy,
}

impl LocalDocumentRepository {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            body_policy: DocumentBodyPolicy::new(DEFAULT_LOCAL_DOCUMENT_BODY_MAX_BYTES)
                .expect("default body policy must be valid"),
        }
    }

    pub fn with_body_policy(workspace_root: PathBuf, body_policy: DocumentBodyPolicy) -> Self {
        Self {
            workspace_root,
            body_policy,
        }
    }

    fn documents_dir(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.workspace_root
            .join(encode_path_segment(workspace_id.as_str()))
            .join(LOCAL_DOCUMENTS_DIR)
    }

    fn document_dir(&self, workspace_id: &WorkspaceId, document_id: &DocumentId) -> PathBuf {
        self.documents_dir(workspace_id)
            .join(DOCUMENTS_BY_ID_DIR)
            .join(encode_path_segment(document_id.as_str()))
    }

    fn metadata_path(&self, workspace_id: &WorkspaceId, document_id: &DocumentId) -> PathBuf {
        self.document_dir(workspace_id, document_id)
            .join(DOCUMENT_METADATA_FILE)
    }

    fn body_path(&self, workspace_id: &WorkspaceId, document_id: &DocumentId) -> PathBuf {
        self.document_dir(workspace_id, document_id)
            .join(DOCUMENT_BODY_FILE)
    }

    fn path_index_path(&self, workspace_id: &WorkspaceId, path: &DocumentPath) -> PathBuf {
        self.documents_dir(workspace_id)
            .join(DOCUMENTS_BY_PATH_DIR)
            .join(format!("{}.ref", path.as_str()))
    }

    fn read_current_from_id(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        let document_dir = self.document_dir(workspace_id, document_id);
        if !document_dir.exists() {
            return Ok(None);
        }

        let metadata = read_metadata(&self.metadata_path(workspace_id, document_id))?;
        let body = read_body(&self.body_path(workspace_id, document_id), self.body_policy)?;
        let snapshot = CurrentDocumentSnapshot::new(metadata.id().clone(), body);
        CurrentDocumentRecord::new(metadata, snapshot).map(Some)
    }
}

impl DocumentRepository for LocalDocumentRepository {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        if let Some(existing) = self.read_current_from_id(workspace_id, record.document_id())? {
            if existing.path() != record.path() {
                remove_file_if_exists(self.path_index_path(workspace_id, existing.path()))?;
            }
        }

        write_file_atomically(
            self.metadata_path(workspace_id, record.document_id()),
            metadata_content(&record),
        )?;
        write_file_atomically(
            self.body_path(workspace_id, record.document_id()),
            record.body().as_str(),
        )?;
        write_file_atomically(
            self.path_index_path(workspace_id, record.path()),
            format!("{}\n", record.document_id().as_str()),
        )
    }

    fn get_current_by_id(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        self.read_current_from_id(workspace_id, document_id)
    }

    fn get_current_by_path(
        &self,
        workspace_id: &WorkspaceId,
        path: &DocumentPath,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        let path_index_path = self.path_index_path(workspace_id, path);
        if !path_index_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path_index_path)
            .map_err(|_| DocumentRepositoryError::StorageUnavailable)?;
        let document_id = DocumentId::new(content.trim())
            .map_err(|_| DocumentRepositoryError::CorruptedMetadata)?;
        self.read_current_from_id(workspace_id, &document_id)?
            .ok_or(DocumentRepositoryError::CorruptedMetadata)
            .map(Some)
    }

    fn delete_current(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<(), DocumentRepositoryError> {
        if let Some(existing) = self.read_current_from_id(workspace_id, document_id)? {
            remove_file_if_exists(self.path_index_path(workspace_id, existing.path()))?;
        }

        let document_dir = self.document_dir(workspace_id, document_id);
        if document_dir.exists() {
            fs::remove_dir_all(document_dir)
                .map_err(|_| DocumentRepositoryError::StorageUnavailable)?;
        }
        Ok(())
    }
}

impl DocumentTitleReader for LocalDocumentRepository {
    fn get_current_title(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentTitle>, DocumentTitleReaderError> {
        let path = self.metadata_path(workspace_id, document_id);
        if !path.exists() {
            return Ok(None);
        }
        read_metadata(&path)
            .map(|metadata| Some(metadata.title().clone()))
            .map_err(|error| match error {
                DocumentRepositoryError::StorageUnavailable => {
                    DocumentTitleReaderError::StorageUnavailable
                }
                _ => DocumentTitleReaderError::CorruptedMetadata,
            })
    }

    fn get_current_titles(
        &self,
        workspace_id: &WorkspaceId,
        document_ids: &[DocumentId],
    ) -> Result<Vec<DocumentTitleLookup>, DocumentTitleReaderError> {
        document_ids
            .iter()
            .map(|document_id| {
                self.get_current_title(workspace_id, document_id)
                    .map(|title| DocumentTitleLookup::new(document_id.clone(), title))
            })
            .collect()
    }
}

impl DocumentExistenceReader for LocalDocumentRepository {
    fn exists(
        &self,
        workspace: &WorkspaceId,
        document: &DocumentId,
    ) -> Result<bool, DocumentExistenceError> {
        self.get_current_by_id(workspace, document)
            .map(|record| record.is_some())
            .map_err(|error| match error {
                DocumentRepositoryError::CorruptedMetadata => {
                    DocumentExistenceError::CorruptedRecord
                }
                _ => DocumentExistenceError::StorageUnavailable,
            })
    }
}

fn metadata_content(record: &CurrentDocumentRecord) -> String {
    format!(
        "id={}\ntitle={}\npath={}\n",
        record.document_id().as_str(),
        record.metadata().title().as_str(),
        record.path().as_str()
    )
}

fn read_metadata(path: &Path) -> Result<DocumentMetadata, DocumentRepositoryError> {
    let content =
        fs::read_to_string(path).map_err(|_| DocumentRepositoryError::CorruptedMetadata)?;
    let mut id = None;
    let mut title = None;
    let mut document_path = None;

    for line in content.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(DocumentRepositoryError::CorruptedMetadata)?;
        match key {
            "id" => id = Some(value),
            "title" => title = Some(value),
            "path" => document_path = Some(value),
            _ => return Err(DocumentRepositoryError::CorruptedMetadata),
        }
    }

    let id = DocumentId::new(id.ok_or(DocumentRepositoryError::CorruptedMetadata)?)
        .map_err(|_| DocumentRepositoryError::CorruptedMetadata)?;
    let title = DocumentTitle::new(title.ok_or(DocumentRepositoryError::CorruptedMetadata)?)
        .map_err(|_| DocumentRepositoryError::CorruptedMetadata)?;
    let document_path =
        DocumentPath::new(document_path.ok_or(DocumentRepositoryError::CorruptedMetadata)?)
            .map_err(|_| DocumentRepositoryError::CorruptedMetadata)?;

    DocumentMetadata::new(id, title, document_path)
        .map_err(|_| DocumentRepositoryError::CorruptedMetadata)
}

fn read_body(
    path: &Path,
    policy: DocumentBodyPolicy,
) -> Result<DocumentBody, DocumentRepositoryError> {
    let content =
        fs::read_to_string(path).map_err(|_| DocumentRepositoryError::CorruptedMetadata)?;
    DocumentBody::new(&content, policy).map_err(|_| DocumentRepositoryError::CorruptedMetadata)
}

fn write_file_atomically(
    path: PathBuf,
    content: impl AsRef<str>,
) -> Result<(), DocumentRepositoryError> {
    write_text_atomically(&path, content)
        .map(|_| ())
        .map_err(|_| DocumentRepositoryError::StorageUnavailable)
}

fn remove_file_if_exists(path: PathBuf) -> Result<(), DocumentRepositoryError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(DocumentRepositoryError::StorageUnavailable),
    }
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
