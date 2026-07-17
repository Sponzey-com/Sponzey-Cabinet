use cabinet_domain::document_revision::DocumentExpectedCurrentVersion;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::document_revision_commit::{
    DocumentRevisionCommitError, DocumentRevisionCommitPort, DocumentRevisionCommitRequest,
    DocumentRevisionCommitResult, DocumentRevisionRecoveryPort,
};
use cabinet_ports::version_preparation::{VersionPreparationError, VersionPreparationPort};
use cabinet_ports::version_publication::VersionPublicationPort;

pub struct GuardedDocumentRevisionCommit<'a, V, P> {
    versions: &'a mut V,
    pointer: &'a mut P,
}

impl<V, P> DocumentRevisionRecoveryPort for GuardedDocumentRevisionCommit<'_, V, P>
where
    V: VersionPreparationPort + VersionPublicationPort,
    P: CurrentDocumentVersionPointerPort,
{
    fn recover_revision(
        &mut self,
        identity: cabinet_domain::document_revision::DocumentOperationIdentity,
    ) -> Result<DocumentRevisionCommitResult, DocumentRevisionCommitError> {
        let prepared = self
            .versions
            .load_prepared(identity.workspace_id(), identity.operation_id())
            .map_err(map_recovery_preparation_error)?
            .ok_or(DocumentRevisionCommitError::RecoveryRequired)?;
        let request = DocumentRevisionCommitRequest::new(identity, prepared.record().clone())?;
        self.commit_revision(request)
    }
}

impl<'a, V, P> GuardedDocumentRevisionCommit<'a, V, P> {
    pub const fn new(versions: &'a mut V, pointer: &'a mut P) -> Self {
        Self { versions, pointer }
    }
}

impl<V, P> DocumentRevisionCommitPort for GuardedDocumentRevisionCommit<'_, V, P>
where
    V: VersionPreparationPort + VersionPublicationPort,
    P: CurrentDocumentVersionPointerPort,
{
    fn commit_revision(
        &mut self,
        request: DocumentRevisionCommitRequest,
    ) -> Result<DocumentRevisionCommitResult, DocumentRevisionCommitError> {
        let identity = request.identity().clone();
        let record = request.record().clone();
        let expected_revision = record
            .entry()
            .revision_number()
            .ok_or(DocumentRevisionCommitError::IdentityMismatch)?;
        let expected_current = match identity.expected_current() {
            DocumentExpectedCurrentVersion::MustNotExist => None,
            DocumentExpectedCurrentVersion::MustMatch(version_id) => Some(version_id.clone()),
        };
        let next_version = record.version_id().clone();

        let prepared = self
            .versions
            .prepare_version(
                identity.workspace_id(),
                identity.operation_id(),
                record.clone(),
            )
            .map_err(map_preparation_error)?;
        if prepared.prepared_version().record() != &record {
            return Err(DocumentRevisionCommitError::IdentityMismatch);
        }

        let cas_result = self.pointer.compare_and_set_current_version(
            identity.workspace_id(),
            identity.document_id(),
            expected_current.as_ref(),
            next_version.clone(),
        );
        if let Err(error) = cas_result {
            let current = self
                .pointer
                .load_current_version(identity.workspace_id(), identity.document_id());
            match current {
                Ok(Some(current)) if current == next_version => {}
                Ok(_) if error == CurrentDocumentVersionPointerError::Conflict => {
                    let _ = self
                        .versions
                        .discard_prepared(identity.workspace_id(), identity.operation_id());
                    return Err(DocumentRevisionCommitError::Conflict);
                }
                Ok(current) if current.as_ref() == expected_current.as_ref() => {
                    return Err(DocumentRevisionCommitError::StorageUnavailable);
                }
                Ok(_) | Err(_) => return Err(DocumentRevisionCommitError::RecoveryRequired),
            }
        }

        let published = self
            .versions
            .publish_prepared(identity.workspace_id(), identity.operation_id())
            .map_err(|_| DocumentRevisionCommitError::RecoveryRequired)?;
        if published.version_id() != &next_version
            || published.revision_number() != expected_revision
        {
            return Err(DocumentRevisionCommitError::RecoveryRequired);
        }

        Ok(DocumentRevisionCommitResult::new(
            next_version,
            expected_revision,
        ))
    }
}

const fn map_preparation_error(error: VersionPreparationError) -> DocumentRevisionCommitError {
    match error {
        VersionPreparationError::InvalidRecord => DocumentRevisionCommitError::IdentityMismatch,
        VersionPreparationError::Conflict => DocumentRevisionCommitError::Conflict,
        VersionPreparationError::StorageUnavailable
        | VersionPreparationError::CorruptedPrepared => {
            DocumentRevisionCommitError::StorageUnavailable
        }
    }
}

const fn map_recovery_preparation_error(
    error: VersionPreparationError,
) -> DocumentRevisionCommitError {
    match error {
        VersionPreparationError::InvalidRecord | VersionPreparationError::Conflict => {
            DocumentRevisionCommitError::IdentityMismatch
        }
        VersionPreparationError::StorageUnavailable
        | VersionPreparationError::CorruptedPrepared => {
            DocumentRevisionCommitError::RecoveryRequired
        }
    }
}
