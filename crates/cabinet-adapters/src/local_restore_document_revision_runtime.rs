use std::path::PathBuf;

use cabinet_domain::document::{DocumentBodyPolicy, DocumentId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::committed_version_record_reader::CommittedVersionRecordReader;
use cabinet_ports::document_repository::DocumentRepository;
use cabinet_ports::version_store::VersionStore;
use cabinet_usecases::project_current_document_attachments::{
    ProjectCurrentDocumentAttachmentsInput, ProjectCurrentDocumentAttachmentsUsecase,
};
use cabinet_usecases::project_current_document_revision::{
    ProjectCurrentDocumentRevisionInput, ProjectCurrentDocumentRevisionUsecase,
};
use cabinet_usecases::restore_document_revision::{
    RestoreDocumentRevisionError, RestoreDocumentRevisionInput, RestoreDocumentRevisionOutput,
    RestoreDocumentRevisionUsecase,
};
use cabinet_usecases::restore_product_log::{
    NoopRestoreProductLogger, RestoreProductEvent, RestoreProductLogger,
};
use cabinet_usecases::restore_target_asset_preflight::{
    RestoreTargetAssetPreflightError, RestoreTargetAssetPreflightInput,
    RestoreTargetAssetPreflightOutcome, RestoreTargetAssetPreflightUsecase,
};

use crate::guarded_document_revision_commit::GuardedDocumentRevisionCommit;
use crate::local_asset_availability_resolver::LocalAssetAvailabilityResolver;
use crate::local_create_document_revision_runtime::{
    LOCAL_DOCUMENT_JOURNAL_ROOT, LOCAL_DOCUMENT_POINTER_ROOT, LOCAL_DOCUMENT_VERSION_ROOT,
};
use crate::local_current_document_attachment_projection::LocalCurrentDocumentAttachmentProjection;
use crate::local_current_document_revision_projection::{
    LOCAL_CURRENT_DOCUMENT_PROJECTION_ROOT, LocalCurrentDocumentRevisionProjectionWriter,
};
use crate::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use crate::local_document_mutation_fingerprint::LocalDocumentMutationFingerprint;
use crate::local_document_operation_journal::LocalDocumentOperationJournal;
use crate::local_document_repository::LocalDocumentRepository;
use crate::local_document_revision_metadata::{
    LocalDocumentRevisionMetadataSource, LocalDocumentRevisionNumberAllocator,
};
use crate::local_version_store::LocalVersionStore;

pub struct LocalRestoreDocumentRevisionRuntime {
    usecase: RestoreDocumentRevisionUsecase,
    fingerprint: LocalDocumentMutationFingerprint,
    metadata: LocalDocumentRevisionMetadataSource,
    versions: LocalVersionStore,
    pointer: LocalCurrentDocumentVersionPointer,
    journal: LocalDocumentOperationJournal,
    current_documents: LocalDocumentRepository,
    attachment_projection: LocalCurrentDocumentAttachmentProjection,
    projection: LocalCurrentDocumentRevisionProjectionWriter,
    availability: LocalAssetAvailabilityResolver,
    preflight: RestoreTargetAssetPreflightUsecase,
}

impl LocalRestoreDocumentRevisionRuntime {
    pub fn new(app_data_root: PathBuf, body_policy: DocumentBodyPolicy) -> Self {
        Self {
            usecase: RestoreDocumentRevisionUsecase::new(body_policy),
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
            projection: LocalCurrentDocumentRevisionProjectionWriter::new(
                app_data_root.clone(),
                body_policy,
            ),
            availability: LocalAssetAvailabilityResolver::new(app_data_root),
            preflight: RestoreTargetAssetPreflightUsecase::new(),
        }
    }

    pub fn execute(
        &mut self,
        input: RestoreDocumentRevisionInput,
    ) -> Result<RestoreDocumentRevisionOutput, RestoreDocumentRevisionError> {
        self.execute_with_logger(input, &mut NoopRestoreProductLogger)
    }

    pub fn execute_with_logger(
        &mut self,
        input: RestoreDocumentRevisionInput,
        logger: &mut impl RestoreProductLogger,
    ) -> Result<RestoreDocumentRevisionOutput, RestoreDocumentRevisionError> {
        logger.write_restore_product(RestoreProductEvent::Requested);
        let mut primary_committed = false;
        let result = self.execute_after_request(input, &mut primary_committed, logger);
        match &result {
            Ok(_) => {}
            Err(error) => logger.write_restore_product(if primary_committed {
                RestoreProductEvent::RecoveryRequired
            } else {
                restore_failure_event(*error)
            }),
        }
        result
    }

    fn execute_after_request(
        &mut self,
        input: RestoreDocumentRevisionInput,
        primary_committed: &mut bool,
        logger: &mut impl RestoreProductLogger,
    ) -> Result<RestoreDocumentRevisionOutput, RestoreDocumentRevisionError> {
        let workspace_id = WorkspaceId::new(input.workspace_id())
            .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?;
        let document_id = DocumentId::new(input.document_id())
            .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?;
        let target_version_id = cabinet_domain::version::VersionId::new(input.target_version_id())
            .map_err(|_| RestoreDocumentRevisionError::InvalidInput)?;
        let target = self
            .versions
            .get_version_snapshot(&workspace_id, &document_id, &target_version_id)
            .map_err(|_| RestoreDocumentRevisionError::StorageUnavailable)?
            .ok_or(RestoreDocumentRevisionError::NotFound)?;
        match self
            .preflight
            .execute(
                RestoreTargetAssetPreflightInput::new(
                    workspace_id.as_str(),
                    target.attachment_state().clone(),
                ),
                &self.availability,
            )
            .map_err(map_preflight_error)?
        {
            RestoreTargetAssetPreflightOutcome::BlockedMissingAssets(_) => {
                return Err(RestoreDocumentRevisionError::MissingDependency);
            }
            RestoreTargetAssetPreflightOutcome::Available
            | RestoreTargetAssetPreflightOutcome::LegacyPreserved => {}
        }
        let path = self
            .current_documents
            .get_current_by_id(&workspace_id, &document_id)
            .map_err(|_| RestoreDocumentRevisionError::RecoveryRequired)?
            .ok_or(RestoreDocumentRevisionError::RecoveryRequired)?
            .path()
            .clone();
        let record_reader = self.versions.clone();
        let allocator_reader = self.versions.clone();
        let pointer_reader = self.pointer.clone();
        let allocator =
            LocalDocumentRevisionNumberAllocator::new(&allocator_reader, &pointer_reader);
        let mut commit = GuardedDocumentRevisionCommit::new(&mut self.versions, &mut self.pointer);
        let output = self.usecase.execute(
            input,
            &record_reader,
            &self.fingerprint,
            &self.metadata,
            &self.metadata,
            &self.metadata,
            &allocator,
            &mut commit,
            &mut self.journal,
        )?;
        *primary_committed = true;
        logger.write_restore_product(RestoreProductEvent::PrimaryCommitted);
        let record = self
            .versions
            .get_committed_version_record(&workspace_id, &document_id, output.version_id())
            .map_err(|_| RestoreDocumentRevisionError::RecoveryRequired)?
            .ok_or(RestoreDocumentRevisionError::RecoveryRequired)?;
        ProjectCurrentDocumentAttachmentsUsecase::new()
            .execute(
                ProjectCurrentDocumentAttachmentsInput::new(
                    workspace_id.as_str(),
                    document_id.as_str(),
                    record.clone(),
                ),
                &mut self.attachment_projection,
            )
            .map_err(|_| RestoreDocumentRevisionError::RecoveryRequired)?;
        ProjectCurrentDocumentRevisionUsecase::new()
            .execute(
                ProjectCurrentDocumentRevisionInput::new(
                    workspace_id.as_str(),
                    path.as_str(),
                    record,
                ),
                &mut self.projection,
            )
            .map_err(|_| RestoreDocumentRevisionError::RecoveryRequired)?;
        Ok(output)
    }
}

const fn restore_failure_event(error: RestoreDocumentRevisionError) -> RestoreProductEvent {
    match error {
        RestoreDocumentRevisionError::CommitConflict
        | RestoreDocumentRevisionError::OperationConflict => RestoreProductEvent::Conflict,
        RestoreDocumentRevisionError::MissingDependency => RestoreProductEvent::BlockedMissingAsset,
        RestoreDocumentRevisionError::RecoveryRequired => RestoreProductEvent::RecoveryRequired,
        RestoreDocumentRevisionError::InvalidInput
        | RestoreDocumentRevisionError::NotFound
        | RestoreDocumentRevisionError::StorageUnavailable
        | RestoreDocumentRevisionError::FingerprintUnavailable
        | RestoreDocumentRevisionError::MetadataUnavailable
        | RestoreDocumentRevisionError::JournalUnavailable
        | RestoreDocumentRevisionError::CommitUnavailable => RestoreProductEvent::Failed,
    }
}

const fn map_preflight_error(
    error: RestoreTargetAssetPreflightError,
) -> RestoreDocumentRevisionError {
    match error {
        RestoreTargetAssetPreflightError::InvalidInput => {
            RestoreDocumentRevisionError::InvalidInput
        }
        RestoreTargetAssetPreflightError::StorageUnavailable => {
            RestoreDocumentRevisionError::StorageUnavailable
        }
        RestoreTargetAssetPreflightError::CorruptedData => {
            RestoreDocumentRevisionError::RecoveryRequired
        }
    }
}
