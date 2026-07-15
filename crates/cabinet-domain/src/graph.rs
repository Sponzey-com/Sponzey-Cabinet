use std::collections::HashSet;

use crate::document::DocumentId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeGraph {
    center_document_id: DocumentId,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    status: GraphProjectionStatus,
}

impl KnowledgeGraph {
    pub fn new(
        nodes: Vec<GraphNode>,
        edges: Vec<GraphEdge>,
        status: GraphProjectionStatus,
    ) -> Result<Self, GraphError> {
        let center_document_id = nodes
            .iter()
            .find_map(GraphNode::document_id)
            .cloned()
            .ok_or(GraphError::MissingCenterDocument)?;
        Self::new_with_center(center_document_id, nodes, edges, status)
    }

    pub fn new_with_center(
        center_document_id: DocumentId,
        nodes: Vec<GraphNode>,
        edges: Vec<GraphEdge>,
        status: GraphProjectionStatus,
    ) -> Result<Self, GraphError> {
        validate_graph(&center_document_id, &nodes, &edges)?;
        Ok(Self {
            center_document_id,
            nodes,
            edges,
            status,
        })
    }

    pub fn center_document_id(&self) -> &DocumentId {
        &self.center_document_id
    }

    pub fn nodes(&self) -> &[GraphNode] {
        &self.nodes
    }

    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }

    pub const fn status(&self) -> GraphProjectionStatus {
        self.status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    id: String,
    kind: GraphNodeKind,
    document_id: Option<DocumentId>,
}

impl GraphNode {
    pub fn new_document(document_id: DocumentId) -> Self {
        Self {
            id: document_id.as_str().to_string(),
            kind: GraphNodeKind::Document,
            document_id: Some(document_id),
        }
    }

    pub fn new_unresolved(id: &str) -> Result<Self, GraphError> {
        Self::new_non_document(id, GraphNodeKind::UnresolvedLink)
    }

    pub fn new_attachment(id: &str) -> Result<Self, GraphError> {
        Self::new_non_document(id, GraphNodeKind::Attachment)
    }

    pub fn new_external_link(id: &str) -> Result<Self, GraphError> {
        Self::new_non_document(id, GraphNodeKind::ExternalLink)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn kind(&self) -> GraphNodeKind {
        self.kind
    }

    pub fn document_id(&self) -> Option<&DocumentId> {
        self.document_id.as_ref()
    }

    fn new_non_document(id: &str, kind: GraphNodeKind) -> Result<Self, GraphError> {
        let normalized = normalize_graph_id(id)?;
        Ok(Self {
            id: normalized,
            kind,
            document_id: None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphNodeKind {
    Document,
    UnresolvedLink,
    Attachment,
    ExternalLink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    id: String,
    source_id: String,
    target_id: String,
    kind: GraphEdgeKind,
}

impl GraphEdge {
    pub fn new(
        id: &str,
        source_id: String,
        target_id: String,
        kind: GraphEdgeKind,
    ) -> Result<Self, GraphError> {
        Ok(Self {
            id: normalize_graph_id(id)?,
            source_id: normalize_graph_id(&source_id)?,
            target_id: normalize_graph_id(&target_id)?,
            kind,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub const fn kind(&self) -> GraphEdgeKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphEdgeKind {
    DocumentLink,
    AttachmentReference,
    ExternalReference,
    CanvasRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphProjectionStatus {
    Clean,
    ReindexRequested,
    Reindexing,
    Degraded,
}

impl GraphProjectionStatus {
    pub const REQUEST_REINDEX: &'static str = "RequestReindex";
    pub const START_REINDEX: &'static str = "StartReindex";
    pub const FINISH_REINDEX: &'static str = "FinishReindex";
    pub const MARK_DEGRADED: &'static str = "MarkDegraded";

    pub fn transition(self, event: &str) -> Result<Self, GraphError> {
        match (self, event) {
            (Self::Clean, Self::REQUEST_REINDEX) => Ok(Self::ReindexRequested),
            (Self::ReindexRequested, Self::START_REINDEX) => Ok(Self::Reindexing),
            (Self::Reindexing, Self::FINISH_REINDEX) => Ok(Self::Clean),
            (Self::Reindexing, Self::MARK_DEGRADED) => Ok(Self::Degraded),
            (Self::Degraded, Self::REQUEST_REINDEX) => Ok(Self::ReindexRequested),
            _ => Err(GraphError::InvalidStatusTransition),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphError {
    EmptyGraphId,
    InvalidGraphId,
    MissingCenterDocument,
    DuplicateNodeId,
    MissingEdgeNode,
    InvalidStatusTransition,
}

impl GraphError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyGraphId => "graph.empty_id",
            Self::InvalidGraphId => "graph.invalid_id",
            Self::MissingCenterDocument => "graph.missing_center_document",
            Self::DuplicateNodeId => "graph.duplicate_node_id",
            Self::MissingEdgeNode => "graph.missing_edge_node",
            Self::InvalidStatusTransition => "graph.invalid_status_transition",
        }
    }
}

fn validate_graph(
    center_document_id: &DocumentId,
    nodes: &[GraphNode],
    edges: &[GraphEdge],
) -> Result<(), GraphError> {
    let mut node_ids = HashSet::new();
    let mut center_found = false;
    for node in nodes {
        if !node_ids.insert(node.id().to_string()) {
            return Err(GraphError::DuplicateNodeId);
        }
        if node.document_id() == Some(center_document_id) {
            center_found = true;
        }
    }
    if !center_found {
        return Err(GraphError::MissingCenterDocument);
    }
    for edge in edges {
        if !node_ids.contains(edge.source_id()) || !node_ids.contains(edge.target_id()) {
            return Err(GraphError::MissingEdgeNode);
        }
    }
    Ok(())
}

fn normalize_graph_id(value: &str) -> Result<String, GraphError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(GraphError::EmptyGraphId);
    }
    if trimmed.chars().any(char::is_control) {
        return Err(GraphError::InvalidGraphId);
    }
    Ok(trimmed.to_string())
}
