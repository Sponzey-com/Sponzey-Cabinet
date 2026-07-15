use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use cabinet_domain::audit::{
    AuditAction, AuditActor, AuditEvent, AuditEventId, AuditMetadata, AuditTarget, AuditTargetId,
    AuditTimestamp,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::audit_log::{
    AuditCursor, AuditEventPage, AuditListQuery, AuditListScope, AuditLogStore, AuditLogStoreError,
};

use crate::local_atomic_file::write_text_atomically;

pub const LOCAL_AUDIT_LOG_DIR: &str = "audit-log";
pub const LOCAL_AUDIT_LOG_EVENTS_DIR: &str = "events";
pub const LOCAL_AUDIT_LOG_BY_ACTOR_DIR: &str = "by-actor";
pub const LOCAL_AUDIT_LOG_BY_TARGET_DIR: &str = "by-target";

#[derive(Clone, PartialEq, Eq)]
pub struct LocalAuditLogStore {
    root: PathBuf,
}

impl fmt::Debug for LocalAuditLogStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalAuditLogStore")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl LocalAuditLogStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn workspace_dir(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.root
            .join(LOCAL_AUDIT_LOG_DIR)
            .join(hex_encode(workspace_id.as_str()))
    }

    fn events_dir(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.workspace_dir(workspace_id)
            .join(LOCAL_AUDIT_LOG_EVENTS_DIR)
    }

    fn actor_index_dir(&self, workspace_id: &WorkspaceId, actor_user_id: &UserId) -> PathBuf {
        self.workspace_dir(workspace_id)
            .join(LOCAL_AUDIT_LOG_BY_ACTOR_DIR)
            .join(hex_encode(actor_user_id.as_str()))
    }

    fn target_index_dir(
        &self,
        workspace_id: &WorkspaceId,
        target_type: &str,
        target_id: &str,
    ) -> PathBuf {
        self.workspace_dir(workspace_id)
            .join(LOCAL_AUDIT_LOG_BY_TARGET_DIR)
            .join(hex_encode(target_type))
            .join(hex_encode(target_id))
    }

    fn event_path(&self, event: &AuditEvent) -> PathBuf {
        self.events_dir(event.workspace_id())
            .join(event_file_name(event))
    }

    fn event_path_from_stem(&self, workspace_id: &WorkspaceId, stem: &str) -> PathBuf {
        self.events_dir(workspace_id).join(format!("{stem}.event"))
    }

    fn candidate_event_paths(
        &self,
        query: &AuditListQuery,
    ) -> Result<Vec<PathBuf>, AuditLogStoreError> {
        match query.scope() {
            AuditListScope::Workspace => {
                read_files_with_extension(&self.events_dir(query.workspace_id()), "event")
            }
            AuditListScope::Actor { actor_user_id } => self.indexed_event_paths(
                &self.actor_index_dir(query.workspace_id(), actor_user_id),
                query.workspace_id(),
            ),
            AuditListScope::Target {
                target_type,
                target_id,
            } => self.indexed_event_paths(
                &self.target_index_dir(query.workspace_id(), target_type, target_id),
                query.workspace_id(),
            ),
        }
    }

    fn indexed_event_paths(
        &self,
        index_dir: &Path,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<PathBuf>, AuditLogStoreError> {
        let mut index_paths = read_files_with_extension(index_dir, "idx")?;
        let mut event_paths = Vec::with_capacity(index_paths.len());
        for index_path in index_paths.drain(..) {
            let stem = index_path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or(AuditLogStoreError::CorruptedState)?;
            event_paths.push(self.event_path_from_stem(workspace_id, stem));
        }
        Ok(event_paths)
    }

    fn write_index(
        &self,
        index_dir: PathBuf,
        event: &AuditEvent,
    ) -> Result<(), AuditLogStoreError> {
        let stem = event_file_stem(event);
        write_text_atomically(
            &index_dir.join(format!("{stem}.idx")),
            format!("event_file={stem}.event\n"),
        )
        .map(|_| ())
        .map_err(|_| AuditLogStoreError::StorageUnavailable)
    }
}

impl AuditLogStore for LocalAuditLogStore {
    fn append_audit_event(&mut self, event: AuditEvent) -> Result<(), AuditLogStoreError> {
        write_text_atomically(&self.event_path(&event), encode_event(&event))
            .map_err(|_| AuditLogStoreError::StorageUnavailable)?;
        if event.actor().actor_type() == "user" {
            let actor_user_id = UserId::new(event.actor().actor_id())
                .map_err(|_| AuditLogStoreError::CorruptedState)?;
            self.write_index(
                self.actor_index_dir(event.workspace_id(), &actor_user_id),
                &event,
            )?;
        }
        self.write_index(
            self.target_index_dir(
                event.workspace_id(),
                event.target().target_type(),
                event.target().target_id(),
            ),
            &event,
        )
    }

    fn list_audit_events(
        &self,
        query: AuditListQuery,
    ) -> Result<AuditEventPage, AuditLogStoreError> {
        let paths = self.candidate_event_paths(&query)?;
        let start = query.page().cursor().map_or(0, AuditCursor::offset);
        let limit = query.page().limit();
        let selected_paths = paths.iter().skip(start).take(limit);
        let mut events = Vec::new();
        for path in selected_paths {
            let event = decode_event_file(path)?;
            if !query.matches(&event) {
                return Err(AuditLogStoreError::CorruptedState);
            }
            events.push(event);
        }
        let next_offset = start + events.len();
        let next_cursor = if next_offset < paths.len() {
            Some(AuditCursor::from_offset(next_offset))
        } else {
            None
        };

        Ok(AuditEventPage::new(events, next_cursor))
    }
}

fn event_file_name(event: &AuditEvent) -> String {
    format!("{}.event", event_file_stem(event))
}

fn event_file_stem(event: &AuditEvent) -> String {
    format!(
        "{:020}-{}",
        event.occurred_at().as_millis(),
        hex_encode(event.event_id().as_str())
    )
}

fn read_files_with_extension(
    dir: &Path,
    extension: &str,
) -> Result<Vec<PathBuf>, AuditLogStoreError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(AuditLogStoreError::StorageUnavailable),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|_| AuditLogStoreError::StorageUnavailable)?
            .path();
        if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn decode_event_file(path: &Path) -> Result<AuditEvent, AuditLogStoreError> {
    let content = fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AuditLogStoreError::CorruptedState
        } else {
            AuditLogStoreError::StorageUnavailable
        }
    })?;
    decode_event(&content)
}

fn encode_event(event: &AuditEvent) -> String {
    let target_document_id = event.target().document_id().map_or("", DocumentId::as_str);
    let mut lines = vec![
        format!("event_id={}", hex_encode(event.event_id().as_str())),
        format!("workspace_id={}", hex_encode(event.workspace_id().as_str())),
        format!("actor_type={}", event.actor().actor_type()),
        format!("actor_id={}", hex_encode(event.actor().actor_id())),
        format!("action={}", event.action().as_str()),
        format!("target_type={}", event.target().target_type()),
        format!("target_id={}", hex_encode(event.target().target_id())),
        format!("target_document_id={}", hex_encode(target_document_id)),
        format!("occurred_at={}", event.occurred_at().as_millis()),
        format!("metadata_count={}", event.metadata().entries().len()),
    ];
    for (index, entry) in event.metadata().entries().iter().enumerate() {
        lines.push(format!("metadata.{index}.key={}", hex_encode(entry.key())));
        lines.push(format!(
            "metadata.{index}.value={}",
            hex_encode(entry.value())
        ));
    }
    lines.push(String::new());
    lines.join("\n")
}

fn decode_event(content: &str) -> Result<AuditEvent, AuditLogStoreError> {
    let fields = parse_fields(content)?;
    let event_id = AuditEventId::new(&required_hex(&fields, "event_id")?)
        .map_err(|_| AuditLogStoreError::CorruptedState)?;
    let workspace_id = WorkspaceId::new(&required_hex(&fields, "workspace_id")?)
        .map_err(|_| AuditLogStoreError::CorruptedState)?;
    let actor = decode_actor(&fields)?;
    let action = decode_action(required(&fields, "action")?)?;
    let target = decode_target(&fields)?;
    let metadata = decode_metadata(&fields)?;
    let occurred_at = required(&fields, "occurred_at")?
        .parse::<u64>()
        .map(AuditTimestamp::from_millis)
        .map_err(|_| AuditLogStoreError::CorruptedState)?;

    Ok(AuditEvent::new(
        event_id,
        workspace_id,
        actor,
        action,
        target,
        metadata,
        occurred_at,
    ))
}

fn parse_fields(content: &str) -> Result<BTreeMap<String, String>, AuditLogStoreError> {
    let mut fields = BTreeMap::new();
    for line in content.lines().filter(|line| !line.is_empty()) {
        let (key, value) = line
            .split_once('=')
            .ok_or(AuditLogStoreError::CorruptedState)?;
        if key.is_empty() {
            return Err(AuditLogStoreError::CorruptedState);
        }
        fields.insert(key.to_string(), value.to_string());
    }
    Ok(fields)
}

fn decode_actor(fields: &BTreeMap<String, String>) -> Result<AuditActor, AuditLogStoreError> {
    match required(fields, "actor_type")? {
        "user" => Ok(AuditActor::user(
            UserId::new(&required_hex(fields, "actor_id")?)
                .map_err(|_| AuditLogStoreError::CorruptedState)?,
        )),
        _ => Err(AuditLogStoreError::CorruptedState),
    }
}

fn decode_target(fields: &BTreeMap<String, String>) -> Result<AuditTarget, AuditLogStoreError> {
    let target_id = required_hex(fields, "target_id")?;
    let target_document_id = optional_hex(fields, "target_document_id")?;
    match required(fields, "target_type")? {
        "workspace" => Ok(AuditTarget::workspace(
            WorkspaceId::new(&target_id).map_err(|_| AuditLogStoreError::CorruptedState)?,
        )),
        "document" => Ok(AuditTarget::document(
            DocumentId::new(&target_id).map_err(|_| AuditLogStoreError::CorruptedState)?,
        )),
        "comment_thread" => Ok(AuditTarget::comment_thread(
            required_document_id(target_document_id)?,
            AuditTargetId::new(&target_id).map_err(|_| AuditLogStoreError::CorruptedState)?,
        )),
        "review_request" => Ok(AuditTarget::review_request(
            required_document_id(target_document_id)?,
            AuditTargetId::new(&target_id).map_err(|_| AuditLogStoreError::CorruptedState)?,
        )),
        "document_lock" => Ok(AuditTarget::document_lock(
            required_document_id(target_document_id)?,
            AuditTargetId::new(&target_id).map_err(|_| AuditLogStoreError::CorruptedState)?,
        )),
        "backup_job" => Ok(AuditTarget::backup_job(
            AuditTargetId::new(&target_id).map_err(|_| AuditLogStoreError::CorruptedState)?,
        )),
        _ => Err(AuditLogStoreError::CorruptedState),
    }
}

fn required_document_id(value: Option<String>) -> Result<DocumentId, AuditLogStoreError> {
    DocumentId::new(&value.ok_or(AuditLogStoreError::CorruptedState)?)
        .map_err(|_| AuditLogStoreError::CorruptedState)
}

fn decode_metadata(fields: &BTreeMap<String, String>) -> Result<AuditMetadata, AuditLogStoreError> {
    let metadata_count = required(fields, "metadata_count")?
        .parse::<usize>()
        .map_err(|_| AuditLogStoreError::CorruptedState)?;
    let mut entries = Vec::with_capacity(metadata_count);
    for index in 0..metadata_count {
        entries.push((
            required_hex(fields, &format!("metadata.{index}.key"))?,
            required_hex(fields, &format!("metadata.{index}.value"))?,
        ));
    }
    AuditMetadata::from_pairs(&entries).map_err(|_| AuditLogStoreError::CorruptedState)
}

fn decode_action(value: &str) -> Result<AuditAction, AuditLogStoreError> {
    match value {
        "permission.denied" => Ok(AuditAction::PermissionDenied),
        "review.requested" => Ok(AuditAction::ReviewRequested),
        "review.approved" => Ok(AuditAction::ReviewApproved),
        "review.rejected" => Ok(AuditAction::ReviewRejected),
        "document.published" => Ok(AuditAction::DocumentPublished),
        "lock.acquired" => Ok(AuditAction::LockAcquired),
        "lock.released" => Ok(AuditAction::LockReleased),
        "lock.expired" => Ok(AuditAction::LockExpired),
        "backup.created" => Ok(AuditAction::BackupCreated),
        "restore.completed" => Ok(AuditAction::RestoreCompleted),
        _ => Err(AuditLogStoreError::CorruptedState),
    }
}

fn required<'a>(
    fields: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, AuditLogStoreError> {
    fields
        .get(key)
        .map(String::as_str)
        .ok_or(AuditLogStoreError::CorruptedState)
}

fn required_hex(
    fields: &BTreeMap<String, String>,
    key: &str,
) -> Result<String, AuditLogStoreError> {
    hex_decode(required(fields, key)?)
}

fn optional_hex(
    fields: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<String>, AuditLogStoreError> {
    let Some(value) = fields.get(key) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    Ok(Some(hex_decode(value)?))
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, AuditLogStoreError> {
    if !value.len().is_multiple_of(2) {
        return Err(AuditLogStoreError::CorruptedState);
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AuditLogStoreError::CorruptedState)?;
    String::from_utf8(bytes).map_err(|_| AuditLogStoreError::CorruptedState)
}
