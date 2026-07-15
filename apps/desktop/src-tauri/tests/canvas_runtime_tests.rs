use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_asset_metadata_catalog::DurableAssetMetadataCatalog;
use cabinet_adapters::local_document_repository::LocalDocumentRepository;
use cabinet_desktop_shell::{
    DesktopCanvasProductEvent, DesktopCanvasRequestDto, DesktopCanvasRuntime,
};
use cabinet_domain::asset::{
    AssetCatalogRecord, AssetExtractionStatus, AssetFileName, AssetId, AssetMediaType,
    AssetMetadata, AssetPreviewCapability,
};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::version::CurrentDocumentSnapshot;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_metadata_catalog::AssetMetadataCatalog;
use cabinet_ports::document_repository::{CurrentDocumentRecord, DocumentRepository};

#[test]
fn desktop_canvas_runtime_persists_complete_dto_and_rejects_stale_mutation() {
    let root = temp_root("restart");
    let runtime = DesktopCanvasRuntime::new(root.clone()).expect("runtime");
    let created = runtime.execute(DesktopCanvasRequestDto::Create {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        title: "Product map".into(),
    });
    assert!(created.ok, "created={created:?}");
    assert_eq!(created.data.as_ref().expect("created data").revision, 1);
    assert_eq!(runtime.product_events().len(), 1);

    let added = runtime.execute(DesktopCanvasRequestDto::AddTextNode {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 1,
        node_id: "note-1".into(),
        text: "Decision note".into(),
        x: 40,
        y: 60,
        width: 320,
        height: 180,
        operation_id: "operation-add-text".into(),
    });
    assert!(added.ok, "added={added:?}");
    assert_eq!(added.operation_id.as_deref(), Some("operation-add-text"));
    assert_eq!(added.data.as_ref().expect("added data").revision, 2);
    assert_eq!(runtime.product_events().len(), 1);

    let viewport = runtime.execute(DesktopCanvasRequestDto::UpdateViewport {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 2,
        center_x: 500,
        center_y: 300,
        zoom_percent: 125,
        operation_id: "operation-viewport".into(),
    });
    assert!(viewport.ok, "viewport={viewport:?}");
    assert_eq!(runtime.product_events().len(), 1);

    let stale = runtime.execute(DesktopCanvasRequestDto::Rename {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 1,
        title: "Stale title".into(),
        operation_id: "operation-stale-rename".into(),
    });
    assert!(!stale.ok);
    assert_eq!(stale.error_code.as_deref(), Some("CANVAS_VERSION_CONFLICT"));
    assert_eq!(runtime.product_events().len(), 2);
    assert!(matches!(
        runtime.product_events().last(),
        Some(DesktopCanvasProductEvent::SaveFailed { canvas_id, error_code })
            if canvas_id == "canvas-1" && error_code == "CANVAS_VERSION_CONFLICT"
    ));
    let product_payload = format!("{:?}", runtime.product_events());
    assert!(!product_payload.contains("Decision note"));
    assert!(!product_payload.contains("Stale title"));

    let restarted = DesktopCanvasRuntime::new(root.clone()).expect("restart");
    let loaded = restarted.execute(DesktopCanvasRequestDto::Get {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
    });
    assert!(loaded.ok, "loaded={loaded:?}");
    let data = loaded.data.expect("loaded data");
    assert_eq!(data.title, "Product map");
    assert_eq!(data.revision, 3);
    assert_eq!(data.viewport.center_x, 500);
    assert_eq!(data.viewport.center_y, 300);
    assert_eq!(data.viewport.zoom_percent, 125);
    assert_eq!(data.nodes.len(), 1);
    assert_eq!(data.nodes[0].target_kind, "text");
    assert_eq!(data.nodes[0].width, 320);
    assert!(data.edges.is_empty());
    let bounded = restarted.execute(DesktopCanvasRequestDto::GetViewport {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        center_x: None,
        center_y: None,
        zoom_percent: None,
        surface_width: 1_200,
        surface_height: 720,
        overscan: 120,
        node_limit: 250,
        edge_limit: 500,
    });
    assert!(bounded.ok, "bounded={bounded:?}");
    let bounded = bounded.data.expect("bounded data");
    assert_eq!(bounded.revision, 3);
    assert_eq!(bounded.total_node_count, 1);
    assert!(!bounded.truncated);
    let serialized = serde_json::to_string(&data).expect("json");
    assert!(!serialized.contains(root.to_string_lossy().as_ref()));
    assert!(!serialized.contains("bytes"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn desktop_canvas_runtime_persists_rename_and_archive_across_restart() {
    let root = temp_root("rename-archive-restart");
    let runtime = DesktopCanvasRuntime::new(root.clone()).expect("runtime");
    assert!(
        runtime
            .execute(DesktopCanvasRequestDto::Create {
                workspace_id: "workspace-1".into(),
                canvas_id: "canvas-1".into(),
                title: "Original canvas".into(),
            })
            .ok
    );

    let renamed = runtime.execute(DesktopCanvasRequestDto::Rename {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 1,
        title: "Renamed canvas".into(),
        operation_id: "rename-operation-1".into(),
    });
    assert!(renamed.ok, "renamed={renamed:?}");
    assert_eq!(renamed.operation_id.as_deref(), Some("rename-operation-1"));
    let renamed_data = renamed.data.expect("renamed data");
    assert_eq!(renamed_data.title, "Renamed canvas");
    assert_eq!(renamed_data.revision, 2);

    let archived = runtime.execute(DesktopCanvasRequestDto::Archive {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 2,
        operation_id: "archive-operation-1".into(),
    });
    assert!(archived.ok, "archived={archived:?}");
    assert_eq!(
        archived.operation_id.as_deref(),
        Some("archive-operation-1")
    );
    let archived_data = archived.data.expect("archived data");
    assert_eq!(archived_data.lifecycle, "archived");
    assert_eq!(archived_data.revision, 3);

    drop(runtime);
    let restarted = DesktopCanvasRuntime::new(root.clone()).expect("restarted runtime");
    let reopened = restarted.execute(DesktopCanvasRequestDto::Get {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
    });
    assert!(reopened.ok, "reopened={reopened:?}");
    let reopened_data = reopened.data.expect("reopened data");
    assert_eq!(reopened_data.title, "Renamed canvas");
    assert_eq!(reopened_data.lifecycle, "archived");
    assert_eq!(reopened_data.revision, 3);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn desktop_canvas_runtime_maps_complete_mutation_contract_and_archives() {
    let root = temp_root("complete-contract");
    seed_canvas_targets(&root);
    let runtime = DesktopCanvasRuntime::new(root.clone()).expect("runtime");
    let created = runtime.execute(DesktopCanvasRequestDto::Create {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        title: "Knowledge flow".into(),
    });
    assert!(created.ok);
    let document = runtime.execute(DesktopCanvasRequestDto::AddDocumentNode {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 1,
        node_id: "document-1".into(),
        document_id: "doc-1".into(),
        x: 10,
        y: 20,
        width: 320,
        height: 180,
        operation_id: "operation-add-document".into(),
    });
    assert!(document.ok, "document={document:?}");
    let asset = runtime.execute(DesktopCanvasRequestDto::AddAssetNode {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 2,
        node_id: "asset-1".into(),
        asset_id: "a".repeat(64),
        x: 500,
        y: 20,
        width: 320,
        height: 180,
        operation_id: "operation-add-asset".into(),
    });
    assert!(asset.ok, "asset={asset:?}");
    let connected = runtime.execute(DesktopCanvasRequestDto::ConnectEdge {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 3,
        edge_id: "edge-1".into(),
        source_node_id: "document-1".into(),
        target_node_id: "asset-1".into(),
        operation_id: "operation-connect".into(),
    });
    assert!(connected.ok, "connected={connected:?}");
    let moved = runtime.execute(DesktopCanvasRequestDto::UpdateNodeGeometry {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 4,
        node_id: "asset-1".into(),
        x: 700,
        y: 240,
        width: 400,
        height: 220,
        operation_id: "operation-move".into(),
    });
    assert!(moved.ok, "moved={moved:?}");
    let preview = runtime.execute(DesktopCanvasRequestDto::PreviewAutoArrange {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 5,
    });
    assert!(preview.ok, "preview={preview:?}");
    let preview_data = preview.data.expect("preview data");
    assert_eq!(preview_data.revision, 5);
    assert_eq!(preview_data.nodes[0].x, 80);
    assert!(preview.operation_id.is_none());
    let unchanged = runtime.execute(DesktopCanvasRequestDto::Get {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
    });
    assert_eq!(unchanged.data.expect("unchanged").revision, 5);
    let arranged = runtime.execute(DesktopCanvasRequestDto::AutoArrange {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 5,
        operation_id: "operation-arrange".into(),
    });
    assert!(arranged.ok, "arranged={arranged:?}");
    let restarted =
        DesktopCanvasRuntime::new(root.clone()).expect("restart after geometry and edge");
    let reopened = restarted.execute(DesktopCanvasRequestDto::Get {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
    });
    let reopened = reopened.data.expect("reopened data");
    assert_eq!(reopened.revision, 6);
    assert_eq!(reopened.nodes.len(), 2);
    assert_eq!(reopened.edges.len(), 1);
    assert_eq!(reopened.nodes[0].x, 80);
    let resized = reopened
        .nodes
        .iter()
        .find(|node| node.node_id == "asset-1")
        .expect("resized asset");
    assert_eq!(resized.width, 400);
    assert_eq!(resized.height, 220);
    let edge_removed = runtime.execute(DesktopCanvasRequestDto::RemoveEdge {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 6,
        edge_id: "edge-1".into(),
        operation_id: "operation-remove-edge".into(),
    });
    assert!(edge_removed.ok, "edge_removed={edge_removed:?}");
    assert!(
        edge_removed
            .data
            .expect("edge removed data")
            .edges
            .is_empty()
    );
    let reopened_after_remove = DesktopCanvasRuntime::new(root.clone())
        .expect("restart after edge remove")
        .execute(DesktopCanvasRequestDto::Get {
            workspace_id: "workspace-1".into(),
            canvas_id: "canvas-1".into(),
        });
    assert!(
        reopened_after_remove
            .data
            .expect("reopened after remove")
            .edges
            .is_empty()
    );
    let removed = runtime.execute(DesktopCanvasRequestDto::RemoveNode {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 7,
        node_id: "asset-1".into(),
        operation_id: "operation-remove".into(),
    });
    assert!(removed.ok, "removed={removed:?}");
    let removed_data = removed.data.expect("removed data");
    assert_eq!(removed_data.nodes.len(), 1);
    assert!(removed_data.edges.is_empty(), "incident edge must cascade");
    let archived = runtime.execute(DesktopCanvasRequestDto::Archive {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 8,
        operation_id: "operation-archive".into(),
    });
    assert!(archived.ok, "archived={archived:?}");
    assert_eq!(archived.data.expect("archived data").lifecycle, "archived");
    assert_eq!(runtime.product_events().len(), 2);
    assert!(matches!(
        runtime.product_events().as_slice(),
        [
            DesktopCanvasProductEvent::Created { .. },
            DesktopCanvasProductEvent::Archived { .. }
        ]
    ));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn desktop_canvas_runtime_rejects_missing_durable_targets_without_revision_write() {
    let root = temp_root("missing-targets");
    let runtime = DesktopCanvasRuntime::new(root.clone()).expect("runtime");
    assert!(
        runtime
            .execute(DesktopCanvasRequestDto::Create {
                workspace_id: "workspace-1".into(),
                canvas_id: "canvas-1".into(),
                title: "Targets".into(),
            })
            .ok
    );
    let missing_document = runtime.execute(DesktopCanvasRequestDto::AddDocumentNode {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 1,
        node_id: "document-1".into(),
        document_id: "missing-doc".into(),
        x: 0,
        y: 0,
        width: 320,
        height: 180,
        operation_id: "operation-missing-document".into(),
    });
    assert_eq!(
        missing_document.error_code.as_deref(),
        Some("CANVAS_DOCUMENT_TARGET_NOT_FOUND")
    );
    let missing_asset = runtime.execute(DesktopCanvasRequestDto::AddAssetNode {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 1,
        node_id: "asset-1".into(),
        asset_id: "b".repeat(64),
        x: 0,
        y: 0,
        width: 320,
        height: 180,
        operation_id: "operation-missing-asset".into(),
    });
    assert_eq!(
        missing_asset.error_code.as_deref(),
        Some("CANVAS_ASSET_TARGET_NOT_FOUND")
    );
    let current = runtime
        .execute(DesktopCanvasRequestDto::Get {
            workspace_id: "workspace-1".into(),
            canvas_id: "canvas-1".into(),
        })
        .data
        .expect("current");
    assert_eq!(current.revision, 1);
    assert!(current.nodes.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn desktop_canvas_runtime_resolves_current_labels_and_preserves_deleted_targets_as_missing() {
    let root = temp_root("target-presentations");
    seed_canvas_targets(&root);
    let runtime = DesktopCanvasRuntime::new(root.clone()).expect("runtime");
    assert!(
        runtime
            .execute(DesktopCanvasRequestDto::Create {
                workspace_id: "workspace-1".into(),
                canvas_id: "canvas-1".into(),
                title: "Targets".into(),
            })
            .ok
    );
    assert!(
        runtime
            .execute(DesktopCanvasRequestDto::AddDocumentNode {
                workspace_id: "workspace-1".into(),
                canvas_id: "canvas-1".into(),
                expected_revision: 1,
                node_id: "document-1".into(),
                document_id: "doc-1".into(),
                x: 0,
                y: 0,
                width: 320,
                height: 180,
                operation_id: "add-document".into(),
            })
            .ok
    );
    let added = runtime.execute(DesktopCanvasRequestDto::AddAssetNode {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        expected_revision: 2,
        node_id: "asset-1".into(),
        asset_id: "a".repeat(64),
        x: 400,
        y: 0,
        width: 320,
        height: 180,
        operation_id: "add-asset".into(),
    });
    let data = added.data.expect("added");
    assert_eq!(data.nodes[0].display_label, "Canvas target");
    assert_eq!(data.nodes[0].target_status, "available");
    assert_eq!(data.nodes[1].display_label, "canvas.txt");

    LocalDocumentRepository::new(root.join("authoring-current"))
        .delete_current(
            &WorkspaceId::new("workspace-1").unwrap(),
            &DocumentId::new("doc-1").unwrap(),
        )
        .unwrap();
    fs::remove_file(
        root.join("assets/metadata")
            .join(hex("workspace-1"))
            .join(format!("{}.asset", "a".repeat(64))),
    )
    .unwrap();

    let missing = runtime
        .execute(DesktopCanvasRequestDto::Get {
            workspace_id: "workspace-1".into(),
            canvas_id: "canvas-1".into(),
        })
        .data
        .expect("missing read");
    assert_eq!(missing.revision, 3);
    assert_eq!(missing.nodes[0].target_id, "doc-1");
    assert_eq!(missing.nodes[0].target_status, "missing");
    assert_eq!(missing.nodes[1].target_id, "a".repeat(64));
    assert_eq!(missing.nodes[1].target_status, "missing");
    let _ = fs::remove_dir_all(root);
}

fn seed_canvas_targets(root: &std::path::Path) {
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let document_id = DocumentId::new("doc-1").expect("document");
    let metadata = DocumentMetadata::new(
        document_id.clone(),
        DocumentTitle::new("Canvas target").expect("title"),
        DocumentPath::new("canvas-target.md").expect("path"),
    )
    .expect("metadata");
    let snapshot = CurrentDocumentSnapshot::new(
        document_id,
        DocumentBody::new(
            "# Canvas target",
            DocumentBodyPolicy::new(1024).expect("policy"),
        )
        .expect("body"),
    );
    LocalDocumentRepository::new(root.join("authoring-current"))
        .put_current(
            &workspace,
            CurrentDocumentRecord::new(metadata, snapshot).expect("record"),
        )
        .expect("put document");

    let asset_id = AssetId::from_sha256_hex(&"a".repeat(64)).expect("asset");
    let media = AssetMediaType::new("text/plain").expect("media");
    let metadata = AssetMetadata::new(
        asset_id,
        AssetFileName::new("canvas.txt").expect("name"),
        media.clone(),
        4,
    )
    .expect("asset metadata");
    let record = AssetCatalogRecord::new(
        metadata,
        1,
        AssetPreviewCapability::for_media_type(&media),
        AssetExtractionStatus::NotRequested,
    )
    .expect("asset record");
    DurableAssetMetadataCatalog::new(root.to_path_buf())
        .put(&workspace, record)
        .expect("put asset");
}

#[test]
fn desktop_canvas_runtime_surfaces_corrupt_and_future_schema_as_recovery_required() {
    for (label, content) in [
        (
            "corrupt",
            "schema\t1\nchecksum\t0000000000000000\nprivate body\n",
        ),
        ("future", "schema\t99\nchecksum\t0000000000000000\n"),
    ] {
        let root = temp_root(label);
        let runtime = DesktopCanvasRuntime::new(root.clone()).expect("runtime");
        assert!(
            runtime
                .execute(DesktopCanvasRequestDto::Create {
                    workspace_id: "workspace-1".into(),
                    canvas_id: "canvas-1".into(),
                    title: "Recovery".into(),
                })
                .ok
        );
        fs::write(canvas_current_path(&root), content).expect("overwrite current");
        let response = runtime.execute(DesktopCanvasRequestDto::Get {
            workspace_id: "workspace-1".into(),
            canvas_id: "canvas-1".into(),
        });
        assert!(!response.ok);
        assert_eq!(
            response.error_code.as_deref(),
            Some("CANVAS_RECOVERY_REQUIRED")
        );
        assert!(response.recovery_required);
        assert!(response.data.is_none());
        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn desktop_canvas_runtime_recovers_corrupt_current_pointer_and_reopens_revision() {
    let root = temp_root("runtime-recover");
    let runtime = DesktopCanvasRuntime::new(root.clone()).expect("runtime");
    assert!(
        runtime
            .execute(DesktopCanvasRequestDto::Create {
                workspace_id: "workspace-1".into(),
                canvas_id: "canvas-1".into(),
                title: "Recovery".into(),
            })
            .ok
    );
    fs::write(canvas_current_path(&root), b"corrupt pointer").expect("corrupt pointer");

    let recovered = runtime.execute(DesktopCanvasRequestDto::Recover {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        operation_id: "recover-operation-1".into(),
    });

    assert!(recovered.ok, "recovered={recovered:?}");
    assert_eq!(
        recovered.operation_id.as_deref(),
        Some("recover-operation-1")
    );
    assert_eq!(recovered.data.as_ref().map(|data| data.revision), Some(1));
    assert!(
        runtime
            .execute(DesktopCanvasRequestDto::Get {
                workspace_id: "workspace-1".into(),
                canvas_id: "canvas-1".into(),
            })
            .ok
    );
    assert!(runtime.product_events().iter().any(|event| matches!(
        event,
        DesktopCanvasProductEvent::Recovered { canvas_id, revision }
            if canvas_id == "canvas-1" && *revision == 1
    )));
    drop(runtime);
    let restarted = DesktopCanvasRuntime::new(root.clone()).expect("restarted recovered runtime");
    let reopened = restarted.execute(DesktopCanvasRequestDto::Get {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
    });
    assert!(reopened.ok, "reopened={reopened:?}");
    assert_eq!(reopened.data.expect("reopened data").revision, 1);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn desktop_canvas_viewport_surfaces_missing_and_corrupt_projection() {
    let root = temp_root("viewport-recovery");
    let runtime = DesktopCanvasRuntime::new(root.clone()).expect("runtime");
    assert!(
        runtime
            .execute(DesktopCanvasRequestDto::Create {
                workspace_id: "workspace-1".into(),
                canvas_id: "canvas-1".into(),
                title: "Viewport recovery".into(),
            })
            .ok
    );
    let manifest = viewport_manifest_path(&root, 1);
    let original = fs::read_to_string(&manifest).expect("manifest");
    fs::remove_file(&manifest).expect("remove manifest");
    let missing = runtime.execute(viewport_request());
    assert_eq!(
        missing.error_code.as_deref(),
        Some("CANVAS_PROJECTION_STALE")
    );
    assert!(!missing.recovery_required);

    fs::write(&manifest, original).expect("restore manifest");
    fs::write(
        &manifest,
        "schema\t1\nchecksum\t0000000000000000\nkind\tmanifest\n",
    )
    .expect("corrupt manifest");
    let corrupt = runtime.execute(viewport_request());
    assert_eq!(
        corrupt.error_code.as_deref(),
        Some("CANVAS_RECOVERY_REQUIRED")
    );
    assert!(corrupt.recovery_required);
    let _ = fs::remove_dir_all(root);
}

fn viewport_request() -> DesktopCanvasRequestDto {
    DesktopCanvasRequestDto::GetViewport {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-1".into(),
        center_x: None,
        center_y: None,
        zoom_percent: None,
        surface_width: 1_200,
        surface_height: 720,
        overscan: 120,
        node_limit: 250,
        edge_limit: 500,
    }
}

fn canvas_current_path(root: &std::path::Path) -> PathBuf {
    root.join("canvases")
        .join(hex("workspace-1"))
        .join(hex("canvas-1"))
        .join("current.canvas")
}

fn viewport_manifest_path(root: &std::path::Path, revision: u64) -> PathBuf {
    canvas_current_path(root)
        .parent()
        .expect("canvas root")
        .join("viewport")
        .join("revisions")
        .join(format!("{revision:020}"))
        .join("manifest.viewport")
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn temp_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "sponzey-canvas-runtime-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("root");
    path
}
