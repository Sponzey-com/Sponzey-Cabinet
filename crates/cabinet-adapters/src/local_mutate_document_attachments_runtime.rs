use std::path::PathBuf;

use cabinet_domain::document::{DocumentBodyPolicy, DocumentId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::committed_version_record_reader::CommittedVersionRecordReader;
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
use crate::local_document_operation_journal::LocalDocumentOperationJournal;
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
    projection: LocalCurrentDocumentRevisionProjectionWriter,
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
}
