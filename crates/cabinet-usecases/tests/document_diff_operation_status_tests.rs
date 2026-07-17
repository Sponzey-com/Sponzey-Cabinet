use std::cell::Cell;

use cabinet_domain::document_diff_operation::{
    DocumentDiffOperation, DocumentDiffOperationEvent, DocumentDiffOperationId,
    DocumentDiffOperationSideEffect, DocumentDiffOperationState,
};
use cabinet_domain::document_diff_query::DocumentDiffQueryTarget;
use cabinet_domain::version::VersionId;
use cabinet_usecases::attachment_diff::AttachmentDiff;
use cabinet_usecases::authoritative_document_diff::CompareAuthoritativeDocumentRevisionsOutput;
use cabinet_usecases::document_diff_operation::{
    CancelDocumentDiffOperationError, CancelDocumentDiffOperationInput,
    CancelDocumentDiffOperationUsecase, DocumentDiffOperationCreateOutcome,
    DocumentDiffOperationEntry, DocumentDiffOperationPayload, DocumentDiffOperationRegistry,
    DocumentDiffOperationRegistryError, GetDocumentDiffOperationStatusError,
    GetDocumentDiffOperationStatusInput, GetDocumentDiffOperationStatusUsecase,
};

struct FakeRegistry {
    entry: Option<DocumentDiffOperationEntry>,
    error: Option<DocumentDiffOperationRegistryError>,
    conflict_on_replace: bool,
    get_calls: Cell<usize>,
    replace_calls: usize,
}

impl FakeRegistry {
    fn with_state(state: DocumentDiffOperationState) -> Self {
        Self {
            entry: Some(entry_in(state)),
            error: None,
            conflict_on_replace: false,
            get_calls: Cell::new(0),
            replace_calls: 0,
        }
    }

    fn empty() -> Self {
        Self {
            entry: None,
            error: None,
            conflict_on_replace: false,
            get_calls: Cell::new(0),
            replace_calls: 0,
        }
    }
}

impl DocumentDiffOperationRegistry for FakeRegistry {
    fn create(
        &mut self,
        entry: DocumentDiffOperationEntry,
    ) -> Result<DocumentDiffOperationCreateOutcome, DocumentDiffOperationRegistryError> {
        self.entry = Some(entry);
        Ok(DocumentDiffOperationCreateOutcome::Created)
    }

    fn get(
        &self,
        _operation_id: &DocumentDiffOperationId,
    ) -> Result<Option<DocumentDiffOperationEntry>, DocumentDiffOperationRegistryError> {
        self.get_calls.set(self.get_calls.get() + 1);
        if let Some(error) = self.error {
            return Err(error);
        }
        Ok(self.entry.clone())
    }

    fn replace(
        &mut self,
        entry: DocumentDiffOperationEntry,
        expected_state: DocumentDiffOperationState,
    ) -> Result<(), DocumentDiffOperationRegistryError> {
        self.replace_calls += 1;
        if let Some(error) = self.error {
            return Err(error);
        }
        if self.conflict_on_replace
            || self.entry.as_ref().map(|stored| stored.operation().state()) != Some(expected_state)
        {
            return Err(DocumentDiffOperationRegistryError::Conflict);
        }
        self.entry = Some(entry);
        Ok(())
    }
}

#[test]
fn status_returns_current_state_without_mutation_and_missing_as_expired() {
    let mut existing = FakeRegistry::with_state(DocumentDiffOperationState::Running);
    let output = GetDocumentDiffOperationStatusUsecase::new()
        .execute(
            GetDocumentDiffOperationStatusInput::new("opaque-operation-1"),
            &mut existing,
        )
        .unwrap();

    assert_eq!(output.state(), DocumentDiffOperationState::Running);
    assert_eq!(output.product_log_event(), None);
    assert_eq!(existing.get_calls.get(), 1);
    assert_eq!(existing.replace_calls, 0);

    let mut missing = FakeRegistry::empty();
    let output = GetDocumentDiffOperationStatusUsecase::new()
        .execute(
            GetDocumentDiffOperationStatusInput::new("opaque-operation-1"),
            &mut missing,
        )
        .unwrap();
    assert_eq!(output.state(), DocumentDiffOperationState::Expired);
    assert_eq!(
        output.product_log_event(),
        Some("document.diff.background.expired")
    );
    assert_eq!(missing.replace_calls, 0);
}

#[test]
fn status_rejects_invalid_token_and_maps_registry_unavailable() {
    let mut registry = FakeRegistry::empty();
    let error = GetDocumentDiffOperationStatusUsecase::new()
        .execute(GetDocumentDiffOperationStatusInput::new(" "), &mut registry)
        .unwrap_err();
    assert_eq!(error, GetDocumentDiffOperationStatusError::InvalidInput);
    assert_eq!(registry.get_calls.get(), 0);

    registry.error = Some(DocumentDiffOperationRegistryError::Unavailable);
    let error = GetDocumentDiffOperationStatusUsecase::new()
        .execute(
            GetDocumentDiffOperationStatusInput::new("opaque-operation-1"),
            &mut registry,
        )
        .unwrap_err();
    assert_eq!(
        error,
        GetDocumentDiffOperationStatusError::RegistryUnavailable
    );
    assert!(error.retryable());
}

#[test]
fn cancel_accepted_and_running_use_domain_transition_and_expected_state_replace() {
    for (state, expected_side_effect) in [
        (DocumentDiffOperationState::Accepted, None),
        (
            DocumentDiffOperationState::Running,
            Some(DocumentDiffOperationSideEffect::RequestCancellation),
        ),
    ] {
        let mut registry = FakeRegistry::with_state(state);
        let output = CancelDocumentDiffOperationUsecase::new()
            .execute(
                CancelDocumentDiffOperationInput::new("opaque-operation-1"),
                &mut registry,
            )
            .unwrap();

        assert_eq!(output.state(), DocumentDiffOperationState::Cancelled);
        assert_eq!(output.side_effect(), expected_side_effect);
        assert_eq!(
            output.product_log_event(),
            Some("document.diff.background.cancelled")
        );
        assert_eq!(registry.get_calls.get(), 1);
        assert_eq!(registry.replace_calls, 1);
        assert_eq!(
            registry.entry.as_ref().unwrap().operation().state(),
            DocumentDiffOperationState::Cancelled
        );
    }
}

#[test]
fn cancel_is_idempotent_for_cancelled_and_missing_or_expired_is_expired() {
    let mut cancelled = FakeRegistry::with_state(DocumentDiffOperationState::Cancelled);
    let output = CancelDocumentDiffOperationUsecase::new()
        .execute(
            CancelDocumentDiffOperationInput::new("opaque-operation-1"),
            &mut cancelled,
        )
        .unwrap();
    assert_eq!(output.state(), DocumentDiffOperationState::Cancelled);
    assert_eq!(output.side_effect(), None);
    assert_eq!(output.product_log_event(), None);
    assert_eq!(cancelled.replace_calls, 0);

    for mut registry in [
        FakeRegistry::empty(),
        FakeRegistry::with_state(DocumentDiffOperationState::Expired),
    ] {
        let output = CancelDocumentDiffOperationUsecase::new()
            .execute(
                CancelDocumentDiffOperationInput::new("opaque-operation-1"),
                &mut registry,
            )
            .unwrap();
        assert_eq!(output.state(), DocumentDiffOperationState::Expired);
        assert_eq!(output.side_effect(), None);
        assert_eq!(registry.replace_calls, 0);
    }
}

#[test]
fn cancel_rejects_completed_or_failed_and_maps_replace_conflict() {
    for state in [
        DocumentDiffOperationState::Completed,
        DocumentDiffOperationState::Failed,
    ] {
        let mut registry = FakeRegistry::with_state(state);
        let error = CancelDocumentDiffOperationUsecase::new()
            .execute(
                CancelDocumentDiffOperationInput::new("opaque-operation-1"),
                &mut registry,
            )
            .unwrap_err();
        assert_eq!(error, CancelDocumentDiffOperationError::CancellationTooLate);
        assert_eq!(registry.replace_calls, 0);
    }

    let mut registry = FakeRegistry::with_state(DocumentDiffOperationState::Running);
    registry.conflict_on_replace = true;
    let error = CancelDocumentDiffOperationUsecase::new()
        .execute(
            CancelDocumentDiffOperationInput::new("opaque-operation-1"),
            &mut registry,
        )
        .unwrap_err();
    assert_eq!(error, CancelDocumentDiffOperationError::Conflict);
    assert!(error.retryable());
}

fn entry_in(state: DocumentDiffOperationState) -> DocumentDiffOperationEntry {
    let id = DocumentDiffOperationId::new("opaque-operation-1").unwrap();
    let operation = if state == DocumentDiffOperationState::Accepted {
        DocumentDiffOperation::accepted(id)
    } else if state == DocumentDiffOperationState::Running {
        DocumentDiffOperation::accepted(id)
            .transition(DocumentDiffOperationEvent::Start)
            .unwrap()
            .into_operation()
    } else {
        DocumentDiffOperation::restore(id, state)
    };
    let target =
        DocumentDiffQueryTarget::current_to_version("workspace-1", "doc-1", "version-1").unwrap();
    match state {
        DocumentDiffOperationState::Completed => {
            let computation = cabinet_usecases::document_diff::DocumentLineDiffService::default()
                .compare("before\n", "after\n");
            DocumentDiffOperationEntry::with_payload(
                operation,
                target,
                DocumentDiffOperationPayload::Completed(
                    CompareAuthoritativeDocumentRevisionsOutput::new(
                        VersionId::new("version-current").unwrap(),
                        VersionId::new("version-1").unwrap(),
                        computation,
                        AttachmentDiff::LegacyUnknown,
                    ),
                ),
            )
            .unwrap()
        }
        DocumentDiffOperationState::Failed => DocumentDiffOperationEntry::with_payload(
            operation,
            target,
            DocumentDiffOperationPayload::Failed {
                error_code: "document.diff.failed",
            },
        )
        .unwrap(),
        _ => DocumentDiffOperationEntry::new(operation, target).unwrap(),
    }
}
