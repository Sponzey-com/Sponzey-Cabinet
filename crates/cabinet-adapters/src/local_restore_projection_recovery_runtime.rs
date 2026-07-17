use std::path::PathBuf;

use cabinet_domain::document::{DocumentBodyPolicy, DocumentId};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::committed_version_record_reader::CommittedVersionRecordReader;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::document_repository::DocumentRepository;
use cabinet_usecases::project_current_document_attachments::{
    ProjectCurrentDocumentAttachmentsInput, ProjectCurrentDocumentAttachmentsUsecase,
};
use cabinet_usecases::project_current_document_revision::{
    ProjectCurrentDocumentRevisionInput, ProjectCurrentDocumentRevisionUsecase,
};
use sha2::{Digest, Sha256};

use crate::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_JOURNAL_ROOT, LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT,
};
use crate::local_current_document_attachment_projection::LocalCurrentDocumentAttachmentProjection;
use crate::local_current_document_revision_projection::{
    LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT, LocalCurrentDocumentRevisionProjectionWriter,
};
use crate::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use crate::local_document_operation_journal::{
    LocalDocumentOperationJournal, LocalRestoreCandidateScanError,
};
use crate::local_document_repository::LocalDocumentRepository;
use crate::local_version_store::LocalVersionStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredRestoreProjection {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    version_id: VersionId,
}

impl RecoveredRestoreProjection {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRestoreProjectionRecoveryOutput {
    recovered: Vec<RecoveredRestoreProjection>,
    skipped_stale_count: usize,
}

impl LocalRestoreProjectionRecoveryOutput {
    pub fn recovered(&self) -> &[RecoveredRestoreProjection] {
        &self.recovered
    }

    pub const fn skipped_stale_count(&self) -> usize {
        self.skipped_stale_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalRestoreProjectionRecoveryError {
    InvalidPolicy,
    StorageUnavailable,
    CorruptedData,
    RecoveryRequired,
}

pub struct LocalRestoreProjectionRecoveryRuntime {
    journal: LocalDocumentOperationJournal,
    versions: LocalVersionStore,
    pointer: LocalCurrentDocumentVersionPointer,
    documents: LocalDocumentRepository,
    attachments: LocalCurrentDocumentAttachmentProjection,
    projection: LocalCurrentDocumentRevisionProjectionWriter,
}

impl LocalRestoreProjectionRecoveryRuntime {
    pub fn new(app_data_root: PathBuf, body_policy: DocumentBodyPolicy) -> Self {
        Self {
            journal: LocalDocumentOperationJournal::new(
                app_data_root.join(LOCAL_DOCUMENT_JOURNAL_ROOT),
            ),
            versions: LocalVersionStore::with_body_policy(
                app_data_root.join(LOCAL_DOCUMENT_VERSION_ROOT),
                body_policy,
            ),
            pointer: LocalCurrentDocumentVersionPointer::new(
                app_data_root.join(LOCAL_DOCUMENT_POINTER_ROOT),
            ),
            documents: LocalDocumentRepository::with_body_policy(
                app_data_root.join(LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT),
                body_policy,
            ),
            attachments: LocalCurrentDocumentAttachmentProjection::new(app_data_root.clone()),
            projection: LocalCurrentDocumentRevisionProjectionWriter::new(
                app_data_root,
                body_policy,
            ),
        }
    }

    pub fn recover(
        &mut self,
        limit: usize,
    ) -> Result<LocalRestoreProjectionRecoveryOutput, LocalRestoreProjectionRecoveryError> {
        let candidates = self
            .journal
            .list_committed_restore_candidates(limit)
            .map_err(map_scan_error)?;
        let mut recovered = Vec::new();
        let mut skipped_stale_count = 0;
        for candidate in candidates {
            let current = self
                .pointer
                .load_current_version(candidate.workspace_id(), candidate.document_id())
                .map_err(|_| LocalRestoreProjectionRecoveryError::StorageUnavailable)?;
            if current.as_ref() != Some(candidate.version_id()) {
                skipped_stale_count += 1;
                continue;
            }
            let record = self
                .versions
                .get_committed_version_record(
                    candidate.workspace_id(),
                    candidate.document_id(),
                    candidate.version_id(),
                )
                .map_err(|_| LocalRestoreProjectionRecoveryError::StorageUnavailable)?
                .ok_or(LocalRestoreProjectionRecoveryError::CorruptedData)?;
            let path = self
                .documents
                .get_current_by_id(candidate.workspace_id(), candidate.document_id())
                .map_err(|_| LocalRestoreProjectionRecoveryError::RecoveryRequired)?
                .map(|document| document.path().as_str().to_string())
                .unwrap_or_else(|| hidden_document_path(candidate.document_id()));
            ProjectCurrentDocumentAttachmentsUsecase::new()
                .execute(
                    ProjectCurrentDocumentAttachmentsInput::new(
                        candidate.workspace_id().as_str(),
                        candidate.document_id().as_str(),
                        record.clone(),
                    ),
                    &mut self.attachments,
                )
                .map_err(|_| LocalRestoreProjectionRecoveryError::RecoveryRequired)?;
            ProjectCurrentDocumentRevisionUsecase::new()
                .execute(
                    ProjectCurrentDocumentRevisionInput::new(
                        candidate.workspace_id().as_str(),
                        &path,
                        record,
                    ),
                    &mut self.projection,
                )
                .map_err(|_| LocalRestoreProjectionRecoveryError::RecoveryRequired)?;
            recovered.push(RecoveredRestoreProjection {
                workspace_id: candidate.workspace_id().clone(),
                document_id: candidate.document_id().clone(),
                version_id: candidate.version_id().clone(),
            });
        }
        Ok(LocalRestoreProjectionRecoveryOutput {
            recovered,
            skipped_stale_count,
        })
    }
}

const fn map_scan_error(
    error: LocalRestoreCandidateScanError,
) -> LocalRestoreProjectionRecoveryError {
    match error {
        LocalRestoreCandidateScanError::InvalidLimit => {
            LocalRestoreProjectionRecoveryError::InvalidPolicy
        }
        LocalRestoreCandidateScanError::StorageUnavailable => {
            LocalRestoreProjectionRecoveryError::StorageUnavailable
        }
        LocalRestoreCandidateScanError::CorruptedJournal => {
            LocalRestoreProjectionRecoveryError::CorruptedData
        }
    }
}

fn hidden_document_path(document_id: &DocumentId) -> String {
    let digest = Sha256::digest(document_id.as_str().as_bytes());
    format!("notes/{digest:x}.md")
}
