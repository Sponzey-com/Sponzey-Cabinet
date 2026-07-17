use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use cabinet_domain::document_diff_operation::{
    DocumentDiffOperationId, DocumentDiffOperationState,
};
use cabinet_usecases::document_diff_operation::{
    DocumentDiffOperationCreateOutcome, DocumentDiffOperationEntry, DocumentDiffOperationRegistry,
    DocumentDiffOperationRegistryError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessLocalDocumentDiffRegistryConfigError {
    InvalidCapacity,
}

#[derive(Debug, Clone)]
pub struct ProcessLocalDocumentDiffOperationRegistry {
    capacity: usize,
    entries: Arc<Mutex<HashMap<String, DocumentDiffOperationEntry>>>,
}

impl ProcessLocalDocumentDiffOperationRegistry {
    pub fn new(capacity: usize) -> Result<Self, ProcessLocalDocumentDiffRegistryConfigError> {
        if capacity == 0 {
            return Err(ProcessLocalDocumentDiffRegistryConfigError::InvalidCapacity);
        }
        Ok(Self {
            capacity,
            entries: Arc::new(Mutex::new(HashMap::with_capacity(capacity))),
        })
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    fn lock_entries(
        &self,
    ) -> Result<
        MutexGuard<'_, HashMap<String, DocumentDiffOperationEntry>>,
        DocumentDiffOperationRegistryError,
    > {
        self.entries
            .lock()
            .map_err(|_| DocumentDiffOperationRegistryError::Unavailable)
    }
}

impl DocumentDiffOperationRegistry for ProcessLocalDocumentDiffOperationRegistry {
    fn create(
        &mut self,
        entry: DocumentDiffOperationEntry,
    ) -> Result<DocumentDiffOperationCreateOutcome, DocumentDiffOperationRegistryError> {
        let key = entry.operation().operation_id().as_str().to_string();
        let mut entries = self.lock_entries()?;
        if entries.contains_key(&key) {
            return Ok(DocumentDiffOperationCreateOutcome::AlreadyExists);
        }
        if entries.len() >= self.capacity {
            return Err(DocumentDiffOperationRegistryError::CapacityExceeded);
        }
        entries.insert(key, entry);
        Ok(DocumentDiffOperationCreateOutcome::Created)
    }

    fn get(
        &self,
        operation_id: &DocumentDiffOperationId,
    ) -> Result<Option<DocumentDiffOperationEntry>, DocumentDiffOperationRegistryError> {
        Ok(self.lock_entries()?.get(operation_id.as_str()).cloned())
    }

    fn replace(
        &mut self,
        entry: DocumentDiffOperationEntry,
        expected_state: DocumentDiffOperationState,
    ) -> Result<(), DocumentDiffOperationRegistryError> {
        let key = entry.operation().operation_id().as_str().to_string();
        let mut entries = self.lock_entries()?;
        let current = entries
            .get(&key)
            .ok_or(DocumentDiffOperationRegistryError::Conflict)?;
        if current.operation().state() != expected_state {
            return Err(DocumentDiffOperationRegistryError::Conflict);
        }
        entries.insert(key, entry);
        Ok(())
    }
}
