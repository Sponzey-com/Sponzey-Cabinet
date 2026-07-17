use std::path::PathBuf;

use cabinet_domain::document::{DocumentBodyPolicy, DocumentId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::committed_version_record_reader::CommittedVersionRecordReader;
use cabinet_usecases::create_document_revision::{
    CreateDocumentRevisionError, CreateDocumentRevisionInput, CreateDocumentRevisionOutput,
    CreateDocumentRevisionUsecase,
};
use cabinet_usecases::project_current_document_revision::{
    ProjectCurrentDocumentRevisionInput, ProjectCurrentDocumentRevisionUsecase,
};
use sha2::{Digest, Sha256};

use crate::guarded_document_revision_commit::GuardedDocumentRevisionCommit;
use crate::local_current_document_revision_projection::LocalCurrentDocumentRevisionProjectionWriter;
use crate::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use crate::local_document_mutation_fingerprint::LocalDocumentMutationFingerprint;
use crate::local_document_operation_journal::LocalDocumentOperationJournal;
use crate::local_document_revision_metadata::{
    LocalDocumentRevisionMetadataSource, LocalDocumentRevisionNumberAllocator,
};
use crate::local_version_store::LocalVersionStore;

pub const LOCAL_DOCUMENT_VERSION_ROOT: &str = "document-versions";
pub const LOCAL_DOCUMENT_POINTER_ROOT: &str = "document-current-pointers";
pub const LOCAL_DOCUMENT_JOURNAL_ROOT: &str = "document-operation-journal-store";

pub struct LocalCreateDocumentRevisionRuntime {
    usecase: CreateDocumentRevisionUsecase,
    fingerprint: LocalDocumentMutationFingerprint,
    metadata: LocalDocumentRevisionMetadataSource,
    versions: LocalVersionStore,
    pointer: LocalCurrentDocumentVersionPointer,
    journal: LocalDocumentOperationJournal,
    projection: LocalCurrentDocumentRevisionProjectionWriter,
}

impl LocalCreateDocumentRevisionRuntime {
    pub fn new(app_data_root: PathBuf, body_policy: DocumentBodyPolicy) -> Self {
        Self {
            usecase: CreateDocumentRevisionUsecase::new(body_policy),
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
            projection: LocalCurrentDocumentRevisionProjectionWriter::new(
                app_data_root,
                body_policy,
            ),
        }
    }

    pub fn execute(
        &mut self,
        input: CreateDocumentRevisionInput,
    ) -> Result<CreateDocumentRevisionOutput, CreateDocumentRevisionError> {
        let workspace_id = WorkspaceId::new(input.workspace_id())
            .map_err(|_| CreateDocumentRevisionError::InvalidInput)?;
        let document_id = DocumentId::new(input.document_id())
            .map_err(|_| CreateDocumentRevisionError::InvalidInput)?;
        let version_reader = self.versions.clone();
        let pointer_reader = self.pointer.clone();
        let allocator = LocalDocumentRevisionNumberAllocator::new(&version_reader, &pointer_reader);
        let mut commit = GuardedDocumentRevisionCommit::new(&mut self.versions, &mut self.pointer);
        let output = self.usecase.execute(
            input,
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
            .map_err(|_| CreateDocumentRevisionError::RecoveryRequired)?
            .ok_or(CreateDocumentRevisionError::RecoveryRequired)?;
        ProjectCurrentDocumentRevisionUsecase::new()
            .execute(
                ProjectCurrentDocumentRevisionInput::new(
                    workspace_id.as_str(),
                    &hidden_document_path(&document_id),
                    record,
                ),
                &mut self.projection,
            )
            .map_err(|_| CreateDocumentRevisionError::RecoveryRequired)?;
        Ok(output)
    }
}

fn hidden_document_path(document_id: &DocumentId) -> String {
    let digest = Sha256::digest(document_id.as_str().as_bytes());
    format!("notes/{digest:x}.md")
}
