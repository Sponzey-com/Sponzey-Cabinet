use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_canvas_repository::DurableCanvasRepository;
use cabinet_adapters::durable_last_canvas_selection::DurableLastCanvasSelection;
use cabinet_domain::asset::AssetId;
use cabinet_domain::canvas::{
    Canvas, CanvasEdge, CanvasEdgeId, CanvasGeometry, CanvasGeometryPolicy, CanvasId,
    CanvasLifecycleState, CanvasNode, CanvasNodeId, CanvasNodeTarget, CanvasPosition,
    CanvasRevision, CanvasSize, CanvasTextCard, CanvasTitle, CanvasViewport,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_catalog::{
    CanvasCatalogError, CanvasCatalogPort, LastCanvasSelectionError, LastCanvasSelectionPort,
};
use cabinet_ports::canvas_recovery::CanvasRecoveryRepository;
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository, CanvasRepositoryError};
use cabinet_ports::canvas_viewport_query::{
    CanvasViewportQuery, CanvasViewportQueryError, CanvasViewportQueryPort,
};

#[test]
fn durable_canvas_roundtrips_revisions_geometry_targets_edges_and_viewport() {
    let root = TempRoot::new("roundtrip");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let record = canvas_record(1, 100, CanvasLifecycleState::Draft);
    let mut repository = DurableCanvasRepository::new(root.path.clone());
    repository
        .create_canvas(&workspace, record.clone())
        .expect("create");
    let persisted = fs::read_to_string(current_path(&root.path)).expect("persisted record");
    assert!(!persisted.contains("private document body"));
    assert!(!persisted.contains("asset object bytes"));
    assert_eq!(
        repository
            .create_canvas(&workspace, record.clone())
            .expect_err("duplicate"),
        CanvasRepositoryError::AlreadyExists,
    );

    let restarted = DurableCanvasRepository::new(root.path.clone());
    let loaded = restarted
        .get_canvas(&workspace, record.canvas().id())
        .expect("get")
        .expect("loaded");
    assert_eq!(loaded, record);
    assert_eq!(loaded.canvas().nodes()[0].geometry().size().width(), 320);
    assert_eq!(loaded.viewport().zoom_percent(), 100);

    let next = canvas_record(2, 125, CanvasLifecycleState::Updated);
    repository
        .replace_canvas(
            &workspace,
            CanvasRevision::new(1).expect("revision"),
            next.clone(),
        )
        .expect("replace");
    assert_eq!(
        repository
            .replace_canvas(
                &workspace,
                CanvasRevision::new(1).expect("stale"),
                next.clone()
            )
            .expect_err("stale"),
        CanvasRepositoryError::VersionConflict,
    );
    let loaded = DurableCanvasRepository::new(root.path.clone())
        .get_canvas(&workspace, next.canvas().id())
        .expect("get next")
        .expect("next");
    assert_eq!(loaded, next);
    assert!(revision_path(&root.path, 1).exists());
    assert!(revision_path(&root.path, 2).exists());
}

#[test]
fn durable_canvas_discovers_current_records_with_workspace_identity_and_limit() {
    let root = TempRoot::new("current-discovery");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let record = canvas_record(1, 100, CanvasLifecycleState::Saved);
    let mut repository = DurableCanvasRepository::new(root.path.clone());
    repository
        .create_canvas(&workspace, record.clone())
        .expect("create");

    let discovered = DurableCanvasRepository::new(root.path.clone())
        .list_current_canvas_records(10)
        .expect("discover current Canvas records");
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].workspace_id(), &workspace);
    assert_eq!(discovered[0].record(), &record);
    assert_eq!(
        repository.list_current_canvas_records(0),
        Err(CanvasRepositoryError::InvalidInput)
    );

    let second_workspace = WorkspaceId::new("workspace-2").expect("second workspace");
    repository
        .create_canvas(&second_workspace, record)
        .expect("create second workspace Canvas");
    assert_eq!(
        repository.list_current_canvas_records(1),
        Err(CanvasRepositoryError::InvalidInput)
    );
}

#[test]
fn durable_canvas_catalog_is_workspace_bounded_and_filters_archived_records() {
    let root = TempRoot::new("catalog-port");
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let another_workspace = WorkspaceId::new("workspace-2").unwrap();
    let mut repository = DurableCanvasRepository::new(root.path.clone());
    repository
        .create_canvas(
            &workspace,
            catalog_record("active-canvas", "작업 Canvas", CanvasLifecycleState::Saved),
        )
        .unwrap();
    repository
        .create_canvas(
            &workspace,
            catalog_record(
                "archived-canvas",
                "보관 Canvas",
                CanvasLifecycleState::Archived,
            ),
        )
        .unwrap();
    repository
        .create_canvas(
            &another_workspace,
            catalog_record("other-canvas", "다른 Canvas", CanvasLifecycleState::Saved),
        )
        .unwrap();

    let active = repository
        .list_canvas_entries(&workspace, 10, false)
        .expect("active catalog");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].canvas_id().as_str(), "active-canvas");
    assert_eq!(active[0].title().as_str(), "작업 Canvas");
    assert_eq!(active[0].revision().value(), 1);
    assert_eq!(
        repository.list_canvas_entries(&workspace, 1, true),
        Err(CanvasCatalogError::LimitExceeded)
    );
}

#[test]
fn durable_last_canvas_selection_roundtrips_and_rejects_corruption() {
    let root = TempRoot::new("last-selection");
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let canvas = CanvasId::new("canvas-recent").unwrap();
    let mut selection = DurableLastCanvasSelection::new(root.path.clone());
    assert_eq!(selection.load_last_canvas_id(&workspace).unwrap(), None);

    selection
        .save_last_canvas_id(&workspace, &canvas)
        .expect("save selection");
    assert_eq!(
        DurableLastCanvasSelection::new(root.path.clone())
            .load_last_canvas_id(&workspace)
            .unwrap(),
        Some(canvas)
    );

    let selection_file = root
        .path
        .join("preferences/canvas-selection")
        .join(format!("{}.selection", hex("workspace-1")));
    fs::write(selection_file, "schema\t99\ncanvas\tprivate-path\n").unwrap();
    assert_eq!(
        DurableLastCanvasSelection::new(root.path.clone()).load_last_canvas_id(&workspace),
        Err(LastCanvasSelectionError::CorruptedSelection)
    );
}

#[test]
fn durable_canvas_reports_checksum_and_future_schema_without_leaking_content() {
    let root = TempRoot::new("corruption");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let record = canvas_record(1, 100, CanvasLifecycleState::Draft);
    let mut repository = DurableCanvasRepository::new(root.path.clone());
    repository
        .create_canvas(&workspace, record.clone())
        .expect("create");
    let current = current_path(&root.path);
    fs::write(
        &current,
        "schema\t1\nchecksum\t0000000000000000\nprivate document body\n",
    )
    .expect("corrupt");
    assert_eq!(
        repository
            .get_canvas(&workspace, record.canvas().id())
            .expect_err("corrupt"),
        CanvasRepositoryError::CorruptedCanvas,
    );
    fs::write(&current, "schema\t99\nchecksum\t0000000000000000\n").expect("future");
    assert_eq!(
        repository
            .get_canvas(&workspace, record.canvas().id())
            .expect_err("future"),
        CanvasRepositoryError::UnsupportedSchema,
    );
}

#[test]
fn durable_canvas_viewport_projection_is_bounded_restartable_and_explicitly_corruptible() {
    let root = TempRoot::new("viewport");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let record = canvas_record(1, 100, CanvasLifecycleState::Updated);
    let mut repository = DurableCanvasRepository::new(root.path.clone());
    repository
        .create_canvas(&workspace, record.clone())
        .expect("create");
    let query = CanvasViewportQuery {
        center_x: None,
        center_y: None,
        zoom_percent: None,
        surface_width: 1_200,
        surface_height: 720,
        overscan: 120,
        node_limit: 2,
        edge_limit: 10,
    };
    let page = DurableCanvasRepository::new(root.path.clone())
        .query_viewport(&workspace, record.canvas().id(), query)
        .expect("query")
        .expect("page");
    assert_eq!(page.revision.value(), 1);
    assert_eq!(page.total_node_count, 3);
    assert_eq!(page.nodes.len(), 2);
    assert!(page.truncated);

    fs::write(
        viewport_manifest_path(&root.path, 1),
        "schema\t1\nchecksum\t0000000000000000\nkind\tmanifest\n",
    )
    .expect("corrupt projection");
    assert_eq!(
        repository
            .query_viewport(&workspace, record.canvas().id(), query)
            .expect_err("corrupt"),
        CanvasViewportQueryError::CorruptedProjection,
    );
}

#[test]
fn durable_canvas_recovery_activates_latest_valid_revision_after_restart() {
    let root = TempRoot::new("recovery");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let first = canvas_record(1, 100, CanvasLifecycleState::Draft);
    let second = canvas_record(2, 125, CanvasLifecycleState::Updated);
    let mut repository = DurableCanvasRepository::new(root.path.clone());
    repository
        .create_canvas(&workspace, first.clone())
        .expect("create");
    repository
        .replace_canvas(&workspace, revision(1), second.clone())
        .expect("replace");
    fs::write(revision_path(&root.path, 2), b"corrupt newest revision").expect("corrupt newest");
    fs::write(current_path(&root.path), b"corrupt current pointer").expect("corrupt pointer");

    let mut restarted = DurableCanvasRepository::new(root.path.clone());
    let candidates = restarted
        .list_valid_revisions(&workspace, first.canvas().id(), 16)
        .expect("list candidates");
    assert_eq!(candidates, vec![revision(1)]);
    restarted
        .activate_revision(&workspace, first.canvas().id(), candidates[0])
        .expect("activate revision");

    let reopened = DurableCanvasRepository::new(root.path.clone())
        .get_canvas(&workspace, first.canvas().id())
        .expect("read after restart")
        .expect("canvas");
    assert_eq!(reopened, first);
    assert!(
        DurableCanvasRepository::new(root.path.clone())
            .query_viewport(
                &workspace,
                first.canvas().id(),
                CanvasViewportQuery {
                    center_x: None,
                    center_y: None,
                    zoom_percent: None,
                    surface_width: 1200,
                    surface_height: 720,
                    overscan: 120,
                    node_limit: 250,
                    edge_limit: 500,
                },
            )
            .expect("viewport after recovery")
            .is_some()
    );
}

#[cfg(unix)]
#[test]
fn durable_canvas_recovery_rejects_revision_symlink_without_following_it() {
    use std::os::unix::fs::symlink;

    let root = TempRoot::new("recovery-symlink");
    let workspace = WorkspaceId::new("workspace-1").expect("workspace");
    let record = canvas_record(1, 100, CanvasLifecycleState::Draft);
    let mut repository = DurableCanvasRepository::new(root.path.clone());
    repository
        .create_canvas(&workspace, record.clone())
        .expect("create");
    let external = root.path.join("outside.canvas");
    fs::write(
        &external,
        fs::read(revision_path(&root.path, 1)).expect("revision"),
    )
    .expect("external");
    symlink(&external, revision_path(&root.path, 2)).expect("revision symlink");

    assert_eq!(
        repository.list_valid_revisions(&workspace, record.canvas().id(), 16),
        Err(cabinet_ports::canvas_recovery::CanvasRecoveryRepositoryError::CorruptedCatalog)
    );
}

fn revision(value: u64) -> CanvasRevision {
    CanvasRevision::new(value).expect("revision")
}

fn canvas_record(revision: u64, zoom: u16, state: CanvasLifecycleState) -> CanvasRecord {
    let policy = CanvasGeometryPolicy::new(80, 1200, 60, 900, 25, 400).expect("policy");
    let document = CanvasNode::with_geometry(
        CanvasNodeId::new("document-node").expect("node"),
        CanvasNodeTarget::Document(DocumentId::new("doc-1").expect("document")),
        CanvasGeometry::new(
            CanvasPosition::new(10, 20),
            CanvasSize::new(320, 180, &policy).expect("size"),
        ),
    )
    .expect("document node");
    let note = CanvasNode::with_geometry(
        CanvasNodeId::new("note-node").expect("node"),
        CanvasNodeTarget::TextCard(CanvasTextCard::new("Decision note").expect("note")),
        CanvasGeometry::new(
            CanvasPosition::new(500, 240),
            CanvasSize::new(240, 120, &policy).expect("size"),
        ),
    )
    .expect("note node");
    let attachment = CanvasNode::new(
        CanvasNodeId::new("asset-node").expect("node"),
        CanvasNodeTarget::Attachment(AssetId::from_sha256_hex(&"a".repeat(64)).expect("asset")),
        CanvasPosition::new(800, 120),
    )
    .expect("asset node");
    let edge = CanvasEdge::new(
        CanvasEdgeId::new("edge-1").expect("edge"),
        document.id().clone(),
        note.id().clone(),
    )
    .expect("edge");
    let canvas = Canvas::new(
        CanvasId::new("canvas-1").expect("canvas"),
        vec![document, note, attachment],
        vec![edge],
        state,
    )
    .expect("canvas");
    CanvasRecord::with_metadata(
        canvas,
        CanvasTitle::new("Product map").expect("title"),
        CanvasRevision::new(revision).expect("revision"),
        CanvasViewport::new(400, 300, zoom, &policy).expect("viewport"),
    )
}

fn catalog_record(id: &str, title: &str, state: CanvasLifecycleState) -> CanvasRecord {
    CanvasRecord::with_metadata(
        Canvas::new(CanvasId::new(id).unwrap(), vec![], vec![], state).unwrap(),
        CanvasTitle::new(title).unwrap(),
        CanvasRevision::new(1).unwrap(),
        CanvasViewport::default(),
    )
}

fn current_path(root: &Path) -> PathBuf {
    root.join("canvases")
        .join(hex("workspace-1"))
        .join(hex("canvas-1"))
        .join("current.canvas")
}

fn revision_path(root: &Path, revision: u64) -> PathBuf {
    current_path(root)
        .parent()
        .expect("canvas root")
        .join("revisions")
        .join(format!("{revision:020}.canvas"))
}

fn viewport_manifest_path(root: &Path, revision: u64) -> PathBuf {
    current_path(root)
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
            "sponzey-durable-canvas-{label}-{}-{nonce}",
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
