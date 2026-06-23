use std::collections::HashSet;

use cabinet_domain::document::DocumentId;
use cabinet_domain::link::{Backlink, DocumentLink, LinkTarget};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::link_index::{LinkIndex, LinkIndexError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphLiteProjectionInput {
    workspace_id: String,
    center_document_id: String,
    known_document_ids: Vec<String>,
}

impl GraphLiteProjectionInput {
    pub fn new(
        workspace_id: &str,
        center_document_id: &str,
        known_document_ids: Vec<&str>,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            center_document_id: center_document_id.to_string(),
            known_document_ids: known_document_ids
                .into_iter()
                .map(ToString::to_string)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphLiteProjectionOutput {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

impl GraphLiteProjectionOutput {
    pub fn nodes(&self) -> &[GraphNode] {
        &self.nodes
    }

    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    id: String,
    kind: GraphNodeKind,
}

impl GraphNode {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> GraphNodeKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphNodeKind {
    Document,
    Unresolved,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    source_id: String,
    target_id: String,
    kind: GraphEdgeKind,
}

impl GraphEdge {
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub fn kind(&self) -> GraphEdgeKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphEdgeKind {
    Resolved,
    Unresolved,
    MissingTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphLiteProjectionUsecase;

impl GraphLiteProjectionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GraphLiteProjectionInput,
        link_index: &impl LinkIndex,
    ) -> Result<GraphLiteProjectionOutput, GraphLiteProjectionError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| GraphLiteProjectionError::InvalidInput)?;
        let center_document_id = DocumentId::new(&input.center_document_id)
            .map_err(|_| GraphLiteProjectionError::InvalidInput)?;
        let known_document_ids: HashSet<String> = input
            .known_document_ids
            .iter()
            .map(|document_id| {
                DocumentId::new(document_id)
                    .map(|id| id.as_str().to_string())
                    .map_err(|_| GraphLiteProjectionError::InvalidInput)
            })
            .collect::<Result<_, _>>()?;

        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        push_node(
            &mut nodes,
            center_document_id.as_str(),
            GraphNodeKind::Document,
        );

        if let Some(projection) = link_index
            .get_document_links(&workspace_id, &center_document_id)
            .map_err(GraphLiteProjectionError::from_link_index_error)?
        {
            for backlink in projection.backlinks() {
                push_backlink_edge(backlink, &known_document_ids, &mut nodes, &mut edges);
            }
            for unresolved_link in projection.unresolved_links() {
                push_unresolved_edge(unresolved_link, &mut nodes, &mut edges);
            }
        }

        for incoming in link_index
            .list_backlinks(&workspace_id, &center_document_id)
            .map_err(GraphLiteProjectionError::from_link_index_error)?
        {
            push_backlink_edge(&incoming, &known_document_ids, &mut nodes, &mut edges);
        }

        Ok(GraphLiteProjectionOutput { nodes, edges })
    }
}

impl Default for GraphLiteProjectionUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphLiteProjectionError {
    InvalidInput,
    StorageUnavailable,
}

impl GraphLiteProjectionError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "graph.invalid_input",
            Self::StorageUnavailable => "graph.storage_unavailable",
        }
    }

    fn from_link_index_error(_error: LinkIndexError) -> Self {
        Self::StorageUnavailable
    }
}

fn push_backlink_edge(
    backlink: &Backlink,
    known_document_ids: &HashSet<String>,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
) {
    let source_id = backlink.source_document_id().as_str();
    let target_id = backlink.target_document_id().as_str();
    let source_kind = document_node_kind(source_id, known_document_ids);
    let target_kind = document_node_kind(target_id, known_document_ids);
    push_node(nodes, source_id, source_kind);
    push_node(nodes, target_id, target_kind);
    edges.push(GraphEdge {
        source_id: source_id.to_string(),
        target_id: target_id.to_string(),
        kind: if target_kind == GraphNodeKind::Missing {
            GraphEdgeKind::MissingTarget
        } else {
            GraphEdgeKind::Resolved
        },
    });
}

fn push_unresolved_edge(
    link: &DocumentLink,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
) {
    let LinkTarget::Unresolved(target_slug) = link.target() else {
        return;
    };
    let source_id = link.source_document_id().as_str();
    let target_id = target_slug.as_str();
    push_node(nodes, source_id, GraphNodeKind::Document);
    push_node(nodes, target_id, GraphNodeKind::Unresolved);
    edges.push(GraphEdge {
        source_id: source_id.to_string(),
        target_id: target_id.to_string(),
        kind: GraphEdgeKind::Unresolved,
    });
}

fn document_node_kind(document_id: &str, known_document_ids: &HashSet<String>) -> GraphNodeKind {
    if known_document_ids.contains(document_id) {
        GraphNodeKind::Document
    } else {
        GraphNodeKind::Missing
    }
}

fn push_node(nodes: &mut Vec<GraphNode>, id: &str, kind: GraphNodeKind) {
    if let Some(node) = nodes.iter_mut().find(|node| node.id == id) {
        if node.kind == GraphNodeKind::Missing && kind == GraphNodeKind::Document {
            node.kind = kind;
        }
        return;
    }
    nodes.push(GraphNode {
        id: id.to_string(),
        kind,
    });
}
