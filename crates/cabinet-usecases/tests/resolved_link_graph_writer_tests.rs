use cabinet_domain::asset::{AssetAssociation, AssetId};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentPath, DocumentSlug, DocumentTitle,
};
use cabinet_domain::graph::GraphProjectionStatus;
use cabinet_domain::link::SourceRange;
use cabinet_domain::projection_work::{ProjectionKind, ProjectionWorkIdentity};
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
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
    ResolvedDocumentLinkTarget,
};
use cabinet_ports::markdown_parser::{
    ParsedAssetReference, ParsedDocumentLink, ParsedExternalLink, ParsedMarkdown, ParsedWikilink,
};
use cabinet_ports::projection_writer::{ProjectionWriteError, VersionedProjectionWriter};
use cabinet_usecases::resolved_link_graph_writer::{
    AssetGraphProjectionPolicy, ResolvedLinkGraphProjectionWriter,
};

#[test]
fn writer_maps_only_resolver_results_into_link_and_graph_projections() {
    let pointer = Pointer("v2");
    let resolver = Resolver;
    let mut links = Links::default();
    let mut graphs = Graphs::default();
    let associations = Associations::with_asset(&"a".repeat(64));
    let parsed = parsed();
    let mut writer = ResolvedLinkGraphProjectionWriter::new(
        &pointer,
        &resolver,
        &associations,
        AssetGraphProjectionPolicy::new(500).unwrap(),
        &mut links,
        &mut graphs,
    );
    writer
        .write(&identity(ProjectionKind::Links, "v2"), &body(), &parsed)
        .unwrap();
    writer
        .write(&identity(ProjectionKind::Graph, "v2"), &body(), &parsed)
        .unwrap();
    assert_eq!(links.record.as_ref().unwrap().backlinks().len(), 2);
    assert_eq!(links.record.as_ref().unwrap().unresolved_links().len(), 2);
    let graph = graphs.record.as_ref().unwrap();
    assert_eq!(graph.freshness_revision(), "v2");
    assert_eq!(graph.graph().status(), GraphProjectionStatus::Clean);
    assert_eq!(graph.graph().edges().len(), 6);
    assert_eq!(
        graph
            .graph()
            .edges()
            .iter()
            .filter(|edge| {
                edge.kind() == cabinet_domain::graph::GraphEdgeKind::ExternalReference
            })
            .count(),
        1
    );
}

#[test]
fn writer_merges_association_only_assets_without_duplicating_markdown_assets() {
    let pointer = Pointer("v2");
    let resolver = Resolver;
    let associations = Associations {
        values: vec![association(&"a".repeat(64)), association(&"b".repeat(64))],
    };
    let mut links = Links::default();
    let mut graphs = Graphs::default();
    let mut writer = ResolvedLinkGraphProjectionWriter::new(
        &pointer,
        &resolver,
        &associations,
        AssetGraphProjectionPolicy::new(500).unwrap(),
        &mut links,
        &mut graphs,
    );

    writer
        .write(&identity(ProjectionKind::Graph, "v2"), &body(), &parsed())
        .unwrap();

    let graph = graphs.record.as_ref().unwrap().graph();
    let attachment_edges = graph
        .edges()
        .iter()
        .filter(|edge| edge.kind() == cabinet_domain::graph::GraphEdgeKind::AttachmentReference)
        .count();
    assert_eq!(attachment_edges, 2);
}

#[test]
fn writer_rejects_stale_version_before_projection_writes() {
    let pointer = Pointer("v2");
    let resolver = Resolver;
    let mut links = Links::default();
    let mut graphs = Graphs::default();
    let associations = Associations::default();
    let mut writer = ResolvedLinkGraphProjectionWriter::new(
        &pointer,
        &resolver,
        &associations,
        AssetGraphProjectionPolicy::new(500).unwrap(),
        &mut links,
        &mut graphs,
    );
    assert_eq!(
        writer.write(&identity(ProjectionKind::Graph, "v1"), &body(), &parsed()),
        Err(ProjectionWriteError::Permanent)
    );
    assert!(links.record.is_none());
    assert!(graphs.record.is_none());
}

#[test]
fn writer_deletes_current_link_and_graph_projection_by_kind() {
    let pointer = Pointer("v2");
    let resolver = Resolver;
    let mut links = Links::default();
    let mut graphs = Graphs::default();
    let associations = Associations::default();
    let mut writer = ResolvedLinkGraphProjectionWriter::new(
        &pointer,
        &resolver,
        &associations,
        AssetGraphProjectionPolicy::new(500).unwrap(),
        &mut links,
        &mut graphs,
    );
    assert_eq!(
        writer.delete(&identity(ProjectionKind::Links, "v2")),
        Ok(())
    );
    assert_eq!(
        writer.delete(&identity(ProjectionKind::Graph, "v2")),
        Ok(())
    );
    assert_eq!(
        writer.delete(&identity(ProjectionKind::Graph, "v1")),
        Err(ProjectionWriteError::Permanent)
    );
    drop(writer);
    assert_eq!(links.delete_count, 1);
    assert_eq!(graphs.delete_count, 1);
}

fn body() -> DocumentBody {
    DocumentBody::new("body", DocumentBodyPolicy::new(1024).unwrap()).unwrap()
}

struct Pointer(&'static str);
impl CurrentDocumentVersionPointerPort for Pointer {
    fn load_current_version(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<Option<VersionId>, CurrentDocumentVersionPointerError> {
        Ok(Some(VersionId::new(self.0).unwrap()))
    }
    fn compare_and_set_current_version(
        &mut self,
        _: &WorkspaceId,
        _: &DocumentId,
        _: Option<&VersionId>,
        _: VersionId,
    ) -> Result<(), CurrentDocumentVersionPointerError> {
        unreachable!()
    }
}
struct Resolver;
impl DocumentLinkTargetResolver for Resolver {
    fn resolve(
        &self,
        _: &WorkspaceId,
        target: &str,
    ) -> Result<LinkTargetResolution, LinkTargetResolverError> {
        if target == "Known" {
            Ok(LinkTargetResolution::Resolved(
                ResolvedDocumentLinkTarget::new(
                    DocumentId::new("doc-2").unwrap(),
                    DocumentPath::new("known.md").unwrap(),
                ),
            ))
        } else {
            Ok(LinkTargetResolution::Unresolved(
                DocumentSlug::from_title(&DocumentTitle::new(target).unwrap()).unwrap(),
            ))
        }
    }

    fn resolve_relative(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
        target: &str,
    ) -> Result<LinkTargetResolution, LinkTargetResolverError> {
        if target == "../known.md" {
            Ok(LinkTargetResolution::Resolved(
                ResolvedDocumentLinkTarget::new(
                    DocumentId::new("doc-relative").unwrap(),
                    DocumentPath::new("known.md").unwrap(),
                ),
            ))
        } else {
            Ok(LinkTargetResolution::Unresolved(
                DocumentSlug::from_title(&DocumentTitle::new("Relative Missing").unwrap()).unwrap(),
            ))
        }
    }
}

#[derive(Default)]
struct Associations {
    values: Vec<AssetAssociation>,
}
impl Associations {
    fn with_asset(id: &str) -> Self {
        Self {
            values: vec![association(id)],
        }
    }
}
impl DocumentAssetAssociationReader for Associations {
    fn list_document_assets(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
        _: usize,
    ) -> Result<Vec<AssetAssociation>, AssetAssociationCatalogError> {
        Ok(self.values.clone())
    }
}
fn association(id: &str) -> AssetAssociation {
    AssetAssociation::new(
        AssetId::from_sha256_hex(id).unwrap(),
        DocumentId::new("doc-1").unwrap(),
        "Asset",
    )
    .unwrap()
}
#[derive(Default)]
struct Links {
    record: Option<LinkProjectionRecord>,
    delete_count: usize,
}
impl LinkIndex for Links {
    fn replace_document_links(
        &mut self,
        _: &WorkspaceId,
        r: LinkProjectionRecord,
    ) -> Result<(), LinkIndexError> {
        self.record = Some(r);
        Ok(())
    }
    fn get_document_links(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<Option<LinkProjectionRecord>, LinkIndexError> {
        Ok(self.record.clone())
    }
    fn delete_document_links(
        &mut self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<(), LinkIndexError> {
        self.delete_count += 1;
        self.record = None;
        Ok(())
    }
    fn list_backlinks(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<Vec<cabinet_domain::link::Backlink>, LinkIndexError> {
        Ok(vec![])
    }
    fn list_unresolved_links(
        &self,
        _: &WorkspaceId,
    ) -> Result<Vec<cabinet_domain::link::DocumentLink>, LinkIndexError> {
        Ok(vec![])
    }
    fn list_orphan_documents(
        &self,
        _: &WorkspaceId,
        _: &[DocumentId],
    ) -> Result<Vec<DocumentId>, LinkIndexError> {
        Ok(vec![])
    }
}
#[derive(Default)]
struct Graphs {
    record: Option<GraphProjectionRecord>,
    delete_count: usize,
}
impl GraphProjectionStore for Graphs {
    fn replace_projection(
        &mut self,
        _: &WorkspaceId,
        r: GraphProjectionRecord,
    ) -> Result<(), GraphProjectionError> {
        self.record = Some(r);
        Ok(())
    }
    fn get_projection(
        &self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<Option<GraphProjectionRecord>, GraphProjectionError> {
        Ok(self.record.clone())
    }
    fn delete_projection(
        &mut self,
        _: &WorkspaceId,
        _: &DocumentId,
    ) -> Result<(), GraphProjectionError> {
        self.delete_count += 1;
        self.record = None;
        Ok(())
    }
}
fn identity(kind: ProjectionKind, v: &str) -> ProjectionWorkIdentity {
    ProjectionWorkIdentity::new(
        WorkspaceId::new("workspace-1").unwrap(),
        DocumentId::new("doc-1").unwrap(),
        VersionId::new(v).unwrap(),
        kind,
    )
}
fn parsed() -> ParsedMarkdown {
    let a = SourceRange::new(1, 4).unwrap();
    let b = SourceRange::new(5, 8).unwrap();
    let c = SourceRange::new(9, 12).unwrap();
    ParsedMarkdown::new(
        vec![],
        vec![
            ParsedWikilink::new("Known", None, a).unwrap(),
            ParsedWikilink::new("Missing", None, b).unwrap(),
        ],
        vec![
            ParsedAssetReference::new(
                AssetId::from_sha256_hex(&"a".repeat(64)).unwrap(),
                "Asset",
                c,
            )
            .unwrap(),
        ],
    )
    .with_external_links(vec![
        ParsedExternalLink::new("https://example.com/docs?q=private", "Example", c).unwrap(),
    ])
    .with_document_links(vec![
        ParsedDocumentLink::new("../known.md", "Known relative", a).unwrap(),
        ParsedDocumentLink::new("missing.md", "Missing relative", b).unwrap(),
    ])
}
