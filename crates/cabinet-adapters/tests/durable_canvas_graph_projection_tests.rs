use cabinet_adapters::durable_canvas_graph_projection::DurableCanvasGraphRelationProjectionStore;
use cabinet_domain::canvas::{CanvasId, CanvasRevision};
use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{GraphEdge, GraphEdgeKind, GraphNode};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_graph_projection::{
    CanvasGraphRelationProjectionBatch, CanvasGraphRelationProjectionReader,
    CanvasGraphRelationProjectionRecord, CanvasGraphRelationProjectionWriter,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn durable_store_replaces_reads_and_removes_canvas_relations_across_restart() {
    let temp = Temp::new("replace");
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let document = DocumentId::new("doc-a").unwrap();
    let mut store = DurableCanvasGraphRelationProjectionStore::new(temp.path.clone());
    store
        .replace_canvas_relations(&workspace, batch(1, relation_record("doc-a", "doc-b")))
        .unwrap();
    drop(store);

    let restarted = DurableCanvasGraphRelationProjectionStore::new(temp.path.clone());
    let relations = restarted
        .get_document_relations(&workspace, &document, 10)
        .unwrap();
    assert_eq!(relations.len(), 1);
    assert_eq!(relations[0].edges().len(), 1);
    drop(restarted);

    let mut store = DurableCanvasGraphRelationProjectionStore::new(temp.path.clone());
    store
        .replace_canvas_relations(
            &workspace,
            CanvasGraphRelationProjectionBatch::new(
                CanvasId::new("canvas-1").unwrap(),
                CanvasRevision::new(2).unwrap(),
                vec![],
            )
            .unwrap(),
        )
        .unwrap();
    assert!(
        store
            .get_document_relations(&workspace, &document, 10)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn durable_reader_ignores_stale_document_reference_after_empty_source_replace() {
    let temp = Temp::new("stale-ref");
    let workspace = WorkspaceId::new("workspace-1").unwrap();
    let document = DocumentId::new("doc-a").unwrap();
    let mut store = DurableCanvasGraphRelationProjectionStore::new(temp.path.clone());
    store
        .replace_canvas_relations(&workspace, batch(1, relation_record("doc-a", "doc-b")))
        .unwrap();
    let reference = find_file(&temp.path, ".ref");
    let stale_content = fs::read_to_string(&reference).unwrap();
    store
        .replace_canvas_relations(
            &workspace,
            CanvasGraphRelationProjectionBatch::new(
                CanvasId::new("canvas-1").unwrap(),
                CanvasRevision::new(2).unwrap(),
                vec![],
            )
            .unwrap(),
        )
        .unwrap();
    fs::create_dir_all(reference.parent().unwrap()).unwrap();
    fs::write(reference, stale_content).unwrap();

    assert!(
        DurableCanvasGraphRelationProjectionStore::new(temp.path.clone())
            .get_document_relations(&workspace, &document, 10)
            .unwrap()
            .is_empty()
    );
}

fn batch(
    revision: u64,
    record: CanvasGraphRelationProjectionRecord,
) -> CanvasGraphRelationProjectionBatch {
    CanvasGraphRelationProjectionBatch::new(
        CanvasId::new("canvas-1").unwrap(),
        CanvasRevision::new(revision).unwrap(),
        vec![record],
    )
    .unwrap()
}

fn relation_record(center: &str, target: &str) -> CanvasGraphRelationProjectionRecord {
    let center_id = DocumentId::new(center).unwrap();
    let source = GraphNode::new_document(center_id.clone());
    let target = GraphNode::new_document(DocumentId::new(target).unwrap());
    let edge = GraphEdge::new(
        "canvas:canvas-1:edge-1",
        source.id().into(),
        target.id().into(),
        GraphEdgeKind::CanvasRelation,
    )
    .unwrap();
    CanvasGraphRelationProjectionRecord::new(center_id, vec![source, target], vec![edge]).unwrap()
}

fn find_file(root: &std::path::Path, suffix: &str) -> PathBuf {
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                pending.push(entry.path());
            } else if entry.file_name().to_string_lossy().ends_with(suffix) {
                return entry.path();
            }
        }
    }
    panic!("file ending with {suffix} not found")
}

struct Temp {
    path: PathBuf,
}

impl Temp {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cabinet-canvas-graph-{label}-{}-{nonce}",
            std::process::id()
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
