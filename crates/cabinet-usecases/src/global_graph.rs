use std::collections::BTreeMap;

use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{GraphEdge, GraphNode, GraphNodeKind, GraphProjectionStatus};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::graph_projection::{GraphProjectionError, WorkspaceGraphProjectionReader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetGlobalKnowledgeGraphInput {
    workspace_id: String,
    cursor: Option<String>,
    include_unresolved: bool,
    include_assets: bool,
    projection_limit: usize,
    node_limit: usize,
    edge_limit: usize,
}
impl GetGlobalKnowledgeGraphInput {
    pub fn new(
        workspace_id: &str,
        cursor: Option<&str>,
        include_unresolved: bool,
        include_assets: bool,
        projection_limit: usize,
        node_limit: usize,
        edge_limit: usize,
    ) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            cursor: cursor.map(str::to_string),
            include_unresolved,
            include_assets,
            projection_limit,
            node_limit,
            edge_limit,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetGlobalKnowledgeGraphOutput {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    status: GraphProjectionStatus,
    next_cursor: Option<String>,
    candidate_count: usize,
}
impl GetGlobalKnowledgeGraphOutput {
    pub fn nodes(&self) -> &[GraphNode] {
        &self.nodes
    }
    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }
    pub const fn status(&self) -> GraphProjectionStatus {
        self.status
    }
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
    pub const fn candidate_count(&self) -> usize {
        self.candidate_count
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetGlobalKnowledgeGraphError {
    InvalidInput,
    ProjectionUnavailable,
    CorruptedProjection,
}
impl GetGlobalKnowledgeGraphError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "global_graph.invalid_input",
            Self::ProjectionUnavailable => "global_graph.projection_unavailable",
            Self::CorruptedProjection => "global_graph.corrupted_projection",
        }
    }
}

pub struct GetGlobalKnowledgeGraphUsecase;
impl GetGlobalKnowledgeGraphUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute(
        &self,
        input: GetGlobalKnowledgeGraphInput,
        reader: &impl WorkspaceGraphProjectionReader,
        current_versions: &impl CurrentDocumentVersionPointerPort,
    ) -> Result<GetGlobalKnowledgeGraphOutput, GetGlobalKnowledgeGraphError> {
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| GetGlobalKnowledgeGraphError::InvalidInput)?;
        if input.projection_limit == 0 || input.node_limit == 0 || input.edge_limit == 0 {
            return Err(GetGlobalKnowledgeGraphError::InvalidInput);
        }
        if input
            .cursor
            .as_deref()
            .is_some_and(|cursor| DocumentId::new(cursor).is_err())
        {
            return Err(GetGlobalKnowledgeGraphError::InvalidInput);
        }
        let page = reader
            .list_workspace_projections(&workspace, input.cursor.as_deref(), input.projection_limit)
            .map_err(map_error)?;
        let candidate_count = page
            .records()
            .iter()
            .map(|record| record.graph().nodes().len())
            .sum();
        let mut nodes = BTreeMap::new();
        let mut edges = BTreeMap::new();
        let mut status = GraphProjectionStatus::Clean;
        for record in page.records() {
            if record.graph().status() != GraphProjectionStatus::Clean {
                status = GraphProjectionStatus::Degraded;
            }
            let current = current_versions
                .load_current_version(&workspace, record.graph().center_document_id())
                .map_err(map_pointer_error)?;
            if current
                .as_ref()
                .is_none_or(|version| version.as_str() != record.freshness_revision())
            {
                status = GraphProjectionStatus::Degraded;
            }
            for node in record.graph().nodes() {
                if matches!(node.kind(), GraphNodeKind::UnresolvedLink) && !input.include_unresolved
                    || matches!(node.kind(), GraphNodeKind::Attachment) && !input.include_assets
                {
                    continue;
                }
                nodes
                    .entry(node.id().to_string())
                    .or_insert_with(|| node.clone());
            }
            for edge in record.graph().edges() {
                if nodes.contains_key(edge.source_id()) && nodes.contains_key(edge.target_id()) {
                    edges
                        .entry(edge.id().to_string())
                        .or_insert_with(|| edge.clone());
                }
            }
        }
        if nodes.len() > input.node_limit || edges.len() > input.edge_limit {
            return Err(GetGlobalKnowledgeGraphError::InvalidInput);
        }
        Ok(GetGlobalKnowledgeGraphOutput {
            nodes: nodes.into_values().collect(),
            edges: edges.into_values().collect(),
            status,
            next_cursor: page.next_cursor().map(str::to_string),
            candidate_count,
        })
    }
}

fn map_pointer_error(error: CurrentDocumentVersionPointerError) -> GetGlobalKnowledgeGraphError {
    match error {
        CurrentDocumentVersionPointerError::StorageUnavailable => {
            GetGlobalKnowledgeGraphError::ProjectionUnavailable
        }
        CurrentDocumentVersionPointerError::Conflict
        | CurrentDocumentVersionPointerError::CorruptedPointer => {
            GetGlobalKnowledgeGraphError::CorruptedProjection
        }
    }
}
fn map_error(error: GraphProjectionError) -> GetGlobalKnowledgeGraphError {
    match error {
        GraphProjectionError::InvalidInput => GetGlobalKnowledgeGraphError::InvalidInput,
        GraphProjectionError::StorageUnavailable => {
            GetGlobalKnowledgeGraphError::ProjectionUnavailable
        }
        GraphProjectionError::CorruptedProjection => {
            GetGlobalKnowledgeGraphError::CorruptedProjection
        }
    }
}
