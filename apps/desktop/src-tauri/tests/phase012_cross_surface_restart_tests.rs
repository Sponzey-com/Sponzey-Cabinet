use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_local_search_index::DurableLocalSearchIndex;
use cabinet_desktop_shell::{
    DesktopAssetDetailRequestDto, DesktopAssetImportRequestDto, DesktopAssetImportSelectionRuntime,
    DesktopCanvasRequestDto, DesktopCanvasRuntime, DesktopDocumentAssetsRuntime,
    DesktopDocumentAuthoringRequestDto, DesktopDocumentAuthoringRuntime,
    DesktopKnowledgeGraphRuntime, DesktopLocalCommandPayloadDto, DesktopLocalCommandRequestDto,
    DesktopProjectionRuntime,
};
use cabinet_domain::document::DocumentBodyPolicy;
use cabinet_usecases::search::{SearchDocumentsInput, SearchDocumentsUsecase};

const WORKSPACE_ID: &str = "workspace-1";
const BODY_LIMIT: usize = 10 * 1024 * 1024;

#[test]
fn native_create_edit_projection_asset_canvas_flow_survives_runtime_restart() {
    let root = TempRoot::new("cross-surface-restart");

    let asset_id = {
        let authoring = DesktopDocumentAuthoringRuntime::new(root.path.clone(), BODY_LIMIT)
            .expect("authoring runtime");
        assert!(
            authoring
                .execute(create_document(
                    "target-doc",
                    "target.md",
                    "# Target Document",
                    "target-v1",
                ))
                .ok
        );
        assert!(
            authoring
                .execute(create_document(
                    "source-doc",
                    "source.md",
                    "# Source Document",
                    "source-v1",
                ))
                .ok
        );
        let updated = authoring.execute(DesktopDocumentAuthoringRequestDto::Update {
            workspace_id: WORKSPACE_ID.into(),
            document_id: "source-doc".into(),
            body: "# Source\nphase012needle [[Target Document]]".into(),
            expected_version_id: "source-v1".into(),
            version_id: "source-v2".into(),
            snapshot_ref: "snapshot-source-v2".into(),
            author: "local-user".into(),
            summary: "Add durable relation".into(),
        });
        assert!(updated.ok, "updated={updated:?}");
        assert_eq!(
            updated.data.expect("updated data").current_version_id,
            "source-v2"
        );

        let projection = DesktopProjectionRuntime::new(root.path.clone(), BODY_LIMIT, 64, 3)
            .expect("projection runtime");
        let projected = projection.run_once();
        assert!(projected.ok, "projected={projected:?}");
        let freshness = projection.get_freshness(WORKSPACE_ID, "source-doc");
        assert_eq!(freshness.state.as_deref(), Some("ready"));
        assert_eq!(freshness.current_version_id.as_deref(), Some("source-v2"));
        assert_search_result(&root.path, "source-doc");
        assert_graph_edges(&root.path, false);

        let source_path = root.path.join("fixture-asset.txt");
        fs::write(&source_path, b"phase012 asset bytes").expect("asset fixture");
        let importer = DesktopAssetImportSelectionRuntime::with_app_data_root(
            root.path.clone(),
            WORKSPACE_ID,
            4,
        )
        .expect("asset import runtime");
        let selection = importer.register_selected_paths(vec![source_path]);
        let handle = selection.data.expect("selection data").files[0]
            .handle
            .clone();
        let imported = importer.import(DesktopAssetImportRequestDto {
            workspace_id: WORKSPACE_ID.into(),
            document_id: "source-doc".into(),
            handle,
            label: "Phase 012 asset".into(),
        });
        assert!(imported.ok, "imported={imported:?}");
        assert_eq!(imported.state.as_deref(), Some("completed"));
        let asset_id = imported.asset_id.expect("asset id");

        let projected_asset = projection.run_once();
        assert!(projected_asset.ok, "projected asset={projected_asset:?}");
        assert_graph_edges(&root.path, true);

        let canvas = DesktopCanvasRuntime::new(root.path.clone()).expect("canvas runtime");
        let created = canvas.execute(DesktopCanvasRequestDto::Create {
            workspace_id: WORKSPACE_ID.into(),
            canvas_id: "canvas-1".into(),
            title: "Cross-surface map".into(),
        });
        assert!(created.ok, "canvas create={created:?}");
        let document_node = canvas.execute(DesktopCanvasRequestDto::AddDocumentNode {
            workspace_id: WORKSPACE_ID.into(),
            canvas_id: "canvas-1".into(),
            expected_revision: 1,
            node_id: "node-document".into(),
            document_id: "source-doc".into(),
            x: 20,
            y: 20,
            width: 320,
            height: 180,
            operation_id: "add-document".into(),
        });
        assert!(document_node.ok, "document node={document_node:?}");
        let asset_node = canvas.execute(DesktopCanvasRequestDto::AddAssetNode {
            workspace_id: WORKSPACE_ID.into(),
            canvas_id: "canvas-1".into(),
            expected_revision: 2,
            node_id: "node-asset".into(),
            asset_id: asset_id.clone(),
            x: 380,
            y: 20,
            width: 320,
            height: 180,
            operation_id: "add-asset".into(),
        });
        assert!(asset_node.ok, "asset node={asset_node:?}");
        assert_eq!(asset_node.data.expect("asset node data").revision, 3);

        asset_id
    };

    let authoring = DesktopDocumentAuthoringRuntime::new(root.path.clone(), BODY_LIMIT)
        .expect("restarted authoring");
    let current = authoring.execute(DesktopDocumentAuthoringRequestDto::GetCurrent {
        workspace_id: WORKSPACE_ID.into(),
        document_id: "source-doc".into(),
    });
    assert!(current.ok, "current={current:?}");
    let current = current.data.expect("current data");
    assert_eq!(current.current_version_id, "source-v2");
    assert_eq!(
        current.body.as_deref(),
        Some("# Source\nphase012needle [[Target Document]]")
    );

    let projection = DesktopProjectionRuntime::new(root.path.clone(), BODY_LIMIT, 64, 3)
        .expect("restarted projection");
    assert_eq!(
        projection
            .get_freshness(WORKSPACE_ID, "source-doc")
            .state
            .as_deref(),
        Some("ready")
    );
    assert_eq!(projection.run_once().ready_count, 0);
    assert_search_result(&root.path, "source-doc");
    assert_graph_edges(&root.path, true);

    let assets =
        DesktopDocumentAssetsRuntime::new(root.path.clone(), BODY_LIMIT).expect("restarted assets");
    let detail = assets.detail(DesktopAssetDetailRequestDto {
        workspace_id: WORKSPACE_ID.into(),
        asset_id: asset_id.clone(),
    });
    assert!(detail.ok, "detail={detail:?}");
    let detail = detail.data.expect("asset detail");
    assert_eq!(detail.asset_id, asset_id);
    assert_eq!(detail.linked_document_ids, vec!["source-doc"]);

    let canvas = DesktopCanvasRuntime::new(root.path.clone()).expect("restarted canvas");
    let loaded = canvas.execute(DesktopCanvasRequestDto::Get {
        workspace_id: WORKSPACE_ID.into(),
        canvas_id: "canvas-1".into(),
    });
    assert!(loaded.ok, "loaded canvas={loaded:?}");
    let loaded = loaded.data.expect("loaded canvas data");
    assert_eq!(loaded.revision, 3);
    assert_eq!(loaded.nodes.len(), 2);
    let document_node = loaded
        .nodes
        .iter()
        .find(|node| node.node_id == "node-document")
        .expect("document node");
    assert_eq!(document_node.target_id, "source-doc");
    assert_eq!(document_node.display_label, "Source");
    assert_eq!(document_node.target_status, "available");
    let asset_node = loaded
        .nodes
        .iter()
        .find(|node| node.node_id == "node-asset")
        .expect("asset node");
    assert_eq!(asset_node.target_id, detail.asset_id);
    assert_eq!(asset_node.display_label, "fixture-asset.txt");
    assert_eq!(asset_node.target_status, "available");

    for response in [
        serde_json::to_string(&detail).expect("detail json"),
        serde_json::to_string(&loaded).expect("canvas json"),
    ] {
        assert!(!response.contains(root.path.to_string_lossy().as_ref()));
        assert!(!response.contains("phase012 asset bytes"));
    }
}

fn assert_search_result(root: &PathBuf, expected_document_id: &str) {
    let search = DurableLocalSearchIndex::new(
        root.clone(),
        DocumentBodyPolicy::new(BODY_LIMIT).expect("body policy"),
    );
    let output = SearchDocumentsUsecase::new()
        .execute(
            SearchDocumentsInput::new(WORKSPACE_ID, "phase012needle", 10),
            &search,
        )
        .expect("search");
    assert_eq!(output.page().results().len(), 1);
    assert_eq!(
        output.page().results()[0].document_id().as_str(),
        expected_document_id
    );
}

fn assert_graph_edges(root: &PathBuf, include_asset: bool) {
    let response =
        DesktopKnowledgeGraphRuntime::new(root.clone()).execute(DesktopLocalCommandRequestDto {
            command_name: "get_graph_projection".into(),
            payload: DesktopLocalCommandPayloadDto::GraphProjection {
                workspace_id: WORKSPACE_ID.into(),
                document_id: "source-doc".into(),
                depth: 1,
                direction: "both".into(),
                include_unresolved: true,
                include_assets: include_asset,
                node_limit: 20,
                edge_limit: 40,
            },
        });
    assert!(response.ok, "graph={response:?}");
    let graph = response.data.expect("graph data");
    assert_eq!(graph.freshness_revision, "source-v2");
    assert!(graph.edges.iter().any(|edge| edge.kind == "document_link"));
    assert_eq!(
        graph
            .edges
            .iter()
            .any(|edge| edge.kind == "attachment_reference"),
        include_asset
    );
}

fn create_document(
    document_id: &str,
    path: &str,
    body: &str,
    version_id: &str,
) -> DesktopDocumentAuthoringRequestDto {
    DesktopDocumentAuthoringRequestDto::Create {
        workspace_id: WORKSPACE_ID.into(),
        document_id: document_id.into(),
        path: path.into(),
        body: body.into(),
        version_id: version_id.into(),
        snapshot_ref: format!("snapshot-{version_id}"),
        author: "local-user".into(),
        summary: "Created".into(),
    }
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
            "sponzey-phase012-{label}-{}-{nonce}",
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
