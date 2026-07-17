use cabinet_domain::document::{DocumentBody, DocumentBodyPolicy, DocumentId};
use cabinet_domain::document_revision::DocumentOperationId;
use cabinet_domain::version::{
    DocumentRevisionNumber, DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId,
    VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::version_preparation::{
    PreparedVersion, VersionPreparationError, VersionPreparationOutcome, VersionPreparationPort,
};
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot};

struct FakePreparationPort {
    prepared: Option<PreparedVersion>,
}

impl VersionPreparationPort for FakePreparationPort {
    fn prepare_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        operation_id: &DocumentOperationId,
        record: VersionRecord,
    ) -> Result<VersionPreparationOutcome, VersionPreparationError> {
        let prepared = PreparedVersion::new(operation_id.clone(), record);
        self.prepared = Some(prepared.clone());
        Ok(VersionPreparationOutcome::Prepared(prepared))
    }

    fn load_prepared(
        &self,
        _workspace_id: &WorkspaceId,
        operation_id: &DocumentOperationId,
    ) -> Result<Option<PreparedVersion>, VersionPreparationError> {
        Ok(self
            .prepared
            .as_ref()
            .filter(|prepared| prepared.operation_id() == operation_id)
            .cloned())
    }

    fn discard_prepared(
        &mut self,
        _workspace_id: &WorkspaceId,
        operation_id: &DocumentOperationId,
    ) -> Result<(), VersionPreparationError> {
        if self
            .prepared
            .as_ref()
            .is_some_and(|prepared| prepared.operation_id() == operation_id)
        {
            self.prepared = None;
        }
        Ok(())
    }
}

#[test]
fn preparation_port_is_replaceable_and_returns_internal_record() {
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let operation = DocumentOperationId::new("operation-1").expect("operation");
    let record = record("version-1", "Body");
    let mut port = FakePreparationPort { prepared: None };

    let outcome = port
        .prepare_version(&workspace, &operation, record.clone())
        .expect("prepare");
    let prepared = outcome.prepared_version();

    assert_eq!(
        outcome.kind(),
        cabinet_ports::version_preparation::VersionPreparationOutcomeKind::Prepared
    );
    assert_eq!(prepared.operation_id(), &operation);
    assert_eq!(prepared.record(), &record);
    assert_eq!(
        port.load_prepared(&workspace, &operation)
            .expect("load")
            .expect("prepared"),
        prepared.clone()
    );
    port.discard_prepared(&workspace, &operation)
        .expect("discard");
    assert!(
        port.load_prepared(&workspace, &operation)
            .expect("load discarded")
            .is_none()
    );
}

#[test]
fn preparation_errors_have_stable_codes() {
    assert_eq!(
        VersionPreparationError::InvalidRecord.code(),
        "version_preparation.invalid_record"
    );
    assert_eq!(
        VersionPreparationError::Conflict.code(),
        "version_preparation.conflict"
    );
    assert_eq!(
        VersionPreparationError::StorageUnavailable.code(),
        "version_preparation.storage_unavailable"
    );
    assert_eq!(
        VersionPreparationError::CorruptedPrepared.code(),
        "version_preparation.corrupted"
    );
}

fn record(version_id: &str, body: &str) -> VersionRecord {
    let document_id = DocumentId::new("doc-1").expect("document");
    let snapshot_ref = DocumentSnapshotRef::new("snapshot-1").expect("snapshot");
    let entry = VersionEntry::new(
        VersionId::new(version_id).expect("version"),
        document_id.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("writer").expect("author"),
        VersionSummary::new("Updated").expect("summary"),
    )
    .expect("entry")
    .with_created_at_epoch_ms(100)
    .expect("timestamp")
    .with_revision_number(DocumentRevisionNumber::new(1).expect("revision"))
    .expect("assigned revision");
    VersionRecord::new(
        entry,
        VersionSnapshot::new(
            document_id,
            snapshot_ref,
            DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
        ),
    )
    .expect("record")
}
