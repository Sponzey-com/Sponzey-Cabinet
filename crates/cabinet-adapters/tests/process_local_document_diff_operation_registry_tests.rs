use std::sync::{Arc, Barrier};
use std::thread;

use cabinet_adapters::process_local_document_diff_operation_registry::{
    ProcessLocalDocumentDiffOperationRegistry, ProcessLocalDocumentDiffRegistryConfigError,
};
use cabinet_domain::document_diff_operation::{
    DocumentDiffOperation, DocumentDiffOperationEvent, DocumentDiffOperationId,
    DocumentDiffOperationState,
};
use cabinet_domain::document_diff_query::DocumentDiffQueryTarget;
use cabinet_usecases::document_diff_operation::{
    DocumentDiffOperationCreateOutcome, DocumentDiffOperationEntry, DocumentDiffOperationRegistry,
    DocumentDiffOperationRegistryError,
};

#[test]
fn registry_requires_positive_immutable_capacity() {
    assert_eq!(
        ProcessLocalDocumentDiffOperationRegistry::new(0).unwrap_err(),
        ProcessLocalDocumentDiffRegistryConfigError::InvalidCapacity
    );
    let registry = ProcessLocalDocumentDiffOperationRegistry::new(2).unwrap();
    assert_eq!(registry.capacity(), 2);
}

#[test]
fn create_get_duplicate_and_capacity_are_distinct() {
    let mut registry = ProcessLocalDocumentDiffOperationRegistry::new(1).unwrap();
    let first = accepted_entry("operation-1");
    assert_eq!(
        registry.create(first.clone()).unwrap(),
        DocumentDiffOperationCreateOutcome::Created
    );
    assert_eq!(
        registry.create(first).unwrap(),
        DocumentDiffOperationCreateOutcome::AlreadyExists
    );
    assert_eq!(
        registry.create(accepted_entry("operation-2")).unwrap_err(),
        DocumentDiffOperationRegistryError::CapacityExceeded
    );
    assert_eq!(
        registry.get(&operation_id("operation-1")).unwrap(),
        Some(accepted_entry("operation-1"))
    );
    assert_eq!(registry.get(&operation_id("operation-2")).unwrap(), None);
}

#[test]
fn clones_share_entries_while_a_new_instance_starts_empty() {
    let mut registry = ProcessLocalDocumentDiffOperationRegistry::new(2).unwrap();
    let clone = registry.clone();
    registry.create(accepted_entry("operation-1")).unwrap();

    assert!(clone.get(&operation_id("operation-1")).unwrap().is_some());

    let fresh = ProcessLocalDocumentDiffOperationRegistry::new(2).unwrap();
    assert_eq!(fresh.get(&operation_id("operation-1")).unwrap(), None);
}

#[test]
fn replace_uses_expected_state_and_conflict_does_not_overwrite() {
    let mut registry = ProcessLocalDocumentDiffOperationRegistry::new(2).unwrap();
    registry.create(accepted_entry("operation-1")).unwrap();
    let running = transitioned_entry("operation-1", DocumentDiffOperationEvent::Start);

    registry
        .replace(running.clone(), DocumentDiffOperationState::Accepted)
        .unwrap();
    assert_eq!(
        registry
            .replace(
                accepted_entry("operation-1"),
                DocumentDiffOperationState::Accepted,
            )
            .unwrap_err(),
        DocumentDiffOperationRegistryError::Conflict
    );
    assert_eq!(
        registry.get(&operation_id("operation-1")).unwrap().unwrap(),
        running
    );
}

#[test]
fn concurrent_compare_and_set_allows_exactly_one_winner() {
    let mut registry = ProcessLocalDocumentDiffOperationRegistry::new(2).unwrap();
    registry.create(accepted_entry("operation-1")).unwrap();
    let barrier = Arc::new(Barrier::new(3));

    let handles = [
        DocumentDiffOperationEvent::Start,
        DocumentDiffOperationEvent::Cancel,
    ]
    .into_iter()
    .map(|event| {
        let mut registry = registry.clone();
        let barrier = Arc::clone(&barrier);
        thread::spawn(move || {
            let replacement = transitioned_entry("operation-1", event);
            barrier.wait();
            registry.replace(replacement, DocumentDiffOperationState::Accepted)
        })
    })
    .collect::<Vec<_>>();

    barrier.wait();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| {
                matches!(result, Err(DocumentDiffOperationRegistryError::Conflict))
            })
            .count(),
        1
    );
    let state = registry
        .get(&operation_id("operation-1"))
        .unwrap()
        .unwrap()
        .operation()
        .state();
    assert!(matches!(
        state,
        DocumentDiffOperationState::Running | DocumentDiffOperationState::Cancelled
    ));
}

fn accepted_entry(id: &str) -> DocumentDiffOperationEntry {
    DocumentDiffOperationEntry::new(DocumentDiffOperation::accepted(operation_id(id)), target())
        .unwrap()
}

fn transitioned_entry(id: &str, event: DocumentDiffOperationEvent) -> DocumentDiffOperationEntry {
    let operation = DocumentDiffOperation::accepted(operation_id(id))
        .transition(event)
        .unwrap()
        .into_operation();
    DocumentDiffOperationEntry::new(operation, target()).unwrap()
}

fn operation_id(value: &str) -> DocumentDiffOperationId {
    DocumentDiffOperationId::new(value).unwrap()
}

fn target() -> DocumentDiffQueryTarget {
    DocumentDiffQueryTarget::versions("workspace-1", "doc-1", "version-1", "version-2").unwrap()
}
