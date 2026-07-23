use std::path::PathBuf;

use cabinet_domain::asset::AssetReference;
use cabinet_domain::document::{DocumentBodyPolicy, DocumentId};
use cabinet_domain::projection_work::ProjectionChangeKind;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::{
    AssetAssociationCatalog, AssetAssociationCatalogError,
};
use cabinet_ports::committed_version_record_reader::CommittedVersionRecordReader;
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::document_repository::DocumentRepository;
use cabinet_usecases::mutate_document_attachments::{
    MutateDocumentAttachmentsError, MutateDocumentAttachmentsInput,
    MutateDocumentAttachmentsOutput, MutateDocumentAttachmentsUsecase,
};
use cabinet_usecases::project_current_document_attachments::{
    ProjectCurrentDocumentAttachmentsInput, ProjectCurrentDocumentAttachmentsUsecase,
};
use cabinet_usecases::project_current_document_revision::{
    ProjectCurrentDocumentRevisionInput, ProjectCurrentDocumentRevisionUsecase,
};

use crate::durable_asset_association_catalog::DurableAssetAssociationCatalog;
use crate::guarded_document_revision_commit::GuardedDocumentRevisionCommit;
use crate::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_JOURNAL_ROOT, LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT,
};
use crate::local_current_document_attachment_projection::LocalCurrentDocumentAttachmentProjection;
use crate::local_current_document_revision_projection::{
    LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT, LocalCurrentDocumentRevisionProjectionWriter,
};
use crate::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use crate::local_document_mutation_fingerprint::LocalDocumentMutationFingerprint;
use crate::local_document_operation_journal::{
    LocalDocumentOperationJournal, LocalRestoreCandidateScanError,
};
use crate::local_document_repository::LocalDocumentRepository;
use crate::local_document_revision_metadata::{
    LocalDocumentRevisionMetadataSource, LocalDocumentRevisionNumberAllocator,
};
use crate::local_version_store::LocalVersionStore;

pub struct LocalMutateDocumentAttachmentsRuntime {
    usecase: MutateDocumentAttachmentsUsecase,
    fingerprint: LocalDocumentMutationFingerprint,
    metadata: LocalDocumentRevisionMetadataSource,
    versions: LocalVersionStore,
    pointer: LocalCurrentDocumentVersionPointer,
    journal: LocalDocumentOperationJournal,
    current_documents: LocalDocumentRepository,
    attachment_projection: LocalCurrentDocumentAttachmentProjection,
    legacy_associations: DurableAssetAssociationCatalog,
    projection: LocalCurrentDocumentRevisionProjectionWriter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredAttachmentMutationProjection {
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    version_id: VersionId,
    change_kind: ProjectionChangeKind,
}

impl RecoveredAttachmentMutationProjection {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAttachmentMutationProjectionRecoveryOutput {
    recovered: Vec<RecoveredAttachmentMutationProjection>,
    skipped_stale_count: usize,
}

impl LocalAttachmentMutationProjectionRecoveryOutput {
    pub fn recovered(&self) -> &[RecoveredAttachmentMutationProjection] {
        &self.recovered
    }

    pub const fn skipped_stale_count(&self) -> usize {
        self.skipped_stale_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalAttachmentMutationProjectionRecoveryError {
    InvalidPolicy,
    StorageUnavailable,
    CorruptedData,
    RecoveryRequired,
}

impl LocalMutateDocumentAttachmentsRuntime {
    pub fn new(app_data_root: PathBuf, body_policy: DocumentBodyPolicy) -> Self {
        Self {
            usecase: MutateDocumentAttachmentsUsecase::new(),
            fingerprint: LocalDocumentMutationFingerprint::new(),
            metadata: LocalDocumentRevisionMetadataSource::new(),
            versions: LocalVersionStore::with_body_policy(
                app_data_root.join(LOCAL_DOCUMENT_VERSION_ROOT),
                body_policy,
            ),
            pointer: LocalCurrentDocumentVersionPointer::new(
                app_data_root.join(LOCAL_DOCUMENT_POINTER_ROOT),
            ),
            journal: LocalDocumentOperationJournal::new(
                app_data_root.join(LOCAL_DOCUMENT_JOURNAL_ROOT),
            ),
            current_documents: LocalDocumentRepository::with_body_policy(
                app_data_root.join(LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT),
                body_policy,
            ),
            attachment_projection: LocalCurrentDocumentAttachmentProjection::new(
                app_data_root.clone(),
            ),
            legacy_associations: DurableAssetAssociationCatalog::new(app_data_root.clone()),
            projection: LocalCurrentDocumentRevisionProjectionWriter::new(
                app_data_root,
                body_policy,
            ),
        }
    }

    pub fn execute(
        &mut self,
        input: MutateDocumentAttachmentsInput,
    ) -> Result<MutateDocumentAttachmentsOutput, MutateDocumentAttachmentsError> {
        let workspace_id = WorkspaceId::new(input.workspace_id())
            .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?;
        let document_id = DocumentId::new(input.document_id())
            .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?;
        let path = self
            .current_documents
            .get_current_by_id(&workspace_id, &document_id)
            .map_err(|_| MutateDocumentAttachmentsError::RecoveryRequired)?
            .ok_or(MutateDocumentAttachmentsError::RecoveryRequired)?
            .path()
            .clone();
        let input = self.resolve_legacy_attachment_baseline(input, &workspace_id, &document_id)?;
        let record_reader = self.versions.clone();
        let allocator_reader = self.versions.clone();
        let pointer_reader = self.pointer.clone();
        let allocator =
            LocalDocumentRevisionNumberAllocator::new(&allocator_reader, &pointer_reader);
        let mut commit = GuardedDocumentRevisionCommit::new(&mut self.versions, &mut self.pointer);
        let output = self.usecase.execute(
            input,
            &record_reader,
            &pointer_reader,
            &self.fingerprint,
            &self.metadata,
            &self.metadata,
            &self.metadata,
            &allocator,
            &mut commit,
            &mut self.journal,
        )?;
        let record = self
            .versions
            .get_committed_version_record(&workspace_id, &document_id, output.version_id())
            .map_err(|_| MutateDocumentAttachmentsError::RecoveryRequired)?
            .ok_or(MutateDocumentAttachmentsError::RecoveryRequired)?;
        ProjectCurrentDocumentAttachmentsUsecase::new()
            .execute(
                ProjectCurrentDocumentAttachmentsInput::new(
                    workspace_id.as_str(),
                    document_id.as_str(),
                    record.clone(),
                ),
                &mut self.attachment_projection,
            )
            .map_err(|_| MutateDocumentAttachmentsError::RecoveryRequired)?;
        ProjectCurrentDocumentRevisionUsecase::new()
            .execute(
                ProjectCurrentDocumentRevisionInput::new(
                    workspace_id.as_str(),
                    path.as_str(),
                    record,
                ),
                &mut self.projection,
            )
            .map_err(|_| MutateDocumentAttachmentsError::RecoveryRequired)?;
        Ok(output)
    }

    fn resolve_legacy_attachment_baseline(
        &self,
        input: MutateDocumentAttachmentsInput,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<MutateDocumentAttachmentsInput, MutateDocumentAttachmentsError> {
        let expected_version = VersionId::new(input.expected_current_version())
            .map_err(|_| MutateDocumentAttachmentsError::InvalidInput)?;
        let expected = self
            .versions
            .get_committed_version_record(workspace_id, document_id, &expected_version)
            .map_err(|_| MutateDocumentAttachmentsError::StorageUnavailable)?;
        let Some(expected) = expected else {
            return Ok(input);
        };
        if !expected.snapshot().attachment_state().is_legacy_unknown() {
            return Ok(input);
        }
        let baseline = self
            .legacy_associations
            .list_assets(workspace_id, document_id, 500)
            .map_err(map_legacy_association_error)?
            .into_iter()
            .map(|association| {
                AssetReference::new(association.asset_id().clone(), association.label())
                    .map_err(|_| MutateDocumentAttachmentsError::CorruptedData)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(input.with_legacy_attachment_baseline(baseline))
    }

    pub fn recover_committed(
        &mut self,
        limit: usize,
    ) -> Result<
        LocalAttachmentMutationProjectionRecoveryOutput,
        LocalAttachmentMutationProjectionRecoveryError,
    > {
        let candidates = self
            .journal
            .list_committed_attachment_mutation_candidates(limit)
            .map_err(map_recovery_scan_error)?;
        let mut recovered = Vec::new();
        let mut skipped_stale_count = 0;
        for candidate in candidates {
            let current = self
                .pointer
                .load_current_version(candidate.workspace_id(), candidate.document_id())
                .map_err(|_| LocalAttachmentMutationProjectionRecoveryError::StorageUnavailable)?;
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
                .map_err(|_| LocalAttachmentMutationProjectionRecoveryError::StorageUnavailable)?
                .ok_or(LocalAttachmentMutationProjectionRecoveryError::CorruptedData)?;
            let path = self
                .current_documents
                .get_current_by_id(candidate.workspace_id(), candidate.document_id())
                .map_err(|_| LocalAttachmentMutationProjectionRecoveryError::RecoveryRequired)?
                .ok_or(LocalAttachmentMutationProjectionRecoveryError::RecoveryRequired)?
                .path()
                .clone();
            ProjectCurrentDocumentAttachmentsUsecase::new()
                .execute(
                    ProjectCurrentDocumentAttachmentsInput::new(
                        candidate.workspace_id().as_str(),
                        candidate.document_id().as_str(),
                        record.clone(),
                    ),
                    &mut self.attachment_projection,
                )
                .map_err(|_| LocalAttachmentMutationProjectionRecoveryError::RecoveryRequired)?;
            ProjectCurrentDocumentRevisionUsecase::new()
                .execute(
                    ProjectCurrentDocumentRevisionInput::new(
                        candidate.workspace_id().as_str(),
                        path.as_str(),
                        record,
                    ),
                    &mut self.projection,
                )
                .map_err(|_| LocalAttachmentMutationProjectionRecoveryError::RecoveryRequired)?;
            recovered.push(RecoveredAttachmentMutationProjection {
                workspace_id: candidate.workspace_id().clone(),
                document_id: candidate.document_id().clone(),
                version_id: candidate.version_id().clone(),
                change_kind: candidate.change_kind(),
            });
        }
        Ok(LocalAttachmentMutationProjectionRecoveryOutput {
            recovered,
            skipped_stale_count,
        })
    }
}

const fn map_recovery_scan_error(
    error: LocalRestoreCandidateScanError,
) -> LocalAttachmentMutationProjectionRecoveryError {
    match error {
        LocalRestoreCandidateScanError::InvalidLimit => {
            LocalAttachmentMutationProjectionRecoveryError::InvalidPolicy
        }
        LocalRestoreCandidateScanError::StorageUnavailable => {
            LocalAttachmentMutationProjectionRecoveryError::StorageUnavailable
        }
        LocalRestoreCandidateScanError::CorruptedJournal => {
            LocalAttachmentMutationProjectionRecoveryError::CorruptedData
        }
    }
}

const fn map_legacy_association_error(
    error: AssetAssociationCatalogError,
) -> MutateDocumentAttachmentsError {
    match error {
        AssetAssociationCatalogError::StorageUnavailable => {
            MutateDocumentAttachmentsError::StorageUnavailable
        }
        AssetAssociationCatalogError::Conflict
        | AssetAssociationCatalogError::InvalidLimit
        | AssetAssociationCatalogError::CorruptedRecord
        | AssetAssociationCatalogError::UnsupportedSchema => {
            MutateDocumentAttachmentsError::CorruptedData
        }
    }
}
