use std::fs;
use std::path::PathBuf;

use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_projection_catalog::{
    CurrentDocumentProjectionCatalog, CurrentDocumentProjectionCatalogError,
    CurrentDocumentProjectionIdentity,
};
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;

use crate::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;

#[derive(Debug, Clone)]
pub struct LocalCurrentDocumentProjectionCatalog {
    app_data_root: PathBuf,
}
impl LocalCurrentDocumentProjectionCatalog {
    pub fn new(app_data_root: PathBuf) -> Self {
        Self { app_data_root }
    }
}

impl CurrentDocumentProjectionCatalog for LocalCurrentDocumentProjectionCatalog {
    fn list_current_projection_identities(
        &self,
        workspace_id: &WorkspaceId,
        limit: usize,
    ) -> Result<Vec<CurrentDocumentProjectionIdentity>, CurrentDocumentProjectionCatalogError> {
        if limit == 0 {
            return Err(CurrentDocumentProjectionCatalogError::InvalidLimit);
        }
        let root = self
            .app_data_root
            .join("authoring-current")
            .join(encode_document_segment(workspace_id.as_str()))
            .join("documents/by-id");
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(CurrentDocumentProjectionCatalogError::StorageUnavailable),
        };
        let mut documents = Vec::new();
        for entry in entries {
            let entry =
                entry.map_err(|_| CurrentDocumentProjectionCatalogError::StorageUnavailable)?;
            let metadata = fs::symlink_metadata(entry.path())
                .map_err(|_| CurrentDocumentProjectionCatalogError::StorageUnavailable)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(CurrentDocumentProjectionCatalogError::CorruptedRecord);
            }
            let encoded = entry
                .file_name()
                .into_string()
                .map_err(|_| CurrentDocumentProjectionCatalogError::CorruptedRecord)?;
            documents.push(
                DocumentId::new(&decode_document_segment(&encoded)?)
                    .map_err(|_| CurrentDocumentProjectionCatalogError::CorruptedRecord)?,
            );
        }
        documents.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        if documents.len() > limit {
            return Err(CurrentDocumentProjectionCatalogError::LimitExceeded);
        }
        let pointers = LocalCurrentDocumentVersionPointer::new(
            self.app_data_root.join("authoring-current-version"),
        );
        documents
            .into_iter()
            .map(|document_id| {
                let version_id = pointers
                    .load_current_version(workspace_id, &document_id)
                    .map_err(|_| CurrentDocumentProjectionCatalogError::CorruptedRecord)?
                    .ok_or(CurrentDocumentProjectionCatalogError::CorruptedRecord)?;
                Ok(CurrentDocumentProjectionIdentity::new(
                    document_id,
                    version_id,
                ))
            })
            .collect()
    }
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
fn decode_document_segment(value: &str) -> Result<String, CurrentDocumentProjectionCatalogError> {
    let mut bytes = Vec::new();
    let raw = value.as_bytes();
    let mut index = 0;
    while index < raw.len() {
        if raw[index] == b'~' {
            if index + 2 >= raw.len() {
                return Err(CurrentDocumentProjectionCatalogError::CorruptedRecord);
            }
            let pair = std::str::from_utf8(&raw[index + 1..index + 3])
                .map_err(|_| CurrentDocumentProjectionCatalogError::CorruptedRecord)?;
            bytes.push(
                u8::from_str_radix(pair, 16)
                    .map_err(|_| CurrentDocumentProjectionCatalogError::CorruptedRecord)?,
            );
            index += 3;
        } else {
            bytes.push(raw[index]);
            index += 1;
        }
    }
    String::from_utf8(bytes).map_err(|_| CurrentDocumentProjectionCatalogError::CorruptedRecord)
}
