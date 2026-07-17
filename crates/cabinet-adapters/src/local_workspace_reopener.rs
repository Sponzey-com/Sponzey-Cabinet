use std::fs;
use std::path::{Path, PathBuf};

use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::backup_restore::{WorkspaceReopenError, WorkspaceReopener};

use crate::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT,
};

#[derive(Debug, Clone)]
pub struct LocalWorkspaceReopener {
    app_data_root: PathBuf,
}

impl LocalWorkspaceReopener {
    pub fn new(app_data_root: PathBuf) -> Self {
        Self { app_data_root }
    }
}

impl WorkspaceReopener for LocalWorkspaceReopener {
    fn reopen_workspace(&mut self, workspace_id: &WorkspaceId) -> Result<(), WorkspaceReopenError> {
        let workspace = hex(workspace_id.as_str());
        let document_workspace = encode_document_segment(workspace_id.as_str());
        validate_directory(
            &self
                .app_data_root
                .join("authoring-current")
                .join(&document_workspace),
        )?;
        validate_directory(
            &self
                .app_data_root
                .join(LOCAL_DOCUMENT_VERSION_ROOT)
                .join(&document_workspace),
        )?;
        validate_directory(
            &self
                .app_data_root
                .join(LOCAL_DOCUMENT_POINTER_ROOT)
                .join(&workspace),
        )?;
        for relative in [
            "canvases",
            "assets/metadata",
            "assets/objects",
            "assets/associations",
        ] {
            validate_directory(&self.app_data_root.join(relative).join(&workspace))?;
        }
        Ok(())
    }
}

fn validate_directory(path: &Path) -> Result<(), WorkspaceReopenError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| WorkspaceReopenError::ReopenFailed)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(WorkspaceReopenError::ReopenFailed);
    }
    fs::read_dir(path).map_err(|_| WorkspaceReopenError::ReopenFailed)?;
    Ok(())
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
