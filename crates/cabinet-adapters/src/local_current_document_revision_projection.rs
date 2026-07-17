use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use cabinet_domain::document::{DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::{DocumentRevisionNumber, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_revision_projection::{
    CurrentDocumentRevisionProjection, CurrentDocumentRevisionProjectionError,
    CurrentDocumentRevisionProjectionOutcome, CurrentDocumentRevisionProjectionWriter,
};
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::document_repository::{DocumentRepository, DocumentRepositoryError};

use crate::local_atomic_file::write_text_atomically;
use crate::local_create_document_revision_runtime::LOCAL_DOCUMENT_POINTER_ROOT;
use crate::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use crate::local_document_repository::LocalDocumentRepository;

pub const LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT: &str = "authoring-current";
pub const LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT: &str =
    "current-document-revision-projections";
const IDENTITY_SCHEMA_HEADER: &str = "schema=1";

#[derive(Debug, Clone)]
pub struct LocalCurrentDocumentRevisionProjectionWriter {
    documents: LocalDocumentRepository,
    pointer: LocalCurrentDocumentVersionPointer,
    identity_root: PathBuf,
}

impl LocalCurrentDocumentRevisionProjectionWriter {
    pub fn new(app_data_root: PathBuf, body_policy: DocumentBodyPolicy) -> Self {
        Self {
            documents: LocalDocumentRepository::with_body_policy(
                app_data_root.join(LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT),
                body_policy,
            ),
            pointer: LocalCurrentDocumentVersionPointer::new(
                app_data_root.join(LOCAL_DOCUMENT_POINTER_ROOT),
            ),
            identity_root: app_data_root.join(LOCAL_CURRENT_DOCUMENT_REVISION_IDENTITY_ROOT),
        }
    }

    fn identity_path(&self, workspace_id: &WorkspaceId, document_id: &DocumentId) -> PathBuf {
        self.identity_root
            .join(hex_encode(workspace_id.as_str()))
            .join(hex_encode(document_id.as_str()))
            .join("current.projection")
    }

    fn ensure_authoritative_version(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<(), CurrentDocumentRevisionProjectionError> {
        let current = self
            .pointer
            .load_current_version(workspace_id, document_id)
            .map_err(map_pointer_error)?;
        if current.as_ref() == Some(version_id) {
            Ok(())
        } else {
            Err(CurrentDocumentRevisionProjectionError::StaleRevision)
        }
    }

    fn load_identity(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<StoredProjectionIdentity>, CurrentDocumentRevisionProjectionError> {
        match fs::read_to_string(self.identity_path(workspace_id, document_id)) {
            Ok(text) => decode_identity(&text).map(Some),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(_) => Err(CurrentDocumentRevisionProjectionError::StorageUnavailable),
        }
    }
}

impl CurrentDocumentRevisionProjectionWriter for LocalCurrentDocumentRevisionProjectionWriter {
    fn write_current_projection(
        &mut self,
        workspace_id: &WorkspaceId,
        projection: CurrentDocumentRevisionProjection,
    ) -> Result<CurrentDocumentRevisionProjectionOutcome, CurrentDocumentRevisionProjectionError>
    {
        let document_id = projection.record().document_id().clone();
        self.ensure_authoritative_version(workspace_id, &document_id, projection.version_id())?;

        if let Some(stored) = self.load_identity(workspace_id, &document_id)? {
            match stored.revision_number.cmp(&projection.revision_number()) {
                std::cmp::Ordering::Greater => {
                    return Err(CurrentDocumentRevisionProjectionError::StaleRevision);
                }
                std::cmp::Ordering::Equal if stored.version_id != *projection.version_id() => {
                    return Err(CurrentDocumentRevisionProjectionError::RevisionConflict);
                }
                std::cmp::Ordering::Equal => {
                    let current = self
                        .documents
                        .get_current_by_id(workspace_id, &document_id)
                        .map_err(map_document_error)?
                        .ok_or(CurrentDocumentRevisionProjectionError::CorruptedProjection)?;
                    if &current == projection.record() {
                        return Ok(CurrentDocumentRevisionProjectionOutcome::AlreadyCurrent);
                    }
                    return Err(CurrentDocumentRevisionProjectionError::CorruptedProjection);
                }
                std::cmp::Ordering::Less => {}
            }
        }

        // Recheck immediately before publishing the projection to narrow stale worker races.
        self.ensure_authoritative_version(workspace_id, &document_id, projection.version_id())?;
        let identity = StoredProjectionIdentity {
            version_id: projection.version_id().clone(),
            revision_number: projection.revision_number(),
        };
        self.documents
            .put_current(workspace_id, projection.into_record())
            .map_err(map_document_error)?;
        write_text_atomically(
            &self.identity_path(workspace_id, &document_id),
            encode_identity(&identity),
        )
        .map_err(|_| CurrentDocumentRevisionProjectionError::StorageUnavailable)?;
        Ok(CurrentDocumentRevisionProjectionOutcome::Applied)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredProjectionIdentity {
    version_id: VersionId,
    revision_number: DocumentRevisionNumber,
}

fn encode_identity(identity: &StoredProjectionIdentity) -> String {
    format!(
        "{IDENTITY_SCHEMA_HEADER}\nversion={}\nrevision={}\n",
        hex_encode(identity.version_id.as_str()),
        identity.revision_number.value()
    )
}

fn decode_identity(
    text: &str,
) -> Result<StoredProjectionIdentity, CurrentDocumentRevisionProjectionError> {
    let mut lines = text.lines();
    if lines.next() != Some(IDENTITY_SCHEMA_HEADER) {
        return Err(CurrentDocumentRevisionProjectionError::CorruptedProjection);
    }
    let version = lines
        .next()
        .and_then(|line| line.strip_prefix("version="))
        .ok_or(CurrentDocumentRevisionProjectionError::CorruptedProjection)?;
    let revision = lines
        .next()
        .and_then(|line| line.strip_prefix("revision="))
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or(CurrentDocumentRevisionProjectionError::CorruptedProjection)?;
    if lines.next().is_some() {
        return Err(CurrentDocumentRevisionProjectionError::CorruptedProjection);
    }
    Ok(StoredProjectionIdentity {
        version_id: VersionId::new(&hex_decode(version)?)
            .map_err(|_| CurrentDocumentRevisionProjectionError::CorruptedProjection)?,
        revision_number: DocumentRevisionNumber::new(revision)
            .map_err(|_| CurrentDocumentRevisionProjectionError::CorruptedProjection)?,
    })
}

const fn map_pointer_error(
    error: CurrentDocumentVersionPointerError,
) -> CurrentDocumentRevisionProjectionError {
    match error {
        CurrentDocumentVersionPointerError::StorageUnavailable => {
            CurrentDocumentRevisionProjectionError::StorageUnavailable
        }
        CurrentDocumentVersionPointerError::Conflict
        | CurrentDocumentVersionPointerError::CorruptedPointer => {
            CurrentDocumentRevisionProjectionError::CorruptedProjection
        }
    }
}

const fn map_document_error(
    error: DocumentRepositoryError,
) -> CurrentDocumentRevisionProjectionError {
    match error {
        DocumentRepositoryError::StorageUnavailable => {
            CurrentDocumentRevisionProjectionError::StorageUnavailable
        }
        DocumentRepositoryError::Conflict => {
            CurrentDocumentRevisionProjectionError::RevisionConflict
        }
        DocumentRepositoryError::MismatchedDocumentIdentity
        | DocumentRepositoryError::CorruptedMetadata => {
            CurrentDocumentRevisionProjectionError::CorruptedProjection
        }
    }
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, CurrentDocumentRevisionProjectionError> {
    if value.len() % 2 != 0 {
        return Err(CurrentDocumentRevisionProjectionError::CorruptedProjection);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair)
                .map_err(|_| CurrentDocumentRevisionProjectionError::CorruptedProjection)?;
            u8::from_str_radix(pair, 16)
                .map_err(|_| CurrentDocumentRevisionProjectionError::CorruptedProjection)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes)
        .map_err(|_| CurrentDocumentRevisionProjectionError::CorruptedProjection)
}
