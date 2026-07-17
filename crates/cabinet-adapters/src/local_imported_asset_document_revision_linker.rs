use cabinet_domain::asset::AssetAssociation;
use cabinet_domain::attachment_snapshot_mutation::AttachmentSnapshotDelta;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::imported_asset_document_link::{
    ImportedAssetDocumentLinkError, ImportedAssetDocumentLinkOutcome, ImportedAssetDocumentLinkPort,
};
use cabinet_usecases::mutate_document_attachments::{
    MutateDocumentAttachmentsError, MutateDocumentAttachmentsInput,
    MutateDocumentAttachmentsOutcomeKind,
};

use crate::local_mutate_document_attachments_runtime::LocalMutateDocumentAttachmentsRuntime;

pub struct LocalImportedAssetDocumentRevisionLinker {
    runtime: LocalMutateDocumentAttachmentsRuntime,
    operation_id: String,
    expected_current_version: String,
    author: String,
    summary: String,
}

impl LocalImportedAssetDocumentRevisionLinker {
    pub fn new(
        runtime: LocalMutateDocumentAttachmentsRuntime,
        operation_id: &str,
        expected_current_version: &str,
        author: &str,
        summary: &str,
    ) -> Self {
        Self {
            runtime,
            operation_id: operation_id.to_string(),
            expected_current_version: expected_current_version.to_string(),
            author: author.to_string(),
            summary: summary.to_string(),
        }
    }
}

impl ImportedAssetDocumentLinkPort for LocalImportedAssetDocumentRevisionLinker {
    fn link_imported_asset(
        &mut self,
        workspace: &WorkspaceId,
        association: AssetAssociation,
    ) -> Result<ImportedAssetDocumentLinkOutcome, ImportedAssetDocumentLinkError> {
        let output = self
            .runtime
            .execute(MutateDocumentAttachmentsInput::link(
                &self.operation_id,
                workspace.as_str(),
                association.document_id().as_str(),
                &self.expected_current_version,
                association.asset_id().as_str(),
                association.label(),
                &self.author,
                &self.summary,
            ))
            .map_err(map_mutation_error)?;
        match (output.kind(), output.delta()) {
            (MutateDocumentAttachmentsOutcomeKind::Fresh, AttachmentSnapshotDelta::Linked)
            | (MutateDocumentAttachmentsOutcomeKind::Fresh, AttachmentSnapshotDelta::Relabeled) => {
                Ok(ImportedAssetDocumentLinkOutcome::Linked)
            }
            (MutateDocumentAttachmentsOutcomeKind::Replayed, _)
            | (
                MutateDocumentAttachmentsOutcomeKind::NoChange,
                AttachmentSnapshotDelta::Unchanged,
            ) => Ok(ImportedAssetDocumentLinkOutcome::AlreadyLinked),
            _ => Err(ImportedAssetDocumentLinkError::CorruptedRecord),
        }
    }
}

const fn map_mutation_error(
    error: MutateDocumentAttachmentsError,
) -> ImportedAssetDocumentLinkError {
    match error {
        MutateDocumentAttachmentsError::InvalidInput => {
            ImportedAssetDocumentLinkError::InvalidInput
        }
        MutateDocumentAttachmentsError::NotFound => ImportedAssetDocumentLinkError::NotFound,
        MutateDocumentAttachmentsError::LegacyBaselineRequired => {
            ImportedAssetDocumentLinkError::LegacyBaselineRequired
        }
        MutateDocumentAttachmentsError::CorruptedData => {
            ImportedAssetDocumentLinkError::CorruptedRecord
        }
        MutateDocumentAttachmentsError::OperationConflict => {
            ImportedAssetDocumentLinkError::OperationConflict
        }
        MutateDocumentAttachmentsError::CommitConflict => {
            ImportedAssetDocumentLinkError::CurrentConflict
        }
        MutateDocumentAttachmentsError::RecoveryRequired => {
            ImportedAssetDocumentLinkError::RecoveryRequired
        }
        MutateDocumentAttachmentsError::StorageUnavailable
        | MutateDocumentAttachmentsError::FingerprintUnavailable
        | MutateDocumentAttachmentsError::MetadataUnavailable
        | MutateDocumentAttachmentsError::JournalUnavailable
        | MutateDocumentAttachmentsError::CommitUnavailable => {
            ImportedAssetDocumentLinkError::StorageUnavailable
        }
    }
}
