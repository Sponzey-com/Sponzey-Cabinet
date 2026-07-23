use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use cabinet_domain::document::DocumentId;
use cabinet_domain::document_revision::{
    DocumentExpectedCurrentVersion, DocumentMutationFingerprint, DocumentMutationKind,
    DocumentOperationId, DocumentOperationIdentity,
};
use cabinet_domain::projection_work::ProjectionChangeKind;
use cabinet_domain::version::{DocumentRevisionNumber, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_revision_commit::{
    DocumentOperationJournalClaim, DocumentOperationJournalError, DocumentOperationJournalPort,
    DocumentOperationJournalRecord, DocumentOperationJournalState,
    DocumentOperationTerminalFailure, DocumentRevisionCommitResult,
};
use serde::{Deserialize, Serialize};

use crate::local_atomic_file::write_text_atomically;

pub const DOCUMENT_OPERATION_JOURNAL_DIR: &str = "document-operation-journal";
const DOCUMENT_OPERATION_JOURNAL_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct LocalDocumentOperationJournal {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCommittedRestoreCandidate {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    version_id: VersionId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCommittedAttachmentMutationCandidate {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    version_id: VersionId,
    change_kind: ProjectionChangeKind,
}

impl LocalCommittedAttachmentMutationCandidate {
    pub const fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub const fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub const fn version_id(&self) -> &VersionId {
        &self.version_id
    }

    pub const fn change_kind(&self) -> ProjectionChangeKind {
        self.change_kind
    }
}

impl LocalCommittedRestoreCandidate {
    pub const fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub const fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub const fn version_id(&self) -> &VersionId {
        &self.version_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalRestoreCandidateScanError {
    InvalidLimit,
    StorageUnavailable,
    CorruptedJournal,
}

impl LocalDocumentOperationJournal {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn list_committed_restore_candidates(
        &self,
        limit: usize,
    ) -> Result<Vec<LocalCommittedRestoreCandidate>, LocalRestoreCandidateScanError> {
        if !(1..=1000).contains(&limit) {
            return Err(LocalRestoreCandidateScanError::InvalidLimit);
        }
        let entries = match fs::read_dir(self.journal_dir()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(LocalRestoreCandidateScanError::StorageUnavailable),
        };
        let mut paths = entries
            .map(|entry| {
                entry
                    .map(|value| value.path())
                    .map_err(|_| LocalRestoreCandidateScanError::StorageUnavailable)
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.retain(|path| path.extension().and_then(|value| value.to_str()) == Some("json"));
        paths.sort();
        let mut candidates = Vec::new();
        for path in paths {
            let content = fs::read_to_string(path)
                .map_err(|_| LocalRestoreCandidateScanError::StorageUnavailable)?;
            let file: JournalRecordFile = serde_json::from_str(&content)
                .map_err(|_| LocalRestoreCandidateScanError::CorruptedJournal)?;
            let operation_id = DocumentOperationId::new(&file.operation_id)
                .map_err(|_| LocalRestoreCandidateScanError::CorruptedJournal)?;
            let record = file
                .into_domain(&operation_id)
                .map_err(|_| LocalRestoreCandidateScanError::CorruptedJournal)?;
            if record.state() != DocumentOperationJournalState::Committed
                || record.identity().kind() != DocumentMutationKind::Restore
            {
                continue;
            }
            let result = record
                .result()
                .ok_or(LocalRestoreCandidateScanError::CorruptedJournal)?;
            candidates.push(LocalCommittedRestoreCandidate {
                workspace_id: record.identity().workspace_id().clone(),
                document_id: record.identity().document_id().clone(),
                version_id: result.version_id().clone(),
            });
            if candidates.len() == limit {
                break;
            }
        }
        Ok(candidates)
    }

    pub fn list_committed_attachment_mutation_candidates(
        &self,
        limit: usize,
    ) -> Result<Vec<LocalCommittedAttachmentMutationCandidate>, LocalRestoreCandidateScanError>
    {
        if !(1..=1000).contains(&limit) {
            return Err(LocalRestoreCandidateScanError::InvalidLimit);
        }
        let entries = match fs::read_dir(self.journal_dir()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(LocalRestoreCandidateScanError::StorageUnavailable),
        };
        let mut paths = entries
            .map(|entry| {
                entry
                    .map(|value| value.path())
                    .map_err(|_| LocalRestoreCandidateScanError::StorageUnavailable)
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.retain(|path| path.extension().and_then(|value| value.to_str()) == Some("json"));
        paths.sort();
        let mut candidates = Vec::new();
        for path in paths {
            let content = fs::read_to_string(path)
                .map_err(|_| LocalRestoreCandidateScanError::StorageUnavailable)?;
            let file: JournalRecordFile = serde_json::from_str(&content)
                .map_err(|_| LocalRestoreCandidateScanError::CorruptedJournal)?;
            let operation_id = DocumentOperationId::new(&file.operation_id)
                .map_err(|_| LocalRestoreCandidateScanError::CorruptedJournal)?;
            let record = file
                .into_domain(&operation_id)
                .map_err(|_| LocalRestoreCandidateScanError::CorruptedJournal)?;
            let change_kind = match record.identity().kind() {
                DocumentMutationKind::AttachAsset | DocumentMutationKind::LinkAsset => {
                    ProjectionChangeKind::AssetAttached
                }
                DocumentMutationKind::UnlinkAsset => ProjectionChangeKind::AssetDetached,
                _ => continue,
            };
            if record.state() != DocumentOperationJournalState::Committed {
                continue;
            }
            let result = record
                .result()
                .ok_or(LocalRestoreCandidateScanError::CorruptedJournal)?;
            candidates.push(LocalCommittedAttachmentMutationCandidate {
                workspace_id: record.identity().workspace_id().clone(),
                document_id: record.identity().document_id().clone(),
                version_id: result.version_id().clone(),
                change_kind,
            });
            if candidates.len() == limit {
                break;
            }
        }
        Ok(candidates)
    }

    fn journal_dir(&self) -> PathBuf {
        self.root.join(DOCUMENT_OPERATION_JOURNAL_DIR)
    }

    fn record_path(&self, operation_id: &DocumentOperationId) -> PathBuf {
        self.journal_dir().join(format!(
            "{}.json",
            encode_path_segment(operation_id.as_str())
        ))
    }

    fn lock_path(&self, operation_id: &DocumentOperationId) -> PathBuf {
        self.journal_dir().join(format!(
            "{}.lock",
            encode_path_segment(operation_id.as_str())
        ))
    }

    fn read_record(
        &self,
        operation_id: &DocumentOperationId,
    ) -> Result<Option<DocumentOperationJournalRecord>, DocumentOperationJournalError> {
        let content = match fs::read_to_string(self.record_path(operation_id)) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(DocumentOperationJournalError::StorageUnavailable),
        };
        let record: JournalRecordFile = serde_json::from_str(&content)
            .map_err(|_| DocumentOperationJournalError::CorruptedJournal)?;
        record.into_domain(operation_id).map(Some)
    }

    fn write_record(
        &self,
        record: &DocumentOperationJournalRecord,
    ) -> Result<(), DocumentOperationJournalError> {
        let file = JournalRecordFile::from_domain(record);
        let content = serde_json::to_string(&file)
            .map_err(|_| DocumentOperationJournalError::StorageUnavailable)?;
        write_text_atomically(&self.record_path(record.identity().operation_id()), content)
            .map(|_| ())
            .map_err(|_| DocumentOperationJournalError::StorageUnavailable)
    }

    fn acquire_lock(
        &self,
        operation_id: &DocumentOperationId,
    ) -> Result<OperationLock, DocumentOperationJournalError> {
        let lock_path = self.lock_path(operation_id);
        let parent = lock_path
            .parent()
            .ok_or(DocumentOperationJournalError::StorageUnavailable)?;
        fs::create_dir_all(parent)
            .map_err(|_| DocumentOperationJournalError::StorageUnavailable)?;
        for _ in 0..64 {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(_) => return Ok(OperationLock { path: lock_path }),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    thread::sleep(Duration::from_millis(1));
                }
                Err(_) => return Err(DocumentOperationJournalError::StorageUnavailable),
            }
        }
        Err(DocumentOperationJournalError::StorageUnavailable)
    }
}

impl DocumentOperationJournalPort for LocalDocumentOperationJournal {
    fn load_operation(
        &self,
        operation_id: &DocumentOperationId,
    ) -> Result<Option<DocumentOperationJournalRecord>, DocumentOperationJournalError> {
        self.read_record(operation_id)
    }

    fn claim_operation(
        &mut self,
        identity: DocumentOperationIdentity,
    ) -> Result<DocumentOperationJournalClaim, DocumentOperationJournalError> {
        let _lock = self.acquire_lock(identity.operation_id())?;
        if let Some(existing) = self.read_record(identity.operation_id())? {
            if existing.identity() != &identity {
                return Err(DocumentOperationJournalError::IdentityConflict);
            }
            return Ok(DocumentOperationJournalClaim::Existing(existing));
        }
        self.write_record(&DocumentOperationJournalRecord::claimed(identity))?;
        Ok(DocumentOperationJournalClaim::Claimed)
    }

    fn complete_operation(
        &mut self,
        operation_id: &DocumentOperationId,
        result: DocumentRevisionCommitResult,
    ) -> Result<(), DocumentOperationJournalError> {
        let _lock = self.acquire_lock(operation_id)?;
        let record = self
            .read_record(operation_id)?
            .ok_or(DocumentOperationJournalError::NotClaimed)?;
        let completed = record.complete(result)?;
        self.write_record(&completed)
    }

    fn fail_operation(
        &mut self,
        operation_id: &DocumentOperationId,
        failure: DocumentOperationTerminalFailure,
    ) -> Result<(), DocumentOperationJournalError> {
        let _lock = self.acquire_lock(operation_id)?;
        let record = self
            .read_record(operation_id)?
            .ok_or(DocumentOperationJournalError::NotClaimed)?;
        let failed = record.fail(failure)?;
        self.write_record(&failed)
    }
}

struct OperationLock {
    path: PathBuf,
}

impl Drop for OperationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalRecordFile {
    schema_version: u32,
    state: String,
    operation_id: String,
    workspace_id: String,
    document_id: String,
    mutation_kind: String,
    expected_current_kind: String,
    expected_current_version: Option<String>,
    request_fingerprint: Option<String>,
    result_version_id: Option<String>,
    result_revision_number: Option<u64>,
    failure_code: Option<String>,
}

impl JournalRecordFile {
    fn from_domain(record: &DocumentOperationJournalRecord) -> Self {
        let (expected_current_kind, expected_current_version) =
            match record.identity().expected_current() {
                DocumentExpectedCurrentVersion::MustNotExist => ("must_not_exist", None),
                DocumentExpectedCurrentVersion::MustMatch(version_id) => {
                    ("must_match", Some(version_id.as_str().to_string()))
                }
            };
        let (result_version_id, result_revision_number) = record
            .result()
            .map(|result| {
                (
                    Some(result.version_id().as_str().to_string()),
                    Some(result.revision_number().value()),
                )
            })
            .unwrap_or((None, None));
        Self {
            schema_version: DOCUMENT_OPERATION_JOURNAL_SCHEMA_VERSION,
            state: match record.state() {
                DocumentOperationJournalState::Claimed => "claimed",
                DocumentOperationJournalState::Committed => "committed",
                DocumentOperationJournalState::Failed => "failed",
            }
            .to_string(),
            operation_id: record.identity().operation_id().as_str().to_string(),
            workspace_id: record.identity().workspace_id().as_str().to_string(),
            document_id: record.identity().document_id().as_str().to_string(),
            mutation_kind: mutation_kind_name(record.identity().kind()).to_string(),
            expected_current_kind: expected_current_kind.to_string(),
            expected_current_version,
            request_fingerprint: record
                .identity()
                .request_fingerprint()
                .map(|fingerprint| fingerprint.as_str().to_string()),
            result_version_id,
            result_revision_number,
            failure_code: record
                .failure()
                .map(|failure| failure_name(failure).to_string()),
        }
    }

    fn into_domain(
        self,
        requested_operation_id: &DocumentOperationId,
    ) -> Result<DocumentOperationJournalRecord, DocumentOperationJournalError> {
        if self.schema_version != DOCUMENT_OPERATION_JOURNAL_SCHEMA_VERSION {
            return Err(DocumentOperationJournalError::CorruptedJournal);
        }
        let operation_id = DocumentOperationId::new(&self.operation_id)
            .map_err(|_| DocumentOperationJournalError::CorruptedJournal)?;
        if &operation_id != requested_operation_id {
            return Err(DocumentOperationJournalError::CorruptedJournal);
        }
        let kind = parse_mutation_kind(&self.mutation_kind)?;
        let expected_current = match (
            self.expected_current_kind.as_str(),
            self.expected_current_version,
        ) {
            ("must_not_exist", None) => DocumentExpectedCurrentVersion::MustNotExist,
            ("must_match", Some(version_id)) => DocumentExpectedCurrentVersion::MustMatch(
                VersionId::new(&version_id)
                    .map_err(|_| DocumentOperationJournalError::CorruptedJournal)?,
            ),
            _ => return Err(DocumentOperationJournalError::CorruptedJournal),
        };
        let mut identity = DocumentOperationIdentity::new(
            operation_id,
            WorkspaceId::new(&self.workspace_id)
                .map_err(|_| DocumentOperationJournalError::CorruptedJournal)?,
            DocumentId::new(&self.document_id)
                .map_err(|_| DocumentOperationJournalError::CorruptedJournal)?,
            kind,
            expected_current,
        )
        .map_err(|_| DocumentOperationJournalError::CorruptedJournal)?;
        if let Some(fingerprint) = self.request_fingerprint {
            identity = identity.with_request_fingerprint(
                DocumentMutationFingerprint::new(&fingerprint)
                    .map_err(|_| DocumentOperationJournalError::CorruptedJournal)?,
            );
        }
        let record = DocumentOperationJournalRecord::claimed(identity);
        match (
            self.state.as_str(),
            self.result_version_id,
            self.result_revision_number,
            self.failure_code,
        ) {
            ("claimed", None, None, None) => Ok(record),
            ("committed", Some(version_id), Some(revision_number), None) => {
                record.complete(DocumentRevisionCommitResult::new(
                    VersionId::new(&version_id)
                        .map_err(|_| DocumentOperationJournalError::CorruptedJournal)?,
                    DocumentRevisionNumber::new(revision_number)
                        .map_err(|_| DocumentOperationJournalError::CorruptedJournal)?,
                ))
            }
            ("failed", None, None, Some(failure)) => record.fail(parse_failure(&failure)?),
            _ => Err(DocumentOperationJournalError::CorruptedJournal),
        }
    }
}

const fn failure_name(failure: DocumentOperationTerminalFailure) -> &'static str {
    match failure {
        DocumentOperationTerminalFailure::Conflict => "conflict",
        DocumentOperationTerminalFailure::InvalidRequest => "invalid_request",
    }
}

fn parse_failure(
    value: &str,
) -> Result<DocumentOperationTerminalFailure, DocumentOperationJournalError> {
    match value {
        "conflict" => Ok(DocumentOperationTerminalFailure::Conflict),
        "invalid_request" => Ok(DocumentOperationTerminalFailure::InvalidRequest),
        _ => Err(DocumentOperationJournalError::CorruptedJournal),
    }
}

const fn mutation_kind_name(kind: DocumentMutationKind) -> &'static str {
    match kind {
        DocumentMutationKind::Create => "create",
        DocumentMutationKind::Update => "update",
        DocumentMutationKind::AttachAsset => "attach_asset",
        DocumentMutationKind::LinkAsset => "link_asset",
        DocumentMutationKind::UnlinkAsset => "unlink_asset",
        DocumentMutationKind::Restore => "restore",
    }
}

fn parse_mutation_kind(value: &str) -> Result<DocumentMutationKind, DocumentOperationJournalError> {
    match value {
        "create" => Ok(DocumentMutationKind::Create),
        "update" => Ok(DocumentMutationKind::Update),
        "attach_asset" => Ok(DocumentMutationKind::AttachAsset),
        "link_asset" => Ok(DocumentMutationKind::LinkAsset),
        "unlink_asset" => Ok(DocumentMutationKind::UnlinkAsset),
        "restore" => Ok(DocumentMutationKind::Restore),
        _ => Err(DocumentOperationJournalError::CorruptedJournal),
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
