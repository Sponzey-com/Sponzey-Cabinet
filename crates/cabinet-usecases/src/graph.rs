use std::collections::HashSet;

use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{GraphEdge as DomainGraphEdge, KnowledgeGraph};
use cabinet_domain::link::{Backlink, DocumentLink, LinkTarget};
use cabinet_domain::permission::{AccessResource, Permission, PermissionDecisionResult};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::graph_projection::{
    GraphProjectionError, GraphProjectionRecord, GraphProjectionStore,
};
use cabinet_ports::link_index::{BacklinkPageReader, BacklinkPageRequest};
use cabinet_ports::link_index::{LinkIndex, LinkIndexError};
use cabinet_ports::permission_aware_query::{PermissionAwareQueryError, PermissionDecisionPort};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetLinkOverviewInput {
    workspace_id: String,
    target_document_id: String,
    cursor: Option<String>,
    limit: usize,
}

impl GetLinkOverviewInput {
    pub fn new(
        workspace_id: &str,
        target_document_id: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            target_document_id: target_document_id.to_string(),
            cursor: cursor.map(str::to_string),
            limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetLinkOverviewOutput {
    backlinks: Vec<Backlink>,
    next_cursor: Option<String>,
}

impl GetLinkOverviewOutput {
    pub fn backlinks(&self) -> &[Backlink] {
        &self.backlinks
    }

    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetLinkOverviewError {
    InvalidInput,
    ProjectionUnavailable,
}

impl GetLinkOverviewError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "link_overview.invalid_input",
            Self::ProjectionUnavailable => "link_overview.projection_unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GetLinkOverviewUsecase;

impl GetLinkOverviewUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetLinkOverviewInput,
        reader: &impl BacklinkPageReader,
    ) -> Result<GetLinkOverviewOutput, GetLinkOverviewError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| GetLinkOverviewError::InvalidInput)?;
        let target_document_id = DocumentId::new(&input.target_document_id)
            .map_err(|_| GetLinkOverviewError::InvalidInput)?;
        let offset = input
            .cursor
            .as_deref()
            .map(str::parse::<usize>)
            .transpose()
            .map_err(|_| GetLinkOverviewError::InvalidInput)?
            .unwrap_or(0);
        let request = BacklinkPageRequest::new(offset, input.limit)
            .map_err(|_| GetLinkOverviewError::InvalidInput)?;
        let page = reader
            .list_backlinks_page(&workspace_id, &target_document_id, request)
            .map_err(|_| GetLinkOverviewError::ProjectionUnavailable)?;
        Ok(GetLinkOverviewOutput {
            backlinks: page.records().to_vec(),
            next_cursor: page.next_offset().map(|value| value.to_string()),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalGraphDirection {
    Incoming,
    Outgoing,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetLocalKnowledgeGraphInput {
    workspace_id: String,
    center_document_id: String,
    depth: u8,
    direction: LocalGraphDirection,
    include_unresolved: bool,
    include_assets: bool,
    node_limit: usize,
    edge_limit: usize,
}

impl GetLocalKnowledgeGraphInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: &str,
        center_document_id: &str,
        depth: u8,
        direction: LocalGraphDirection,
        include_unresolved: bool,
        include_assets: bool,
        node_limit: usize,
        edge_limit: usize,
    ) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            center_document_id: center_document_id.to_string(),
            depth,
            direction,
            include_unresolved,
            include_assets,
            node_limit,
            edge_limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetLocalKnowledgeGraphOutput {
    graph: KnowledgeGraph,
    candidate_count: usize,
    filtered_count: usize,
    freshness_revision: String,
}

impl GetLocalKnowledgeGraphOutput {
    pub fn graph(&self) -> &KnowledgeGraph {
        &self.graph
    }

    pub const fn candidate_count(&self) -> usize {
        self.candidate_count
    }

    pub const fn filtered_count(&self) -> usize {
        self.filtered_count
    }

    pub fn freshness_revision(&self) -> &str {
        &self.freshness_revision
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetLocalKnowledgeGraphError {
    InvalidInput,
    ProjectionNotFound,
    ProjectionUnavailable,
    CorruptedProjection,
}

impl GetLocalKnowledgeGraphError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "graph.invalid_input",
            Self::ProjectionNotFound => "graph.projection_not_found",
            Self::ProjectionUnavailable => "graph.projection_unavailable",
            Self::CorruptedProjection => "graph.corrupted_projection",
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(self, Self::ProjectionUnavailable)
    }

    const fn from_projection_error(error: GraphProjectionError) -> Self {
        match error {
            GraphProjectionError::InvalidInput => Self::InvalidInput,
            GraphProjectionError::StorageUnavailable => Self::ProjectionUnavailable,
            GraphProjectionError::CorruptedProjection => Self::CorruptedProjection,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GetLocalKnowledgeGraphUsecase;

impl GetLocalKnowledgeGraphUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetLocalKnowledgeGraphInput,
        projection_store: &impl GraphProjectionStore,
    ) -> Result<GetLocalKnowledgeGraphOutput, GetLocalKnowledgeGraphError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| GetLocalKnowledgeGraphError::InvalidInput)?;
        let center_document_id = DocumentId::new(&input.center_document_id)
            .map_err(|_| GetLocalKnowledgeGraphError::InvalidInput)?;
        if !matches!(input.depth, 1 | 2) || input.node_limit == 0 || input.edge_limit == 0 {
            return Err(GetLocalKnowledgeGraphError::InvalidInput);
        }

        let record = projection_store
            .get_projection(&workspace_id, &center_document_id)
            .map_err(GetLocalKnowledgeGraphError::from_projection_error)?
            .ok_or(GetLocalKnowledgeGraphError::ProjectionNotFound)?;

        bounded_local_graph(record, &input)
    }
}

fn bounded_local_graph(
    record: GraphProjectionRecord,
    input: &GetLocalKnowledgeGraphInput,
) -> Result<GetLocalKnowledgeGraphOutput, GetLocalKnowledgeGraphError> {
    let freshness_revision = record.freshness_revision().to_string();
    let source = record.graph();
    let candidate_count = source.nodes().len();
    let center_id = source.center_document_id().as_str();
    let allowed_node_ids = source
        .nodes()
        .iter()
        .filter(|node| match node.kind() {
            cabinet_domain::graph::GraphNodeKind::UnresolvedLink => input.include_unresolved,
            cabinet_domain::graph::GraphNodeKind::Attachment => input.include_assets,
            _ => true,
        })
        .map(|node| node.id().to_string())
        .collect::<HashSet<_>>();

    let mut edges = source
        .edges()
        .iter()
        .filter(|edge| {
            allowed_node_ids.contains(edge.source_id())
                && allowed_node_ids.contains(edge.target_id())
                && match input.direction {
                    LocalGraphDirection::Incoming => edge.target_id() == center_id,
                    LocalGraphDirection::Outgoing => edge.source_id() == center_id,
                    LocalGraphDirection::Both => true,
                }
        })
        .take(input.edge_limit)
        .cloned()
        .collect::<Vec<_>>();

    let mut referenced_ids = HashSet::from([center_id.to_string()]);
    for edge in &edges {
        referenced_ids.insert(edge.source_id().to_string());
        referenced_ids.insert(edge.target_id().to_string());
    }

    let mut retained_ids = HashSet::from([center_id.to_string()]);
    for node in source.nodes() {
        if retained_ids.len() >= input.node_limit {
            break;
        }
        if referenced_ids.contains(node.id()) {
            retained_ids.insert(node.id().to_string());
        }
    }
    edges.retain(|edge| {
        retained_ids.contains(edge.source_id()) && retained_ids.contains(edge.target_id())
    });
    let nodes = source
        .nodes()
        .iter()
        .filter(|node| retained_ids.contains(node.id()))
        .cloned()
        .collect::<Vec<_>>();

    let graph = KnowledgeGraph::new_with_center(
        source.center_document_id().clone(),
        nodes,
        edges,
        source.status(),
    )
    .map_err(|_| GetLocalKnowledgeGraphError::CorruptedProjection)?;
    let filtered_count = candidate_count.saturating_sub(graph.nodes().len());

    Ok(GetLocalKnowledgeGraphOutput {
        graph,
        candidate_count,
        filtered_count,
        freshness_revision,
    })
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionAwareGraphInput {
    workspace_id: String,
    actor_user_id: String,
    center_document_id: String,
}

impl PermissionAwareGraphInput {
    pub fn new(workspace_id: &str, actor_user_id: &str, center_document_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            actor_user_id: actor_user_id.to_string(),
            center_document_id: center_document_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionAwareGraphOutput {
    graph: KnowledgeGraph,
    stats: PermissionAwareGraphStats,
}

impl PermissionAwareGraphOutput {
    pub fn new(graph: KnowledgeGraph, stats: PermissionAwareGraphStats) -> Self {
        Self { graph, stats }
    }

    pub fn graph(&self) -> &KnowledgeGraph {
        &self.graph
    }

    pub const fn stats(&self) -> PermissionAwareGraphStats {
        self.stats
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionAwareGraphStats {
    candidate_count: usize,
    filtered_count: usize,
}

impl PermissionAwareGraphStats {
    pub const fn new(candidate_count: usize, filtered_count: usize) -> Self {
        Self {
            candidate_count,
            filtered_count,
        }
    }

    pub const fn candidate_count(self) -> usize {
        self.candidate_count
    }

    pub const fn filtered_count(self) -> usize {
        self.filtered_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionAwareGraphUsecase;

impl PermissionAwareGraphUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: PermissionAwareGraphInput,
        graph_projection_store: &impl GraphProjectionStore,
        permission_decision: &impl PermissionDecisionPort,
    ) -> Result<PermissionAwareGraphOutput, PermissionAwareGraphError> {
        let workspace_id = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| PermissionAwareGraphError::InvalidInput)?;
        let actor_user_id = UserId::new(&input.actor_user_id)
            .map_err(|_| PermissionAwareGraphError::InvalidInput)?;
        let center_document_id = DocumentId::new(&input.center_document_id)
            .map_err(|_| PermissionAwareGraphError::InvalidInput)?;

        let record = graph_projection_store
            .get_projection(&workspace_id, &center_document_id)
            .map_err(PermissionAwareGraphError::from_graph_projection_error)?
            .ok_or(PermissionAwareGraphError::ProjectionNotFound)?;

        filter_graph_by_permission(record, &workspace_id, &actor_user_id, permission_decision)
    }
}

impl Default for PermissionAwareGraphUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionAwareGraphError {
    InvalidInput,
    ProjectionNotFound,
    PermissionUnavailable,
    ProjectionUnavailable,
    CorruptedProjection,
    CenterDocumentDenied,
}

impl PermissionAwareGraphError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "graph.invalid_input",
            Self::ProjectionNotFound => "graph.projection_not_found",
            Self::PermissionUnavailable => "graph.permission_unavailable",
            Self::ProjectionUnavailable => "graph.projection_unavailable",
            Self::CorruptedProjection => "graph.corrupted_projection",
            Self::CenterDocumentDenied => "graph.center_document_denied",
        }
    }

    const fn from_graph_projection_error(error: GraphProjectionError) -> Self {
        match error {
            GraphProjectionError::InvalidInput => Self::InvalidInput,
            GraphProjectionError::StorageUnavailable => Self::ProjectionUnavailable,
            GraphProjectionError::CorruptedProjection => Self::CorruptedProjection,
        }
    }
}

fn filter_graph_by_permission(
    record: GraphProjectionRecord,
    workspace_id: &WorkspaceId,
    actor_user_id: &UserId,
    permission_decision: &impl PermissionDecisionPort,
) -> Result<PermissionAwareGraphOutput, PermissionAwareGraphError> {
    let source_graph = record.graph();
    let candidate_count = source_graph.nodes().len();
    let mut allowed_node_ids = HashSet::new();
    let mut filtered_count = 0;
    let mut retained_nodes = Vec::new();

    for node in source_graph.nodes() {
        if let Some(document_id) = node.document_id() {
            if can_read_document(
                permission_decision,
                actor_user_id,
                workspace_id,
                document_id,
            )? {
                allowed_node_ids.insert(node.id().to_string());
                retained_nodes.push(node.clone());
            } else {
                filtered_count += 1;
            }
        } else {
            allowed_node_ids.insert(node.id().to_string());
            retained_nodes.push(node.clone());
        }
    }

    if !allowed_node_ids.contains(source_graph.center_document_id().as_str()) {
        return Err(PermissionAwareGraphError::CenterDocumentDenied);
    }

    let retained_edges = source_graph
        .edges()
        .iter()
        .filter(|edge| {
            allowed_node_ids.contains(edge.source_id())
                && allowed_node_ids.contains(edge.target_id())
        })
        .cloned()
        .collect::<Vec<_>>();
    let referenced_node_ids =
        referenced_node_ids(&retained_edges, source_graph.center_document_id());
    retained_nodes
        .retain(|node| node.document_id().is_some() || referenced_node_ids.contains(node.id()));

    let filtered_graph = KnowledgeGraph::new_with_center(
        source_graph.center_document_id().clone(),
        retained_nodes,
        retained_edges,
        source_graph.status(),
    )
    .map_err(|_| PermissionAwareGraphError::CorruptedProjection)?;

    Ok(PermissionAwareGraphOutput::new(
        filtered_graph,
        PermissionAwareGraphStats::new(candidate_count, filtered_count),
    ))
}

fn can_read_document(
    permission_decision: &impl PermissionDecisionPort,
    actor_user_id: &UserId,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
) -> Result<bool, PermissionAwareGraphError> {
    let resource = AccessResource::document(workspace_id.clone(), None, document_id.clone());
    let decision = permission_decision
        .check_permission(actor_user_id, &resource, Permission::Read)
        .map_err(PermissionAwareGraphError::from_permission_error)?;
    Ok(decision.result() == PermissionDecisionResult::Allowed)
}

fn referenced_node_ids(
    edges: &[DomainGraphEdge],
    center_document_id: &DocumentId,
) -> HashSet<String> {
    let mut ids = HashSet::from([center_document_id.as_str().to_string()]);
    for edge in edges {
        ids.insert(edge.source_id().to_string());
        ids.insert(edge.target_id().to_string());
    }
    ids
}

impl PermissionAwareGraphError {
    const fn from_permission_error(error: PermissionAwareQueryError) -> Self {
        match error {
            PermissionAwareQueryError::InvalidInput => Self::InvalidInput,
            PermissionAwareQueryError::NotFound
            | PermissionAwareQueryError::IndexStale
            | PermissionAwareQueryError::StorageUnavailable => Self::PermissionUnavailable,
            PermissionAwareQueryError::CorruptedProjection => Self::CorruptedProjection,
        }
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
