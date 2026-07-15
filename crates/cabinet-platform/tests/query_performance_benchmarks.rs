use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use cabinet_adapters::local_document_asset_repository::LocalDocumentAssetRepository;
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_adapters::local_graph_projection::LocalGraphProjectionStore;
use cabinet_adapters::local_link_index::LocalLinkIndex;
use cabinet_adapters::local_search_index::LocalSearchIndex;
use cabinet_adapters::local_version_store::LocalVersionStore;
use cabinet_core::performance::{
    MeasurementEnvironment, PerformanceFixtureProfile, PerformanceReport, PerformanceSample,
    PerformanceTarget,
};
use cabinet_domain::asset::{
    AssetFileName, AssetId, AssetMediaType, AssetMetadata, AssetReference,
};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentSlug,
    DocumentTitle,
};
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::link::{Backlink, DocumentLink, LinkTarget, SourceRange};
use cabinet_domain::permission::{
    AccessResource, Permission, PermissionDecision, PermissionDecisionReason, PolicySource,
};
use cabinet_domain::user::UserId;
use cabinet_domain::version::{
    CurrentDocumentSnapshot, DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId,
    VersionSummary,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_asset_repository::{DocumentAssetRecord, DocumentAssetRepository};
use cabinet_ports::document_repository::{CurrentDocumentRecord, DocumentRepository};
use cabinet_ports::graph_projection::{GraphProjectionRecord, GraphProjectionStore};
use cabinet_ports::link_index::{LinkIndex, LinkProjectionRecord};
use cabinet_ports::permission_aware_query::{PermissionAwareQueryError, PermissionDecisionPort};
use cabinet_ports::search_index::{SearchDocumentRecord, SearchIndex};
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot, VersionStore};
use cabinet_usecases::document::{
    GetCurrentDocumentInput, GetCurrentDocumentUsecase, GetDocumentHistoryInput,
    GetDocumentHistoryUsecase, GetDocumentVersionInput, GetDocumentVersionUsecase,
    ListDocumentAssetsInput, ListDocumentAssetsUsecase,
};
use cabinet_usecases::graph::{
    GraphLiteProjectionInput, GraphLiteProjectionUsecase, PermissionAwareGraphInput,
    PermissionAwareGraphUsecase,
};
use cabinet_usecases::search::{SearchDocumentsInput, SearchDocumentsUsecase};

const P95_GOAL_MS: u64 = 300;
const SAMPLE_COUNT: usize = 20;

#[test]
fn local_query_paths_meet_p95_300ms_goal_on_small_fixture() {
    let root = unique_root("query-performance");
    let profile = PerformanceFixtureProfile::small();
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let body_policy = DocumentBodyPolicy::new(64 * 1024).expect("body policy");
    let mut document_repository =
        LocalDocumentRepository::with_body_policy(root.join("documents"), body_policy);
    let mut version_store = LocalVersionStore::with_body_policy(root.join("versions"), body_policy);
    let mut search_index = LocalSearchIndex::default();
    let mut link_index = LocalLinkIndex::default();
    let mut document_assets = LocalDocumentAssetRepository::new(root.join("document-assets"));
    let mut graph_projection_store = LocalGraphProjectionStore::new();
    let target_document_id = document_id(0);
    let target_version_id = version_id(0, 4);
    let mut document_ids = Vec::new();

    for index in 0..profile.document_count() {
        let id = document_id(index);
        document_ids.push(id.as_str().to_string());
        seed_current_document(
            &mut document_repository,
            &mut search_index,
            &workspace_id,
            &id,
            index,
            body_policy,
        );
        if index == 0 {
            seed_versions(&mut version_store, &workspace_id, &id, body_policy);
            seed_assets(&mut document_assets, &workspace_id, &id);
        } else {
            seed_backlink(&mut link_index, &workspace_id, &id, &target_document_id);
        }
    }
    seed_unresolved_link(&mut link_index, &workspace_id, &target_document_id);
    seed_permission_aware_graph_projection(
        &mut graph_projection_store,
        &workspace_id,
        &target_document_id,
        &document_ids,
    );

    let known_document_ids = document_ids.iter().map(String::as_str).collect::<Vec<_>>();
    let current = GetCurrentDocumentUsecase::new();
    let history = GetDocumentHistoryUsecase::new();
    let version = GetDocumentVersionUsecase::new();
    let search = SearchDocumentsUsecase::new();
    let graph = GraphLiteProjectionUsecase::new();
    let permission_aware_graph = PermissionAwareGraphUsecase::new();
    let permission_checker = AllowAllPermissionDecision;
    let assets = ListDocumentAssetsUsecase::new();
    let mut samples = Vec::new();

    for _ in 0..SAMPLE_COUNT {
        push_sample(
            &mut samples,
            PerformanceTarget::CurrentDocumentLookup,
            || {
                current
                    .execute(
                        GetCurrentDocumentInput::by_id("workspace-1", target_document_id.as_str()),
                        &document_repository,
                    )
                    .expect("current lookup");
            },
        );
        push_sample(&mut samples, PerformanceTarget::HistoryListLookup, || {
            history
                .execute(
                    GetDocumentHistoryInput::new(
                        "workspace-1",
                        target_document_id.as_str(),
                        None,
                        10,
                    ),
                    &version_store,
                )
                .expect("history lookup");
        });
        push_sample(
            &mut samples,
            PerformanceTarget::SpecificVersionLookup,
            || {
                version
                    .execute(
                        GetDocumentVersionInput::new(
                            "workspace-1",
                            target_document_id.as_str(),
                            target_version_id.as_str(),
                        ),
                        &version_store,
                    )
                    .expect("version lookup");
            },
        );
        push_sample(&mut samples, PerformanceTarget::SearchLookup, || {
            search
                .execute(
                    SearchDocumentsInput::new("workspace-1", "alpha", 10),
                    &search_index,
                )
                .expect("search lookup");
        });
        push_sample(&mut samples, PerformanceTarget::LinkBacklinkLookup, || {
            graph
                .execute(
                    GraphLiteProjectionInput::new(
                        "workspace-1",
                        target_document_id.as_str(),
                        known_document_ids.clone(),
                    ),
                    &link_index,
                )
                .expect("graph lookup");
        });
        push_sample(
            &mut samples,
            PerformanceTarget::PermissionAwareGraphLookup,
            || {
                permission_aware_graph
                    .execute(
                        PermissionAwareGraphInput::new(
                            "workspace-1",
                            "benchmark-viewer",
                            target_document_id.as_str(),
                        ),
                        &graph_projection_store,
                        &permission_checker,
                    )
                    .expect("permission-aware graph lookup");
            },
        );
        push_sample(&mut samples, PerformanceTarget::AssetMetadataLookup, || {
            assets
                .execute(
                    ListDocumentAssetsInput::new("workspace-1", target_document_id.as_str()),
                    &document_repository,
                    &document_assets,
                )
                .expect("asset metadata lookup");
        });
    }

    let report = PerformanceReport::new(
        profile,
        MeasurementEnvironment::new("local-test", "small fixture integration benchmark"),
        samples,
    );
    for target in [
        PerformanceTarget::CurrentDocumentLookup,
        PerformanceTarget::HistoryListLookup,
        PerformanceTarget::SpecificVersionLookup,
        PerformanceTarget::SearchLookup,
        PerformanceTarget::LinkBacklinkLookup,
        PerformanceTarget::PermissionAwareGraphLookup,
        PerformanceTarget::AssetMetadataLookup,
    ] {
        assert!(
            report.passes_ms_goal(target, P95_GOAL_MS),
            "{target:?} p95={:?}ms exceeded {P95_GOAL_MS}ms",
            report.p95_ms(target)
        );
    }

    fs::remove_dir_all(root).ok();
}

struct AllowAllPermissionDecision;

impl PermissionDecisionPort for AllowAllPermissionDecision {
    fn check_permission(
        &self,
        _actor_user_id: &UserId,
        _resource: &AccessResource,
        permission: Permission,
    ) -> Result<PermissionDecision, PermissionAwareQueryError> {
        assert_eq!(permission, Permission::Read);
        Ok(PermissionDecision::allowed(
            PolicySource::Document,
            PermissionDecisionReason::RoleAllowsPermission,
        ))
    }
}

fn push_sample(
    samples: &mut Vec<PerformanceSample>,
    target: PerformanceTarget,
    operation: impl FnOnce(),
) {
    let started = Instant::now();
    operation();
    samples.push(PerformanceSample::new(
        target,
        started.elapsed().as_millis() as u64,
    ));
}

fn seed_current_document(
    document_repository: &mut LocalDocumentRepository,
    search_index: &mut LocalSearchIndex,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
    index: usize,
    body_policy: DocumentBodyPolicy,
) {
    let title = DocumentTitle::new(&format!("Alpha Document {index:04}")).expect("title");
    let path = DocumentPath::new(&format!("docs/doc-{index:04}.md")).expect("path");
    let body = DocumentBody::new(
        &format!("alpha body {index:04}\nlinked content for benchmark"),
        body_policy,
    )
    .expect("body");
    let metadata =
        DocumentMetadata::new(document_id.clone(), title.clone(), path.clone()).expect("metadata");
    let snapshot = CurrentDocumentSnapshot::new(document_id.clone(), body.clone());
    document_repository
        .put_current(
            workspace_id,
            CurrentDocumentRecord::new(metadata, snapshot).expect("current record"),
        )
        .expect("put current");
    search_index
        .upsert_document(
            workspace_id,
            SearchDocumentRecord::new(document_id.clone(), title, path, body),
        )
        .expect("upsert search");
}

fn seed_versions(
    version_store: &mut LocalVersionStore,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
    body_policy: DocumentBodyPolicy,
) {
    for index in 0..5 {
        let version_id = version_id(0, index);
        let snapshot_ref =
            DocumentSnapshotRef::new(&format!("snapshot-0000-{index:02}")).expect("snapshot ref");
        let body = DocumentBody::new(&format!("alpha version {index}"), body_policy).expect("body");
        let entry = VersionEntry::new(
            version_id,
            document_id.clone(),
            snapshot_ref.clone(),
            VersionAuthor::new("system").expect("author"),
            VersionSummary::new("benchmark").expect("summary"),
        )
        .expect("version entry");
        let snapshot = VersionSnapshot::new(document_id.clone(), snapshot_ref, body);
        version_store
            .append_version(
                workspace_id,
                VersionRecord::new(entry, snapshot).expect("version record"),
            )
            .expect("append version");
    }
}

fn seed_backlink(
    link_index: &mut LocalLinkIndex,
    workspace_id: &WorkspaceId,
    source_document_id: &DocumentId,
    target_document_id: &DocumentId,
) {
    let range = SourceRange::new(0, 5).expect("range");
    let backlink = Backlink::new(
        source_document_id.clone(),
        target_document_id.clone(),
        range,
    );
    link_index
        .replace_document_links(
            workspace_id,
            LinkProjectionRecord::new(source_document_id.clone(), vec![backlink], Vec::new())
                .expect("projection"),
        )
        .expect("replace links");
}

fn seed_unresolved_link(
    link_index: &mut LocalLinkIndex,
    workspace_id: &WorkspaceId,
    source_document_id: &DocumentId,
) {
    let target_slug =
        DocumentSlug::from_title(&DocumentTitle::new("Missing Target").expect("title"))
            .expect("slug");
    let link = DocumentLink::new(
        source_document_id.clone(),
        LinkTarget::unresolved(target_slug),
        SourceRange::new(0, 5).expect("range"),
    );
    link_index
        .replace_document_links(
            workspace_id,
            LinkProjectionRecord::new(source_document_id.clone(), Vec::new(), vec![link])
                .expect("projection"),
        )
        .expect("replace links");
}

fn seed_permission_aware_graph_projection(
    graph_projection_store: &mut LocalGraphProjectionStore,
    workspace_id: &WorkspaceId,
    center_document_id: &DocumentId,
    document_ids: &[String],
) {
    let mut nodes = vec![GraphNode::new_document(center_document_id.clone())];
    let mut edges = Vec::new();

    for (index, document_id) in document_ids.iter().skip(1).take(12).enumerate() {
        let neighbor_id = DocumentId::new(document_id).expect("neighbor document id");
        nodes.push(GraphNode::new_document(neighbor_id.clone()));
        edges.push(
            GraphEdge::new(
                &format!("graph-edge-{index}"),
                center_document_id.as_str().to_string(),
                neighbor_id.as_str().to_string(),
                GraphEdgeKind::DocumentLink,
            )
            .expect("graph edge"),
        );
    }

    let graph = KnowledgeGraph::new_with_center(
        center_document_id.clone(),
        nodes,
        edges,
        GraphProjectionStatus::Clean,
    )
    .expect("knowledge graph");
    graph_projection_store
        .replace_projection(
            workspace_id,
            GraphProjectionRecord::new(graph).expect("graph projection record"),
        )
        .expect("replace graph projection");
}

fn seed_assets(
    document_assets: &mut LocalDocumentAssetRepository,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
) {
    for index in 0..5 {
        let asset_id = AssetId::from_sha256_hex(&format!("{:064x}", index + 1)).expect("asset id");
        let metadata = AssetMetadata::new(
            asset_id.clone(),
            AssetFileName::new(&format!("asset-{index}.png")).expect("file name"),
            AssetMediaType::new("image/png").expect("media type"),
            128,
        )
        .expect("metadata");
        let reference =
            AssetReference::new(asset_id, &format!("Asset {index}")).expect("reference");
        document_assets
            .attach_asset(
                workspace_id,
                document_id,
                DocumentAssetRecord::new(reference, metadata).expect("record"),
            )
            .expect("attach asset");
    }
}

fn document_id(index: usize) -> DocumentId {
    DocumentId::new(&format!("doc-{index:04}")).expect("document id")
}

fn version_id(document_index: usize, version_index: usize) -> VersionId {
    VersionId::new(&format!("v-{document_index:04}-{version_index:02}")).expect("version id")
}

fn unique_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("sponzey-cabinet-{label}-{}", std::process::id()));
    fs::remove_dir_all(&root).ok();
    root
}
