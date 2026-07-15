use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::KnowledgeGraph;
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphProjectionRecord {
    graph: KnowledgeGraph,
    freshness_revision: String,
}

impl GraphProjectionRecord {
    pub fn new(graph: KnowledgeGraph) -> Result<Self, GraphProjectionError> {
        Self::new_with_revision(graph, "legacy-unversioned")
    }

    pub fn new_with_revision(
        graph: KnowledgeGraph,
        freshness_revision: &str,
    ) -> Result<Self, GraphProjectionError> {
        let freshness_revision = freshness_revision.trim();
        if freshness_revision.is_empty() || freshness_revision.chars().any(char::is_control) {
            return Err(GraphProjectionError::InvalidInput);
        }
        Ok(Self {
            graph,
            freshness_revision: freshness_revision.to_string(),
        })
    }

    pub fn graph(&self) -> &KnowledgeGraph {
        &self.graph
    }

    pub fn freshness_revision(&self) -> &str {
        &self.freshness_revision
    }
}

pub trait GraphProjectionStore {
    fn replace_projection(
        &mut self,
        workspace_id: &WorkspaceId,
        record: GraphProjectionRecord,
    ) -> Result<(), GraphProjectionError>;

    fn delete_projection(
        &mut self,
        _workspace_id: &WorkspaceId,
        _center_document_id: &DocumentId,
    ) -> Result<(), GraphProjectionError> {
        Err(GraphProjectionError::StorageUnavailable)
    }

    fn get_projection(
        &self,
        workspace_id: &WorkspaceId,
        center_document_id: &DocumentId,
    ) -> Result<Option<GraphProjectionRecord>, GraphProjectionError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceGraphProjectionPage {
    records: Vec<GraphProjectionRecord>,
    next_cursor: Option<String>,
}
impl WorkspaceGraphProjectionPage {
    pub fn new(records: Vec<GraphProjectionRecord>, next_cursor: Option<String>) -> Self {
        Self {
            records,
            next_cursor,
        }
    }
    pub fn records(&self) -> &[GraphProjectionRecord] {
        &self.records
    }
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}

pub trait WorkspaceGraphProjectionReader {
    fn list_workspace_projections(
        &self,
        workspace_id: &WorkspaceId,
        after_center_id: Option<&str>,
        limit: usize,
    ) -> Result<WorkspaceGraphProjectionPage, GraphProjectionError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphProjectionError {
    InvalidInput,
    StorageUnavailable,
    CorruptedProjection,
}

impl GraphProjectionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "graph_projection.invalid_input",
            Self::StorageUnavailable => "graph_projection.storage_unavailable",
            Self::CorruptedProjection => "graph_projection.corrupted_projection",
        }
    }
}
