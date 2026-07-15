use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_repair::{
    ProjectionRepairOperation, ProjectionRepairOperationId, ProjectionRepairProgress,
    ProjectionRepairState,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::projection_repair::{
    ProjectionRepairCreateOutcome, ProjectionRepairRepository, ProjectionRepairRepositoryError,
};

use crate::local_atomic_file::write_text_atomically;

const SCHEMA_HEADER: &str = "schema\t1";

#[derive(Debug, Clone)]
pub struct DurableProjectionRepairRepository {
    root: PathBuf,
}

impl DurableProjectionRepairRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn records_root(&self) -> PathBuf {
        self.root.join("operations").join("projection-repair")
    }
    fn record_path(&self, id: &ProjectionRepairOperationId) -> PathBuf {
        self.records_root()
            .join(format!("{}.repair", hex_encode(id.as_str())))
    }
}

impl ProjectionRepairRepository for DurableProjectionRepairRepository {
    fn create(
        &mut self,
        operation: ProjectionRepairOperation,
    ) -> Result<ProjectionRepairCreateOutcome, ProjectionRepairRepositoryError> {
        let path = self.record_path(operation.operation_id());
        if path.exists() {
            read_operation(&path).map_err(|error| match error {
                ReadError::NotFound => ProjectionRepairRepositoryError::StorageUnavailable,
                ReadError::Repository(error) => error,
            })?;
            return Ok(ProjectionRepairCreateOutcome::AlreadyExists);
        }
        write_operation(&path, &operation)?;
        Ok(ProjectionRepairCreateOutcome::Created)
    }

    fn get(
        &self,
        operation_id: &ProjectionRepairOperationId,
    ) -> Result<Option<ProjectionRepairOperation>, ProjectionRepairRepositoryError> {
        match read_operation(&self.record_path(operation_id)) {
            Ok(value) => Ok(Some(value)),
            Err(ReadError::NotFound) => Ok(None),
            Err(ReadError::Repository(error)) => Err(error),
        }
    }

    fn replace(
        &mut self,
        operation: ProjectionRepairOperation,
        expected_state: ProjectionRepairState,
    ) -> Result<(), ProjectionRepairRepositoryError> {
        let path = self.record_path(operation.operation_id());
        let current = match read_operation(&path) {
            Ok(value) => value,
            Err(ReadError::NotFound) => return Err(ProjectionRepairRepositoryError::NotFound),
            Err(ReadError::Repository(error)) => return Err(error),
        };
        if current.state() != expected_state {
            return Err(ProjectionRepairRepositoryError::Conflict);
        }
        write_operation(&path, &operation)
    }

    fn list_active(
        &self,
        workspace_id: &WorkspaceId,
        limit: usize,
    ) -> Result<Vec<ProjectionRepairOperation>, ProjectionRepairRepositoryError> {
        if limit == 0 {
            return Err(ProjectionRepairRepositoryError::InvalidLimit);
        }
        let entries = match fs::read_dir(self.records_root()) {
            Ok(value) => value,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(ProjectionRepairRepositoryError::StorageUnavailable),
        };
        let mut paths = entries
            .map(|entry| {
                entry
                    .map(|value| value.path())
                    .map_err(|_| ProjectionRepairRepositoryError::StorageUnavailable)
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        let mut result = Vec::new();
        for path in paths {
            if path.extension().and_then(|value| value.to_str()) != Some("repair") {
                continue;
            }
            let operation = match read_operation(&path) {
                Ok(value) => value,
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
    operation: &ProjectionRepairOperation,
) -> Result<(), ProjectionRepairRepositoryError> {
    write_text_atomically(path, encode(operation))
        .map(|_| ())
        .map_err(|_| ProjectionRepairRepositoryError::StorageUnavailable)
}

enum ReadError {
    NotFound,
    Repository(ProjectionRepairRepositoryError),
}
fn read_operation(path: &Path) -> Result<ProjectionRepairOperation, ReadError> {
    let text = fs::read_to_string(path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            ReadError::NotFound
        } else {
            ReadError::Repository(ProjectionRepairRepositoryError::StorageUnavailable)
        }
    })?;
    decode(&text).map_err(ReadError::Repository)
}

fn encode(operation: &ProjectionRepairOperation) -> String {
    let progress = operation.progress();
    let payload = format!(
        "operation\t{}\nworkspace\t{}\ndocument\t{}\nstate\t{}\nattempt\t{}\ncompleted\t{}\ntotal\t{}\n",
        hex_encode(operation.operation_id().as_str()),
        hex_encode(operation.workspace_id().as_str()),
        hex_encode(operation.document_id().as_str()),
        encode_state(operation.state()),
        operation.attempt(),
        progress.completed_units(),
        progress.total_units(),
    );
    format!(
        "{SCHEMA_HEADER}\nchecksum\t{:016x}\n{payload}",
        checksum(payload.as_bytes())
    )
}

fn decode(text: &str) -> Result<ProjectionRepairOperation, ProjectionRepairRepositoryError> {
    let mut lines = text.lines();
    match lines.next() {
        Some(SCHEMA_HEADER) => {}
        Some(line) if line.starts_with("schema\t") => {
            return Err(ProjectionRepairRepositoryError::UnsupportedSchema);
        }
        _ => return Err(ProjectionRepairRepositoryError::CorruptedRecord),
    }
    let expected = lines
        .next()
        .and_then(|line| line.strip_prefix("checksum\t"))
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .ok_or(ProjectionRepairRepositoryError::CorruptedRecord)?;
    let payload = format!("{}\n", lines.collect::<Vec<_>>().join("\n"));
    if checksum(payload.as_bytes()) != expected {
        return Err(ProjectionRepairRepositoryError::CorruptedRecord);
    }
    let fields = payload
        .lines()
        .map(|line| line.split_once('\t'))
        .collect::<Option<Vec<_>>>()
        .ok_or(ProjectionRepairRepositoryError::CorruptedRecord)?;
    if fields.len() != 7 {
        return Err(ProjectionRepairRepositoryError::CorruptedRecord);
    }
    let find = |name: &str| {
        fields
            .iter()
            .find_map(|(key, value)| (*key == name).then_some(*value))
            .ok_or(ProjectionRepairRepositoryError::CorruptedRecord)
    };
    let id = ProjectionRepairOperationId::new(&hex_decode(find("operation")?)?)
        .map_err(|_| ProjectionRepairRepositoryError::CorruptedRecord)?;
    let workspace = WorkspaceId::new(&hex_decode(find("workspace")?)?)
        .map_err(|_| ProjectionRepairRepositoryError::CorruptedRecord)?;
    let document = DocumentId::new(&hex_decode(find("document")?)?)
        .map_err(|_| ProjectionRepairRepositoryError::CorruptedRecord)?;
    let attempt = find("attempt")?
        .parse()
        .map_err(|_| ProjectionRepairRepositoryError::CorruptedRecord)?;
    let completed = find("completed")?
        .parse()
        .map_err(|_| ProjectionRepairRepositoryError::CorruptedRecord)?;
    let total = find("total")?
        .parse()
        .map_err(|_| ProjectionRepairRepositoryError::CorruptedRecord)?;
    let progress = ProjectionRepairProgress::new(completed, total)
        .map_err(|_| ProjectionRepairRepositoryError::CorruptedRecord)?;
    ProjectionRepairOperation::restore(
        id,
        workspace,
        document,
        decode_state(find("state")?)?,
        attempt,
        progress,
    )
    .map_err(|_| ProjectionRepairRepositoryError::CorruptedRecord)
}

const fn encode_state(state: ProjectionRepairState) -> &'static str {
    match state {
        ProjectionRepairState::Queued => "queued",
        ProjectionRepairState::Running => "running",
        ProjectionRepairState::Publishing => "publishing",
        ProjectionRepairState::CancelPending => "cancel_pending",
        ProjectionRepairState::Succeeded => "succeeded",
        ProjectionRepairState::FailedRetryable => "failed_retryable",
        ProjectionRepairState::FailedFatal => "failed_fatal",
        ProjectionRepairState::Cancelled => "cancelled",
    }
}
fn decode_state(value: &str) -> Result<ProjectionRepairState, ProjectionRepairRepositoryError> {
    match value {
        "queued" => Ok(ProjectionRepairState::Queued),
        "running" => Ok(ProjectionRepairState::Running),
        "publishing" => Ok(ProjectionRepairState::Publishing),
        "cancel_pending" => Ok(ProjectionRepairState::CancelPending),
        "succeeded" => Ok(ProjectionRepairState::Succeeded),
        "failed_retryable" => Ok(ProjectionRepairState::FailedRetryable),
        "failed_fatal" => Ok(ProjectionRepairState::FailedFatal),
        "cancelled" => Ok(ProjectionRepairState::Cancelled),
        _ => Err(ProjectionRepairRepositoryError::CorruptedRecord),
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
fn hex_decode(value: &str) -> Result<String, ProjectionRepairRepositoryError> {
    if value.len() % 2 != 0 {
        return Err(ProjectionRepairRepositoryError::CorruptedRecord);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)
                .map_err(|_| ProjectionRepairRepositoryError::CorruptedRecord)?;
            u8::from_str_radix(text, 16)
                .map_err(|_| ProjectionRepairRepositoryError::CorruptedRecord)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| ProjectionRepairRepositoryError::CorruptedRecord)
}
