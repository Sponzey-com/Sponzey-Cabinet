use std::collections::HashMap;

use cabinet_domain::canvas::{CanvasId, CanvasRevision};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository, CanvasRepositoryError};

#[derive(Debug, Default, Clone)]
pub struct LocalCanvasRepository {
    records: HashMap<(String, String), CanvasRecord>,
}

impl LocalCanvasRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CanvasRepository for LocalCanvasRepository {
    fn create_canvas(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        let key = (
            workspace_id.as_str().to_string(),
            record.canvas().id().as_str().to_string(),
        );
        if self.records.contains_key(&key) {
            return Err(CanvasRepositoryError::AlreadyExists);
        }
        self.records.insert(key, record);
        Ok(())
    }

    fn replace_canvas(
        &mut self,
        workspace_id: &WorkspaceId,
        expected_revision: CanvasRevision,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        let key = (
            workspace_id.as_str().to_string(),
            record.canvas().id().as_str().to_string(),
        );
        let current = self
            .records
            .get(&key)
            .ok_or(CanvasRepositoryError::VersionConflict)?;
        if current.revision() != expected_revision
            || record.revision().value() != expected_revision.value() + 1
        {
            return Err(CanvasRepositoryError::VersionConflict);
        }
        self.records.insert(key, record);
        Ok(())
    }

    fn get_canvas(
        &self,
        workspace_id: &WorkspaceId,
        canvas_id: &CanvasId,
    ) -> Result<Option<CanvasRecord>, CanvasRepositoryError> {
        Ok(self
            .records
            .get(&(
                workspace_id.as_str().to_string(),
                canvas_id.as_str().to_string(),
            ))
            .cloned())
    }
}
