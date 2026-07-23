use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use cabinet_domain::canvas::CanvasId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_catalog::{LastCanvasSelectionError, LastCanvasSelectionPort};

use crate::local_atomic_file::write_text_atomically;

const SCHEMA: &str = "schema\t1";

#[derive(Debug, Clone)]
pub struct DurableLastCanvasSelection {
    root: PathBuf,
}

impl DurableLastCanvasSelection {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.root
            .join("preferences")
            .join("canvas-selection")
            .join(format!("{}.selection", hex(workspace_id.as_str())))
    }
}

impl LastCanvasSelectionPort for DurableLastCanvasSelection {
    fn load_last_canvas_id(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Option<CanvasId>, LastCanvasSelectionError> {
        let path = self.path(workspace_id);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(LastCanvasSelectionError::StorageUnavailable),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(LastCanvasSelectionError::CorruptedSelection);
        }
        let text =
            fs::read_to_string(path).map_err(|_| LastCanvasSelectionError::StorageUnavailable)?;
        decode(&text).map(Some)
    }

    fn save_last_canvas_id(
        &mut self,
        workspace_id: &WorkspaceId,
        canvas_id: &CanvasId,
    ) -> Result<(), LastCanvasSelectionError> {
        write_text_atomically(&self.path(workspace_id), encode(canvas_id))
            .map(|_| ())
            .map_err(|_| LastCanvasSelectionError::StorageUnavailable)
    }
}

fn encode(canvas_id: &CanvasId) -> String {
    let payload = format!("canvas\t{}\n", hex(canvas_id.as_str()));
    format!(
        "{SCHEMA}\nchecksum\t{:016x}\n{payload}",
        checksum(payload.as_bytes())
    )
}

fn decode(text: &str) -> Result<CanvasId, LastCanvasSelectionError> {
    let mut lines = text.lines();
    if lines.next() != Some(SCHEMA) {
        return Err(LastCanvasSelectionError::CorruptedSelection);
    }
    let expected = lines
        .next()
        .and_then(|line| line.strip_prefix("checksum\t"))
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .ok_or(LastCanvasSelectionError::CorruptedSelection)?;
    let payload = format!("{}\n", lines.collect::<Vec<_>>().join("\n"));
    if checksum(payload.as_bytes()) != expected {
        return Err(LastCanvasSelectionError::CorruptedSelection);
    }
    let encoded = payload
        .strip_prefix("canvas\t")
        .and_then(|value| value.strip_suffix('\n'))
        .ok_or(LastCanvasSelectionError::CorruptedSelection)?;
    let decoded = unhex(encoded)?;
    if hex(&decoded) != encoded {
        return Err(LastCanvasSelectionError::CorruptedSelection);
    }
    CanvasId::new(&decoded).map_err(|_| LastCanvasSelectionError::CorruptedSelection)
}

fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn unhex(value: &str) -> Result<String, LastCanvasSelectionError> {
    if value.is_empty() || !value.len().is_multiple_of(2) {
        return Err(LastCanvasSelectionError::CorruptedSelection);
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| LastCanvasSelectionError::CorruptedSelection)?;
    String::from_utf8(bytes).map_err(|_| LastCanvasSelectionError::CorruptedSelection)
}
