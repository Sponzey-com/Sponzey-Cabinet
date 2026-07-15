use std::collections::HashMap;

use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::graph_projection::{
    GraphProjectionError, GraphProjectionRecord, GraphProjectionStore,
};

#[derive(Debug, Default, Clone)]
pub struct LocalGraphProjectionStore {
    records: HashMap<(String, String), GraphProjectionRecord>,
}

impl LocalGraphProjectionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl GraphProjectionStore for LocalGraphProjectionStore {
    fn replace_projection(
        &mut self,
        workspace_id: &WorkspaceId,
        record: GraphProjectionRecord,
    ) -> Result<(), GraphProjectionError> {
        self.records.insert(
            (
                workspace_id.as_str().to_string(),
                record.graph().center_document_id().as_str().to_string(),
            ),
            record,
        );
        Ok(())
    }

    fn delete_projection(
        &mut self,
        workspace_id: &WorkspaceId,
        center_document_id: &DocumentId,
    ) -> Result<(), GraphProjectionError> {
        self.records.remove(&(
            workspace_id.as_str().to_string(),
            center_document_id.as_str().to_string(),
        ));
        Ok(())
    }

    fn get_projection(
        &self,
        workspace_id: &WorkspaceId,
        center_document_id: &DocumentId,
    ) -> Result<Option<GraphProjectionRecord>, GraphProjectionError> {
        Ok(self
            .records
            .get(&(
                workspace_id.as_str().to_string(),
                center_document_id.as_str().to_string(),
            ))
            .cloned())
    }
}
