use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_metadata_catalog::DurableAssetMetadataCatalog;
use cabinet_adapters::durable_canvas_repository::DurableCanvasRepository;
use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_adapters::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_adapters::local_link_index::LocalLinkIndex;
use cabinet_adapters::local_search_index::LocalSearchIndex;
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_domain::asset::{
    AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType,
    AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::canvas::{
    Canvas, CanvasEdge, CanvasEdgeId, CanvasId, CanvasLifecycleState, CanvasNode, CanvasNodeId,
    CanvasNodeTarget, CanvasPosition, CanvasRevision, CanvasTextCard, CanvasTitle, CanvasViewport,
};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::link::{Backlink, SourceRange};
use cabinet_domain::version::{
    CurrentDocumentSnapshot, DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId,
    VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_metadata_catalog::AssetMetadataCatalog;
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository};
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::document_repository::{CurrentDocumentRecord, DocumentRepository};
use cabinet_ports::graph_projection::{GraphProjectionRecord, GraphProjectionStore};
use cabinet_ports::link_index::{LinkIndex, LinkProjectionRecord};
use cabinet_ports::search_index::{SearchDocumentRecord, SearchIndex};
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot, VersionStore};
use cabinet_usecases::asset_lifecycle::{ListWorkspaceAssetsInput, ListWorkspaceAssetsUsecase};
use cabinet_usecases::canvas_viewport::{GetCanvasViewportInput, GetCanvasViewportUsecase};
use cabinet_usecases::document::{
    GetCurrentDocumentInput, GetCurrentDocumentUsecase, GetDocumentHistoryInput,
    GetDocumentHistoryUsecase,
};
use cabinet_usecases::global_graph::{
    GetGlobalKnowledgeGraphInput, GetGlobalKnowledgeGraphUsecase,
};
use cabinet_usecases::graph::{
    GetLinkOverviewInput, GetLinkOverviewUsecase, GetLocalKnowledgeGraphInput,
    GetLocalKnowledgeGraphUsecase, LocalGraphDirection,
};
use cabinet_usecases::search::{SearchDocumentsInput, SearchDocumentsUsecase};

const DOCUMENT_COUNT: usize = 10_000;
const HISTORY_VERSION_COUNT: usize = 1_000;
const LINK_COUNT: usize = 50_000;
const GRAPH_NODE_COUNT: usize = 10_000;
const GRAPH_EDGE_COUNT: usize = 50_000;
const GLOBAL_STANDARD_NODE_COUNT: usize = 500;
const GLOBAL_STANDARD_EDGE_COUNT: usize = 2_000;
const CANVAS_NODE_COUNT: usize = 2_000;
const CANVAS_EDGE_COUNT: usize = 4_000;
const ASSET_COUNT: usize = 10_000;
const PAGE_SIZE: usize = 50;
const WARMUP_COUNT: usize = 30;
const SAMPLE_COUNT: usize = 200;
const BUDGET_MS: f64 = 300.0;
const WORKSPACE: &str = "workspace-performance";
const TARGET_DOCUMENT: &str = "doc-09999";

fn main() {
    let root = benchmark_root();
    fs::create_dir_all(&root).expect("benchmark root");
    let fixture = Fixture::seed(root.clone());
    let mut measurements = Vec::new();

    measurements.push(measure(
        "current_document",
        "direct_current_pointer",
        || {
            let output = GetCurrentDocumentUsecase::new()
                .execute(
                    GetCurrentDocumentInput::by_id(WORKSPACE, TARGET_DOCUMENT),
                    &fixture.documents,
                )
                .map_err(|error| error.code().to_string())?;
            Ok(usize::from(
                output.record().document_id().as_str() == TARGET_DOCUMENT,
            ))
        },
    ));
    measurements.push(measure("history_page", "paged_version_history", || {
        let output = GetDocumentHistoryUsecase::new()
            .execute(
                GetDocumentHistoryInput::new(WORKSPACE, TARGET_DOCUMENT, None, PAGE_SIZE),
                &fixture.versions,
            )
            .map_err(|error| error.code().to_string())?;
        Ok(output.page().entries().len())
    }));
    measurements.push(measure("search", "local_search_projection_page_50", || {
        let output = SearchDocumentsUsecase::new()
            .execute(
                SearchDocumentsInput::new(WORKSPACE, "benchmark", PAGE_SIZE),
                &fixture.search,
            )
            .map_err(|error| error.code().to_string())?;
        Ok(output.page().results().len())
    }));
    measurements.push(measure(
        "link_overview",
        "local_link_projection_page_50",
        || {
            let output = GetLinkOverviewUsecase::new()
                .execute(
                    GetLinkOverviewInput::new(WORKSPACE, TARGET_DOCUMENT, None, PAGE_SIZE),
                    &fixture.links,
                )
                .map_err(|error| error.code().to_string())?;
            Ok(output.backlinks().len())
        },
    ));
    measurements.push(measure(
        "local_graph",
        "bounded_durable_graph_projection",
        || {
            let output = GetLocalKnowledgeGraphUsecase::new()
                .execute(
                    GetLocalKnowledgeGraphInput::new(
                        WORKSPACE,
                        TARGET_DOCUMENT,
                        2,
                        LocalGraphDirection::Both,
                        true,
                        true,
                        500,
                        1_000,
                    ),
                    &fixture.graphs,
                )
                .map_err(|error| error.code().to_string())?;
            Ok(output.graph().nodes().len().min(PAGE_SIZE))
        },
    ));
    measurements.push(measure(
        "global_graph",
        "bounded_workspace_graph_projection",
        || {
            let output = GetGlobalKnowledgeGraphUsecase::new()
                .execute(
                    GetGlobalKnowledgeGraphInput::new(WORKSPACE, None, true, true, 1, 1_000, 2_000),
                    &fixture.graphs,
                    &fixture.current_versions,
                )
                .map_err(|error| error.code().to_string())?;
            Ok(output.nodes().len().min(PAGE_SIZE))
        },
    ));
    measurements.push(measure(
        "canvas_viewport",
        "bounded_durable_canvas_viewport_projection",
        || {
            let output = GetCanvasViewportUsecase::new()
                .execute(
                    GetCanvasViewportInput::new(
                        WORKSPACE,
                        "canvas-performance",
                        Some(1_000),
                        Some(1_000),
                        Some(100),
                        1_280,
                        800,
                        200,
                        PAGE_SIZE,
                        100,
                    ),
                    &fixture.canvas,
                )
                .map_err(|error| error.code().to_string())?;
            Ok(output.nodes.len())
        },
    ));
    measurements.push(measure(
        "asset_metadata",
        "durable_asset_metadata_page_50",
        || {
            let input = ListWorkspaceAssetsInput::new(WORKSPACE, None, PAGE_SIZE)
                .map_err(|error| error.code().to_string())?;
            let output = ListWorkspaceAssetsUsecase::new()
                .execute(input, &fixture.assets)
                .map_err(|error| error.code().to_string())?;
            Ok(output.records().len())
        },
    ));

    let passed = measurements
        .iter()
        .all(|measurement| measurement.errors == 0 && measurement.p95_ms <= BUDGET_MS);
    println!(
        "phase012_native_query_benchmark={}",
        if passed { "passed" } else { "failed" }
    );
    println!("warmup_count={WARMUP_COUNT}");
    println!("sample_count={SAMPLE_COUNT}");
    println!("percentile_method=nearest_rank");
    for measurement in &measurements {
        println!(
            "query={};p50_ms={:.6};p95_ms={:.6};max_ms={:.6};error_count={};result_count={};query_path={}",
            measurement.id,
            measurement.p50_ms,
            measurement.p95_ms,
            measurement.max_ms,
            measurement.errors,
            measurement.result_count,
            measurement.query_path,
        );
    }
    let _ = fs::remove_dir_all(root);
    if !passed {
        std::process::exit(2);
    }
}

struct Fixture {
    documents: LocalDocumentRepository,
    versions: LocalVersionStore,
    search: LocalSearchIndex,
    links: LocalLinkIndex,
    graphs: DurableLocalGraphProjectionStore,
    current_versions: LocalCurrentDocumentVersionPointer,
    canvas: DurableCanvasRepository,
    assets: DurableAssetMetadataCatalog,
}

impl Fixture {
    fn seed(root: PathBuf) -> Self {
        let workspace = WorkspaceId::new(WORKSPACE).expect("workspace");
        let body_policy = DocumentBodyPolicy::new(16 * 1024).expect("body policy");
        let mut documents =
            LocalDocumentRepository::with_body_policy(root.join("current"), body_policy);
        let mut versions = LocalVersionStore::with_body_policy(root.join("versions"), body_policy);
        let mut search = LocalSearchIndex::default();
        let mut links = LocalLinkIndex::default();
        let mut graphs = DurableLocalGraphProjectionStore::new(root.clone());
        let mut current_versions =
            LocalCurrentDocumentVersionPointer::new(root.join("document-current-pointers"));
        let mut canvas = DurableCanvasRepository::new(root.clone());
        let mut assets = DurableAssetMetadataCatalog::new(root.clone());

        for index in 0..DOCUMENT_COUNT {
            let id = document_id(index);
            let title = if index.is_multiple_of(2) {
                format!("Benchmark Note {index:05}")
            } else {
                format!("Benchmark Knowledge {index:05}")
            };
            let title = DocumentTitle::new(&title).expect("title");
            let path = DocumentPath::new(&format!("fixture/{index:05}.md")).expect("path");
            let body = DocumentBody::new(
                &format!("benchmark indexed content {index:05}"),
                body_policy,
            )
            .expect("body");
            let metadata =
                DocumentMetadata::new(id.clone(), title.clone(), path.clone()).expect("metadata");
            documents
                .put_current(
                    &workspace,
                    CurrentDocumentRecord::new(
                        metadata,
                        CurrentDocumentSnapshot::new(id.clone(), body.clone()),
                    )
                    .expect("current record"),
                )
                .expect("seed current");
            search
                .upsert_document(
                    &workspace,
                    SearchDocumentRecord::new(id.clone(), title, path, body),
                )
                .expect("seed search");
        }

        seed_history(&mut versions, &workspace, body_policy);
        seed_links(&mut links, &workspace);
        seed_graph(&mut graphs, &mut current_versions, &workspace);
        seed_canvas(&mut canvas, &workspace);
        seed_assets(&mut assets, &workspace);

        Self {
            documents,
            versions,
            search,
            links,
            graphs,
            current_versions,
            canvas,
            assets,
        }
    }
}

fn seed_canvas(canvas: &mut DurableCanvasRepository, workspace: &WorkspaceId) {
    let nodes = (0..CANVAS_NODE_COUNT)
        .map(|index| {
            CanvasNode::new(
                CanvasNodeId::new(&format!("node-{index:04}")).expect("canvas node id"),
                CanvasNodeTarget::TextCard(
                    CanvasTextCard::new(&format!("fixture card {index:04}")).expect("canvas card"),
                ),
                CanvasPosition::new(((index % 40) * 100) as i32, ((index / 40) * 100) as i32),
            )
            .expect("canvas node")
        })
        .collect::<Vec<_>>();
    let edges = (0..CANVAS_EDGE_COUNT)
        .map(|index| {
            let source = index % CANVAS_NODE_COUNT;
            let target = (source + 1 + index / CANVAS_NODE_COUNT) % CANVAS_NODE_COUNT;
            CanvasEdge::new(
                CanvasEdgeId::new(&format!("edge-{index:04}")).expect("canvas edge id"),
                nodes[source].id().clone(),
                nodes[target].id().clone(),
            )
            .expect("canvas edge")
        })
        .collect::<Vec<_>>();
    let record = CanvasRecord::with_metadata(
        Canvas::new(
            CanvasId::new("canvas-performance").expect("canvas id"),
            nodes,
            edges,
            CanvasLifecycleState::Updated,
        )
        .expect("canvas"),
        CanvasTitle::new("Performance fixture").expect("canvas title"),
        CanvasRevision::new(1).expect("canvas revision"),
        CanvasViewport::default(),
    );
    canvas
        .create_canvas(workspace, record)
        .expect("seed canvas");
}

fn seed_history(
    versions: &mut LocalVersionStore,
    workspace: &WorkspaceId,
    body_policy: DocumentBodyPolicy,
) {
    let document = DocumentId::new(TARGET_DOCUMENT).expect("target document");
    for index in 0..HISTORY_VERSION_COUNT {
        let version_id = VersionId::new(&format!("version-{index:04}")).expect("version");
        let snapshot_ref =
            DocumentSnapshotRef::new(&format!("snapshot-{index:04}")).expect("snapshot");
        let entry = VersionEntry::new(
            version_id,
            document.clone(),
            snapshot_ref.clone(),
            VersionAuthor::new("fixture").expect("author"),
            VersionSummary::new("performance fixture").expect("summary"),
        )
        .expect("entry");
        let snapshot = VersionSnapshot::new(
            document.clone(),
            snapshot_ref,
            DocumentBody::new("benchmark history content", body_policy).expect("body"),
        );
        versions
            .append_version(
                workspace,
                VersionRecord::new(entry, snapshot).expect("record"),
            )
            .expect("seed history");
    }
}

fn seed_links(links: &mut LocalLinkIndex, workspace: &WorkspaceId) {
    let target = DocumentId::new(TARGET_DOCUMENT).expect("target");
    for source_index in 0..DOCUMENT_COUNT {
        let source = document_id(source_index);
        let backlinks = (0..(LINK_COUNT / DOCUMENT_COUNT))
            .map(|offset| {
                Backlink::new(
                    source.clone(),
                    target.clone(),
                    SourceRange::new(offset, offset + 1).expect("range"),
                )
            })
            .collect::<Vec<_>>();
        links
            .replace_document_links(
                workspace,
                LinkProjectionRecord::new(source, backlinks, Vec::new()).expect("links"),
            )
            .expect("seed links");
    }
}

fn seed_graph(
    graphs: &mut DurableLocalGraphProjectionStore,
    current_versions: &mut LocalCurrentDocumentVersionPointer,
    workspace: &WorkspaceId,
) {
    graphs
        .replace_projection(
            workspace,
            graph_record(
                "doc-00000",
                "global-standard",
                GLOBAL_STANDARD_NODE_COUNT,
                GLOBAL_STANDARD_EDGE_COUNT,
            ),
        )
        .expect("seed global standard graph");
    graphs
        .replace_projection(
            workspace,
            graph_record(
                TARGET_DOCUMENT,
                "local-stress",
                GRAPH_NODE_COUNT,
                GRAPH_EDGE_COUNT,
            ),
        )
        .expect("seed local stress graph");
    for center in ["doc-00000", TARGET_DOCUMENT] {
        current_versions
            .compare_and_set_current_version(
                workspace,
                &DocumentId::new(center).expect("graph center"),
                None,
                VersionId::new("performance-revision").expect("graph version"),
            )
            .expect("seed graph current pointer");
    }
}

fn graph_record(
    center: &str,
    edge_prefix: &str,
    node_count: usize,
    edge_count: usize,
) -> GraphProjectionRecord {
    let nodes = (0..node_count)
        .map(|index| GraphNode::new_document(document_id(index)))
        .collect::<Vec<_>>();
    let edges = (0..edge_count)
        .map(|index| {
            let source = index % node_count;
            let mut target = (index * 7 + index / node_count + 1) % node_count;
            if target == source {
                target = (target + 1) % node_count;
            }
            GraphEdge::new(
                &format!("{edge_prefix}-edge-{index:05}"),
                document_id(source).as_str().to_string(),
                document_id(target).as_str().to_string(),
                GraphEdgeKind::DocumentLink,
            )
            .expect("edge")
        })
        .collect::<Vec<_>>();
    let graph = KnowledgeGraph::new_with_center(
        DocumentId::new(center).expect("center"),
        nodes,
        edges,
        GraphProjectionStatus::Clean,
    )
    .expect("graph");
    GraphProjectionRecord::new_with_revision(graph, "performance-revision").expect("record")
}

fn seed_assets(assets: &mut DurableAssetMetadataCatalog, workspace: &WorkspaceId) {
    for index in 0..ASSET_COUNT {
        let asset_id = AssetId::from_sha256_hex(&format!("{index:064x}")).expect("asset id");
        let metadata = AssetMetadata::new(
            asset_id,
            AssetFileName::new(&format!("fixture-{index:05}.png")).expect("file name"),
            AssetMediaType::new("image/png").expect("media type"),
            1_024,
        )
        .expect("metadata");
        assets
            .put(
                workspace,
                AssetCatalogRecord::new(
                    metadata,
                    1,
                    AssetPreviewCapability::Image,
                    AssetExtractionStatus::NotRequested,
                )
                .expect("asset record"),
            )
            .expect("seed asset");
    }
}

struct Measurement {
    id: &'static str,
    query_path: &'static str,
    p50_ms: f64,
    p95_ms: f64,
    max_ms: f64,
    errors: usize,
    result_count: usize,
}

fn measure(
    id: &'static str,
    query_path: &'static str,
    mut operation: impl FnMut() -> Result<usize, String>,
) -> Measurement {
    let mut errors = 0;
    let mut result_count = 0;
    for _ in 0..WARMUP_COUNT {
        match operation() {
            Ok(count) => result_count = count,
            Err(_) => errors += 1,
        }
    }
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let started = Instant::now();
        match operation() {
            Ok(count) => result_count = count,
            Err(_) => errors += 1,
        }
        samples.push(started.elapsed().as_nanos());
    }
    samples.sort_unstable();
    Measurement {
        id,
        query_path,
        p50_ms: percentile_ms(&samples, 50),
        p95_ms: percentile_ms(&samples, 95),
        max_ms: nanos_to_ms(*samples.last().expect("measurement sample")),
        errors,
        result_count,
    }
}

fn percentile_ms(samples: &[u128], percentile: usize) -> f64 {
    let rank = ((samples.len() * percentile).div_ceil(100)).saturating_sub(1);
    nanos_to_ms(samples[rank])
}

fn nanos_to_ms(value: u128) -> f64 {
    value as f64 / 1_000_000.0
}

fn document_id(index: usize) -> DocumentId {
    DocumentId::new(&format!("doc-{index:05}")).expect("document id")
}

fn benchmark_root() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "sponzey-phase012-query-performance-{}-{stamp}",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_percentile_is_stable() {
        let samples = (1_u128..=200)
            .map(|value| value * 1_000_000)
            .collect::<Vec<_>>();
        assert_eq!(percentile_ms(&samples, 50), 100.0);
        assert_eq!(percentile_ms(&samples, 95), 190.0);
    }
}
