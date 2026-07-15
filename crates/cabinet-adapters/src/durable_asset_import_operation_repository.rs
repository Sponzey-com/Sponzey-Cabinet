use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use cabinet_domain::asset_import_operation::{
    AssetImportOperation, AssetImportOperationId, AssetImportState,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_import_operation_repository::{
    AssetImportOperationCreateOutcome, AssetImportOperationRepository,
    AssetImportOperationRepositoryError,
};

use crate::local_atomic_file::write_text_atomically;

const SCHEMA_HEADER: &str = "schema\t1";

#[derive(Debug, Clone)]
pub struct DurableAssetImportOperationRepository {
    root: PathBuf,
}

impl DurableAssetImportOperationRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn records_root(&self) -> PathBuf {
        self.root.join("operations").join("asset-import")
    }
    fn record_path(&self, id: &AssetImportOperationId) -> PathBuf {
        self.records_root()
            .join(format!("{}.import", hex_encode(id.as_str())))
    }
}

impl AssetImportOperationRepository for DurableAssetImportOperationRepository {
    fn create(
        &mut self,
        operation: AssetImportOperation,
    ) -> Result<AssetImportOperationCreateOutcome, AssetImportOperationRepositoryError> {
        let path = self.record_path(operation.operation_id());
        if path.exists() {
            read_operation(&path).map_err(map_existing_read_error)?;
            return Ok(AssetImportOperationCreateOutcome::AlreadyExists);
        }
        write_operation(&path, &operation)?;
        Ok(AssetImportOperationCreateOutcome::Created)
    }

    fn get(
        &self,
        id: &AssetImportOperationId,
    ) -> Result<Option<AssetImportOperation>, AssetImportOperationRepositoryError> {
        match read_operation(&self.record_path(id)) {
            Ok(operation) => Ok(Some(operation)),
            Err(ReadError::NotFound) => Ok(None),
            Err(ReadError::Repository(error)) => Err(error),
        }
    }

    fn replace(
        &mut self,
        operation: AssetImportOperation,
        expected_state: AssetImportState,
    ) -> Result<(), AssetImportOperationRepositoryError> {
        let path = self.record_path(operation.operation_id());
        let current = match read_operation(&path) {
            Ok(operation) => operation,
            Err(ReadError::NotFound) => return Err(AssetImportOperationRepositoryError::NotFound),
            Err(ReadError::Repository(error)) => return Err(error),
        };
        if current.state() != expected_state {
            return Err(AssetImportOperationRepositoryError::Conflict);
        }
        write_operation(&path, &operation)
    }

    fn list_active(
        &self,
        workspace_id: &WorkspaceId,
        limit: usize,
    ) -> Result<Vec<AssetImportOperation>, AssetImportOperationRepositoryError> {
        if limit == 0 {
            return Err(AssetImportOperationRepositoryError::InvalidLimit);
        }
        let entries = match fs::read_dir(self.records_root()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(AssetImportOperationRepositoryError::StorageUnavailable),
        };
        let mut paths = entries
            .map(|entry| {
                entry
                    .map(|value| value.path())
                    .map_err(|_| AssetImportOperationRepositoryError::StorageUnavailable)
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        let mut result = Vec::new();
        for path in paths {
            if path.extension().and_then(|value| value.to_str()) != Some("import") {
                continue;
            }
            let operation = match read_operation(&path) {
                Ok(operation) => operation,
                Err(ReadError::NotFound) => continue,
                Err(ReadError::Repository(error)) => return Err(error),
            };
            if operation.workspace_id() == workspace_id && !operation.state().is_terminal() {
                result.push(operation);
                if result.len() == limit {
                    break;
                }
            }
        }
        Ok(result)
    }
}

fn write_operation(
    path: &Path,
    operation: &AssetImportOperation,
) -> Result<(), AssetImportOperationRepositoryError> {
    write_text_atomically(path, encode(operation))
        .map(|_| ())
        .map_err(|_| AssetImportOperationRepositoryError::StorageUnavailable)
}

enum ReadError {
    NotFound,
    Repository(AssetImportOperationRepositoryError),
}

fn read_operation(path: &Path) -> Result<AssetImportOperation, ReadError> {
    let text = fs::read_to_string(path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            ReadError::NotFound
        } else {
            ReadError::Repository(AssetImportOperationRepositoryError::StorageUnavailable)
        }
    })?;
    decode(&text).map_err(ReadError::Repository)
}

fn map_existing_read_error(error: ReadError) -> AssetImportOperationRepositoryError {
    match error {
        ReadError::NotFound => AssetImportOperationRepositoryError::StorageUnavailable,
        ReadError::Repository(error) => error,
    }
}

fn encode(operation: &AssetImportOperation) -> String {
    let payload = format!(
        "operation\t{}\nworkspace\t{}\ndocument\t{}\nstate\t{}\nattempt\t{}\ncompleted\t{}\ntotal\t{}\n",
        hex_encode(operation.operation_id().as_str()),
        hex_encode(operation.workspace_id().as_str()),
        hex_encode(operation.document_id().as_str()),
        encode_state(operation.state()),
        operation.attempt(),
        operation.completed_bytes(),
        operation.total_bytes()
    );
    format!(
        "{SCHEMA_HEADER}\nchecksum\t{:016x}\n{payload}",
        checksum(payload.as_bytes())
    )
}

fn decode(text: &str) -> Result<AssetImportOperation, AssetImportOperationRepositoryError> {
    let mut lines = text.lines();
    match lines.next() {
        Some(SCHEMA_HEADER) => {}
        Some(line) if line.starts_with("schema\t") => {
            return Err(AssetImportOperationRepositoryError::UnsupportedSchema);
        }
        _ => return Err(AssetImportOperationRepositoryError::CorruptedRecord),
    }
    let expected = lines
        .next()
        .and_then(|line| line.strip_prefix("checksum\t"))
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .ok_or(AssetImportOperationRepositoryError::CorruptedRecord)?;
    let payload = format!("{}\n", lines.collect::<Vec<_>>().join("\n"));
    if checksum(payload.as_bytes()) != expected {
        return Err(AssetImportOperationRepositoryError::CorruptedRecord);
    }
    let fields = payload
        .lines()
        .map(|line| line.split_once('\t'))
        .collect::<Option<Vec<_>>>()
        .ok_or(AssetImportOperationRepositoryError::CorruptedRecord)?;
    if fields.len() != 7 {
        return Err(AssetImportOperationRepositoryError::CorruptedRecord);
    }
    let find = |name: &str| {
        fields
            .iter()
            .find_map(|(key, value)| (*key == name).then_some(*value))
            .ok_or(AssetImportOperationRepositoryError::CorruptedRecord)
    };
    AssetImportOperation::restore(
        AssetImportOperationId::new(&hex_decode(find("operation")?)?)
            .map_err(|_| AssetImportOperationRepositoryError::CorruptedRecord)?,
        WorkspaceId::new(&hex_decode(find("workspace")?)?)
            .map_err(|_| AssetImportOperationRepositoryError::CorruptedRecord)?,
        DocumentId::new(&hex_decode(find("document")?)?)
            .map_err(|_| AssetImportOperationRepositoryError::CorruptedRecord)?,
        decode_state(find("state")?)?,
        find("attempt")?
            .parse()
            .map_err(|_| AssetImportOperationRepositoryError::CorruptedRecord)?,
        find("completed")?
            .parse()
            .map_err(|_| AssetImportOperationRepositoryError::CorruptedRecord)?,
        find("total")?
            .parse()
            .map_err(|_| AssetImportOperationRepositoryError::CorruptedRecord)?,
    )
    .map_err(|_| AssetImportOperationRepositoryError::CorruptedRecord)
}

const fn encode_state(state: AssetImportState) -> &'static str {
    match state {
        AssetImportState::Selected => "selected",
        AssetImportState::Validating => "validating",
        AssetImportState::Staging => "staging",
        AssetImportState::Hashing => "hashing",
        AssetImportState::PublishingObject => "publishing_object",
        AssetImportState::PersistingMetadata => "persisting_metadata",
        AssetImportState::Linking => "linking",
        AssetImportState::Completed => "completed",
        AssetImportState::ValidationFailed => "validation_failed",
        AssetImportState::StagingFailed => "staging_failed",
        AssetImportState::ObjectPublishFailed => "object_publish_failed",
        AssetImportState::MetadataPersistFailed => "metadata_persist_failed",
        AssetImportState::LinkFailed => "link_failed",
        AssetImportState::Cancelling => "cancelling",
        AssetImportState::Cancelled => "cancelled",
        AssetImportState::CleanupRequired => "cleanup_required",
    }
}

fn decode_state(value: &str) -> Result<AssetImportState, AssetImportOperationRepositoryError> {
    match value {
        "selected" => Ok(AssetImportState::Selected),
        "validating" => Ok(AssetImportState::Validating),
        "staging" => Ok(AssetImportState::Staging),
        "hashing" => Ok(AssetImportState::Hashing),
        "publishing_object" => Ok(AssetImportState::PublishingObject),
        "persisting_metadata" => Ok(AssetImportState::PersistingMetadata),
        "linking" => Ok(AssetImportState::Linking),
        "completed" => Ok(AssetImportState::Completed),
        "validation_failed" => Ok(AssetImportState::ValidationFailed),
        "staging_failed" => Ok(AssetImportState::StagingFailed),
        "object_publish_failed" => Ok(AssetImportState::ObjectPublishFailed),
        "metadata_persist_failed" => Ok(AssetImportState::MetadataPersistFailed),
        "link_failed" => Ok(AssetImportState::LinkFailed),
        "cancelling" => Ok(AssetImportState::Cancelling),
        "cancelled" => Ok(AssetImportState::Cancelled),
        "cleanup_required" => Ok(AssetImportState::CleanupRequired),
        _ => Err(AssetImportOperationRepositoryError::CorruptedRecord),
    }
}

fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}
fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn hex_decode(value: &str) -> Result<String, AssetImportOperationRepositoryError> {
    if !value.len().is_multiple_of(2) {
        return Err(AssetImportOperationRepositoryError::CorruptedRecord);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)
                .map_err(|_| AssetImportOperationRepositoryError::CorruptedRecord)?;
            u8::from_str_radix(text, 16)
                .map_err(|_| AssetImportOperationRepositoryError::CorruptedRecord)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| AssetImportOperationRepositoryError::CorruptedRecord)
}
