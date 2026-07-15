use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_canvas_repository::DurableCanvasRepository;
use cabinet_desktop_shell::{DesktopCanvasRequestDto, DesktopCanvasRuntime};
use cabinet_domain::canvas::{
    Canvas, CanvasEdge, CanvasEdgeId, CanvasGeometry, CanvasGeometryPolicy, CanvasId,
    CanvasLifecycleState, CanvasNode, CanvasNodeId, CanvasNodeTarget, CanvasPosition,
    CanvasRevision, CanvasSize, CanvasTextCard, CanvasTitle, CanvasViewport,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository};

#[test]
fn durable_canvas_query_p95_is_below_300ms_for_2000_nodes_and_4000_edges() {
    let root = temp_root();
    let policy = CanvasGeometryPolicy::new(80, 1200, 60, 900, 25, 400).expect("policy");
    let nodes = (0..2_000)
        .map(|index| {
            CanvasNode::with_geometry(
                CanvasNodeId::new(&format!("node-{index}")).expect("node id"),
                CanvasNodeTarget::TextCard(
                    CanvasTextCard::new(&format!("Memo {index}")).expect("text"),
                ),
                CanvasGeometry::new(
                    CanvasPosition::new((index % 50) * 360, (index / 50) * 240),
                    CanvasSize::new(320, 180, &policy).expect("size"),
                ),
            )
            .expect("node")
        })
        .collect::<Vec<_>>();
    let edges = (0..4_000)
        .map(|index| {
            CanvasEdge::new(
                CanvasEdgeId::new(&format!("edge-{index}")).expect("edge id"),
                CanvasNodeId::new(&format!("node-{}", index % 2_000)).expect("source"),
                CanvasNodeId::new(&format!("node-{}", (index + 1) % 2_000)).expect("target"),
            )
            .expect("edge")
        })
        .collect::<Vec<_>>();
    let canvas = Canvas::new(
        CanvasId::new("canvas-performance").expect("canvas id"),
        nodes,
        edges,
        CanvasLifecycleState::Updated,
    )
    .expect("canvas");
    let record = CanvasRecord::with_metadata(
        canvas,
        CanvasTitle::new("Performance fixture").expect("title"),
        CanvasRevision::new(1).expect("revision"),
        CanvasViewport::new(600, 360, 100, &policy).expect("viewport"),
    );
    DurableCanvasRepository::new(root.clone())
        .create_canvas(&WorkspaceId::new("workspace-1").expect("workspace"), record)
        .expect("persist fixture");
    let runtime = DesktopCanvasRuntime::new(root.clone()).expect("runtime");
    let request = || DesktopCanvasRequestDto::GetViewport {
        workspace_id: "workspace-1".into(),
        canvas_id: "canvas-performance".into(),
        center_x: None,
        center_y: None,
        zoom_percent: None,
        surface_width: 1_200,
        surface_height: 720,
        overscan: 120,
        node_limit: 250,
        edge_limit: 500,
    };
    for _ in 0..5 {
        assert!(runtime.execute(request()).ok);
    }
    let mut samples = (0..30)
        .map(|_| {
            let started = Instant::now();
            let response = runtime.execute(request());
            assert!(response.ok);
            let data = response.data.expect("data");
            assert!(data.nodes.len() <= 250);
            assert!(data.edges.len() <= 500);
            assert_eq!(data.total_node_count, 2_000);
            assert_eq!(data.total_edge_count, 4_000);
            started.elapsed().as_secs_f64() * 1_000.0
        })
        .collect::<Vec<_>>();
    samples.sort_by(f64::total_cmp);
    let p95 = samples[(samples.len() * 95).div_ceil(100) - 1];
    println!("canvas_native_bounded_viewport_p95_ms={p95:.3}");
    assert!(p95 < 300.0, "durable Canvas query p95={p95:.3}ms");
    let _ = fs::remove_dir_all(root);
}

fn temp_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "sponzey-canvas-performance-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("root");
    path
}
