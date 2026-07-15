use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphNodeKind, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::graph_projection::{
    GraphProjectionError, GraphProjectionRecord, GraphProjectionStore,
    WorkspaceGraphProjectionPage, WorkspaceGraphProjectionReader,
};

use crate::local_atomic_file::write_text_atomically;

const SCHEMA_HEADER: &str = "schema\t1";

#[derive(Debug, Clone)]
pub struct DurableLocalGraphProjectionStore {
    root: PathBuf,
    cache: Arc<RwLock<HashMap<(String, String), CachedProjection>>>,
}

#[derive(Debug, Clone)]
struct CachedProjection {
    stamp: FileStamp,
    record: GraphProjectionRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    length: u64,
    modified: Option<SystemTime>,
}

impl WorkspaceGraphProjectionReader for DurableLocalGraphProjectionStore {
    fn list_workspace_projections(
        &self,
        workspace_id: &WorkspaceId,
        after_center_id: Option<&str>,
        limit: usize,
    ) -> Result<WorkspaceGraphProjectionPage, GraphProjectionError> {
        if limit == 0 {
            return Err(GraphProjectionError::InvalidInput);
        }
        let root = self
            .root
            .join("graph-projections")
            .join(hex_encode(workspace_id.as_str()));
        let entries = match fs::read_dir(root) {
            Ok(value) => value,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(WorkspaceGraphProjectionPage::new(vec![], None));
            }
            Err(_) => return Err(GraphProjectionError::StorageUnavailable),
        };
        let mut paths = entries
            .map(|entry| {
                entry
                    .map(|value| value.path())
                    .map_err(|_| GraphProjectionError::StorageUnavailable)
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        let mut records = Vec::new();
        let mut has_more = false;
        for path in paths {
            if path.extension().and_then(|value| value.to_str()) != Some("snapshot") {
                continue;
            }
            let center_id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or(GraphProjectionError::CorruptedProjection)
                .and_then(hex_decode)?;
            if after_center_id.is_some_and(|cursor| center_id.as_str() <= cursor) {
                continue;
            }
            if records.len() == limit {
                has_more = true;
                break;
            }
            records.push(self.read_cached_projection(workspace_id, &center_id, &path)?);
        }
        let next_cursor = has_more.then(|| {
            records
                .last()
                .expect("bounded page has record")
                .graph()
                .center_document_id()
                .as_str()
                .to_string()
        });
        Ok(WorkspaceGraphProjectionPage::new(records, next_cursor))
    }
}

impl DurableLocalGraphProjectionStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn projection_path(
        &self,
        workspace_id: &WorkspaceId,
        center_document_id: &DocumentId,
    ) -> PathBuf {
        self.root
            .join("graph-projections")
            .join(hex_encode(workspace_id.as_str()))
            .join(format!(
                "{}.snapshot",
                hex_encode(center_document_id.as_str())
            ))
    }

    fn read_cached_projection(
        &self,
        workspace_id: &WorkspaceId,
        center_id: &str,
        path: &PathBuf,
    ) -> Result<GraphProjectionRecord, GraphProjectionError> {
        let metadata = fs::metadata(path).map_err(|_| GraphProjectionError::StorageUnavailable)?;
        let stamp = FileStamp {
            length: metadata.len(),
            modified: metadata.modified().ok(),
        };
        let key = (workspace_id.as_str().to_string(), center_id.to_string());
        if let Some(record) = self
            .cache
            .read()
            .map_err(|_| GraphProjectionError::StorageUnavailable)?
            .get(&key)
            .filter(|entry| entry.stamp == stamp)
            .map(|entry| entry.record.clone())
        {
            return Ok(record);
        }
        let text =
            fs::read_to_string(path).map_err(|_| GraphProjectionError::StorageUnavailable)?;
        let record = decode_record(&text)?;
        if record.graph().center_document_id().as_str() != center_id {
            return Err(GraphProjectionError::CorruptedProjection);
        }
        self.cache
            .write()
            .map_err(|_| GraphProjectionError::StorageUnavailable)?
            .insert(
                key,
                CachedProjection {
                    stamp,
                    record: record.clone(),
                },
            );
        Ok(record)
    }

    fn invalidate(&self, workspace_id: &WorkspaceId, center_document_id: &DocumentId) {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(&(
                workspace_id.as_str().to_string(),
                center_document_id.as_str().to_string(),
            ));
        }
    }
}

impl GraphProjectionStore for DurableLocalGraphProjectionStore {
    fn replace_projection(
        &mut self,
        workspace_id: &WorkspaceId,
        record: GraphProjectionRecord,
    ) -> Result<(), GraphProjectionError> {
        let path = self.projection_path(workspace_id, record.graph().center_document_id());
        let center_document_id = record.graph().center_document_id().clone();
        let result = write_text_atomically(&path, encode_record(&record))
            .map(|_| ())
            .map_err(|_| GraphProjectionError::StorageUnavailable);
        if result.is_ok() {
            self.invalidate(workspace_id, &center_document_id);
        }
        result
    }

    fn delete_projection(
        &mut self,
        workspace_id: &WorkspaceId,
        center_document_id: &DocumentId,
    ) -> Result<(), GraphProjectionError> {
        let result = match fs::remove_file(self.projection_path(workspace_id, center_document_id)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(_) => Err(GraphProjectionError::StorageUnavailable),
        };
        if result.is_ok() {
            self.invalidate(workspace_id, center_document_id);
        }
        result
    }

    fn get_projection(
        &self,
        workspace_id: &WorkspaceId,
        center_document_id: &DocumentId,
    ) -> Result<Option<GraphProjectionRecord>, GraphProjectionError> {
        let path = self.projection_path(workspace_id, center_document_id);
        match fs::metadata(&path) {
            Ok(_) => self
                .read_cached_projection(workspace_id, center_document_id.as_str(), &path)
                .map(Some),
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(GraphProjectionError::StorageUnavailable),
        }
    }
}

fn encode_record(record: &GraphProjectionRecord) -> String {
    let graph = record.graph();
    let mut payload = vec![
        format!("revision\t{}", hex_encode(record.freshness_revision())),
        format!(
            "center\t{}",
            hex_encode(graph.center_document_id().as_str())
        ),
        format!("status\t{}", encode_status(graph.status())),
    ];
    payload.extend(graph.nodes().iter().map(|node| {
        format!(
            "node\t{}\t{}",
            encode_node_kind(node.kind()),
            hex_encode(node.id())
        )
    }));
    payload.extend(graph.edges().iter().map(|edge| {
        format!(
            "edge\t{}\t{}\t{}\t{}",
            encode_edge_kind(edge.kind()),
            hex_encode(edge.id()),
            hex_encode(edge.source_id()),
            hex_encode(edge.target_id())
        )
    }));
    let payload = format!("{}\n", payload.join("\n"));
    format!(
        "{SCHEMA_HEADER}\nchecksum\t{:016x}\n{payload}",
        checksum(payload.as_bytes())
    )
}

fn decode_record(text: &str) -> Result<GraphProjectionRecord, GraphProjectionError> {
    let mut lines = text.lines();
    if lines.next() != Some(SCHEMA_HEADER) {
        return Err(GraphProjectionError::CorruptedProjection);
    }
    let checksum_line = lines
        .next()
        .ok_or(GraphProjectionError::CorruptedProjection)?;
    let expected_checksum = checksum_line
        .strip_prefix("checksum\t")
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .ok_or(GraphProjectionError::CorruptedProjection)?;
    let payload = format!("{}\n", lines.collect::<Vec<_>>().join("\n"));
    if checksum(payload.as_bytes()) != expected_checksum {
        return Err(GraphProjectionError::CorruptedProjection);
    }

    let mut revision = None;
    let mut center = None;
    let mut status = None;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for line in payload.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        match fields.as_slice() {
            ["revision", value] if revision.is_none() => revision = Some(hex_decode(value)?),
            ["center", value] if center.is_none() => center = Some(decode_document_id(value)?),
            ["status", value] if status.is_none() => status = Some(decode_status(value)?),
            ["node", kind, id] => nodes.push(decode_node(kind, id)?),
            ["edge", kind, id, source, target] => {
                edges.push(decode_edge(kind, id, source, target)?)
            }
            _ => return Err(GraphProjectionError::CorruptedProjection),
        }
    }

    let center = center.ok_or(GraphProjectionError::CorruptedProjection)?;
    let graph = KnowledgeGraph::new_with_center(
        center,
        nodes,
        edges,
        status.ok_or(GraphProjectionError::CorruptedProjection)?,
    )
    .map_err(|_| GraphProjectionError::CorruptedProjection)?;
    GraphProjectionRecord::new_with_revision(
        graph,
        &revision.ok_or(GraphProjectionError::CorruptedProjection)?,
    )
    .map_err(|_| GraphProjectionError::CorruptedProjection)
}

fn decode_node(kind: &str, id: &str) -> Result<GraphNode, GraphProjectionError> {
    let id = hex_decode(id)?;
    match kind {
        "document" => Ok(GraphNode::new_document(
            DocumentId::new(&id).map_err(|_| GraphProjectionError::CorruptedProjection)?,
        )),
        "unresolved_link" => GraphNode::new_unresolved(&id),
        "attachment" => GraphNode::new_attachment(&id),
        "external_link" => GraphNode::new_external_link(&id),
        _ => return Err(GraphProjectionError::CorruptedProjection),
    }
    .map_err(|_| GraphProjectionError::CorruptedProjection)
}

fn decode_edge(
    kind: &str,
    id: &str,
    source: &str,
    target: &str,
) -> Result<GraphEdge, GraphProjectionError> {
    GraphEdge::new(
        &hex_decode(id)?,
        hex_decode(source)?,
        hex_decode(target)?,
        decode_edge_kind(kind)?,
    )
    .map_err(|_| GraphProjectionError::CorruptedProjection)
}

const fn encode_node_kind(kind: GraphNodeKind) -> &'static str {
    match kind {
        GraphNodeKind::Document => "document",
        GraphNodeKind::UnresolvedLink => "unresolved_link",
        GraphNodeKind::Attachment => "attachment",
        GraphNodeKind::ExternalLink => "external_link",
    }
}

const fn encode_edge_kind(kind: GraphEdgeKind) -> &'static str {
    match kind {
        GraphEdgeKind::DocumentLink => "document_link",
        GraphEdgeKind::AttachmentReference => "attachment_reference",
        GraphEdgeKind::ExternalReference => "external_reference",
        GraphEdgeKind::CanvasRelation => "canvas_relation",
    }
}

fn decode_edge_kind(value: &str) -> Result<GraphEdgeKind, GraphProjectionError> {
    match value {
        "document_link" => Ok(GraphEdgeKind::DocumentLink),
        "attachment_reference" => Ok(GraphEdgeKind::AttachmentReference),
        "external_reference" => Ok(GraphEdgeKind::ExternalReference),
        "canvas_relation" => Ok(GraphEdgeKind::CanvasRelation),
        _ => Err(GraphProjectionError::CorruptedProjection),
    }
}

const fn encode_status(status: GraphProjectionStatus) -> &'static str {
    match status {
        GraphProjectionStatus::Clean => "clean",
        GraphProjectionStatus::ReindexRequested => "reindex_requested",
        GraphProjectionStatus::Reindexing => "reindexing",
        GraphProjectionStatus::Degraded => "degraded",
    }
}

fn decode_status(value: &str) -> Result<GraphProjectionStatus, GraphProjectionError> {
    match value {
        "clean" => Ok(GraphProjectionStatus::Clean),
        "reindex_requested" => Ok(GraphProjectionStatus::ReindexRequested),
        "reindexing" => Ok(GraphProjectionStatus::Reindexing),
        "degraded" => Ok(GraphProjectionStatus::Degraded),
        _ => Err(GraphProjectionError::CorruptedProjection),
    }
}

fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, GraphProjectionError> {
    if value.len() % 2 != 0 {
        return Err(GraphProjectionError::CorruptedProjection);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text =
                std::str::from_utf8(pair).map_err(|_| GraphProjectionError::CorruptedProjection)?;
            u8::from_str_radix(text, 16).map_err(|_| GraphProjectionError::CorruptedProjection)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| GraphProjectionError::CorruptedProjection)
}

fn decode_document_id(value: &str) -> Result<DocumentId, GraphProjectionError> {
    DocumentId::new(&hex_decode(value)?).map_err(|_| GraphProjectionError::CorruptedProjection)
}
