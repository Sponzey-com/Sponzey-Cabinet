use cabinet_domain::canvas::{CanvasId, CanvasRevision};
use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::workspace::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasGraphRelationProjectionRecord {
    graph: KnowledgeGraph,
}

impl CanvasGraphRelationProjectionRecord {
    pub fn new(
        center_document_id: DocumentId,
        nodes: Vec<GraphNode>,
        edges: Vec<GraphEdge>,
    ) -> Result<Self, CanvasGraphRelationProjectionError> {
        if edges
            .iter()
            .any(|edge| edge.kind() != GraphEdgeKind::CanvasRelation)
        {
            return Err(CanvasGraphRelationProjectionError::InvalidInput);
        }
        let graph = KnowledgeGraph::new_with_center(
            center_document_id,
            nodes,
            edges,
            GraphProjectionStatus::Clean,
        )
        .map_err(|_| CanvasGraphRelationProjectionError::InvalidInput)?;
        Ok(Self { graph })
    }

    pub fn center_document_id(&self) -> &DocumentId {
        self.graph.center_document_id()
    }

    pub fn nodes(&self) -> &[GraphNode] {
        self.graph.nodes()
    }

    pub fn edges(&self) -> &[GraphEdge] {
        self.graph.edges()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasGraphRelationProjectionBatch {
    canvas_id: CanvasId,
    canvas_revision: CanvasRevision,
    records: Vec<CanvasGraphRelationProjectionRecord>,
}

impl CanvasGraphRelationProjectionBatch {
    pub fn new(
        canvas_id: CanvasId,
        canvas_revision: CanvasRevision,
        records: Vec<CanvasGraphRelationProjectionRecord>,
    ) -> Result<Self, CanvasGraphRelationProjectionError> {
        let mut centers = std::collections::HashSet::new();
        if records
            .iter()
            .any(|record| !centers.insert(record.center_document_id().as_str()))
        {
            return Err(CanvasGraphRelationProjectionError::InvalidInput);
        }
        Ok(Self {
            canvas_id,
            canvas_revision,
            records,
        })
    }

    pub fn canvas_id(&self) -> &CanvasId {
        &self.canvas_id
    }

    pub const fn canvas_revision(&self) -> CanvasRevision {
        self.canvas_revision
    }

    pub fn records(&self) -> &[CanvasGraphRelationProjectionRecord] {
        &self.records
    }
}

pub trait CanvasGraphRelationProjectionWriter {
    fn replace_canvas_relations(
        &mut self,
        workspace_id: &WorkspaceId,
        batch: CanvasGraphRelationProjectionBatch,
    ) -> Result<(), CanvasGraphRelationProjectionError>;
}

pub trait CanvasGraphRelationProjectionReader {
    fn get_document_relations(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        source_limit: usize,
    ) -> Result<Vec<CanvasGraphRelationProjectionRecord>, CanvasGraphRelationProjectionError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasGraphRelationProjectionError {
    InvalidInput,
    RelationLimitExceeded,
    StorageUnavailable,
    CorruptedProjection,
}

impl CanvasGraphRelationProjectionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "canvas_graph_projection.invalid_input",
            Self::RelationLimitExceeded => "canvas_graph_projection.relation_limit_exceeded",
            Self::StorageUnavailable => "canvas_graph_projection.storage_unavailable",
            Self::CorruptedProjection => "canvas_graph_projection.corrupted_projection",
        }
    }
}
