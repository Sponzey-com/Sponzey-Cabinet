use cabinet_domain::document_diff_operation::DocumentDiffOperationState;
use cabinet_usecases::document_diff_operation::{
    DocumentDiffOperationCreateOutcome, DocumentDiffOperationEntry,
    DocumentDiffOperationIdGenerator, DocumentDiffOperationRegistry,
    DocumentDiffOperationRegistryError, StartDocumentDiffOperationError,
    StartDocumentDiffOperationInput, StartDocumentDiffOperationUsecase,
};

struct FakeIdGenerator {
    next: Result<String, ()>,
    calls: usize,
}

impl FakeIdGenerator {
    fn succeeding(value: &str) -> Self {
        Self {
            next: Ok(value.to_string()),
            calls: 0,
        }
    }
}

impl DocumentDiffOperationIdGenerator for FakeIdGenerator {
    fn next_id(&mut self) -> Result<String, ()> {
        self.calls += 1;
        self.next.clone()
    }
}

struct FakeRegistry {
    entries: Vec<DocumentDiffOperationEntry>,
    result: Result<DocumentDiffOperationCreateOutcome, DocumentDiffOperationRegistryError>,
}

impl Default for FakeRegistry {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            result: Ok(DocumentDiffOperationCreateOutcome::Created),
        }
    }
}

impl DocumentDiffOperationRegistry for FakeRegistry {
    fn create(
        &mut self,
        entry: DocumentDiffOperationEntry,
    ) -> Result<DocumentDiffOperationCreateOutcome, DocumentDiffOperationRegistryError> {
        self.entries.push(entry);
        self.result
    }

    fn get(
        &self,
        _operation_id: &cabinet_domain::document_diff_operation::DocumentDiffOperationId,
    ) -> Result<Option<DocumentDiffOperationEntry>, DocumentDiffOperationRegistryError> {
        Ok(None)
    }

    fn replace(
        &mut self,
        _entry: DocumentDiffOperationEntry,
        _expected_state: DocumentDiffOperationState,
    ) -> Result<(), DocumentDiffOperationRegistryError> {
        Ok(())
    }
}

#[test]
fn start_current_to_version_accepts_typed_target_and_returns_opaque_token() {
    let mut ids = FakeIdGenerator::succeeding("opaque-operation-1");
    let mut registry = FakeRegistry::default();

    let output = StartDocumentDiffOperationUsecase::new()
        .execute(
            StartDocumentDiffOperationInput::current_to_version(
                "workspace-1",
                "doc-1",
                "version-1",
            ),
            &mut ids,
            &mut registry,
        )
        .unwrap();

    assert_eq!(output.operation_id().as_str(), "opaque-operation-1");
    assert_eq!(output.state(), DocumentDiffOperationState::Accepted);
    assert_eq!(
        output.product_log_event(),
        "document.diff.background.accepted"
    );
    assert_eq!(ids.calls, 1);
    assert_eq!(registry.entries.len(), 1);
    let entry = &registry.entries[0];
    assert_eq!(entry.operation().operation_id(), output.operation_id());
    assert_eq!(entry.target().workspace_id().as_str(), "workspace-1");
    assert_eq!(entry.target().document_id().as_str(), "doc-1");
    assert_eq!(
        entry.target().current_version_id().unwrap().as_str(),
        "version-1"
    );
}

#[test]
fn start_versions_preserves_both_typed_version_ids() {
    let mut ids = FakeIdGenerator::succeeding("opaque-operation-2");
    let mut registry = FakeRegistry::default();

    StartDocumentDiffOperationUsecase::new()
        .execute(
            StartDocumentDiffOperationInput::versions(
                "workspace-1",
                "doc-1",
                "version-1",
                "version-2",
            ),
            &mut ids,
            &mut registry,
        )
        .unwrap();

    let (left, right) = registry.entries[0].target().version_pair().unwrap();
    assert_eq!(left.as_str(), "version-1");
    assert_eq!(right.as_str(), "version-2");
}

#[test]
fn invalid_target_stops_before_id_generation_and_registry_access() {
    let mut ids = FakeIdGenerator::succeeding("unused-operation");
    let mut registry = FakeRegistry::default();

    let error = StartDocumentDiffOperationUsecase::new()
        .execute(
            StartDocumentDiffOperationInput::current_to_version("", "doc-1", "version-1"),
            &mut ids,
            &mut registry,
        )
        .unwrap_err();

    assert_eq!(error, StartDocumentDiffOperationError::InvalidInput);
    assert_eq!(error.code(), "document_diff_operation.invalid_input");
    assert_eq!(ids.calls, 0);
    assert!(registry.entries.is_empty());
}

#[test]
fn invalid_or_unavailable_generated_id_does_not_create_registry_entry() {
    for next in [Ok(" ".to_string()), Err(())] {
        let mut ids = FakeIdGenerator { next, calls: 0 };
        let mut registry = FakeRegistry::default();

        let error = StartDocumentDiffOperationUsecase::new()
            .execute(
                StartDocumentDiffOperationInput::current_to_version(
                    "workspace-1",
                    "doc-1",
                    "version-1",
                ),
                &mut ids,
                &mut registry,
            )
            .unwrap_err();

        assert_eq!(
            error,
            StartDocumentDiffOperationError::OperationIdUnavailable
        );
        assert!(error.retryable());
        assert!(registry.entries.is_empty());
    }
}

#[test]
fn registry_duplicate_and_unavailable_errors_are_distinct() {
    let cases = [
        (
            Ok(DocumentDiffOperationCreateOutcome::AlreadyExists),
            StartDocumentDiffOperationError::AlreadyExists,
            false,
        ),
        (
            Err(DocumentDiffOperationRegistryError::CapacityExceeded),
            StartDocumentDiffOperationError::CapacityExceeded,
            false,
        ),
        (
            Err(DocumentDiffOperationRegistryError::Unavailable),
            StartDocumentDiffOperationError::RegistryUnavailable,
            true,
        ),
    ];

    for (registry_result, expected, retryable) in cases {
        let mut ids = FakeIdGenerator::succeeding("opaque-operation-1");
        let mut registry = FakeRegistry {
            entries: Vec::new(),
            result: registry_result,
        };

        let error = StartDocumentDiffOperationUsecase::new()
            .execute(
                StartDocumentDiffOperationInput::current_to_version(
                    "workspace-1",
                    "doc-1",
                    "version-1",
                ),
                &mut ids,
                &mut registry,
            )
            .unwrap_err();

        assert_eq!(error, expected);
        assert_eq!(error.retryable(), retryable);
        assert_eq!(registry.entries.len(), 1);
    }
}
