use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use cabinet_domain::canvas::{CanvasId, CanvasRevision};
use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{GraphEdge, GraphEdgeKind, GraphNode, GraphNodeKind};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_graph_projection::{
    CanvasGraphRelationProjectionBatch, CanvasGraphRelationProjectionError,
    CanvasGraphRelationProjectionReader, CanvasGraphRelationProjectionRecord,
    CanvasGraphRelationProjectionWriter,
};

use crate::local_atomic_file::write_text_atomically;

const SOURCE_SCHEMA: &str = "schema\t1";
const REFERENCE_SCHEMA: &str = "schema\t1";

#[derive(Debug, Clone)]
pub struct DurableCanvasGraphRelationProjectionStore {
    root: PathBuf,
}

impl DurableCanvasGraphRelationProjectionStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn workspace_root(&self, workspace: &WorkspaceId) -> PathBuf {
        self.root
            .join("canvas-graph-relations")
            .join(hex(workspace.as_str()))
    }

    fn source_path(&self, workspace: &WorkspaceId, canvas: &CanvasId) -> PathBuf {
        self.workspace_root(workspace)
            .join("sources")
            .join(format!("{}.snapshot", hex(canvas.as_str())))
    }

    fn document_index_root(&self, workspace: &WorkspaceId, document: &DocumentId) -> PathBuf {
        self.workspace_root(workspace)
            .join("by-document")
            .join(hex(document.as_str()))
    }

    fn reference_path(
        &self,
        workspace: &WorkspaceId,
        document: &DocumentId,
        canvas: &CanvasId,
    ) -> PathBuf {
        self.document_index_root(workspace, document)
            .join(format!("{}.ref", hex(canvas.as_str())))
    }

    fn read_source(
        &self,
        workspace: &WorkspaceId,
        canvas: &CanvasId,
    ) -> Result<Option<CanvasGraphRelationProjectionBatch>, CanvasGraphRelationProjectionError>
    {
        match fs::read_to_string(self.source_path(workspace, canvas)) {
            Ok(value) => decode_source(&value).map(Some),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(_) => Err(CanvasGraphRelationProjectionError::StorageUnavailable),
        }
    }
}

impl CanvasGraphRelationProjectionWriter for DurableCanvasGraphRelationProjectionStore {
    fn replace_canvas_relations(
        &mut self,
        workspace_id: &WorkspaceId,
        batch: CanvasGraphRelationProjectionBatch,
    ) -> Result<(), CanvasGraphRelationProjectionError> {
        let previous = self.read_source(workspace_id, batch.canvas_id())?;
        let previous_centers = previous.as_ref().map(center_ids).unwrap_or_default();
        let current_centers = center_ids(&batch);
        write_text_atomically(
            &self.source_path(workspace_id, batch.canvas_id()),
            encode_source(&batch),
        )
        .map_err(|_| CanvasGraphRelationProjectionError::StorageUnavailable)?;

        for center in previous_centers.union(&current_centers) {
            let document = DocumentId::new(center)
                .map_err(|_| CanvasGraphRelationProjectionError::CorruptedProjection)?;
            let reference = self.reference_path(workspace_id, &document, batch.canvas_id());
            if current_centers.contains(center) {
                write_text_atomically(&reference, encode_reference(batch.canvas_revision()))
                    .map_err(|_| CanvasGraphRelationProjectionError::StorageUnavailable)?;
            } else {
                match fs::remove_file(reference) {
                    Ok(()) => {}
                    Err(error) if error.kind() == ErrorKind::NotFound => {}
                    Err(_) => return Err(CanvasGraphRelationProjectionError::StorageUnavailable),
                }
            }
        }
        Ok(())
    }
}

impl CanvasGraphRelationProjectionReader for DurableCanvasGraphRelationProjectionStore {
    fn get_document_relations(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        source_limit: usize,
    ) -> Result<Vec<CanvasGraphRelationProjectionRecord>, CanvasGraphRelationProjectionError> {
        if source_limit == 0 {
            return Err(CanvasGraphRelationProjectionError::InvalidInput);
        }
        let entries = match fs::read_dir(self.document_index_root(workspace_id, document_id)) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(CanvasGraphRelationProjectionError::StorageUnavailable),
        };
        let mut paths = entries
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|_| CanvasGraphRelationProjectionError::StorageUnavailable)
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        if paths.len() > source_limit {
            return Err(CanvasGraphRelationProjectionError::RelationLimitExceeded);
        }
        let mut records = Vec::new();
        for path in paths {
            if path.extension().and_then(|value| value.to_str()) != Some("ref") {
                return Err(CanvasGraphRelationProjectionError::CorruptedProjection);
            }
            let canvas_id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or(CanvasGraphRelationProjectionError::CorruptedProjection)
                .and_then(unhex)
                .and_then(|value| {
                    CanvasId::new(&value)
                        .map_err(|_| CanvasGraphRelationProjectionError::CorruptedProjection)
                })?;
            let reference_revision = fs::read_to_string(&path)
                .map_err(|_| CanvasGraphRelationProjectionError::StorageUnavailable)
                .and_then(|value| decode_reference(&value))?;
            let Some(source) = self.read_source(workspace_id, &canvas_id)? else {
                continue;
            };
            if source.canvas_revision() != reference_revision {
                continue;
            }
            if let Some(record) = source
                .records()
                .iter()
                .find(|record| record.center_document_id() == document_id)
            {
                records.push(record.clone());
            }
        }
        Ok(records)
    }
}

fn center_ids(batch: &CanvasGraphRelationProjectionBatch) -> BTreeSet<String> {
    batch
        .records()
        .iter()
        .map(|record| record.center_document_id().as_str().to_string())
        .collect()
}

fn encode_source(batch: &CanvasGraphRelationProjectionBatch) -> String {
    let mut payload = vec![
        format!("canvas\t{}", hex(batch.canvas_id().as_str())),
        format!("revision\t{}", batch.canvas_revision().value()),
    ];
    for record in batch.records() {
        payload.push(format!(
            "record\t{}",
            hex(record.center_document_id().as_str())
        ));
        for node in record.nodes() {
            payload.push(format!(
                "node\t{}\t{}",
                node_kind(node.kind()),
                hex(node.id())
            ));
        }
        for edge in record.edges() {
            payload.push(format!(
                "edge\t{}\t{}\t{}\t{}",
                edge_kind(edge.kind()),
                hex(edge.id()),
                hex(edge.source_id()),
                hex(edge.target_id())
            ));
        }
        payload.push("end".to_string());
    }
    let payload = format!("{}\n", payload.join("\n"));
    format!(
        "{SOURCE_SCHEMA}\nchecksum\t{:016x}\n{payload}",
        checksum(payload.as_bytes())
    )
}

fn decode_source(
    value: &str,
) -> Result<CanvasGraphRelationProjectionBatch, CanvasGraphRelationProjectionError> {
    let mut lines = value.lines();
    if lines.next() != Some(SOURCE_SCHEMA) {
        return Err(CanvasGraphRelationProjectionError::CorruptedProjection);
    }
    let expected = lines
        .next()
        .and_then(|line| line.strip_prefix("checksum\t"))
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .ok_or(CanvasGraphRelationProjectionError::CorruptedProjection)?;
    let remaining = lines.collect::<Vec<_>>();
    let payload = format!("{}\n", remaining.join("\n"));
    if checksum(payload.as_bytes()) != expected {
        return Err(CanvasGraphRelationProjectionError::CorruptedProjection);
    }
    let mut canvas = None;
    let mut revision = None;
    let mut current_center = None;
    let mut current_nodes = Vec::new();
    let mut current_edges = Vec::new();
    let mut records = Vec::new();
    for line in remaining {
        let fields = line.split('\t').collect::<Vec<_>>();
        match fields.as_slice() {
            ["canvas", value] if canvas.is_none() && current_center.is_none() => {
                canvas = Some(
                    CanvasId::new(&unhex(value)?)
                        .map_err(|_| CanvasGraphRelationProjectionError::CorruptedProjection)?,
                );
            }
            ["revision", value] if revision.is_none() && current_center.is_none() => {
                revision = Some(
                    value
                        .parse::<u64>()
                        .ok()
                        .and_then(|value| CanvasRevision::new(value).ok())
                        .ok_or(CanvasGraphRelationProjectionError::CorruptedProjection)?,
                );
            }
            ["record", value] if current_center.is_none() => {
                current_center = Some(
                    DocumentId::new(&unhex(value)?)
                        .map_err(|_| CanvasGraphRelationProjectionError::CorruptedProjection)?,
                );
            }
            ["node", kind, id] if current_center.is_some() => {
                current_nodes.push(decode_node(kind, id)?);
            }
            ["edge", kind, id, source, target] if current_center.is_some() => {
                current_edges.push(decode_edge(kind, id, source, target)?);
            }
            ["end"] if current_center.is_some() => {
                records.push(CanvasGraphRelationProjectionRecord::new(
                    current_center.take().expect("guarded center"),
                    std::mem::take(&mut current_nodes),
                    std::mem::take(&mut current_edges),
                )?);
            }
            _ => return Err(CanvasGraphRelationProjectionError::CorruptedProjection),
        }
    }
    if current_center.is_some() {
        return Err(CanvasGraphRelationProjectionError::CorruptedProjection);
    }
    CanvasGraphRelationProjectionBatch::new(
        canvas.ok_or(CanvasGraphRelationProjectionError::CorruptedProjection)?,
        revision.ok_or(CanvasGraphRelationProjectionError::CorruptedProjection)?,
        records,
    )
}

fn encode_reference(revision: CanvasRevision) -> String {
    format!("{REFERENCE_SCHEMA}\nrevision\t{}\n", revision.value())
}

fn decode_reference(value: &str) -> Result<CanvasRevision, CanvasGraphRelationProjectionError> {
    let mut lines = value.lines();
    if lines.next() != Some(REFERENCE_SCHEMA) {
        return Err(CanvasGraphRelationProjectionError::CorruptedProjection);
    }
    let revision = lines
        .next()
        .and_then(|line| line.strip_prefix("revision\t"))
        .and_then(|value| value.parse::<u64>().ok())
        .and_then(|value| CanvasRevision::new(value).ok())
        .ok_or(CanvasGraphRelationProjectionError::CorruptedProjection)?;
    if lines.next().is_some() {
        return Err(CanvasGraphRelationProjectionError::CorruptedProjection);
    }
    Ok(revision)
}

fn decode_node(kind: &str, id: &str) -> Result<GraphNode, CanvasGraphRelationProjectionError> {
    let id = unhex(id)?;
    match kind {
        "document" => DocumentId::new(&id)
            .map(GraphNode::new_document)
            .map_err(|_| CanvasGraphRelationProjectionError::CorruptedProjection),
        "attachment" => GraphNode::new_attachment(&id)
            .map_err(|_| CanvasGraphRelationProjectionError::CorruptedProjection),
        "external_link" => GraphNode::new_external_link(&id)
            .map_err(|_| CanvasGraphRelationProjectionError::CorruptedProjection),
        "unresolved_link" => GraphNode::new_unresolved(&id)
            .map_err(|_| CanvasGraphRelationProjectionError::CorruptedProjection),
        _ => Err(CanvasGraphRelationProjectionError::CorruptedProjection),
    }
}

fn decode_edge(
    kind: &str,
    id: &str,
    source: &str,
    target: &str,
) -> Result<GraphEdge, CanvasGraphRelationProjectionError> {
    GraphEdge::new(
        &unhex(id)?,
        unhex(source)?,
        unhex(target)?,
        match kind {
            "canvas_relation" => GraphEdgeKind::CanvasRelation,
            _ => return Err(CanvasGraphRelationProjectionError::CorruptedProjection),
        },
    )
    .map_err(|_| CanvasGraphRelationProjectionError::CorruptedProjection)
}

const fn node_kind(kind: GraphNodeKind) -> &'static str {
    match kind {
        GraphNodeKind::Document => "document",
        GraphNodeKind::Attachment => "attachment",
        GraphNodeKind::ExternalLink => "external_link",
        GraphNodeKind::UnresolvedLink => "unresolved_link",
    }
}

const fn edge_kind(kind: GraphEdgeKind) -> &'static str {
    match kind {
        GraphEdgeKind::CanvasRelation => "canvas_relation",
        _ => "invalid",
    }
}

fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn unhex(value: &str) -> Result<String, CanvasGraphRelationProjectionError> {
    if value.len() % 2 != 0 {
        return Err(CanvasGraphRelationProjectionError::CorruptedProjection);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            std::str::from_utf8(pair)
                .ok()
                .and_then(|text| u8::from_str_radix(text, 16).ok())
                .ok_or(CanvasGraphRelationProjectionError::CorruptedProjection)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| CanvasGraphRelationProjectionError::CorruptedProjection)
}
