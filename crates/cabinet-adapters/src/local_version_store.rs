use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::DocumentOperationId;
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentRevisionNumber, DocumentRevisionNumberState,
    DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId, VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::committed_version_record_reader::{
    CommittedVersionRecordReadError, CommittedVersionRecordReader,
};
use cabinet_ports::version_preparation::{
    PreparedVersion, VersionPreparationError, VersionPreparationOutcome, VersionPreparationPort,
};
use cabinet_ports::version_publication::{
    PublishedVersion, VersionPublicationError, VersionPublicationPort,
};
use cabinet_ports::version_store::{
    HistoryCursor, HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};
use serde::{Deserialize, Serialize};

use crate::local_atomic_file::write_text_atomically;

pub const VERSION_DOCUMENTS_DIR: &str = "documents";
pub const VERSION_HISTORY_FILE: &str = "history.txt";
pub const VERSION_SNAPSHOTS_DIR: &str = "snapshots";
pub const VERSION_ENTRY_FILE: &str = "entry.txt";
pub const VERSION_BODY_FILE: &str = "body.md";
pub const VERSION_ATTACHMENTS_FILE: &str = "attachments.json";
pub const VERSION_PREPARED_DIR: &str = "prepared";
pub const VERSION_PREPARED_MANIFEST_FILE: &str = "manifest.json";
const VERSION_ATTACHMENTS_SCHEMA_VERSION: u32 = 1;
const VERSION_PREPARED_SCHEMA_VERSION: u32 = 1;
const DEFAULT_VERSION_BODY_MAX_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AttachmentSidecar {
    schema_version: u32,
    state: String,
    references: Vec<AttachmentReferenceSidecar>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AttachmentReferenceSidecar {
    asset_id: String,
    label: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedVersionManifest {
    schema_version: u32,
    workspace_id: String,
    operation_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevisionNumberMigrationReport {
    documents_scanned: usize,
    entries_scanned: usize,
    entries_assigned: usize,
}

impl RevisionNumberMigrationReport {
    pub const fn documents_scanned(self) -> usize {
        self.documents_scanned
    }

    pub const fn entries_scanned(self) -> usize {
        self.entries_scanned
    }

    pub const fn entries_assigned(self) -> usize {
        self.entries_assigned
    }
}

struct RevisionNumberMigrationWrite {
    entry_path: PathBuf,
    entry: VersionEntry,
}

#[derive(Debug, Clone)]
pub struct LocalVersionStore {
    version_store_root: PathBuf,
    body_policy: DocumentBodyPolicy,
    clock: fn() -> u64,
}

impl LocalVersionStore {
    pub fn new(version_store_root: PathBuf) -> Self {
        Self {
            version_store_root,
            body_policy: DocumentBodyPolicy::new(DEFAULT_VERSION_BODY_MAX_BYTES)
                .expect("default version body policy must be valid"),
            clock: system_epoch_ms,
        }
    }

    pub fn with_body_policy(version_store_root: PathBuf, body_policy: DocumentBodyPolicy) -> Self {
        Self {
            version_store_root,
            body_policy,
            clock: system_epoch_ms,
        }
    }

    pub fn with_body_policy_and_clock(
        version_store_root: PathBuf,
        body_policy: DocumentBodyPolicy,
        clock: fn() -> u64,
    ) -> Self {
        Self {
            version_store_root,
            body_policy,
            clock,
        }
    }

    pub fn migrate_revision_numbers(
        &self,
    ) -> Result<RevisionNumberMigrationReport, VersionStoreError> {
        let (report, writes) = self.revision_number_migration_plan()?;
        for write in writes {
            write_file_atomically(
                write.entry_path,
                entry_content(&write.entry, write.entry.created_at_epoch_ms()),
            )?;
        }
        Ok(report)
    }

    fn revision_number_migration_plan(
        &self,
    ) -> Result<
        (
            RevisionNumberMigrationReport,
            Vec<RevisionNumberMigrationWrite>,
        ),
        VersionStoreError,
    > {
        if !self.version_store_root.exists() {
            return Ok((
                RevisionNumberMigrationReport {
                    documents_scanned: 0,
                    entries_scanned: 0,
                    entries_assigned: 0,
                },
                Vec::new(),
            ));
        }

        let mut document_dirs = Vec::new();
        for workspace_dir in child_directories(&self.version_store_root)? {
            let documents_dir = workspace_dir.join(VERSION_DOCUMENTS_DIR);
            if documents_dir.exists() {
                document_dirs.extend(child_directories(&documents_dir)?);
            }
        }
        document_dirs.sort();

        let mut documents_scanned = 0;
        let mut entries_scanned = 0;
        let mut writes = Vec::new();
        for document_dir in document_dirs {
            let history_path = document_dir.join(VERSION_HISTORY_FILE);
            if !history_path.exists() {
                continue;
            }
            documents_scanned += 1;
            let history = fs::read_to_string(&history_path)
                .map_err(|_| VersionStoreError::CorruptedHistory)?;
            let mut seen_version_ids = Vec::new();
            for (index, line) in history.lines().enumerate() {
                let raw_version_id = line.trim();
                if raw_version_id.is_empty()
                    || seen_version_ids
                        .iter()
                        .any(|seen: &String| seen == raw_version_id)
                {
                    return Err(VersionStoreError::CorruptedHistory);
                }
                seen_version_ids.push(raw_version_id.to_string());
                let version_id = VersionId::new(raw_version_id)
                    .map_err(|_| VersionStoreError::CorruptedHistory)?;
                let entry_path = document_dir
                    .join(VERSION_SNAPSHOTS_DIR)
                    .join(encode_path_segment(version_id.as_str()))
                    .join(VERSION_ENTRY_FILE);
                let entry = read_entry(&entry_path)?;
                let document_dir_name = document_dir
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or(VersionStoreError::CorruptedHistory)?;
                if entry.version_id() != &version_id
                    || encode_path_segment(entry.document_id().as_str()) != document_dir_name
                {
                    return Err(VersionStoreError::CorruptedHistory);
                }
                entries_scanned += 1;
                let expected = DocumentRevisionNumber::new(
                    u64::try_from(index)
                        .ok()
                        .and_then(|value| value.checked_add(1))
                        .ok_or(VersionStoreError::CorruptedHistory)?,
                )
                .map_err(|_| VersionStoreError::CorruptedHistory)?;
                match entry.revision_number_state() {
                    DocumentRevisionNumberState::Assigned(actual) if actual != &expected => {
                        return Err(VersionStoreError::CorruptedHistory);
                    }
                    DocumentRevisionNumberState::Assigned(_) => {}
                    DocumentRevisionNumberState::LegacyUnassigned => {
                        let entry = entry
                            .with_revision_number(expected)
                            .map_err(|_| VersionStoreError::CorruptedHistory)?;
                        writes.push(RevisionNumberMigrationWrite { entry_path, entry });
                    }
                }
            }
        }

        Ok((
            RevisionNumberMigrationReport {
                documents_scanned,
                entries_scanned,
                entries_assigned: writes.len(),
            },
            writes,
        ))
    }

    fn document_dir(&self, workspace_id: &WorkspaceId, document_id: &DocumentId) -> PathBuf {
        self.version_store_root
            .join(encode_path_segment(workspace_id.as_str()))
            .join(VERSION_DOCUMENTS_DIR)
            .join(encode_path_segment(document_id.as_str()))
    }

    fn history_path(&self, workspace_id: &WorkspaceId, document_id: &DocumentId) -> PathBuf {
        self.document_dir(workspace_id, document_id)
            .join(VERSION_HISTORY_FILE)
    }

    fn version_dir(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> PathBuf {
        self.document_dir(workspace_id, document_id)
            .join(VERSION_SNAPSHOTS_DIR)
            .join(encode_path_segment(version_id.as_str()))
    }

    fn prepared_dir(
        &self,
        workspace_id: &WorkspaceId,
        operation_id: &DocumentOperationId,
    ) -> PathBuf {
        self.version_store_root
            .join(encode_path_segment(workspace_id.as_str()))
            .join(VERSION_PREPARED_DIR)
            .join(encode_path_segment(operation_id.as_str()))
    }

    fn entry_path(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> PathBuf {
        self.version_dir(workspace_id, document_id, version_id)
            .join(VERSION_ENTRY_FILE)
    }

    fn body_path(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> PathBuf {
        self.version_dir(workspace_id, document_id, version_id)
            .join(VERSION_BODY_FILE)
    }

    fn attachments_path(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> PathBuf {
        self.version_dir(workspace_id, document_id, version_id)
            .join(VERSION_ATTACHMENTS_FILE)
    }

    fn read_entry_by_version(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<VersionEntry, VersionStoreError> {
        let entry = read_entry(&self.entry_path(workspace_id, document_id, version_id))?;
        if entry.document_id() != document_id || entry.version_id() != version_id {
            return Err(VersionStoreError::CorruptedHistory);
        }
        Ok(entry)
    }

    fn next_revision_number(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<DocumentRevisionNumber, VersionStoreError> {
        let history_path = self.history_path(workspace_id, document_id);
        let history = match fs::read_to_string(history_path) {
            Ok(history) => history,
            Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
            Err(_) => return Err(VersionStoreError::StorageUnavailable),
        };
        let mut count = 0_u64;
        let mut seen_version_ids = Vec::new();
        for line in history.lines() {
            let raw_version_id = line.trim();
            if raw_version_id.is_empty()
                || seen_version_ids
                    .iter()
                    .any(|seen: &String| seen == raw_version_id)
            {
                return Err(VersionStoreError::CorruptedHistory);
            }
            seen_version_ids.push(raw_version_id.to_string());
            count = count
                .checked_add(1)
                .ok_or(VersionStoreError::CorruptedHistory)?;
            let version_id =
                VersionId::new(raw_version_id).map_err(|_| VersionStoreError::CorruptedHistory)?;
            let entry = self.read_entry_by_version(workspace_id, document_id, &version_id)?;
            if entry.revision_number().map(|number| number.value()) != Some(count) {
                return Err(VersionStoreError::CorruptedHistory);
            }
        }

        DocumentRevisionNumber::new(
            count
                .checked_add(1)
                .ok_or(VersionStoreError::CorruptedHistory)?,
        )
        .map_err(|_| VersionStoreError::CorruptedHistory)
    }
}

impl VersionPreparationPort for LocalVersionStore {
    fn prepare_version(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &DocumentOperationId,
        record: VersionRecord,
    ) -> Result<VersionPreparationOutcome, VersionPreparationError> {
        if record.entry().revision_number().is_none()
            || record.entry().created_at_epoch_ms().is_none()
        {
            return Err(VersionPreparationError::InvalidRecord);
        }

        let prepared_dir = self.prepared_dir(workspace_id, operation_id);
        if prepared_dir.exists() {
            return existing_prepared_outcome(self, workspace_id, operation_id, &record);
        }

        let parent = prepared_dir
            .parent()
            .ok_or(VersionPreparationError::StorageUnavailable)?;
        fs::create_dir_all(parent).map_err(|_| VersionPreparationError::StorageUnavailable)?;
        let temporary_dir = create_prepared_temporary_dir(parent, operation_id)?;
        let write_result =
            write_prepared_record(&temporary_dir, workspace_id, operation_id, &record);
        if let Err(error) = write_result {
            let _ = fs::remove_dir_all(&temporary_dir);
            return Err(error);
        }

        if fs::rename(&temporary_dir, &prepared_dir).is_err() {
            let _ = fs::remove_dir_all(&temporary_dir);
            if prepared_dir.exists() {
                return existing_prepared_outcome(self, workspace_id, operation_id, &record);
            }
            return Err(VersionPreparationError::StorageUnavailable);
        }

        Ok(VersionPreparationOutcome::Prepared(PreparedVersion::new(
            operation_id.clone(),
            record,
        )))
    }

    fn load_prepared(
        &self,
        workspace_id: &WorkspaceId,
        operation_id: &DocumentOperationId,
    ) -> Result<Option<PreparedVersion>, VersionPreparationError> {
        let prepared_dir = self.prepared_dir(workspace_id, operation_id);
        if !prepared_dir.exists() {
            return Ok(None);
        }

        read_prepared_record(&prepared_dir, workspace_id, operation_id, self.body_policy).map(Some)
    }

    fn discard_prepared(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &DocumentOperationId,
    ) -> Result<(), VersionPreparationError> {
        let prepared_dir = self.prepared_dir(workspace_id, operation_id);
        match fs::remove_dir_all(prepared_dir) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(_) => Err(VersionPreparationError::StorageUnavailable),
        }
    }
}

impl VersionPublicationPort for LocalVersionStore {
    fn publish_prepared(
        &mut self,
        workspace_id: &WorkspaceId,
        operation_id: &DocumentOperationId,
    ) -> Result<PublishedVersion, VersionPublicationError> {
        let prepared = self
            .load_prepared(workspace_id, operation_id)
            .map_err(map_preparation_to_publication)?
            .ok_or(VersionPublicationError::NotPrepared)?;
        let record = prepared.record();
        let revision_number = record
            .entry()
            .revision_number()
            .ok_or(VersionPublicationError::CorruptedPublication)?;
        let history_ids =
            read_publication_history(&self.history_path(workspace_id, record.document_id()))?;

        if let Some(position) = history_ids
            .iter()
            .position(|version_id| version_id == record.version_id())
        {
            let expected_position = usize::try_from(revision_number.value())
                .ok()
                .and_then(|value| value.checked_sub(1))
                .ok_or(VersionPublicationError::CorruptedPublication)?;
            if position != expected_position {
                return Err(VersionPublicationError::CorruptedPublication);
            }
            ensure_committed_record_matches(
                self,
                workspace_id,
                record,
                VersionPublicationError::Conflict,
            )?;
            return Ok(PublishedVersion::new(
                record.version_id().clone(),
                revision_number,
            ));
        }

        let expected_revision = u64::try_from(history_ids.len())
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(VersionPublicationError::CorruptedPublication)?;
        if revision_number.value() != expected_revision {
            return Err(VersionPublicationError::Conflict);
        }

        let final_dir = self.version_dir(workspace_id, record.document_id(), record.version_id());
        if final_dir.exists() {
            ensure_committed_record_matches(
                self,
                workspace_id,
                record,
                VersionPublicationError::Conflict,
            )?;
        } else {
            publish_committed_record_directory(&final_dir, operation_id, record)?;
        }

        append_history(
            self.history_path(workspace_id, record.document_id()),
            record.version_id(),
        )
        .map_err(|_| VersionPublicationError::StorageUnavailable)?;

        Ok(PublishedVersion::new(
            record.version_id().clone(),
            revision_number,
        ))
    }
}

impl VersionStore for LocalVersionStore {
    fn append_version(
        &mut self,
        workspace_id: &WorkspaceId,
        record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        let version_dir = self.version_dir(workspace_id, record.document_id(), record.version_id());
        if version_dir.exists() {
            return Err(VersionStoreError::Conflict);
        }
        let expected_revision = self.next_revision_number(workspace_id, record.document_id())?;
        let entry = match record.entry().revision_number_state() {
            DocumentRevisionNumberState::LegacyUnassigned => record
                .entry()
                .clone()
                .with_revision_number(expected_revision)
                .map_err(|_| VersionStoreError::Conflict)?,
            DocumentRevisionNumberState::Assigned(actual) if actual == &expected_revision => {
                record.entry().clone()
            }
            DocumentRevisionNumberState::Assigned(_) => return Err(VersionStoreError::Conflict),
        };

        write_file_atomically(
            self.entry_path(workspace_id, record.document_id(), record.version_id()),
            entry_content(&entry, Some((self.clock)())),
        )?;
        write_file_atomically(
            self.body_path(workspace_id, record.document_id(), record.version_id()),
            record.snapshot().body().as_str(),
        )?;
        if let Some(content) = attachment_sidecar_content(record.snapshot().attachment_state())? {
            write_file_atomically(
                self.attachments_path(workspace_id, record.document_id(), record.version_id()),
                content,
            )?;
        }
        append_history(
            self.history_path(workspace_id, record.document_id()),
            record.version_id(),
        )
    }

    fn get_version_snapshot(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        let version_dir = self.version_dir(workspace_id, document_id, version_id);
        if !version_dir.exists() {
            return Ok(None);
        }

        let entry = self.read_entry_by_version(workspace_id, document_id, version_id)?;
        let body = read_body(
            &self.body_path(workspace_id, document_id, version_id),
            self.body_policy,
        )?;
        let attachment_state =
            read_attachment_state(&self.attachments_path(workspace_id, document_id, version_id))?;
        Ok(Some(VersionSnapshot::with_attachment_state(
            entry.document_id().clone(),
            entry.snapshot_ref().clone(),
            body,
            attachment_state,
        )))
    }

    fn list_history(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        request: HistoryPageRequest,
    ) -> Result<HistoryPage, VersionStoreError> {
        let history_path = self.history_path(workspace_id, document_id);
        let file = match fs::File::open(history_path) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(HistoryPage::new(Vec::new(), None));
            }
            Err(_) => return Err(VersionStoreError::StorageUnavailable),
        };

        let start = request
            .cursor()
            .map(|cursor| cursor.as_str().parse::<usize>())
            .transpose()
            .map_err(|_| VersionStoreError::CorruptedHistory)?
            .unwrap_or(0);
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        let mut next_cursor = None;

        for (index, line) in reader.lines().enumerate() {
            if index < start {
                continue;
            }
            if entries.len() == request.limit() {
                next_cursor = Some(
                    HistoryCursor::new(&index.to_string())
                        .map_err(|_| VersionStoreError::CorruptedHistory)?,
                );
                break;
            }

            let line = line.map_err(|_| VersionStoreError::StorageUnavailable)?;
            let version_id =
                VersionId::new(line.trim()).map_err(|_| VersionStoreError::CorruptedHistory)?;
            entries.push(self.read_entry_by_version(workspace_id, document_id, &version_id)?);
        }

        Ok(HistoryPage::new(entries, next_cursor))
    }
}

impl CommittedVersionRecordReader for LocalVersionStore {
    fn get_committed_version_record(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<VersionRecord>, CommittedVersionRecordReadError> {
        if !self
            .version_dir(workspace_id, document_id, version_id)
            .exists()
        {
            return Ok(None);
        }
        let entry = self
            .read_entry_by_version(workspace_id, document_id, version_id)
            .map_err(map_version_record_read_error)?;
        if entry.revision_number().is_none() || entry.created_at_epoch_ms().is_none() {
            return Err(CommittedVersionRecordReadError::CorruptedRecord);
        }
        let snapshot = self
            .get_version_snapshot(workspace_id, document_id, version_id)
            .map_err(map_version_record_read_error)?
            .ok_or(CommittedVersionRecordReadError::CorruptedRecord)?;
        VersionRecord::new(entry, snapshot)
            .map(Some)
            .map_err(|_| CommittedVersionRecordReadError::CorruptedRecord)
    }
}

const fn map_version_record_read_error(
    error: VersionStoreError,
) -> CommittedVersionRecordReadError {
    match error {
        VersionStoreError::StorageUnavailable => {
            CommittedVersionRecordReadError::StorageUnavailable
        }
        VersionStoreError::MismatchedVersionSnapshot
        | VersionStoreError::InvalidHistoryPageLimit
        | VersionStoreError::InvalidHistoryCursor
        | VersionStoreError::CorruptedHistory
        | VersionStoreError::Conflict => CommittedVersionRecordReadError::CorruptedRecord,
    }
}

fn entry_content(entry: &VersionEntry, created_at_epoch_ms: Option<u64>) -> String {
    let mut content = format!(
        "version_id={}\ndocument_id={}\nsnapshot_ref={}\nauthor={}\nsummary={}\n",
        entry.version_id().as_str(),
        entry.document_id().as_str(),
        entry.snapshot_ref().as_str(),
        entry.author().as_str(),
        entry.summary().as_str(),
    );
    if let Some(created_at_epoch_ms) = created_at_epoch_ms {
        content.push_str(&format!("created_at_epoch_ms={created_at_epoch_ms}\n"));
    }
    if let Some(revision_number) = entry.revision_number() {
        content.push_str(&format!("revision_number={}\n", revision_number.value()));
    }
    content
}

fn existing_prepared_outcome(
    store: &LocalVersionStore,
    workspace_id: &WorkspaceId,
    operation_id: &DocumentOperationId,
    requested_record: &VersionRecord,
) -> Result<VersionPreparationOutcome, VersionPreparationError> {
    let existing = store
        .load_prepared(workspace_id, operation_id)?
        .ok_or(VersionPreparationError::CorruptedPrepared)?;
    if existing.record() != requested_record {
        return Err(VersionPreparationError::Conflict);
    }
    Ok(VersionPreparationOutcome::Existing(existing))
}

fn map_preparation_to_publication(error: VersionPreparationError) -> VersionPublicationError {
    match error {
        VersionPreparationError::StorageUnavailable => VersionPublicationError::StorageUnavailable,
        VersionPreparationError::InvalidRecord
        | VersionPreparationError::Conflict
        | VersionPreparationError::CorruptedPrepared => {
            VersionPublicationError::CorruptedPublication
        }
    }
}

fn read_publication_history(path: &Path) -> Result<Vec<VersionId>, VersionPublicationError> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(VersionPublicationError::StorageUnavailable),
    };
    let mut version_ids = Vec::new();
    for line in content.lines() {
        let value = line.trim();
        if value.is_empty() {
            return Err(VersionPublicationError::CorruptedPublication);
        }
        let version_id =
            VersionId::new(value).map_err(|_| VersionPublicationError::CorruptedPublication)?;
        if version_ids.iter().any(|existing| existing == &version_id) {
            return Err(VersionPublicationError::CorruptedPublication);
        }
        version_ids.push(version_id);
    }
    Ok(version_ids)
}

fn ensure_committed_record_matches(
    store: &LocalVersionStore,
    workspace_id: &WorkspaceId,
    expected: &VersionRecord,
    mismatch_error: VersionPublicationError,
) -> Result<(), VersionPublicationError> {
    let directory = store.version_dir(workspace_id, expected.document_id(), expected.version_id());
    if !directory.exists() {
        return Err(VersionPublicationError::CorruptedPublication);
    }
    let actual = read_committed_record(&directory, store.body_policy)?;
    if &actual != expected {
        return Err(mismatch_error);
    }
    Ok(())
}

fn publish_committed_record_directory(
    final_dir: &Path,
    operation_id: &DocumentOperationId,
    record: &VersionRecord,
) -> Result<(), VersionPublicationError> {
    let parent = final_dir
        .parent()
        .ok_or(VersionPublicationError::StorageUnavailable)?;
    fs::create_dir_all(parent).map_err(|_| VersionPublicationError::StorageUnavailable)?;
    let temporary_dir = create_publication_temporary_dir(parent, operation_id)?;
    if write_version_record_payload(&temporary_dir, record).is_err() {
        let _ = fs::remove_dir_all(&temporary_dir);
        return Err(VersionPublicationError::StorageUnavailable);
    }
    if fs::rename(&temporary_dir, final_dir).is_err() {
        let _ = fs::remove_dir_all(&temporary_dir);
        return Err(if final_dir.exists() {
            VersionPublicationError::Conflict
        } else {
            VersionPublicationError::StorageUnavailable
        });
    }
    Ok(())
}

fn create_publication_temporary_dir(
    parent: &Path,
    operation_id: &DocumentOperationId,
) -> Result<PathBuf, VersionPublicationError> {
    for attempt in 0..64_u8 {
        let path = parent.join(format!(
            ".publish-{}-{}-{attempt}",
            std::process::id(),
            encode_path_segment(operation_id.as_str())
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(VersionPublicationError::StorageUnavailable),
        }
    }
    Err(VersionPublicationError::StorageUnavailable)
}

fn read_committed_record(
    directory: &Path,
    body_policy: DocumentBodyPolicy,
) -> Result<VersionRecord, VersionPublicationError> {
    let entry = read_entry(&directory.join(VERSION_ENTRY_FILE))
        .map_err(|_| VersionPublicationError::CorruptedPublication)?;
    if entry.revision_number().is_none() || entry.created_at_epoch_ms().is_none() {
        return Err(VersionPublicationError::CorruptedPublication);
    }
    let body = read_body(&directory.join(VERSION_BODY_FILE), body_policy)
        .map_err(|_| VersionPublicationError::CorruptedPublication)?;
    let attachment_state = read_attachment_state(&directory.join(VERSION_ATTACHMENTS_FILE))
        .map_err(|_| VersionPublicationError::CorruptedPublication)?;
    let snapshot = VersionSnapshot::with_attachment_state(
        entry.document_id().clone(),
        entry.snapshot_ref().clone(),
        body,
        attachment_state,
    );
    VersionRecord::new(entry, snapshot).map_err(|_| VersionPublicationError::CorruptedPublication)
}

fn create_prepared_temporary_dir(
    parent: &Path,
    operation_id: &DocumentOperationId,
) -> Result<PathBuf, VersionPreparationError> {
    for attempt in 0..64_u8 {
        let path = parent.join(format!(
            ".prepare-{}-{}-{attempt}",
            std::process::id(),
            encode_path_segment(operation_id.as_str())
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(VersionPreparationError::StorageUnavailable),
        }
    }
    Err(VersionPreparationError::StorageUnavailable)
}

fn write_prepared_record(
    directory: &Path,
    workspace_id: &WorkspaceId,
    operation_id: &DocumentOperationId,
    record: &VersionRecord,
) -> Result<(), VersionPreparationError> {
    write_version_record_payload(directory, record)
        .map_err(|_| VersionPreparationError::StorageUnavailable)?;

    let manifest = PreparedVersionManifest {
        schema_version: VERSION_PREPARED_SCHEMA_VERSION,
        workspace_id: workspace_id.as_str().to_string(),
        operation_id: operation_id.as_str().to_string(),
    };
    let content = serde_json::to_string(&manifest)
        .map_err(|_| VersionPreparationError::StorageUnavailable)?;
    write_file_atomically(directory.join(VERSION_PREPARED_MANIFEST_FILE), content)
        .map_err(|_| VersionPreparationError::StorageUnavailable)
}

fn write_version_record_payload(
    directory: &Path,
    record: &VersionRecord,
) -> Result<(), VersionStoreError> {
    write_file_atomically(
        directory.join(VERSION_ENTRY_FILE),
        entry_content(record.entry(), record.entry().created_at_epoch_ms()),
    )?;
    write_file_atomically(
        directory.join(VERSION_BODY_FILE),
        record.snapshot().body().as_str(),
    )?;
    if let Some(content) = attachment_sidecar_content(record.snapshot().attachment_state())? {
        write_file_atomically(directory.join(VERSION_ATTACHMENTS_FILE), content)?;
    }
    Ok(())
}

fn read_prepared_record(
    directory: &Path,
    workspace_id: &WorkspaceId,
    operation_id: &DocumentOperationId,
    body_policy: DocumentBodyPolicy,
) -> Result<PreparedVersion, VersionPreparationError> {
    let manifest = fs::read_to_string(directory.join(VERSION_PREPARED_MANIFEST_FILE))
        .map_err(|_| VersionPreparationError::CorruptedPrepared)?;
    let manifest: PreparedVersionManifest =
        serde_json::from_str(&manifest).map_err(|_| VersionPreparationError::CorruptedPrepared)?;
    if manifest.schema_version != VERSION_PREPARED_SCHEMA_VERSION
        || manifest.workspace_id != workspace_id.as_str()
        || manifest.operation_id != operation_id.as_str()
    {
        return Err(VersionPreparationError::CorruptedPrepared);
    }

    let entry = read_entry(&directory.join(VERSION_ENTRY_FILE))
        .map_err(|_| VersionPreparationError::CorruptedPrepared)?;
    if entry.revision_number().is_none() || entry.created_at_epoch_ms().is_none() {
        return Err(VersionPreparationError::CorruptedPrepared);
    }
    let body = read_body(&directory.join(VERSION_BODY_FILE), body_policy)
        .map_err(|_| VersionPreparationError::CorruptedPrepared)?;
    let attachment_state = read_attachment_state(&directory.join(VERSION_ATTACHMENTS_FILE))
        .map_err(|_| VersionPreparationError::CorruptedPrepared)?;
    let snapshot = VersionSnapshot::with_attachment_state(
        entry.document_id().clone(),
        entry.snapshot_ref().clone(),
        body,
        attachment_state,
    );
    let record = VersionRecord::new(entry, snapshot)
        .map_err(|_| VersionPreparationError::CorruptedPrepared)?;

    Ok(PreparedVersion::new(operation_id.clone(), record))
}

fn read_entry(path: &Path) -> Result<VersionEntry, VersionStoreError> {
    let content = fs::read_to_string(path).map_err(|_| VersionStoreError::CorruptedHistory)?;
    let mut version_id = None;
    let mut document_id = None;
    let mut snapshot_ref = None;
    let mut author = None;
    let mut summary = None;
    let mut created_at_epoch_ms = None;
    let mut revision_number = None;

    for line in content.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(VersionStoreError::CorruptedHistory)?;
        match key {
            "version_id" => version_id = Some(value),
            "document_id" => document_id = Some(value),
            "snapshot_ref" => snapshot_ref = Some(value),
            "author" => author = Some(value),
            "summary" => summary = Some(value),
            "created_at_epoch_ms" => {
                if created_at_epoch_ms.is_some() {
                    return Err(VersionStoreError::CorruptedHistory);
                }
                created_at_epoch_ms = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| VersionStoreError::CorruptedHistory)?,
                )
            }
            "revision_number" => {
                if revision_number.is_some() {
                    return Err(VersionStoreError::CorruptedHistory);
                }
                revision_number = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| VersionStoreError::CorruptedHistory)?,
                );
            }
            _ => return Err(VersionStoreError::CorruptedHistory),
        }
    }

    let entry = VersionEntry::new(
        VersionId::new(version_id.ok_or(VersionStoreError::CorruptedHistory)?)
            .map_err(|_| VersionStoreError::CorruptedHistory)?,
        DocumentId::new(document_id.ok_or(VersionStoreError::CorruptedHistory)?)
            .map_err(|_| VersionStoreError::CorruptedHistory)?,
        DocumentSnapshotRef::new(snapshot_ref.ok_or(VersionStoreError::CorruptedHistory)?)
            .map_err(|_| VersionStoreError::CorruptedHistory)?,
        VersionAuthor::new(author.ok_or(VersionStoreError::CorruptedHistory)?)
            .map_err(|_| VersionStoreError::CorruptedHistory)?,
        VersionSummary::new(summary.ok_or(VersionStoreError::CorruptedHistory)?)
            .map_err(|_| VersionStoreError::CorruptedHistory)?,
    )
    .map_err(|_| VersionStoreError::CorruptedHistory)?;
    let entry = match created_at_epoch_ms {
        Some(value) => entry
            .with_created_at_epoch_ms(value)
            .map_err(|_| VersionStoreError::CorruptedHistory),
        None => Ok(entry),
    }?;
    match revision_number {
        Some(value) => entry
            .with_revision_number(
                DocumentRevisionNumber::new(value)
                    .map_err(|_| VersionStoreError::CorruptedHistory)?,
            )
            .map_err(|_| VersionStoreError::CorruptedHistory),
        None => Ok(entry),
    }
}

fn system_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(1)
        .max(1)
}

fn read_body(path: &Path, policy: DocumentBodyPolicy) -> Result<DocumentBody, VersionStoreError> {
    let content = fs::read_to_string(path).map_err(|_| VersionStoreError::CorruptedHistory)?;
    DocumentBody::new(&content, policy).map_err(|_| VersionStoreError::CorruptedHistory)
}

fn attachment_sidecar_content(
    state: &AttachmentSnapshotState,
) -> Result<Option<String>, VersionStoreError> {
    let references = match state.references() {
        Some(references) => references,
        None => return Ok(None),
    };
    let sidecar = AttachmentSidecar {
        schema_version: VERSION_ATTACHMENTS_SCHEMA_VERSION,
        state: "known".to_string(),
        references: references
            .iter()
            .map(|reference| AttachmentReferenceSidecar {
                asset_id: reference.asset_id().as_str().to_string(),
                label: reference.label().to_string(),
            })
            .collect(),
    };

    serde_json::to_string(&sidecar)
        .map(Some)
        .map_err(|_| VersionStoreError::StorageUnavailable)
}

fn read_attachment_state(path: &Path) -> Result<AttachmentSnapshotState, VersionStoreError> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(AttachmentSnapshotState::legacy_unknown());
        }
        Err(_) => return Err(VersionStoreError::CorruptedHistory),
    };
    let sidecar: AttachmentSidecar =
        serde_json::from_str(&content).map_err(|_| VersionStoreError::CorruptedHistory)?;
    if sidecar.schema_version != VERSION_ATTACHMENTS_SCHEMA_VERSION || sidecar.state != "known" {
        return Err(VersionStoreError::CorruptedHistory);
    }

    let references = sidecar
        .references
        .into_iter()
        .map(|reference| {
            let asset_id = AssetId::from_sha256_hex(&reference.asset_id)
                .map_err(|_| VersionStoreError::CorruptedHistory)?;
            AssetReference::new(asset_id, &reference.label)
                .map_err(|_| VersionStoreError::CorruptedHistory)
        })
        .collect::<Result<Vec<_>, _>>()?;

    AttachmentSnapshotState::known(references).map_err(|_| VersionStoreError::CorruptedHistory)
}

fn append_history(path: PathBuf, version_id: &VersionId) -> Result<(), VersionStoreError> {
    let parent = path.parent().ok_or(VersionStoreError::StorageUnavailable)?;
    fs::create_dir_all(parent).map_err(|_| VersionStoreError::StorageUnavailable)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| VersionStoreError::StorageUnavailable)?;
    writeln!(file, "{}", version_id.as_str()).map_err(|_| VersionStoreError::StorageUnavailable)
}

fn write_file_atomically(path: PathBuf, content: impl AsRef<str>) -> Result<(), VersionStoreError> {
    write_text_atomically(&path, content)
        .map(|_| ())
        .map_err(|_| VersionStoreError::StorageUnavailable)
}

fn child_directories(path: &Path) -> Result<Vec<PathBuf>, VersionStoreError> {
    let entries = fs::read_dir(path).map_err(|_| VersionStoreError::StorageUnavailable)?;
    let mut directories = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| VersionStoreError::StorageUnavailable)?;
        let file_type = entry
            .file_type()
            .map_err(|_| VersionStoreError::StorageUnavailable)?;
        if file_type.is_dir() {
            directories.push(entry.path());
        }
    }
    Ok(directories)
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
