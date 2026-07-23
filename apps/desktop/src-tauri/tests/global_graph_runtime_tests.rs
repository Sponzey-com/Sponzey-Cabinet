use cabinet_adapters::durable_asset_metadata_catalog::DurableAssetMetadataCatalog;
use cabinet_adapters::durable_canvas_graph_projection::DurableCanvasGraphRelationProjectionStore;
use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_adapters::local_create_document_revision_runtime::LOCAL_DOCUMENT_POINTER_ROOT;
use cabinet_adapters::local_current_document_version_pointer::LocalCurrentDocumentVersionPointer;
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_desktop_shell::{
    DesktopGlobalKnowledgeGraphRequestDto, DesktopGlobalKnowledgeGraphRuntime,
};
use cabinet_domain::version::{
    AttachmentSnapshotState, DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionSummary,
};
use cabinet_domain::{
    asset::{
        AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType,
        AssetMetadata, AssetPreviewCapability,
    },
    canvas::{CanvasId, CanvasRevision},
    document::{
        DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
    },
    graph::{GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph},
    version::{CurrentDocumentSnapshot, VersionId},
    workspace::WorkspaceId,
};
use cabinet_ports::asset_metadata_catalog::AssetMetadataCatalog;
use cabinet_ports::canvas_graph_projection::{
    CanvasGraphRelationProjectionBatch, CanvasGraphRelationProjectionRecord,
    CanvasGraphRelationProjectionWriter,
};
use cabinet_ports::current_document_version::CurrentDocumentVersionPointerPort;
use cabinet_ports::document_repository::{CurrentDocumentRecord, DocumentRepository};
use cabinet_ports::graph_projection::{GraphProjectionRecord, GraphProjectionStore};
use cabinet_ports::version_store::{VersionRecord, VersionSnapshot, VersionStore};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn native_global_graph_returns_bounded_camel_case_page_without_fake_center() {
    let temp = Temp::new();
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let mut store = DurableLocalGraphProjectionStore::new(temp.path.clone());
    for id in ["doc-a", "doc-b"] {
        seed_document(&temp.path, &workspace, id, &format!("문서 {id}"));
        let center = DocumentId::new(id).unwrap();
        let graph = KnowledgeGraph::new_with_center(
            center.clone(),
            vec![GraphNode::new_document(center)],
            vec![],
            GraphProjectionStatus::Clean,
        )
        .unwrap();
        store
            .replace_projection(
                &workspace,
                GraphProjectionRecord::new_with_revision(graph, "v1").unwrap(),
            )
            .unwrap();
        seed_graph_pointer(&temp.path, &workspace, id, None, "v1");
    }
    let source = GraphNode::new_document(DocumentId::new("doc-a").unwrap());
    let target = GraphNode::new_document(DocumentId::new("doc-b").unwrap());
    let relation = CanvasGraphRelationProjectionRecord::new(
        DocumentId::new("doc-a").unwrap(),
        vec![source.clone(), target.clone()],
        vec![
            GraphEdge::new(
                "canvas:canvas-1:edge-1",
                source.id().to_string(),
                target.id().to_string(),
                GraphEdgeKind::CanvasRelation,
            )
            .unwrap(),
        ],
    )
    .unwrap();
    DurableCanvasGraphRelationProjectionStore::new(temp.path.clone())
        .replace_canvas_relations(
            &workspace,
            CanvasGraphRelationProjectionBatch::new(
                CanvasId::new("canvas-1").unwrap(),
                CanvasRevision::new(1).unwrap(),
                vec![relation],
            )
            .unwrap(),
        )
        .unwrap();
    let response = DesktopGlobalKnowledgeGraphRuntime::new(temp.path.clone()).execute(
        DesktopGlobalKnowledgeGraphRequestDto {
            workspace_id: "workspace-1".into(),
            cursor: None,
            include_unresolved: true,
            include_assets: true,
            projection_limit: 1,
            node_limit: 10,
            edge_limit: 10,
        },
    );
    assert!(response.ok);
    let data = response.data.unwrap();
    assert_eq!(data.nodes.len(), 2);
    assert_eq!(data.status, "clean");
    assert_eq!(data.nodes[0].label, "문서 doc-a");
    assert_eq!(data.nodes[0].breadcrumb_label, "지도");
    assert_eq!(data.nodes[0].availability, "available");
    assert!(data.nodes[0].can_navigate);
    assert_eq!(data.next_cursor.as_deref(), Some("doc-a"));
    assert!(data.edges.iter().all(|edge| {
        data.nodes.iter().any(|node| node.id == edge.source_id)
            && data.nodes.iter().any(|node| node.id == edge.target_id)
    }));
    assert!(data.edges.iter().any(|edge| edge.kind == "canvas_relation"));
    let json = serde_json::to_string(&data).unwrap();
    assert!(json.contains("nextCursor"));
    assert!(json.contains("breadcrumbLabel"));
    assert!(json.contains("canNavigate"));
    assert!(!json.contains("centerDocumentId"));
    assert!(!json.contains(&temp.path.display().to_string()));

    seed_graph_pointer(&temp.path, &workspace, "doc-b", Some("v1"), "v2");
    let next = DesktopGlobalKnowledgeGraphRuntime::new(temp.path.clone()).execute(
        DesktopGlobalKnowledgeGraphRequestDto {
            workspace_id: "workspace-1".into(),
            cursor: data.next_cursor.clone(),
            include_unresolved: true,
            include_assets: true,
            projection_limit: 1,
            node_limit: 10,
            edge_limit: 10,
        },
    );
    assert!(next.ok, "next={next:?}");
    let next = next.data.unwrap();
    assert_eq!(next.status, "degraded");
    assert!(next.nodes.iter().any(|node| node.label == "문서 doc-b"));
    assert!(next.next_cursor.is_none());
}

#[test]
fn startup_reconcile_backfills_missing_current_document_projections_once() {
    let temp = Temp::new();
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    for id in ["doc-a", "doc-b"] {
        seed_document(&temp.path, &workspace, id, &format!("문서 {id}"));
        seed_current_version(&temp.path, &workspace, id, "v1");
    }
    let runtime =
        cabinet_desktop_shell::DesktopProjectionRuntime::new(temp.path.clone(), 1024 * 1024, 64, 3)
            .unwrap();

    let first = runtime.reconcile_current("workspace-1", 100);
    assert!(first.ok, "first={first:?}");
    assert_eq!(first.document_count, 2);
    assert_eq!(first.ready_document_count, 0);
    assert_eq!(first.enqueued_count, 6);
    let processed = runtime.run_once();
    assert!(processed.ok, "processed={processed:?}");
    assert_eq!(processed.ready_count, 6);

    let graph = DesktopGlobalKnowledgeGraphRuntime::new(temp.path.clone()).execute(
        DesktopGlobalKnowledgeGraphRequestDto {
            workspace_id: "workspace-1".into(),
            cursor: None,
            include_unresolved: true,
            include_assets: false,
            projection_limit: 100,
            node_limit: 100,
            edge_limit: 100,
        },
    );
    assert!(graph.ok, "graph={graph:?}");
    assert_eq!(graph.data.unwrap().nodes.len(), 2);

    let second = runtime.reconcile_current("workspace-1", 100);
    assert!(second.ok, "second={second:?}");
    assert_eq!(second.document_count, 2);
    assert_eq!(second.ready_document_count, 2);
    assert_eq!(second.enqueued_count, 0);
    assert_eq!(second.reset_count, 0);
}

fn seed_document(root: &std::path::Path, workspace: &WorkspaceId, id: &str, title: &str) {
    let document_id = DocumentId::new(id).unwrap();
    let metadata = DocumentMetadata::new(
        document_id.clone(),
        DocumentTitle::new(title).unwrap(),
        DocumentPath::new(&format!("지도/{id}.md")).unwrap(),
    )
    .unwrap();
    let body = DocumentBody::new(
        &format!("# {title}"),
        DocumentBodyPolicy::new(1024).unwrap(),
    )
    .unwrap();
    let record =
        CurrentDocumentRecord::new(metadata, CurrentDocumentSnapshot::new(document_id, body))
            .unwrap();
    LocalDocumentRepository::new(root.join("authoring-current"))
        .put_current(workspace, record)
        .unwrap();
}

fn seed_current_version(
    root: &std::path::Path,
    workspace: &WorkspaceId,
    document_id: &str,
    version_id: &str,
) {
    let document = DocumentId::new(document_id).unwrap();
    let version = VersionId::new(version_id).unwrap();
    let snapshot_ref = DocumentSnapshotRef::new(&format!("snapshot-{document_id}")).unwrap();
    let entry = VersionEntry::new(
        version.clone(),
        document.clone(),
        snapshot_ref.clone(),
        VersionAuthor::new("local-user").unwrap(),
        VersionSummary::new("Projection backfill fixture").unwrap(),
    )
    .unwrap();
    let snapshot = VersionSnapshot::with_attachment_state(
        document.clone(),
        snapshot_ref,
        DocumentBody::new(
            &format!("# 문서 {document_id}\n"),
            DocumentBodyPolicy::new(1024 * 1024).unwrap(),
        )
        .unwrap(),
        AttachmentSnapshotState::known(Vec::new()).unwrap(),
    );
    cabinet_adapters::local_version_store::LocalVersionStore::with_body_policy(
        root.join(
            cabinet_adapters::local_create_document_revision_runtime::LOCAL_DOCUMENT_VERSION_ROOT,
        ),
        DocumentBodyPolicy::new(1024 * 1024).unwrap(),
    )
    .append_version(workspace, VersionRecord::new(entry, snapshot).unwrap())
    .unwrap();
    LocalCurrentDocumentVersionPointer::new(root.join(LOCAL_DOCUMENT_POINTER_ROOT))
        .compare_and_set_current_version(workspace, &document, None, version)
        .unwrap();
}

#[test]
fn native_global_graph_rejects_zero_limit_safely() {
    let temp = Temp::new();
    let response = DesktopGlobalKnowledgeGraphRuntime::new(temp.path.clone()).execute(
        DesktopGlobalKnowledgeGraphRequestDto {
            workspace_id: "workspace-1".into(),
            cursor: None,
            include_unresolved: true,
            include_assets: true,
            projection_limit: 0,
            node_limit: 10,
            edge_limit: 10,
        },
    );
    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("GLOBAL_GRAPH_INVALID_INPUT")
    );
}

#[test]
fn native_global_graph_uses_the_same_safe_document_and_attachment_labels_as_local_graph() {
    let temp = Temp::new();
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    seed_document(&temp.path, &workspace, "doc-a", "본문 제목");
    let asset_id = AssetId::from_sha256_hex(&"b".repeat(64)).unwrap();
    let metadata = AssetMetadata::new(
        asset_id.clone(),
        AssetFileName::new("architecture.pdf").unwrap(),
        AssetMediaType::new("application/pdf").unwrap(),
        32,
    )
    .unwrap();
    DurableAssetMetadataCatalog::new(temp.path.clone())
        .put(
            &workspace,
            AssetCatalogRecord::new(
                metadata,
                1,
                AssetPreviewCapability::Pdf,
                AssetExtractionStatus::NotRequested,
            )
            .unwrap(),
        )
        .unwrap();
    let document_id = DocumentId::new("doc-a").unwrap();
    let document = GraphNode::new_document(document_id.clone());
    let attachment = GraphNode::new_attachment(asset_id.as_str()).unwrap();
    let edge = GraphEdge::new(
        "asset-edge",
        document.id().into(),
        attachment.id().into(),
        GraphEdgeKind::AttachmentReference,
    )
    .unwrap();
    DurableLocalGraphProjectionStore::new(temp.path.clone())
        .replace_projection(
            &workspace,
            GraphProjectionRecord::new_with_revision(
                KnowledgeGraph::new_with_center(
                    document_id,
                    vec![document, attachment],
                    vec![edge],
                    GraphProjectionStatus::Clean,
                )
                .unwrap(),
                "v-safe",
            )
            .unwrap(),
        )
        .unwrap();
    seed_graph_pointer(&temp.path, &workspace, "doc-a", None, "v-safe");

    let hidden = DesktopGlobalKnowledgeGraphRuntime::new(temp.path.clone()).execute(
        DesktopGlobalKnowledgeGraphRequestDto {
            workspace_id: "workspace-1".into(),
            cursor: None,
            include_unresolved: true,
            include_assets: false,
            projection_limit: 1,
            node_limit: 10,
            edge_limit: 10,
        },
    );
    assert!(hidden.ok, "hidden={hidden:?}");
    let hidden = hidden.data.unwrap();
    assert!(hidden.nodes.iter().all(|node| node.kind != "attachment"));
    assert!(
        hidden
            .edges
            .iter()
            .all(|edge| edge.kind != "attachment_reference")
    );

    let response = DesktopGlobalKnowledgeGraphRuntime::new(temp.path.clone()).execute(
        DesktopGlobalKnowledgeGraphRequestDto {
            workspace_id: "workspace-1".into(),
            cursor: None,
            include_unresolved: true,
            include_assets: true,
            projection_limit: 1,
            node_limit: 10,
            edge_limit: 10,
        },
    );
    assert!(response.ok);
    let data = response.data.unwrap();
    assert!(data.nodes.iter().any(|node| node.label == "본문 제목"));
    assert!(data.nodes.iter().any(|node| {
        node.kind == "attachment"
            && node.label == "architecture.pdf"
            && node.availability == "available"
            && node.can_navigate
    }));
    assert!(
        !data
            .nodes
            .iter()
            .any(|node| node.label == asset_id.as_str())
    );
}

fn seed_graph_pointer(
    root: &std::path::Path,
    workspace: &WorkspaceId,
    document_id: &str,
    expected: Option<&str>,
    next: &str,
) {
    LocalCurrentDocumentVersionPointer::new(root.join(LOCAL_DOCUMENT_POINTER_ROOT))
        .compare_and_set_current_version(
            workspace,
            &DocumentId::new(document_id).unwrap(),
            expected
                .map(|value| VersionId::new(value).unwrap())
                .as_ref(),
            VersionId::new(next).unwrap(),
        )
        .unwrap();
}
struct Temp {
    path: PathBuf,
}
impl Temp {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "cabinet-global-runtime-{}-{nonce}-{sequence}",
            std::process::id(),
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
