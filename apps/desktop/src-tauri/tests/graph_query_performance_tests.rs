use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cabinet_adapters::durable_local_graph_projection::DurableLocalGraphProjectionStore;
use cabinet_desktop_shell::{
    DesktopKnowledgeGraphRuntime, DesktopLocalCommandPayloadDto, DesktopLocalCommandRequestDto,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::graph_projection::{GraphProjectionRecord, GraphProjectionStore};

const NODE_COUNT: usize = 120;
const SAMPLE_COUNT: usize = 100;

#[test]
fn bounded_durable_graph_query_p95_is_below_300ms() {
    let temp = Temp::new();
    seed(&temp.path);
    let runtime = DesktopKnowledgeGraphRuntime::new(temp.path.clone());
    for _ in 0..5 {
        assert!(runtime.execute(request()).ok);
    }
    let mut samples = (0..SAMPLE_COUNT)
        .map(|_| {
            let started = Instant::now();
            let response = runtime.execute(request());
            assert!(response.ok);
            started.elapsed()
        })
        .collect::<Vec<_>>();
    samples.sort();
    let p95 = samples[(SAMPLE_COUNT * 95 / 100) - 1];
    eprintln!(
        "graph_fixture_nodes={NODE_COUNT} graph_fixture_edges={} samples={SAMPLE_COUNT} p95_ms={:.3}",
        NODE_COUNT - 1,
        p95.as_secs_f64() * 1000.0
    );
    assert!(
        p95 < Duration::from_millis(300),
        "graph p95 {:?} exceeded 300ms",
        p95
    );
}

fn request() -> DesktopLocalCommandRequestDto {
    DesktopLocalCommandRequestDto {
        command_name: "get_graph_projection".into(),
        payload: DesktopLocalCommandPayloadDto::GraphProjection {
            workspace_id: "workspace-1".into(),
            document_id: "doc-0".into(),
            depth: 1,
            direction: "both".into(),
            include_unresolved: true,
            include_assets: false,
            node_limit: 120,
            edge_limit: 240,
        },
    }
}
fn seed(root: &PathBuf) {
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let center_id = DocumentId::new("doc-0").unwrap();
    let nodes = (0..NODE_COUNT)
        .map(|index| GraphNode::new_document(DocumentId::new(&format!("doc-{index}")).unwrap()))
        .collect::<Vec<_>>();
    let edges = (1..NODE_COUNT)
        .map(|index| {
            GraphEdge::new(
                &format!("edge-{index}"),
                "doc-0".to_string(),
                format!("doc-{index}"),
                GraphEdgeKind::DocumentLink,
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let graph =
        KnowledgeGraph::new_with_center(center_id, nodes, edges, GraphProjectionStatus::Clean)
            .unwrap();
    DurableLocalGraphProjectionStore::new(root.clone())
        .replace_projection(
            &workspace,
            GraphProjectionRecord::new_with_revision(graph, "version-perf").unwrap(),
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
        let path =
            std::env::temp_dir().join(format!("cabinet-graph-perf-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
