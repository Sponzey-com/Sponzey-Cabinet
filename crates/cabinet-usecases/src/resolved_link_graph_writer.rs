use cabinet_domain::document::DocumentBody;
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::link::{Backlink, DocumentLink, LinkTarget};
use cabinet_domain::projection_work::{ProjectionKind, ProjectionWorkIdentity};
use cabinet_ports::asset_association_catalog::{
    AssetAssociationCatalogError, DocumentAssetAssociationReader,
};
use cabinet_ports::current_document_version::{
    CurrentDocumentVersionPointerError, CurrentDocumentVersionPointerPort,
};
use cabinet_ports::graph_projection::{
    GraphProjectionError, GraphProjectionRecord, GraphProjectionStore,
};
use cabinet_ports::link_index::{LinkIndex, LinkIndexError, LinkProjectionRecord};
use cabinet_ports::link_target_resolver::{
    DocumentLinkTargetResolver, LinkTargetResolution, LinkTargetResolverError,
};
use cabinet_ports::markdown_parser::ParsedMarkdown;
use cabinet_ports::projection_writer::{ProjectionWriteError, VersionedProjectionWriter};
use std::collections::{BTreeSet, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetGraphProjectionPolicy {
    association_limit: usize,
}

impl AssetGraphProjectionPolicy {
    pub fn new(association_limit: usize) -> Result<Self, AssetGraphProjectionPolicyError> {
        if association_limit == 0 || association_limit > 500 {
            return Err(AssetGraphProjectionPolicyError::InvalidAssociationLimit);
        }
        Ok(Self { association_limit })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetGraphProjectionPolicyError {
    InvalidAssociationLimit,
}

pub struct ResolvedLinkGraphProjectionWriter<'a, P, R, A, L, G> {
    pointer: &'a P,
    resolver: &'a R,
    associations: &'a A,
    asset_policy: AssetGraphProjectionPolicy,
    links: &'a mut L,
    graphs: &'a mut G,
}
impl<'a, P, R, A, L, G> ResolvedLinkGraphProjectionWriter<'a, P, R, A, L, G> {
    pub fn new(
        pointer: &'a P,
        resolver: &'a R,
        associations: &'a A,
        asset_policy: AssetGraphProjectionPolicy,
        links: &'a mut L,
        graphs: &'a mut G,
    ) -> Self {
        Self {
            pointer,
            resolver,
            associations,
            asset_policy,
            links,
            graphs,
        }
    }
}
impl<
    P: CurrentDocumentVersionPointerPort,
    R: DocumentLinkTargetResolver,
    A: DocumentAssetAssociationReader,
    L: LinkIndex,
    G: GraphProjectionStore,
> VersionedProjectionWriter for ResolvedLinkGraphProjectionWriter<'_, P, R, A, L, G>
{
    fn write(
        &mut self,
        id: &ProjectionWorkIdentity,
        _: &DocumentBody,
        parsed: &ParsedMarkdown,
    ) -> Result<(), ProjectionWriteError> {
        let current = self
            .pointer
            .load_current_version(id.workspace_id(), id.document_id())
            .map_err(map_pointer)?
            .ok_or(ProjectionWriteError::Permanent)?;
        if &current != id.version_id() {
            return Err(ProjectionWriteError::Permanent);
        }
        match id.kind() {
            ProjectionKind::Links => self.write_links(id, parsed),
            ProjectionKind::Graph => self.write_graph(id, parsed),
            ProjectionKind::Search => Err(ProjectionWriteError::Permanent),
        }
    }

    fn delete(&mut self, id: &ProjectionWorkIdentity) -> Result<(), ProjectionWriteError> {
        let current = self
            .pointer
            .load_current_version(id.workspace_id(), id.document_id())
            .map_err(map_pointer)?
            .ok_or(ProjectionWriteError::Permanent)?;
        if &current != id.version_id() {
            return Err(ProjectionWriteError::Permanent);
        }
        match id.kind() {
            ProjectionKind::Links => self
                .links
                .delete_document_links(id.workspace_id(), id.document_id())
                .map_err(map_link),
            ProjectionKind::Graph => self
                .graphs
                .delete_projection(id.workspace_id(), id.document_id())
                .map_err(map_graph),
            ProjectionKind::Search => Err(ProjectionWriteError::Permanent),
        }
    }
}
impl<
    P: CurrentDocumentVersionPointerPort,
    R: DocumentLinkTargetResolver,
    A: DocumentAssetAssociationReader,
    L: LinkIndex,
    G: GraphProjectionStore,
> ResolvedLinkGraphProjectionWriter<'_, P, R, A, L, G>
{
    fn resolve(
        &self,
        id: &ProjectionWorkIdentity,
        target: &str,
    ) -> Result<LinkTargetResolution, ProjectionWriteError> {
        self.resolver
            .resolve(id.workspace_id(), target)
            .map_err(|e| match e {
                LinkTargetResolverError::Unavailable => ProjectionWriteError::Retryable,
                _ => ProjectionWriteError::Permanent,
            })
    }
    fn write_links(
        &mut self,
        id: &ProjectionWorkIdentity,
        parsed: &ParsedMarkdown,
    ) -> Result<(), ProjectionWriteError> {
        let mut backlinks = vec![];
        let mut unresolved = vec![];
        for link in parsed.wikilinks() {
            match self.resolve(id, link.target())? {
                LinkTargetResolution::Resolved(target) => backlinks.push(Backlink::new(
                    id.document_id().clone(),
                    target.document_id().clone(),
                    link.source_range(),
                )),
                LinkTargetResolution::Unresolved(slug) => unresolved.push(DocumentLink::new(
                    id.document_id().clone(),
                    LinkTarget::unresolved(slug),
                    link.source_range(),
                )),
            }
        }
        let record = LinkProjectionRecord::new(id.document_id().clone(), backlinks, unresolved)
            .map_err(map_link)?;
        self.links
            .replace_document_links(id.workspace_id(), record)
            .map_err(map_link)
    }
    fn write_graph(
        &mut self,
        id: &ProjectionWorkIdentity,
        parsed: &ParsedMarkdown,
    ) -> Result<(), ProjectionWriteError> {
        let center = GraphNode::new_document(id.document_id().clone());
        let mut nodes = vec![center.clone()];
        let mut seen = HashSet::from([center.id().to_string()]);
        let mut edges = vec![];
        for (index, link) in parsed.wikilinks().iter().enumerate() {
            let node = match self.resolve(id, link.target())? {
                LinkTargetResolution::Resolved(t) => {
                    GraphNode::new_document(t.document_id().clone())
                }
                LinkTargetResolution::Unresolved(s) => GraphNode::new_unresolved(s.as_str())
                    .map_err(|_| ProjectionWriteError::Permanent)?,
            };
            if seen.insert(node.id().to_string()) {
                nodes.push(node.clone())
            }
            edges.push(
                GraphEdge::new(
                    &format!("link-{index}"),
                    center.id().to_string(),
                    node.id().to_string(),
                    GraphEdgeKind::DocumentLink,
                )
                .map_err(|_| ProjectionWriteError::Permanent)?,
            )
        }
        let mut asset_ids = parsed
            .asset_references()
            .iter()
            .map(|asset| asset.asset_id().as_str().to_string())
            .collect::<BTreeSet<_>>();
        for association in self
            .associations
            .list_document_assets(
                id.workspace_id(),
                id.document_id(),
                self.asset_policy.association_limit,
            )
            .map_err(map_association)?
        {
            asset_ids.insert(association.asset_id().as_str().to_string());
        }
        for (index, asset_id) in asset_ids.into_iter().enumerate() {
            let node = GraphNode::new_attachment(&asset_id)
                .map_err(|_| ProjectionWriteError::Permanent)?;
            if seen.insert(node.id().to_string()) {
                nodes.push(node.clone())
            }
            edges.push(
                GraphEdge::new(
                    &format!("asset-{index}"),
                    center.id().to_string(),
                    node.id().to_string(),
                    GraphEdgeKind::AttachmentReference,
                )
                .map_err(|_| ProjectionWriteError::Permanent)?,
            )
        }
        let graph = KnowledgeGraph::new_with_center(
            id.document_id().clone(),
            nodes,
            edges,
            GraphProjectionStatus::Clean,
        )
        .map_err(|_| ProjectionWriteError::Permanent)?;
        let record = GraphProjectionRecord::new_with_revision(graph, id.version_id().as_str())
            .map_err(map_graph)?;
        self.graphs
            .replace_projection(id.workspace_id(), record)
            .map_err(map_graph)
    }
}
fn map_pointer(e: CurrentDocumentVersionPointerError) -> ProjectionWriteError {
    match e {
        CurrentDocumentVersionPointerError::StorageUnavailable => ProjectionWriteError::Retryable,
        _ => ProjectionWriteError::Permanent,
    }
}
fn map_link(e: LinkIndexError) -> ProjectionWriteError {
    match e {
        LinkIndexError::StorageUnavailable => ProjectionWriteError::Retryable,
        _ => ProjectionWriteError::Permanent,
    }
}
fn map_graph(e: GraphProjectionError) -> ProjectionWriteError {
    match e {
        GraphProjectionError::StorageUnavailable => ProjectionWriteError::Retryable,
        _ => ProjectionWriteError::Permanent,
    }
}
fn map_association(e: AssetAssociationCatalogError) -> ProjectionWriteError {
    match e {
        AssetAssociationCatalogError::StorageUnavailable => ProjectionWriteError::Retryable,
        AssetAssociationCatalogError::InvalidLimit
        | AssetAssociationCatalogError::Conflict
        | AssetAssociationCatalogError::CorruptedRecord
        | AssetAssociationCatalogError::UnsupportedSchema => ProjectionWriteError::Permanent,
    }
}
