use std::cell::RefCell;
use std::rc::Rc;

use cabinet_domain::document::DocumentId;
use cabinet_domain::document_revision::DocumentExpectedCurrentVersion;
use cabinet_domain::version::{DocumentRevisionNumber, DocumentSnapshotRef, VersionId};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_revision_metadata::{
    DocumentRevisionClock, DocumentRevisionMetadataPortError, DocumentRevisionNumberAllocator,
    DocumentSnapshotRefGenerator, DocumentVersionIdGenerator,
};
use cabinet_usecases::document_revision_metadata::{
    GenerateDocumentRevisionMetadataError, GenerateDocumentRevisionMetadataInput,
    GenerateDocumentRevisionMetadataUsecase,
};

struct Fakes {
    calls: Rc<RefCell<Vec<&'static str>>>,
    version: Result<VersionId, DocumentRevisionMetadataPortError>,
    timestamp: u64,
    allocation: Result<DocumentRevisionNumber, DocumentRevisionMetadataPortError>,
}

impl DocumentVersionIdGenerator for Fakes {
    fn generate_version_id(&self) -> Result<VersionId, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("version");
        self.version.clone()
    }
}

impl DocumentSnapshotRefGenerator for Fakes {
    fn generate_snapshot_ref(
        &self,
        version_id: &VersionId,
    ) -> Result<DocumentSnapshotRef, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("snapshot");
        DocumentSnapshotRef::new(&format!("snapshot-{}", version_id.as_str()))
            .map_err(|_| DocumentRevisionMetadataPortError::GenerationUnavailable)
    }
}

impl DocumentRevisionClock for Fakes {
    fn now_epoch_ms(&self) -> Result<u64, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("clock");
        Ok(self.timestamp)
    }
}

impl DocumentRevisionNumberAllocator for Fakes {
    fn allocate_next_revision(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        expected_current: &DocumentExpectedCurrentVersion,
    ) -> Result<DocumentRevisionNumber, DocumentRevisionMetadataPortError> {
        self.calls.borrow_mut().push("revision");
        assert!(matches!(
            expected_current,
            DocumentExpectedCurrentVersion::MustMatch(version) if version.as_str() == "version-1"
        ));
        self.allocation
    }
}

#[test]
fn metadata_is_generated_in_explicit_order_with_expected_current() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let fakes = fakes(Rc::clone(&calls));

    let output = GenerateDocumentRevisionMetadataUsecase::new()
        .execute(input(), &fakes, &fakes, &fakes, &fakes)
        .expect("metadata");

    assert_eq!(output.version_id().as_str(), "version-2");
    assert_eq!(output.snapshot_ref().as_str(), "snapshot-version-2");
    assert_eq!(output.created_at_epoch_ms(), 200);
    assert_eq!(output.revision_number().value(), 2);
    assert_eq!(
        *calls.borrow(),
        vec!["version", "snapshot", "clock", "revision"]
    );
}

#[test]
fn generator_failure_short_circuits_remaining_ports() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut fakes = fakes(Rc::clone(&calls));
    fakes.version = Err(DocumentRevisionMetadataPortError::GenerationUnavailable);

    let error = GenerateDocumentRevisionMetadataUsecase::new()
        .execute(input(), &fakes, &fakes, &fakes, &fakes)
        .expect_err("generation failure");

    assert_eq!(
        error,
        GenerateDocumentRevisionMetadataError::GenerationUnavailable
    );
    assert_eq!(*calls.borrow(), vec!["version"]);
}

#[test]
fn zero_timestamp_and_allocator_conflict_are_typed() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut zero_clock = fakes(Rc::clone(&calls));
    zero_clock.timestamp = 0;
    let error = GenerateDocumentRevisionMetadataUsecase::new()
        .execute(input(), &zero_clock, &zero_clock, &zero_clock, &zero_clock)
        .expect_err("zero timestamp");
    assert_eq!(
        error,
        GenerateDocumentRevisionMetadataError::InvalidTimestamp
    );
    assert_eq!(*calls.borrow(), vec!["version", "snapshot", "clock"]);

    calls.borrow_mut().clear();
    let mut conflict = fakes(Rc::clone(&calls));
    conflict.allocation = Err(DocumentRevisionMetadataPortError::Conflict);
    let error = GenerateDocumentRevisionMetadataUsecase::new()
        .execute(input(), &conflict, &conflict, &conflict, &conflict)
        .expect_err("allocation conflict");
    assert_eq!(error, GenerateDocumentRevisionMetadataError::Conflict);
    assert_eq!(
        *calls.borrow(),
        vec!["version", "snapshot", "clock", "revision"]
    );
}

#[test]
fn metadata_port_and_usecase_errors_have_stable_codes() {
    assert_eq!(
        DocumentRevisionMetadataPortError::GenerationUnavailable.code(),
        "document_revision_metadata.generation_unavailable"
    );
    assert_eq!(
        DocumentRevisionMetadataPortError::Conflict.code(),
        "document_revision_metadata.conflict"
    );
    assert_eq!(
        DocumentRevisionMetadataPortError::StorageUnavailable.code(),
        "document_revision_metadata.storage_unavailable"
    );
    assert_eq!(
        GenerateDocumentRevisionMetadataError::InvalidTimestamp.code(),
        "document_revision_metadata.invalid_timestamp"
    );
    assert_eq!(
        GenerateDocumentRevisionMetadataError::GenerationUnavailable.code(),
        "document_revision_metadata.generation_unavailable"
    );
    assert_eq!(
        GenerateDocumentRevisionMetadataError::Conflict.code(),
        "document_revision_metadata.conflict"
    );
    assert_eq!(
        GenerateDocumentRevisionMetadataError::StorageUnavailable.code(),
        "document_revision_metadata.storage_unavailable"
    );
}

fn input() -> GenerateDocumentRevisionMetadataInput {
    GenerateDocumentRevisionMetadataInput::new(
        WorkspaceId::new("workspace-1").expect("workspace"),
        DocumentId::new("doc-1").expect("document"),
        DocumentExpectedCurrentVersion::MustMatch(VersionId::new("version-1").expect("expected")),
    )
}

fn fakes(calls: Rc<RefCell<Vec<&'static str>>>) -> Fakes {
    Fakes {
        calls,
        version: Ok(VersionId::new("version-2").expect("version")),
        timestamp: 200,
        allocation: Ok(DocumentRevisionNumber::new(2).expect("revision")),
    }
}
