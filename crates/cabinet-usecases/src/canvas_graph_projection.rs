use std::collections::BTreeMap;

use cabinet_domain::canvas::{CanvasLifecycleState, CanvasNodeTarget};
use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{GraphEdge, GraphEdgeKind, GraphNode};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_graph_projection::{
    CanvasGraphRelationProjectionBatch, CanvasGraphRelationProjectionError,
    CanvasGraphRelationProjectionRecord, CanvasGraphRelationProjectionWriter,
};
use cabinet_ports::canvas_repository::CanvasRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasGraphProjectionPolicy {
    relation_limit: usize,
}

impl CanvasGraphProjectionPolicy {
    pub const fn new(relation_limit: usize) -> Result<Self, CanvasGraphRelationProjectionError> {
        if relation_limit == 0 || relation_limit > 10_000 {
            return Err(CanvasGraphRelationProjectionError::InvalidInput);
        }
        Ok(Self { relation_limit })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCanvasGraphRelationsInput {
    workspace_id: String,
    record: CanvasRecord,
}

impl ProjectCanvasGraphRelationsInput {
    pub fn new(workspace_id: &str, record: CanvasRecord) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            record,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectCanvasGraphRelationsOutput {
    projected_record_count: usize,
    projected_edge_count: usize,
    skipped_edge_count: usize,
}

impl ProjectCanvasGraphRelationsOutput {
    pub const fn projected_record_count(self) -> usize {
        self.projected_record_count
    }

    pub const fn projected_edge_count(self) -> usize {
        self.projected_edge_count
    }

    pub const fn skipped_edge_count(self) -> usize {
        self.skipped_edge_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectCanvasGraphRelationsUsecase {
    policy: CanvasGraphProjectionPolicy,
}

impl ProjectCanvasGraphRelationsUsecase {
    pub const fn new(policy: CanvasGraphProjectionPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: ProjectCanvasGraphRelationsInput,
        writer: &mut impl CanvasGraphRelationProjectionWriter,
    ) -> Result<ProjectCanvasGraphRelationsOutput, CanvasGraphRelationProjectionError> {
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| CanvasGraphRelationProjectionError::InvalidInput)?;
        let canvas = input.record.canvas();

        let mut skipped_edge_count = 0;
        let mut eligible_edge_count = 0;
        let mut builders = BTreeMap::<String, RelationRecordBuilder>::new();
        if canvas.state() != CanvasLifecycleState::Archived {
            let nodes = canvas
                .nodes()
                .iter()
                .map(|node| (node.id().as_str(), node.target()))
                .collect::<BTreeMap<_, _>>();
            for edge in canvas.edges() {
                let Some(source_target) = nodes.get(edge.source_node_id().as_str()) else {
                    return Err(CanvasGraphRelationProjectionError::CorruptedProjection);
                };
                let Some(target_target) = nodes.get(edge.target_node_id().as_str()) else {
                    return Err(CanvasGraphRelationProjectionError::CorruptedProjection);
                };
                let Some(source) = graph_node(source_target)? else {
                    skipped_edge_count += 1;
                    continue;
                };
                let Some(target) = graph_node(target_target)? else {
                    skipped_edge_count += 1;
                    continue;
                };
                let centers = document_centers(source_target, target_target);
                if centers.is_empty() {
                    skipped_edge_count += 1;
                    continue;
                }
                eligible_edge_count += 1;
                if eligible_edge_count > self.policy.relation_limit {
                    return Err(CanvasGraphRelationProjectionError::RelationLimitExceeded);
                }
                let relation = GraphEdge::new(
                    &format!("canvas:{}:{}", canvas.id().as_str(), edge.id().as_str()),
                    source.id().to_string(),
                    target.id().to_string(),
                    GraphEdgeKind::CanvasRelation,
                )
                .map_err(|_| CanvasGraphRelationProjectionError::InvalidInput)?;
                for center in centers {
                    builders
                        .entry(center.as_str().to_string())
                        .or_insert_with(|| RelationRecordBuilder::new(center.clone()))
                        .insert(source.clone(), target.clone(), relation.clone());
                }
            }
        } else {
            skipped_edge_count = canvas.edges().len();
        }

        let records = builders
            .into_values()
            .map(RelationRecordBuilder::build)
            .collect::<Result<Vec<_>, _>>()?;
        let output = ProjectCanvasGraphRelationsOutput {
            projected_record_count: records.len(),
            projected_edge_count: records.iter().map(|record| record.edges().len()).sum(),
            skipped_edge_count,
        };
        let batch = CanvasGraphRelationProjectionBatch::new(
            canvas.id().clone(),
            input.record.revision(),
            records,
        )?;
        writer.replace_canvas_relations(&workspace, batch)?;
        Ok(output)
    }
}

fn graph_node(
    target: &CanvasNodeTarget,
) -> Result<Option<GraphNode>, CanvasGraphRelationProjectionError> {
    match target {
        CanvasNodeTarget::Document(document) => Ok(Some(GraphNode::new_document(document.clone()))),
        CanvasNodeTarget::Attachment(asset) => GraphNode::new_attachment(asset.as_str())
            .map(Some)
            .map_err(|_| CanvasGraphRelationProjectionError::InvalidInput),
        CanvasNodeTarget::ExternalLink(link) => GraphNode::new_external_link(link.as_str())
            .map(Some)
            .map_err(|_| CanvasGraphRelationProjectionError::InvalidInput),
        CanvasNodeTarget::TextCard(_) => Ok(None),
    }
}

fn document_centers<'a>(
    source: &'a CanvasNodeTarget,
    target: &'a CanvasNodeTarget,
) -> Vec<&'a DocumentId> {
    let mut centers = Vec::new();
    if let CanvasNodeTarget::Document(document) = source {
        centers.push(document);
    }
    if let CanvasNodeTarget::Document(document) = target
        && !centers.contains(&document)
    {
        centers.push(document);
    }
    centers
}

struct RelationRecordBuilder {
    center: DocumentId,
    nodes: BTreeMap<String, GraphNode>,
    edges: BTreeMap<String, GraphEdge>,
}

impl RelationRecordBuilder {
    fn new(center: DocumentId) -> Self {
        Self {
            center,
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
        }
    }

    fn insert(&mut self, source: GraphNode, target: GraphNode, edge: GraphEdge) {
        self.nodes.entry(source.id().to_string()).or_insert(source);
        self.nodes.entry(target.id().to_string()).or_insert(target);
        self.edges.entry(edge.id().to_string()).or_insert(edge);
    }

    fn build(
        self,
    ) -> Result<CanvasGraphRelationProjectionRecord, CanvasGraphRelationProjectionError> {
        CanvasGraphRelationProjectionRecord::new(
            self.center,
            self.nodes.into_values().collect(),
            self.edges.into_values().collect(),
        )
    }
}
