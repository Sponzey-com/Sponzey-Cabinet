use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use cabinet_domain::document::{DocumentId, DocumentPath, DocumentTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::workspace_home::{
    WorkspaceHomeBackupStatus, WorkspaceHomeChangeProjection, WorkspaceHomeDocumentMutation,
    WorkspaceHomeDocumentMutationPort, WorkspaceHomeDocumentProjection, WorkspaceHomeHealthStatus,
    WorkspaceHomeProjection, WorkspaceHomeProjectionError, WorkspaceHomeProjectionLimits,
    WorkspaceHomeProjectionPort, WorkspaceHomeSummaryProjection, WorkspaceHomeTagProjection,
    WorkspaceHomeUnfinishedProjection,
};

use crate::local_atomic_file::write_text_atomically;

const SCHEMA_HEADER: &str = "schema\t1";

#[derive(Debug, Clone)]
pub struct LocalWorkspaceHomeProjectionStore {
    root: PathBuf,
}

impl LocalWorkspaceHomeProjectionStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn replace_projection(
        &self,
        workspace_id: &WorkspaceId,
        projection: &WorkspaceHomeProjection,
    ) -> Result<(), WorkspaceHomeProjectionError> {
        let path = self.projection_path(workspace_id);
        write_text_atomically(&path, encode_projection(projection))
            .map(|_| ())
            .map_err(|_| WorkspaceHomeProjectionError::StorageUnavailable)
    }

    fn projection_path(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.root
            .join("home-projections")
            .join(format!("{}.snapshot", hex_encode(workspace_id.as_str())))
    }
}

impl WorkspaceHomeProjectionPort for LocalWorkspaceHomeProjectionStore {
    fn load_workspace_home(
        &self,
        workspace_id: &WorkspaceId,
        limits: WorkspaceHomeProjectionLimits,
    ) -> Result<WorkspaceHomeProjection, WorkspaceHomeProjectionError> {
        let path = self.projection_path(workspace_id);
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(WorkspaceHomeProjection::empty(
                    WorkspaceHomeBackupStatus::NeverCreated,
                    WorkspaceHomeHealthStatus::Healthy,
                ));
            }
            Err(_) => return Err(WorkspaceHomeProjectionError::StorageUnavailable),
        };
        let projection = decode_projection(&text)?;
        Ok(apply_limits(projection, limits))
    }
}

impl WorkspaceHomeDocumentMutationPort for LocalWorkspaceHomeProjectionStore {
    fn apply_document_mutation(
        &mut self,
        workspace_id: &WorkspaceId,
        mutation: WorkspaceHomeDocumentMutation,
        capacity: u16,
    ) -> Result<(), WorkspaceHomeProjectionError> {
        if capacity == 0 || capacity > 100 {
            return Err(WorkspaceHomeProjectionError::InvalidLimit);
        }
        let full_limits = WorkspaceHomeProjectionLimits::new(100, 100, 100, 100, 100)?;
        let current = self.load_workspace_home(workspace_id, full_limits)?;
        let updated = apply_document_mutation(current, mutation, capacity)?;
        self.replace_projection(workspace_id, &updated)
    }
}

fn apply_document_mutation(
    current: WorkspaceHomeProjection,
    mutation: WorkspaceHomeDocumentMutation,
    capacity: u16,
) -> Result<WorkspaceHomeProjection, WorkspaceHomeProjectionError> {
    match mutation {
        WorkspaceHomeDocumentMutation::UpsertRecent {
            document,
            change_summary,
        } => {
            let document_id = document.document_id().to_string();
            let mut recent_documents = vec![document];
            recent_documents.extend(
                current
                    .recent_documents()
                    .iter()
                    .filter(|item| item.document_id() != document_id)
                    .cloned(),
            );
            recent_documents.truncate(capacity as usize);

            let mut recent_changes = vec![WorkspaceHomeChangeProjection::new(
                DocumentId::new(&document_id)
                    .map_err(|_| WorkspaceHomeProjectionError::InvalidProjectionText)?,
                &change_summary,
            )?];
            recent_changes.extend(
                current
                    .recent_changes()
                    .iter()
                    .filter(|item| item.document_id() != document_id)
                    .cloned(),
            );
            recent_changes.truncate(capacity as usize);

            Ok(WorkspaceHomeProjection::new(
                recent_documents,
                current.favorites().to_vec(),
                current.tags().to_vec(),
                recent_changes,
                current.unfinished_items().to_vec(),
                current.backup_status(),
                current.health_status(),
            )
            .with_summary(current.summary()))
        }
        WorkspaceHomeDocumentMutation::RemoveDocument { document_id } => {
            let document_id = document_id.as_str();
            Ok(WorkspaceHomeProjection::new(
                current
                    .recent_documents()
                    .iter()
                    .filter(|item| item.document_id() != document_id)
                    .cloned()
                    .collect(),
                current
                    .favorites()
                    .iter()
                    .filter(|item| item.document_id() != document_id)
                    .cloned()
                    .collect(),
                current.tags().to_vec(),
                current
                    .recent_changes()
                    .iter()
                    .filter(|item| item.document_id() != document_id)
                    .cloned()
                    .collect(),
                current
                    .unfinished_items()
                    .iter()
                    .filter(|item| item.document_id() != document_id)
                    .cloned()
                    .collect(),
                current.backup_status(),
                current.health_status(),
            )
            .with_summary(current.summary()))
        }
    }
}

fn encode_projection(projection: &WorkspaceHomeProjection) -> String {
    let mut lines = vec![
        SCHEMA_HEADER.to_string(),
        format!(
            "backup\t{}",
            encode_backup_status(projection.backup_status())
        ),
        format!(
            "health\t{}",
            encode_health_status(projection.health_status())
        ),
        format!(
            "summary\t{}\t{}\t{}",
            projection.summary().document_count(),
            projection.summary().asset_count(),
            projection.summary().canvas_count()
        ),
    ];
    lines.extend(
        projection
            .recent_documents()
            .iter()
            .map(|item| encode_document("recent", item)),
    );
    lines.extend(
        projection
            .favorites()
            .iter()
            .map(|item| encode_document("favorite", item)),
    );
    lines.extend(projection.tags().iter().map(|item| {
        format!(
            "tag\t{}\t{}",
            hex_encode(item.label()),
            item.document_count()
        )
    }));
    lines.extend(projection.recent_changes().iter().map(|item| {
        format!(
            "change\t{}\t{}",
            hex_encode(item.document_id()),
            hex_encode(item.summary())
        )
    }));
    lines.extend(projection.unfinished_items().iter().map(|item| {
        format!(
            "unfinished\t{}\t{}",
            hex_encode(item.document_id()),
            hex_encode(item.label())
        )
    }));
    format!("{}\n", lines.join("\n"))
}

fn decode_projection(text: &str) -> Result<WorkspaceHomeProjection, WorkspaceHomeProjectionError> {
    let mut lines = text.lines();
    if lines.next() != Some(SCHEMA_HEADER) {
        return Err(WorkspaceHomeProjectionError::CorruptedProjection);
    }

    let mut backup_status = None;
    let mut health_status = None;
    let mut summary = None;
    let mut recent_documents = Vec::new();
    let mut favorites = Vec::new();
    let mut tags = Vec::new();
    let mut recent_changes = Vec::new();
    let mut unfinished_items = Vec::new();

    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        match fields.as_slice() {
            ["backup", value] if backup_status.is_none() => {
                backup_status = Some(decode_backup_status(value)?);
            }
            ["health", value] if health_status.is_none() => {
                health_status = Some(decode_health_status(value)?);
            }
            ["summary", document_count, asset_count, canvas_count] if summary.is_none() => {
                summary = Some(WorkspaceHomeSummaryProjection::new(
                    decode_count(document_count)?,
                    decode_count(asset_count)?,
                    decode_count(canvas_count)?,
                ));
            }
            ["recent", id, title, path] => {
                recent_documents.push(decode_document(id, title, path)?);
            }
            ["favorite", id, title, path] => {
                favorites.push(decode_document(id, title, path)?);
            }
            ["tag", label, count] => {
                let count = count
                    .parse::<u32>()
                    .map_err(|_| WorkspaceHomeProjectionError::CorruptedProjection)?;
                tags.push(
                    WorkspaceHomeTagProjection::new(&hex_decode(label)?, count)
                        .map_err(|_| WorkspaceHomeProjectionError::CorruptedProjection)?,
                );
            }
            ["change", id, summary] => {
                recent_changes.push(
                    WorkspaceHomeChangeProjection::new(
                        decode_document_id(id)?,
                        &hex_decode(summary)?,
                    )
                    .map_err(|_| WorkspaceHomeProjectionError::CorruptedProjection)?,
                );
            }
            ["unfinished", id, label] => {
                unfinished_items.push(
                    WorkspaceHomeUnfinishedProjection::new(
                        decode_document_id(id)?,
                        &hex_decode(label)?,
                    )
                    .map_err(|_| WorkspaceHomeProjectionError::CorruptedProjection)?,
                );
            }
            _ => return Err(WorkspaceHomeProjectionError::CorruptedProjection),
        }
    }

    Ok(WorkspaceHomeProjection::new(
        recent_documents,
        favorites,
        tags,
        recent_changes,
        unfinished_items,
        backup_status.ok_or(WorkspaceHomeProjectionError::CorruptedProjection)?,
        health_status.ok_or(WorkspaceHomeProjectionError::CorruptedProjection)?,
    )
    .with_summary(summary.unwrap_or_default()))
}

fn apply_limits(
    projection: WorkspaceHomeProjection,
    limits: WorkspaceHomeProjectionLimits,
) -> WorkspaceHomeProjection {
    let summary = projection.summary();
    WorkspaceHomeProjection::new(
        projection
            .recent_documents()
            .iter()
            .take(limits.recent_documents() as usize)
            .cloned()
            .collect(),
        projection
            .favorites()
            .iter()
            .take(limits.favorites() as usize)
            .cloned()
            .collect(),
        projection
            .tags()
            .iter()
            .take(limits.tags() as usize)
            .cloned()
            .collect(),
        projection
            .recent_changes()
            .iter()
            .take(limits.recent_changes() as usize)
            .cloned()
            .collect(),
        projection
            .unfinished_items()
            .iter()
            .take(limits.unfinished_items() as usize)
            .cloned()
            .collect(),
        projection.backup_status(),
        projection.health_status(),
    )
    .with_summary(summary)
}

fn decode_count(value: &str) -> Result<u32, WorkspaceHomeProjectionError> {
    value
        .parse::<u32>()
        .map_err(|_| WorkspaceHomeProjectionError::CorruptedProjection)
}

fn encode_document(kind: &str, item: &WorkspaceHomeDocumentProjection) -> String {
    format!(
        "{kind}\t{}\t{}\t{}",
        hex_encode(item.document_id()),
        hex_encode(item.title()),
        hex_encode(item.path())
    )
}

fn decode_document(
    id: &str,
    title: &str,
    path: &str,
) -> Result<WorkspaceHomeDocumentProjection, WorkspaceHomeProjectionError> {
    Ok(WorkspaceHomeDocumentProjection::new(
        decode_document_id(id)?,
        DocumentTitle::new(&hex_decode(title)?)
            .map_err(|_| WorkspaceHomeProjectionError::CorruptedProjection)?,
        DocumentPath::new(&hex_decode(path)?)
            .map_err(|_| WorkspaceHomeProjectionError::CorruptedProjection)?,
    ))
}

fn decode_document_id(value: &str) -> Result<DocumentId, WorkspaceHomeProjectionError> {
    DocumentId::new(&hex_decode(value)?)
        .map_err(|_| WorkspaceHomeProjectionError::CorruptedProjection)
}

fn encode_backup_status(status: WorkspaceHomeBackupStatus) -> &'static str {
    match status {
        WorkspaceHomeBackupStatus::NeverCreated => "NeverCreated",
        WorkspaceHomeBackupStatus::Fresh => "Fresh",
        WorkspaceHomeBackupStatus::Stale => "Stale",
        WorkspaceHomeBackupStatus::Failed => "Failed",
    }
}

fn decode_backup_status(
    value: &str,
) -> Result<WorkspaceHomeBackupStatus, WorkspaceHomeProjectionError> {
    match value {
        "NeverCreated" => Ok(WorkspaceHomeBackupStatus::NeverCreated),
        "Fresh" => Ok(WorkspaceHomeBackupStatus::Fresh),
        "Stale" => Ok(WorkspaceHomeBackupStatus::Stale),
        "Failed" => Ok(WorkspaceHomeBackupStatus::Failed),
        _ => Err(WorkspaceHomeProjectionError::CorruptedProjection),
    }
}

fn encode_health_status(status: WorkspaceHomeHealthStatus) -> &'static str {
    match status {
        WorkspaceHomeHealthStatus::Healthy => "Healthy",
        WorkspaceHomeHealthStatus::Degraded => "Degraded",
        WorkspaceHomeHealthStatus::ReadOnlyRecovery => "ReadOnlyRecovery",
    }
}

fn decode_health_status(
    value: &str,
) -> Result<WorkspaceHomeHealthStatus, WorkspaceHomeProjectionError> {
    match value {
        "Healthy" => Ok(WorkspaceHomeHealthStatus::Healthy),
        "Degraded" => Ok(WorkspaceHomeHealthStatus::Degraded),
        "ReadOnlyRecovery" => Ok(WorkspaceHomeHealthStatus::ReadOnlyRecovery),
        _ => Err(WorkspaceHomeProjectionError::CorruptedProjection),
    }
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, WorkspaceHomeProjectionError> {
    if value.len() % 2 != 0 {
        return Err(WorkspaceHomeProjectionError::CorruptedProjection);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)
                .map_err(|_| WorkspaceHomeProjectionError::CorruptedProjection)?;
            u8::from_str_radix(text, 16)
                .map_err(|_| WorkspaceHomeProjectionError::CorruptedProjection)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| WorkspaceHomeProjectionError::CorruptedProjection)
}
