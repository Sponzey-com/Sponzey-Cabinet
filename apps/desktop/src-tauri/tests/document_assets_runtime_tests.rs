use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_association_catalog::DurableAssetAssociationCatalog;
use cabinet_adapters::durable_asset_metadata_catalog::DurableAssetMetadataCatalog;
use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_desktop_shell::{
    DesktopAssetDetailRequestDto, DesktopAssetLinkRequestDto, DesktopAssetSearchRequestDto,
    DesktopAssetSearchRuntime, DesktopAssetUnlinkRequestDto, DesktopDocumentAssetsRuntime,
    DesktopDocumentAuthoringRequestDto, DesktopDocumentAuthoringRuntime,
    DesktopLocalCommandPayloadDto, DesktopLocalCommandRequestDto, DesktopProjectionRuntime,
    DesktopWorkspaceAssetsRequestDto,
};
use cabinet_domain::graph::GraphEdgeKind;
use cabinet_ports::graph_projection::GraphProjectionStore;

#[test]
fn native_workspace_asset_page_and_existing_link_survive_restart() {
    let temp = TempRoot::new("workspace-link");
    seed_document_id(&temp.path, "doc-1");
    seed_document_id(&temp.path, "doc-2");
    seed_asset(&temp.path);
    let runtime =
        DesktopDocumentAssetsRuntime::new(temp.path.clone(), 10 * 1024 * 1024).expect("runtime");

    let page = runtime.list_workspace(DesktopWorkspaceAssetsRequestDto {
        workspace_id: "workspace-1".into(),
        cursor: None,
        limit: 25,
    });
    assert!(page.ok, "page={page:?}");
    let data = page.data.expect("page data");
    assert_eq!(data.assets.len(), 1);
    assert_eq!(data.assets[0].file_name, "spec.pdf");
    assert!(data.next_cursor.is_none());

    let linked = runtime.link(DesktopAssetLinkRequestDto {
        workspace_id: "workspace-1".into(),
        document_id: "doc-2".into(),
        asset_id: "a".repeat(64),
        label: "Spec for doc 2".into(),
    });
    assert!(linked.ok, "link={linked:?}");
    assert!(linked.linked);
    assert_eq!(linked.reference_count, 2);

    let restarted =
        DesktopDocumentAssetsRuntime::new(temp.path.clone(), 10 * 1024 * 1024).expect("restart");
    let doc_two = restarted.execute(request("doc-2"));
    assert!(doc_two.ok, "doc two={doc_two:?}");
    assert_eq!(doc_two.data.expect("doc two data").assets.len(), 1);
    let detail = restarted.detail(DesktopAssetDetailRequestDto {
        workspace_id: "workspace-1".into(),
        asset_id: "a".repeat(64),
    });
    assert_eq!(detail.data.expect("detail").reference_count, 2);
}
use cabinet_domain::asset::{
    AssetAssociation, AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId,
    AssetMediaType, AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_association_catalog::AssetAssociationCatalog;
use cabinet_ports::asset_external_open::{AssetExternalOpenError, AssetExternalOpener};
use cabinet_ports::asset_metadata_catalog::AssetMetadataCatalog;

#[test]
fn native_document_assets_runtime_returns_durable_safe_metadata() {
    let temp = TempRoot::new("ready");
    seed_document(&temp.path);
    seed_asset(&temp.path);
    let runtime =
        DesktopDocumentAssetsRuntime::new(temp.path.clone(), 10 * 1024 * 1024).expect("runtime");

    let response = runtime.execute(request("doc-1"));
    let json = serde_json::to_string(&response).expect("json");

    assert!(response.ok);
    let data = response.data.expect("data");
    assert_eq!(data.assets.len(), 1);
    assert_eq!(data.assets[0].file_name, "spec.pdf");
    assert!(json.contains("\"queryName\":\"list-document-assets\""));
    assert!(!json.contains("raw body"));
    assert!(!json.contains(&temp.path.display().to_string()));
}

#[test]
fn native_asset_search_runtime_finds_metadata_without_exposing_root_path() {
    let temp = TempRoot::new("asset-search");
    seed_asset(&temp.path);
    let runtime = DesktopAssetSearchRuntime::new(temp.path.clone());

    let response = runtime.execute(DesktopAssetSearchRequestDto {
        workspace_id: "workspace-1".into(),
        text: "spec".into(),
        limit: 10,
    });
    let json = serde_json::to_string(&response).expect("json");

    assert!(response.ok, "response={response:?}");
    let data = response.data.expect("data");
    assert_eq!(data.query_name, "search-assets");
    assert_eq!(data.workspace_id, "workspace-1");
    assert_eq!(data.results.len(), 1);
    assert_eq!(data.results[0].file_name, "spec.pdf");
    assert_eq!(data.results[0].media_type, "application/pdf");
    assert!(!json.contains(&temp.path.display().to_string()));
}

#[test]
fn native_document_assets_runtime_returns_empty_missing_and_corrupt_states() {
    let temp = TempRoot::new("states");
    seed_document(&temp.path);
    let runtime =
        DesktopDocumentAssetsRuntime::new(temp.path.clone(), 10 * 1024 * 1024).expect("runtime");
    let empty = runtime.execute(request("doc-1"));
    let missing = runtime.execute(request("missing-doc"));
    assert!(empty.ok);
    assert!(empty.data.expect("empty").assets.is_empty());
    assert_eq!(
        missing.error_code.as_deref(),
        Some("asset_query.document_not_found")
    );

    seed_asset(&temp.path);
    let asset_id = "a".repeat(64);
    let path = temp
        .path
        .join("assets/metadata")
        .join(hex("workspace-1"))
        .join(format!("{asset_id}.asset"));
    fs::create_dir_all(path.parent().expect("parent")).expect("dir");
    fs::write(path, "private-corrupt-metadata\n").expect("corrupt");
    let corrupt = runtime.execute(request("doc-1"));
    assert_eq!(
        corrupt.error_code.as_deref(),
        Some("asset_metadata.corrupted")
    );
    assert!(!format!("{corrupt:?}").contains("private-corrupt-metadata"));
}

#[test]
fn native_asset_detail_and_unlink_return_capability_and_durable_readback() {
    let temp = TempRoot::new("detail-unlink");
    seed_document(&temp.path);
    seed_asset(&temp.path);
    let runtime =
        DesktopDocumentAssetsRuntime::new(temp.path.clone(), 10 * 1024 * 1024).expect("runtime");

    let detail = runtime.detail(DesktopAssetDetailRequestDto {
        workspace_id: "workspace-1".into(),
        asset_id: "a".repeat(64),
    });
    assert!(detail.ok, "detail={detail:?}");
    let data = detail.data.expect("detail data");
    assert_eq!(data.preview_capability, "pdf");
    assert_eq!(data.extraction_status, "not_requested");
    assert_eq!(data.version, 1);
    assert_eq!(data.reference_count, 1);

    let unlinked = runtime.unlink(DesktopAssetUnlinkRequestDto {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        asset_id: "a".repeat(64),
    });
    assert!(unlinked.ok);
    assert!(unlinked.removed);
    assert_eq!(unlinked.remaining_references, 0);
    let restarted =
        DesktopDocumentAssetsRuntime::new(temp.path.clone(), 10 * 1024 * 1024).expect("restart");
    assert!(
        restarted
            .execute(request("doc-1"))
            .data
            .expect("readback")
            .assets
            .is_empty()
    );
    assert!(
        restarted
            .detail(DesktopAssetDetailRequestDto {
                workspace_id: "workspace-1".into(),
                asset_id: "a".repeat(64),
            })
            .ok
    );
}

#[test]
fn native_external_open_response_contains_no_internal_path() {
    let temp = TempRoot::new("external-open");
    seed_asset(&temp.path);
    let opener = Arc::new(RecordingExternalOpener::default());
    let runtime = DesktopDocumentAssetsRuntime::with_preview_limit_and_opener(
        temp.path.clone(),
        10 * 1024 * 1024,
        2 * 1024 * 1024,
        opener.clone(),
    )
    .expect("runtime");

    let response = runtime.open_external(DesktopAssetDetailRequestDto {
        workspace_id: "workspace-1".into(),
        asset_id: "a".repeat(64),
    });
    let json = serde_json::to_string(&response).expect("json");

    assert!(response.ok);
    assert!(response.opened);
    assert_eq!(
        opener.calls.lock().expect("calls").as_slice(),
        &[("workspace-1".into(), "a".repeat(64), "spec.pdf".into())]
    );
    assert!(!json.contains(&temp.path.display().to_string()));
    assert!(!json.contains("path"));
}

#[derive(Default)]
struct RecordingExternalOpener {
    calls: Mutex<Vec<(String, String, String)>>,
}

impl AssetExternalOpener for RecordingExternalOpener {
    fn open(
        &self,
        workspace: &WorkspaceId,
        asset: &AssetId,
        file_name: &AssetFileName,
    ) -> Result<(), AssetExternalOpenError> {
        self.calls.lock().expect("calls").push((
            workspace.as_str().into(),
            asset.as_str().into(),
            file_name.as_str().into(),
        ));
        Ok(())
    }
}

#[test]
fn native_asset_link_and_unlink_reindex_graph_attachment_edge_across_restart() {
    let temp = TempRoot::new("graph-association");
    seed_document_id(&temp.path, "doc-1");
    seed_document_id(&temp.path, "doc-2");
    seed_asset(&temp.path);
    let assets =
        DesktopDocumentAssetsRuntime::new(temp.path.clone(), 10 * 1024 * 1024).expect("assets");

    let linked = assets.link(DesktopAssetLinkRequestDto {
        workspace_id: "workspace-1".into(),
        document_id: "doc-2".into(),
        asset_id: "a".repeat(64),
        label: "Graph fixture".into(),
    });
    assert!(linked.ok, "linked={linked:?}");

    let projection = DesktopProjectionRuntime::new(temp.path.clone(), 10 * 1024 * 1024, 20, 3)
        .expect("projection");
    let projected = projection.run_once();
    assert!(projected.ok, "projected={projected:?}");
    assert_eq!(attachment_edge_count(&temp.path, "doc-2"), 1);

    let unlinked = assets.unlink(DesktopAssetUnlinkRequestDto {
        workspace_id: "workspace-1".into(),
        document_id: "doc-2".into(),
        asset_id: "a".repeat(64),
    });
    assert!(unlinked.ok, "unlinked={unlinked:?}");
    let reprojected = projection.run_once();
    assert!(reprojected.ok, "reprojected={reprojected:?}");
    assert_eq!(attachment_edge_count(&temp.path, "doc-2"), 0);

    drop(projection);
    let restarted =
        DesktopProjectionRuntime::new(temp.path.clone(), 10 * 1024 * 1024, 20, 3).expect("restart");
    assert!(restarted.run_once().ok);
    assert_eq!(attachment_edge_count(&temp.path, "doc-2"), 0);
}

fn attachment_edge_count(root: &PathBuf, document_id: &str) -> usize {
    DurableLocalGraphProjectionStore::new(root.clone())
        .get_projection(
            &WorkspaceId::new("workspace-1").expect("workspace"),
            &DocumentId::new(document_id).expect("document"),
        )
        .expect("graph read")
        .expect("graph projection")
        .graph()
        .edges()
        .iter()
        .filter(|edge| edge.kind() == GraphEdgeKind::AttachmentReference)
        .count()
}

fn request(document_id: &str) -> DesktopLocalCommandRequestDto {
    DesktopLocalCommandRequestDto {
        command_name: "list_document_assets".to_string(),
        payload: DesktopLocalCommandPayloadDto::DocumentIdentity {
            workspace_id: "workspace-1".to_string(),
            document_id: document_id.to_string(),
        },
    }
}

fn seed_document(root: &PathBuf) {
    seed_document_id(root, "doc-1");
}

fn seed_document_id(root: &PathBuf, document_id: &str) {
    let runtime =
        DesktopDocumentAuthoringRuntime::new(root.clone(), 10 * 1024 * 1024).expect("authoring");
    let response = runtime.execute(DesktopDocumentAuthoringRequestDto::Create {
        workspace_id: "workspace-1".to_string(),
        document_id: document_id.to_string(),
        path: format!("notes/{document_id}.md"),
        body: "raw body".to_string(),
        version_id: "version-1".to_string(),
        snapshot_ref: "snapshot-1".to_string(),
        author: "local-user".to_string(),
        summary: "Created".to_string(),
    });
    assert!(response.ok);
}

fn seed_asset(root: &PathBuf) {
    let id = AssetId::from_sha256_hex(&"a".repeat(64)).expect("id");
    let metadata = AssetMetadata::new(
        id.clone(),
        AssetFileName::new("spec.pdf").expect("file"),
        AssetMediaType::new("application/pdf").expect("media"),
        42,
    )
    .expect("metadata");
    let record = AssetCatalogRecord::new(
        metadata,
        1,
        AssetPreviewCapability::Pdf,
        AssetExtractionStatus::NotRequested,
    )
    .expect("record");
    DurableAssetMetadataCatalog::new(root.clone())
        .put(&WorkspaceId::new("workspace-1").expect("workspace"), record)
        .expect("metadata");
    DurableAssetAssociationCatalog::new(root.clone())
        .link(
            &WorkspaceId::new("workspace-1").expect("workspace"),
            AssetAssociation::new(id, DocumentId::new("doc-1").expect("doc"), "Spec")
                .expect("association"),
        )
        .expect("association link");
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
            "sponzey-phase012-assets-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("root");
        Self { path }
    }
}
impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
