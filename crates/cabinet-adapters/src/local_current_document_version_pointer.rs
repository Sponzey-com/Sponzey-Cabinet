use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use cabinet_domain::document::DocumentId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};

use crate::local_atomic_file::write_text_atomically;

const SCHEMA_HEADER: &str = "schema=1";

#[derive(Debug, Clone)]
pub struct LocalCurrentDocumentVersionPointer {
    root: PathBuf,
}

impl LocalCurrentDocumentVersionPointer {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn pointer_path(&self, workspace_id: &WorkspaceId, document_id: &DocumentId) -> PathBuf {
        self.root
            .join(hex_encode(workspace_id.as_str()))
            .join(hex_encode(document_id.as_str()))
            .join("current.pointer")
    }
}

impl CurrentDocumentVersionPointerPort for LocalCurrentDocumentVersionPointer {
    fn load_current_version(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<VersionId>, CurrentDocumentVersionPointerError> {
        let text = match fs::read_to_string(self.pointer_path(workspace_id, document_id)) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(CurrentDocumentVersionPointerError::StorageUnavailable),
        };
        decode_pointer(&text).map(Some)
    }

    fn compare_and_set_current_version(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        expected: Option<&VersionId>,
        next: VersionId,
    ) -> Result<(), CurrentDocumentVersionPointerError> {
        let current = self.load_current_version(workspace_id, document_id)?;
        if current.as_ref() != expected {
            return Err(CurrentDocumentVersionPointerError::Conflict);
        }
        write_text_atomically(
            &self.pointer_path(workspace_id, document_id),
            format!("{SCHEMA_HEADER}\nversion={}\n", hex_encode(next.as_str())),
        )
        .map(|_| ())
        .map_err(|_| CurrentDocumentVersionPointerError::StorageUnavailable)
    }
}

fn decode_pointer(text: &str) -> Result<VersionId, CurrentDocumentVersionPointerError> {
    let mut lines = text.lines();
    if lines.next() != Some(SCHEMA_HEADER) {
        return Err(CurrentDocumentVersionPointerError::CorruptedPointer);
    }
    let encoded = lines
        .next()
        .and_then(|line| line.strip_prefix("version="))
        .ok_or(CurrentDocumentVersionPointerError::CorruptedPointer)?;
    if lines.next().is_some() {
        return Err(CurrentDocumentVersionPointerError::CorruptedPointer);
    }
    VersionId::new(&hex_decode(encoded)?)
        .map_err(|_| CurrentDocumentVersionPointerError::CorruptedPointer)
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, CurrentDocumentVersionPointerError> {
    if value.len() % 2 != 0 {
        return Err(CurrentDocumentVersionPointerError::CorruptedPointer);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair)
                .map_err(|_| CurrentDocumentVersionPointerError::CorruptedPointer)?;
            u8::from_str_radix(pair, 16)
                .map_err(|_| CurrentDocumentVersionPointerError::CorruptedPointer)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| CurrentDocumentVersionPointerError::CorruptedPointer)
}
