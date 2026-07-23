use std::collections::BTreeMap;

use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{GraphEdge, GraphNode, KnowledgeGraph};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_graph_projection::{
    CanvasGraphRelationProjectionError, CanvasGraphRelationProjectionReader,
};
use cabinet_ports::graph_projection::{
    GraphProjectionError, GraphProjectionRecord, GraphProjectionStore,
    WorkspaceGraphProjectionPage, WorkspaceGraphProjectionReader,
};

#[derive(Debug, Clone)]
pub struct CompositeGraphProjectionStore<B, C> {
    base: B,
    canvas: C,
    canvas_source_limit: usize,
}

impl<B, C> CompositeGraphProjectionStore<B, C> {
    pub fn new(
        base: B,
        canvas: C,
        canvas_source_limit: usize,
    ) -> Result<Self, GraphProjectionError> {
        if canvas_source_limit == 0 || canvas_source_limit > 10_000 {
            return Err(GraphProjectionError::InvalidInput);
        }
        Ok(Self {
            base,
            canvas,
            canvas_source_limit,
        })
    }
}

impl<B: GraphProjectionStore, C: CanvasGraphRelationProjectionReader> GraphProjectionStore
    for CompositeGraphProjectionStore<B, C>
{
    fn replace_projection(
        &mut self,
        workspace_id: &WorkspaceId,
        record: GraphProjectionRecord,
    ) -> Result<(), GraphProjectionError> {
        self.base.replace_projection(workspace_id, record)
    }

    fn delete_projection(
        &mut self,
        workspace_id: &WorkspaceId,
        center_document_id: &DocumentId,
    ) -> Result<(), GraphProjectionError> {
        self.base
            .delete_projection(workspace_id, center_document_id)
    }

    fn get_projection(
        &self,
        workspace_id: &WorkspaceId,
        center_document_id: &DocumentId,
    ) -> Result<Option<GraphProjectionRecord>, GraphProjectionError> {
        self.base
            .get_projection(workspace_id, center_document_id)?
            .map(|record| self.merge(workspace_id, record))
            .transpose()
    }
}

impl<B: WorkspaceGraphProjectionReader, C: CanvasGraphRelationProjectionReader>
    WorkspaceGraphProjectionReader for CompositeGraphProjectionStore<B, C>
{
    fn list_workspace_projections(
        &self,
        workspace_id: &WorkspaceId,
        after_center_id: Option<&str>,
        limit: usize,
    ) -> Result<WorkspaceGraphProjectionPage, GraphProjectionError> {
        let page = self
            .base
            .list_workspace_projections(workspace_id, after_center_id, limit)?;
        let records = page
            .records()
            .iter()
            .cloned()
            .map(|record| self.merge(workspace_id, record))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(WorkspaceGraphProjectionPage::new(
            records,
            page.next_cursor().map(str::to_string),
        ))
    }
}

impl<B, C: CanvasGraphRelationProjectionReader> CompositeGraphProjectionStore<B, C> {
    fn merge(
        &self,
        workspace_id: &WorkspaceId,
        base: GraphProjectionRecord,
    ) -> Result<GraphProjectionRecord, GraphProjectionError> {
        let center = base.graph().center_document_id().clone();
        let relations = self
            .canvas
            .get_document_relations(workspace_id, &center, self.canvas_source_limit)
            .map_err(map_canvas_error)?;
        let mut nodes = BTreeMap::<String, GraphNode>::new();
        let mut edges = BTreeMap::<String, GraphEdge>::new();
        for node in base.graph().nodes() {
            insert_node(&mut nodes, node.clone())?;
        }
        for edge in base.graph().edges() {
            insert_edge(&mut edges, edge.clone())?;
        }
        for relation in relations {
            if relation.center_document_id() != &center {
                return Err(GraphProjectionError::CorruptedProjection);
            }
            for node in relation.nodes() {
                insert_node(&mut nodes, node.clone())?;
            }
            for edge in relation.edges() {
                insert_edge(&mut edges, edge.clone())?;
            }
        }
        let graph = KnowledgeGraph::new_with_center(
            center,
            nodes.into_values().collect(),
            edges.into_values().collect(),
            base.graph().status(),
        )
        .map_err(|_| GraphProjectionError::CorruptedProjection)?;
        GraphProjectionRecord::new_with_revision(graph, base.freshness_revision())
    }
}

fn insert_node(
    values: &mut BTreeMap<String, GraphNode>,
    node: GraphNode,
) -> Result<(), GraphProjectionError> {
    if let Some(current) = values.get(node.id())
        && current != &node
    {
        return Err(GraphProjectionError::CorruptedProjection);
    }
    values.entry(node.id().to_string()).or_insert(node);
    Ok(())
}

fn insert_edge(
    values: &mut BTreeMap<String, GraphEdge>,
    edge: GraphEdge,
) -> Result<(), GraphProjectionError> {
    if let Some(current) = values.get(edge.id())
        && current != &edge
    {
        return Err(GraphProjectionError::CorruptedProjection);
    }
    values.entry(edge.id().to_string()).or_insert(edge);
    Ok(())
}

fn map_canvas_error(error: CanvasGraphRelationProjectionError) -> GraphProjectionError {
    match error {
        CanvasGraphRelationProjectionError::InvalidInput
        | CanvasGraphRelationProjectionError::RelationLimitExceeded => {
            GraphProjectionError::InvalidInput
        }
        CanvasGraphRelationProjectionError::StorageUnavailable => {
            GraphProjectionError::StorageUnavailable
        }
        CanvasGraphRelationProjectionError::CorruptedProjection => {
            GraphProjectionError::CorruptedProjection
        }
    }
}
