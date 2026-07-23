use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_metadata_catalog::DurableAssetMetadataCatalog;
use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_desktop_shell::{
    DesktopKnowledgeGraphRuntime, DesktopLocalCommandPayloadDto, DesktopLocalCommandRequestDto,
};
use cabinet_domain::asset::{
    AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType,
    AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::version::CurrentDocumentSnapshot;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_metadata_catalog::AssetMetadataCatalog;
use cabinet_ports::document_repository::{CurrentDocumentRecord, DocumentRepository};
use cabinet_ports::graph_projection::{GraphProjectionRecord, GraphProjectionStore};

#[test]
fn native_graph_runtime_returns_bounded_camel_case_data_from_durable_projection() {
    let temp = TempRoot::new("ready");
    seed(&temp.path);
    let runtime = DesktopKnowledgeGraphRuntime::new(temp.path.clone());

    let response = runtime.execute(request("outgoing", false, 10, 10));
    let json = serde_json::to_string(&response).expect("json");

    assert!(response.ok);
    let data = response.data.expect("data");
    assert_eq!(data.center_document_id, "center-doc");
    assert_eq!(data.status, "degraded");
    assert_eq!(data.nodes.len(), 2);
    assert!(
        data.nodes
            .iter()
            .all(|node| node.availability == "available")
    );
    assert!(data.nodes.iter().all(|node| node.can_navigate));
    assert!(data.nodes.iter().any(|node| {
        node.id == "center-doc"
            && node.label == "중심 문서"
            && node.breadcrumb_label == "제품 / 설계"
    }));
    assert!(data.nodes.iter().any(|node| {
        node.id == "neighbor-doc"
            && node.label == "연결 문서"
            && node.breadcrumb_label == "제품 / 참고"
    }));
    assert_eq!(data.edges.len(), 1);
    assert_eq!(data.freshness_revision, "version-9");
    assert!(json.contains("\"centerDocumentId\""));
    assert!(json.contains("\"freshnessRevision\""));
    assert!(json.contains("\"breadcrumbLabel\""));
    assert!(json.contains("\"canNavigate\":true"));
    assert!(!json.contains("center_document_id"));
    assert!(!json.contains(&temp.path.display().to_string()));
    assert!(!json.contains("raw document body"));
}

#[test]
fn native_graph_runtime_returns_stable_missing_and_invalid_failures() {
    let temp = TempRoot::new("missing-invalid");
    let runtime = DesktopKnowledgeGraphRuntime::new(temp.path.clone());

    let missing = runtime.execute(request("both", true, 10, 10));
    let invalid = runtime.execute(request("sideways", true, 0, 10));

    assert!(!missing.ok);
    assert_eq!(
        missing.error_code.as_deref(),
        Some("GRAPH_PROJECTION_NOT_FOUND")
    );
    assert!(!missing.retryable);
    assert!(!invalid.ok);
    assert_eq!(invalid.error_code.as_deref(), Some("GRAPH_INVALID_INPUT"));
    assert!(!invalid.retryable);
}

#[test]
fn native_graph_runtime_uses_safe_non_navigable_labels_for_missing_documents() {
    let temp = TempRoot::new("missing-document-label");
    seed_projection(&temp.path);
    let response = DesktopKnowledgeGraphRuntime::new(temp.path.clone())
        .execute(request("outgoing", false, 10, 10));

    assert!(response.ok);
    let data = response.data.expect("data");
    assert!(data.nodes.iter().all(|node| {
        node.label == "찾을 수 없는 문서"
            && node.availability == "missing"
            && !node.can_navigate
            && node.label != node.id
    }));
}

#[test]
fn native_graph_runtime_resolves_safe_labels_for_every_node_kind_without_recent_documents() {
    let temp = TempRoot::new("safe-node-labels");
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    seed_document_with_body(
        &temp.path,
        &workspace,
        "center-doc",
        "오래된 metadata 제목",
        "제품/center.md",
        "# 본문 첫 줄 제목\n본문",
    );
    let asset_id = "a".repeat(64);
    seed_asset_metadata(&temp.path, &workspace, &asset_id, "roadmap.pdf");
    seed_safe_label_projection(&temp.path, &workspace, &asset_id);

    let response = DesktopKnowledgeGraphRuntime::new(temp.path.clone())
        .execute(request_with_visibility("both", true, true, 20, 20));
    assert!(response.ok, "response={response:?}");
    let data = response.data.unwrap();

    let document = data
        .nodes
        .iter()
        .find(|node| node.kind == "document")
        .unwrap();
    assert_eq!(document.label, "본문 첫 줄 제목");
    assert_eq!(document.availability, "available");
    assert!(document.can_navigate);

    let attachment = data
        .nodes
        .iter()
        .find(|node| node.kind == "attachment")
        .unwrap();
    assert_eq!(attachment.label, "roadmap.pdf");
    assert_eq!(attachment.availability, "available");
    assert!(attachment.can_navigate);
    assert_ne!(attachment.label, asset_id);

    let unresolved = data
        .nodes
        .iter()
        .find(|node| node.kind == "unresolved_link")
        .unwrap();
    assert_eq!(unresolved.label, "Missing Note");
    assert_eq!(unresolved.availability, "missing");

    let external = data
        .nodes
        .iter()
        .find(|node| node.kind == "external_link")
        .unwrap();
    assert_eq!(external.label, "example.com");
    assert!(!external.label.contains("secret"));
    assert!(!external.label.contains("token"));
}

#[test]
fn native_graph_runtime_returns_sanitized_corruption_failure() {
    let temp = TempRoot::new("corrupt");
    seed(&temp.path);
    let snapshot = find_snapshot(&temp.path);
    fs::write(snapshot, "schema\t999\nprivate-document-body\n").expect("corrupt");
    let runtime = DesktopKnowledgeGraphRuntime::new(temp.path.clone());

    let response = runtime.execute(request("both", true, 10, 10));
    let debug = format!("{response:?}");

    assert!(!response.ok);
    assert_eq!(
        response.error_code.as_deref(),
        Some("GRAPH_PROJECTION_CORRUPTED")
    );
    assert!(!response.retryable);
    assert!(!debug.contains("private-document-body"));
    assert!(!debug.contains(&temp.path.display().to_string()));
}

fn request(
    direction: &str,
    include_unresolved: bool,
    node_limit: u16,
    edge_limit: u16,
) -> DesktopLocalCommandRequestDto {
    request_with_visibility(direction, include_unresolved, false, node_limit, edge_limit)
}

fn request_with_visibility(
    direction: &str,
    include_unresolved: bool,
    include_assets: bool,
    node_limit: u16,
    edge_limit: u16,
) -> DesktopLocalCommandRequestDto {
    DesktopLocalCommandRequestDto {
        command_name: "get_graph_projection".to_string(),
        payload: DesktopLocalCommandPayloadDto::GraphProjection {
            workspace_id: "workspace-1".to_string(),
            document_id: "center-doc".to_string(),
            depth: 1,
            direction: direction.to_string(),
            include_unresolved,
            include_assets,
            node_limit,
            edge_limit,
        },
    }
}

fn seed(root: &PathBuf) {
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    seed_document(
        root,
        &workspace,
        "center-doc",
        "중심 문서",
        "제품/설계/center.md",
    );
    seed_document(
        root,
        &workspace,
        "neighbor-doc",
        "연결 문서",
        "제품/참고/neighbor.md",
    );
    seed_projection(root);
}

fn seed_projection(root: &PathBuf) {
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let mut store = DurableLocalGraphProjectionStore::new(root.clone());
    store
        .replace_projection(
            &workspace,
            GraphProjectionRecord::new_with_revision(graph(), "version-9").expect("record"),
        )
        .expect("seed");
}

fn seed_document(root: &PathBuf, workspace: &WorkspaceId, id: &str, title: &str, path: &str) {
    seed_document_with_body(root, workspace, id, title, path, &format!("# {title}\n"));
}

fn seed_document_with_body(
    root: &PathBuf,
    workspace: &WorkspaceId,
    id: &str,
    title: &str,
    path: &str,
    body: &str,
) {
    let document_id = DocumentId::new(id).expect("document id");
    let metadata = DocumentMetadata::new(
        document_id.clone(),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
    )
    .expect("metadata");
    let body =
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("body policy")).expect("body");
    let record =
        CurrentDocumentRecord::new(metadata, CurrentDocumentSnapshot::new(document_id, body))
            .expect("record");
    LocalDocumentRepository::new(root.join("authoring-current"))
        .put_current(workspace, record)
        .expect("seed document");
}

fn seed_asset_metadata(root: &PathBuf, workspace: &WorkspaceId, id: &str, file_name: &str) {
    let asset_id = AssetId::from_sha256_hex(id).unwrap();
    let metadata = AssetMetadata::new(
        asset_id,
        AssetFileName::new(file_name).unwrap(),
        AssetMediaType::new("application/pdf").unwrap(),
        42,
    )
    .unwrap();
    DurableAssetMetadataCatalog::new(root.clone())
        .put(
            workspace,
            AssetCatalogRecord::new(
                metadata,
                1,
                AssetPreviewCapability::Pdf,
                AssetExtractionStatus::NotRequested,
            )
            .unwrap(),
        )
        .unwrap();
}

fn seed_safe_label_projection(root: &PathBuf, workspace: &WorkspaceId, asset_id: &str) {
    let center_id = DocumentId::new("center-doc").unwrap();
    let center = GraphNode::new_document(center_id.clone());
    let attachment = GraphNode::new_attachment(asset_id).unwrap();
    let unresolved = GraphNode::new_unresolved("Missing Note").unwrap();
    let external =
        GraphNode::new_external_link("https://user:secret@example.com/private?token=private")
            .unwrap();
    let nodes = vec![
        center.clone(),
        attachment.clone(),
        unresolved.clone(),
        external.clone(),
    ];
    let edges = vec![
        GraphEdge::new(
            "attachment-edge",
            center.id().into(),
            attachment.id().into(),
            GraphEdgeKind::AttachmentReference,
        )
        .unwrap(),
        GraphEdge::new(
            "unresolved-edge",
            center.id().into(),
            unresolved.id().into(),
            GraphEdgeKind::DocumentLink,
        )
        .unwrap(),
        GraphEdge::new(
            "external-edge",
            center.id().into(),
            external.id().into(),
            GraphEdgeKind::ExternalReference,
        )
        .unwrap(),
    ];
    DurableLocalGraphProjectionStore::new(root.clone())
        .replace_projection(
            workspace,
            GraphProjectionRecord::new_with_revision(
                KnowledgeGraph::new_with_center(
                    center_id,
                    nodes,
                    edges,
                    GraphProjectionStatus::Clean,
                )
                .unwrap(),
                "version-safe-labels",
            )
            .unwrap(),
        )
        .unwrap();
}

fn graph() -> KnowledgeGraph {
    let center_id = DocumentId::new("center-doc").expect("center");
    let center = GraphNode::new_document(center_id.clone());
    let neighbor = GraphNode::new_document(DocumentId::new("neighbor-doc").expect("neighbor"));
    let unresolved = GraphNode::new_unresolved("missing-doc").expect("unresolved");
    let edges = vec![
        GraphEdge::new(
            "edge-1",
            center.id().to_string(),
            neighbor.id().to_string(),
            GraphEdgeKind::DocumentLink,
        )
        .expect("edge"),
        GraphEdge::new(
            "edge-2",
            center.id().to_string(),
            unresolved.id().to_string(),
            GraphEdgeKind::DocumentLink,
        )
        .expect("edge"),
    ];
    KnowledgeGraph::new_with_center(
        center_id,
        vec![center, neighbor, unresolved],
        edges,
        GraphProjectionStatus::Degraded,
    )
    .expect("graph")
}

fn find_snapshot(root: &PathBuf) -> PathBuf {
    let workspace = fs::read_dir(root.join("graph-projections"))
        .expect("root")
        .next()
        .expect("workspace")
        .expect("workspace entry")
        .path();
    fs::read_dir(workspace)
        .expect("workspace")
        .next()
        .expect("snapshot")
        .expect("snapshot entry")
        .path()
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-cabinet-phase012-native-graph-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp root");
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
