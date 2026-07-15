use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use cabinet_domain::document::DocumentId;
use cabinet_domain::projection_work::{
    ProjectionChangeKind, ProjectionKind, ProjectionWork, ProjectionWorkIdentity,
    ProjectionWorkState,
};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::projection_work::{
    ProjectionEnqueueOutcome, ProjectionWorkRepository, ProjectionWorkRepositoryError,
};

use crate::local_atomic_file::write_text_atomically;

const CURRENT_SCHEMA_HEADER: &str = "schema\t2";
const LEGACY_SCHEMA_HEADER: &str = "schema\t1";

#[derive(Debug, Clone)]
pub struct DurableProjectionWorkRepository {
    root: PathBuf,
}

impl DurableProjectionWorkRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn records_root(&self) -> PathBuf {
        self.root.join("operations").join("projection")
    }

    fn record_path(&self, identity: &ProjectionWorkIdentity) -> PathBuf {
        let key = identity.idempotency_key();
        let first = checksum_with_seed(key.as_bytes(), 0xcbf29ce484222325);
        let second = checksum_with_seed(key.as_bytes(), 0x84222325cbf29ce4);
        self.records_root()
            .join(format!("v2-{first:016x}{second:016x}.work"))
    }

    fn legacy_record_path(&self, identity: &ProjectionWorkIdentity) -> Option<PathBuf> {
        let file_name = format!("{}.work", hex_encode(&identity.idempotency_key()));
        (file_name.len() <= 255).then(|| self.records_root().join(file_name))
    }
}

impl ProjectionWorkRepository for DurableProjectionWorkRepository {
    fn enqueue(
        &mut self,
        work: ProjectionWork,
    ) -> Result<ProjectionEnqueueOutcome, ProjectionWorkRepositoryError> {
        let path = self.record_path(work.identity());
        if let Some(existing) = self.get(work.identity())? {
            if existing.identity() != work.identity() {
                return Err(ProjectionWorkRepositoryError::Conflict);
            }
            return Ok(ProjectionEnqueueOutcome::AlreadyExists);
        }
        write_work(&path, &work)?;
        Ok(ProjectionEnqueueOutcome::Enqueued)
    }

    fn get(
        &self,
        identity: &ProjectionWorkIdentity,
    ) -> Result<Option<ProjectionWork>, ProjectionWorkRepositoryError> {
        match read_work(&self.record_path(identity)) {
            Ok(work) if work.identity() == identity => Ok(Some(work)),
            Ok(_) => Err(ProjectionWorkRepositoryError::Conflict),
            Err(ReadWorkError::NotFound) => {
                let Some(legacy_path) = self.legacy_record_path(identity) else {
                    return Ok(None);
                };
                match read_work(&legacy_path) {
                    Ok(work) if work.identity() == identity => Ok(Some(work)),
                    Ok(_) => Err(ProjectionWorkRepositoryError::Conflict),
                    Err(ReadWorkError::NotFound) => Ok(None),
                    Err(ReadWorkError::Repository(error)) => Err(error),
                }
            }
            Err(ReadWorkError::Repository(error)) => Err(error),
        }
    }

    fn replace(
        &mut self,
        work: ProjectionWork,
        expected_state: ProjectionWorkState,
    ) -> Result<(), ProjectionWorkRepositoryError> {
        let primary_path = self.record_path(work.identity());
        let (path, current) = match read_work(&primary_path) {
            Ok(current) => (primary_path, current),
            Err(ReadWorkError::NotFound) => {
                let Some(legacy_path) = self.legacy_record_path(work.identity()) else {
                    return Err(ProjectionWorkRepositoryError::NotFound);
                };
                match read_work(&legacy_path) {
                    Ok(current) => (legacy_path, current),
                    Err(ReadWorkError::NotFound) => {
                        return Err(ProjectionWorkRepositoryError::NotFound);
                    }
                    Err(ReadWorkError::Repository(error)) => return Err(error),
                }
            }
            Err(ReadWorkError::Repository(error)) => return Err(error),
        };
        if current.identity() != work.identity() {
            return Err(ProjectionWorkRepositoryError::Conflict);
        }
        if current.state() != expected_state {
            return Err(ProjectionWorkRepositoryError::Conflict);
        }
        write_work(&path, &work)
    }

    fn list_resumable(
        &self,
        limit: usize,
    ) -> Result<Vec<ProjectionWork>, ProjectionWorkRepositoryError> {
        if limit == 0 {
            return Err(ProjectionWorkRepositoryError::InvalidLimit);
        }
        let entries = match fs::read_dir(self.records_root()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(ProjectionWorkRepositoryError::StorageUnavailable),
        };
        let mut paths = entries
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|_| ProjectionWorkRepositoryError::StorageUnavailable)
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        let mut resumable = Vec::new();
        for path in paths {
            if path.extension().and_then(|value| value.to_str()) != Some("work") {
                continue;
            }
            let work = match read_work(&path) {
                Ok(work) => work,
                Err(ReadWorkError::NotFound) => continue,
                Err(ReadWorkError::Repository(error)) => return Err(error),
            };
            if work.state().is_resumable() {
                resumable.push(work);
                if resumable.len() == limit {
                    break;
                }
            }
        }
        Ok(resumable)
    }
}

fn write_work(path: &Path, work: &ProjectionWork) -> Result<(), ProjectionWorkRepositoryError> {
    write_text_atomically(path, encode_work(work))
        .map(|_| ())
        .map_err(|_| ProjectionWorkRepositoryError::StorageUnavailable)
}

enum ReadWorkError {
    NotFound,
    Repository(ProjectionWorkRepositoryError),
}

fn read_work(path: &Path) -> Result<ProjectionWork, ReadWorkError> {
    let text = fs::read_to_string(path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            ReadWorkError::NotFound
        } else {
            ReadWorkError::Repository(ProjectionWorkRepositoryError::StorageUnavailable)
        }
    })?;
    decode_work(&text).map_err(ReadWorkError::Repository)
}

fn encode_work(work: &ProjectionWork) -> String {
    let identity = work.identity();
    let payload = format!(
        "workspace\t{}\ndocument\t{}\nversion\t{}\nkind\t{}\nchange\t{}\nstate\t{}\nattempt\t{}\n",
        hex_encode(identity.workspace_id().as_str()),
        hex_encode(identity.document_id().as_str()),
        hex_encode(identity.version_id().as_str()),
        identity.kind().as_str(),
        identity.change_kind().as_str(),
        encode_state(work.state()),
        work.attempt(),
    );
    format!(
        "{CURRENT_SCHEMA_HEADER}\nchecksum\t{:016x}\n{payload}",
        checksum(payload.as_bytes())
    )
}

fn decode_work(text: &str) -> Result<ProjectionWork, ProjectionWorkRepositoryError> {
    let mut lines = text.lines();
    let schema = match lines.next() {
        Some(CURRENT_SCHEMA_HEADER) => 2,
        Some(LEGACY_SCHEMA_HEADER) => 1,
        Some(line) if line.starts_with("schema\t") => {
            return Err(ProjectionWorkRepositoryError::UnsupportedSchema);
        }
        _ => return Err(ProjectionWorkRepositoryError::CorruptedRecord),
    };
    let expected = lines
        .next()
        .and_then(|line| line.strip_prefix("checksum\t"))
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .ok_or(ProjectionWorkRepositoryError::CorruptedRecord)?;
    let payload = format!("{}\n", lines.collect::<Vec<_>>().join("\n"));
    if checksum(payload.as_bytes()) != expected {
        return Err(ProjectionWorkRepositoryError::CorruptedRecord);
    }
    let fields = payload
        .lines()
        .map(|line| line.split_once('\t'))
        .collect::<Option<Vec<_>>>()
        .ok_or(ProjectionWorkRepositoryError::CorruptedRecord)?;
    if fields.len() != if schema == 2 { 7 } else { 6 } {
        return Err(ProjectionWorkRepositoryError::CorruptedRecord);
    }
    let find = |name: &str| {
        fields
            .iter()
            .find_map(|(key, value)| (*key == name).then_some(*value))
            .ok_or(ProjectionWorkRepositoryError::CorruptedRecord)
    };
    let workspace = WorkspaceId::new(&hex_decode(find("workspace")?)?)
        .map_err(|_| ProjectionWorkRepositoryError::CorruptedRecord)?;
    let document = DocumentId::new(&hex_decode(find("document")?)?)
        .map_err(|_| ProjectionWorkRepositoryError::CorruptedRecord)?;
    let version = VersionId::new(&hex_decode(find("version")?)?)
        .map_err(|_| ProjectionWorkRepositoryError::CorruptedRecord)?;
    let kind = decode_kind(find("kind")?)?;
    let change_kind = if schema == 2 {
        decode_change_kind(find("change")?)?
    } else {
        ProjectionChangeKind::Updated
    };
    let identity =
        ProjectionWorkIdentity::for_change(workspace, document, version, kind, change_kind);
    let attempt = find("attempt")?
        .parse::<u32>()
        .map_err(|_| ProjectionWorkRepositoryError::CorruptedRecord)?;
    ProjectionWork::restore(identity, decode_state(find("state")?)?, attempt)
        .map_err(|_| ProjectionWorkRepositoryError::CorruptedRecord)
}

const fn encode_state(state: ProjectionWorkState) -> &'static str {
    match state {
        ProjectionWorkState::Pending => "pending",
        ProjectionWorkState::Indexing => "indexing",
        ProjectionWorkState::Ready => "ready",
        ProjectionWorkState::RetryScheduled => "retry_scheduled",
        ProjectionWorkState::Failed => "failed",
    }
}

fn decode_state(value: &str) -> Result<ProjectionWorkState, ProjectionWorkRepositoryError> {
    match value {
        "pending" => Ok(ProjectionWorkState::Pending),
        "indexing" => Ok(ProjectionWorkState::Indexing),
        "ready" => Ok(ProjectionWorkState::Ready),
        "retry_scheduled" => Ok(ProjectionWorkState::RetryScheduled),
        "failed" => Ok(ProjectionWorkState::Failed),
        _ => Err(ProjectionWorkRepositoryError::CorruptedRecord),
    }
}

fn decode_kind(value: &str) -> Result<ProjectionKind, ProjectionWorkRepositoryError> {
    match value {
        "search" => Ok(ProjectionKind::Search),
        "links" => Ok(ProjectionKind::Links),
        "graph" => Ok(ProjectionKind::Graph),
        _ => Err(ProjectionWorkRepositoryError::CorruptedRecord),
    }
}

fn decode_change_kind(value: &str) -> Result<ProjectionChangeKind, ProjectionWorkRepositoryError> {
    match value {
        "created" => Ok(ProjectionChangeKind::Created),
        "updated" => Ok(ProjectionChangeKind::Updated),
        "restored" => Ok(ProjectionChangeKind::Restored),
        "renamed" => Ok(ProjectionChangeKind::Renamed),
        "deleted" => Ok(ProjectionChangeKind::Deleted),
        "asset_attached" => Ok(ProjectionChangeKind::AssetAttached),
        "asset_detached" => Ok(ProjectionChangeKind::AssetDetached),
        _ => Err(ProjectionWorkRepositoryError::CorruptedRecord),
    }
}

fn checksum(bytes: &[u8]) -> u64 {
    checksum_with_seed(bytes, 0xcbf29ce484222325)
}

fn checksum_with_seed(bytes: &[u8], seed: u64) -> u64 {
    bytes.iter().fold(seed, |hash, byte| {
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

fn hex_decode(value: &str) -> Result<String, ProjectionWorkRepositoryError> {
    if value.len() % 2 != 0 {
        return Err(ProjectionWorkRepositoryError::CorruptedRecord);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)
                .map_err(|_| ProjectionWorkRepositoryError::CorruptedRecord)?;
            u8::from_str_radix(text, 16).map_err(|_| ProjectionWorkRepositoryError::CorruptedRecord)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| ProjectionWorkRepositoryError::CorruptedRecord)
}
