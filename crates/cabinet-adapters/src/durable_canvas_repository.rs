use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use cabinet_domain::asset::AssetId;
use cabinet_domain::canvas::{
    Canvas, CanvasEdge, CanvasEdgeId, CanvasExternalLink, CanvasGeometry, CanvasGeometryPolicy,
    CanvasId, CanvasLifecycleState, CanvasNode, CanvasNodeId, CanvasNodeTarget, CanvasPosition,
    CanvasRevision, CanvasSize, CanvasTextCard, CanvasTitle, CanvasViewport,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_catalog::{CanvasCatalogEntry, CanvasCatalogError, CanvasCatalogPort};
use cabinet_ports::canvas_recovery::{CanvasRecoveryRepository, CanvasRecoveryRepositoryError};
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository, CanvasRepositoryError};
use cabinet_ports::canvas_viewport_query::{
    CanvasViewportPage, CanvasViewportQuery, CanvasViewportQueryError, CanvasViewportQueryPort,
};

use crate::local_atomic_file::write_text_atomically;

const SCHEMA: &str = "schema\t1";
const POINTER_SCHEMA: &str = "schema\t2";
const VIEWPORT_SCHEMA: &str = "schema\t1";
const TILE_SIZE: i32 = 1_024;

#[derive(Debug, Clone)]
pub struct DurableCanvasRepository {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredCurrentCanvasRecord {
    workspace_id: WorkspaceId,
    record: CanvasRecord,
}

impl DiscoveredCurrentCanvasRecord {
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn record(&self) -> &CanvasRecord {
        &self.record
    }
}

impl DurableCanvasRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn list_current_canvas_records(
        &self,
        limit: usize,
    ) -> Result<Vec<DiscoveredCurrentCanvasRecord>, CanvasRepositoryError> {
        if limit == 0 {
            return Err(CanvasRepositoryError::InvalidInput);
        }
        let root = self.root.join("canvases");
        let mut workspace_paths = match sorted_directories(&root) {
            Ok(paths) => paths,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(CanvasRepositoryError::StorageUnavailable),
        };
        workspace_paths.sort();

        let mut discovered = Vec::new();
        for workspace_path in workspace_paths {
            let workspace = decode_workspace_directory(&workspace_path)?;
            let mut canvas_paths = sorted_directories(&workspace_path)
                .map_err(|_| CanvasRepositoryError::StorageUnavailable)?;
            canvas_paths.sort();
            for canvas_path in canvas_paths {
                if discovered.len() >= limit {
                    return Err(CanvasRepositoryError::InvalidInput);
                }
                let canvas = decode_canvas_directory(&canvas_path)?;
                let record = self
                    .read_current(&workspace, &canvas)?
                    .ok_or(CanvasRepositoryError::CorruptedCanvas)?;
                if record.canvas().id() != &canvas {
                    return Err(CanvasRepositoryError::CorruptedCanvas);
                }
                discovered.push(DiscoveredCurrentCanvasRecord {
                    workspace_id: workspace.clone(),
                    record,
                });
            }
        }
        Ok(discovered)
    }

    fn canvas_root(&self, workspace: &WorkspaceId, canvas: &CanvasId) -> PathBuf {
        self.root
            .join("canvases")
            .join(hex(workspace.as_str()))
            .join(hex(canvas.as_str()))
    }

    fn current_path(&self, workspace: &WorkspaceId, canvas: &CanvasId) -> PathBuf {
        self.canvas_root(workspace, canvas).join("current.canvas")
    }

    fn revision_path(
        &self,
        workspace: &WorkspaceId,
        canvas: &CanvasId,
        revision: CanvasRevision,
    ) -> PathBuf {
        self.canvas_root(workspace, canvas)
            .join("revisions")
            .join(format!("{:020}.canvas", revision.value()))
    }

    fn write_revision(
        &self,
        workspace: &WorkspaceId,
        record: &CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        let path = self.revision_path(workspace, record.canvas().id(), record.revision());
        let encoded = encode(record);
        if path.exists() {
            let current = read(&path)?;
            return if current == *record {
                Ok(())
            } else {
                Err(CanvasRepositoryError::VersionConflict)
            };
        }
        write_text_atomically(&path, &encoded)
            .map_err(|_| CanvasRepositoryError::StorageUnavailable)?;
        Ok(())
    }

    fn write_viewport_projection(
        &self,
        workspace: &WorkspaceId,
        record: &CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        let root = self
            .canvas_root(workspace, record.canvas().id())
            .join("viewport")
            .join("revisions")
            .join(format!("{:020}", record.revision().value()));
        let mut tiles = BTreeMap::<(i32, i32), TileProjection>::new();
        let mut node_tiles = BTreeMap::<String, BTreeSet<(i32, i32)>>::new();
        for node in record.canvas().nodes() {
            let geometry = node.geometry();
            let left = geometry.position().x().div_euclid(TILE_SIZE);
            let right =
                (geometry.position().x() + geometry.size().width() as i32).div_euclid(TILE_SIZE);
            let top = geometry.position().y().div_euclid(TILE_SIZE);
            let bottom =
                (geometry.position().y() + geometry.size().height() as i32).div_euclid(TILE_SIZE);
            for tile_x in left..=right {
                for tile_y in top..=bottom {
                    tiles
                        .entry((tile_x, tile_y))
                        .or_default()
                        .nodes
                        .push(node.clone());
                    node_tiles
                        .entry(node.id().as_str().into())
                        .or_default()
                        .insert((tile_x, tile_y));
                }
            }
        }
        for edge in record.canvas().edges() {
            let mut occupied = BTreeSet::new();
            if let Some(values) = node_tiles.get(edge.source_node_id().as_str()) {
                occupied.extend(values.iter().copied());
            }
            if let Some(values) = node_tiles.get(edge.target_node_id().as_str()) {
                occupied.extend(values.iter().copied());
            }
            for tile in occupied {
                tiles.entry(tile).or_default().edges.push(edge.clone());
            }
        }
        for ((tile_x, tile_y), tile) in tiles {
            write_text_atomically(
                &root
                    .join("tiles")
                    .join(format!("{tile_x}_{tile_y}.viewport")),
                encode_tile(&tile),
            )
            .map_err(|_| CanvasRepositoryError::StorageUnavailable)?;
        }
        write_text_atomically(&root.join("manifest.viewport"), encode_manifest(record))
            .map(|_| ())
            .map_err(|_| CanvasRepositoryError::StorageUnavailable)
    }

    fn read_current(
        &self,
        workspace: &WorkspaceId,
        canvas: &CanvasId,
    ) -> Result<Option<CanvasRecord>, CanvasRepositoryError> {
        let text = match fs::read_to_string(self.current_path(workspace, canvas)) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(CanvasRepositoryError::StorageUnavailable),
        };
        if text.starts_with(SCHEMA) {
            return decode(&text).map(Some);
        }
        let revision = decode_pointer(&text)?;
        read(&self.revision_path(workspace, canvas, revision)).map(Some)
    }
}

impl CanvasCatalogPort for DurableCanvasRepository {
    fn list_canvas_entries(
        &self,
        workspace_id: &WorkspaceId,
        limit: usize,
        include_archived: bool,
    ) -> Result<Vec<CanvasCatalogEntry>, CanvasCatalogError> {
        if limit == 0 {
            return Err(CanvasCatalogError::InvalidLimit);
        }
        let workspace_root = self.root.join("canvases").join(hex(workspace_id.as_str()));
        let paths = match sorted_directories(&workspace_root) {
            Ok(paths) => paths,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) if error.kind() == ErrorKind::InvalidData => {
                return Err(CanvasCatalogError::CorruptedCatalog);
            }
            Err(_) => return Err(CanvasCatalogError::StorageUnavailable),
        };
        let mut entries = Vec::new();
        for path in paths {
            let canvas_id =
                decode_canvas_directory(&path).map_err(|_| CanvasCatalogError::CorruptedCatalog)?;
            let record = self
                .read_current(workspace_id, &canvas_id)
                .map_err(map_canvas_catalog_error)?
                .ok_or(CanvasCatalogError::CorruptedCatalog)?;
            if !include_archived && record.canvas().state() == CanvasLifecycleState::Archived {
                continue;
            }
            if entries.len() == limit {
                return Err(CanvasCatalogError::LimitExceeded);
            }
            entries.push(CanvasCatalogEntry::new(
                record.canvas().id().clone(),
                record.title().clone(),
                record.canvas().state(),
                record.revision(),
            ));
        }
        Ok(entries)
    }
}

const fn map_canvas_catalog_error(error: CanvasRepositoryError) -> CanvasCatalogError {
    match error {
        CanvasRepositoryError::StorageUnavailable => CanvasCatalogError::StorageUnavailable,
        CanvasRepositoryError::InvalidInput => CanvasCatalogError::InvalidLimit,
        CanvasRepositoryError::CorruptedCanvas
        | CanvasRepositoryError::UnsupportedSchema
        | CanvasRepositoryError::AlreadyExists
        | CanvasRepositoryError::VersionConflict => CanvasCatalogError::CorruptedCatalog,
    }
}

fn sorted_directories(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "Canvas catalog contains a non-directory entry",
            ));
        }
        paths.push(entry.path());
    }
    Ok(paths)
}

fn decode_workspace_directory(path: &Path) -> Result<WorkspaceId, CanvasRepositoryError> {
    let encoded = encoded_directory_name(path)?;
    let decoded = unhex(encoded)?;
    if hex(&decoded) != encoded {
        return Err(CanvasRepositoryError::CorruptedCanvas);
    }
    WorkspaceId::new(&decoded).map_err(|_| CanvasRepositoryError::CorruptedCanvas)
}

fn decode_canvas_directory(path: &Path) -> Result<CanvasId, CanvasRepositoryError> {
    let encoded = encoded_directory_name(path)?;
    let decoded = unhex(encoded)?;
    if hex(&decoded) != encoded {
        return Err(CanvasRepositoryError::CorruptedCanvas);
    }
    CanvasId::new(&decoded).map_err(|_| CanvasRepositoryError::CorruptedCanvas)
}

fn encoded_directory_name(path: &Path) -> Result<&str, CanvasRepositoryError> {
    path.file_name()
        .and_then(OsStr::to_str)
        .ok_or(CanvasRepositoryError::CorruptedCanvas)
}

impl CanvasRepository for DurableCanvasRepository {
    fn create_canvas(
        &mut self,
        workspace: &WorkspaceId,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        let current = self.current_path(workspace, record.canvas().id());
        if current.exists() {
            return Err(CanvasRepositoryError::AlreadyExists);
        }
        if record.revision().value() != 1 {
            return Err(CanvasRepositoryError::InvalidInput);
        }
        self.write_revision(workspace, &record)?;
        self.write_viewport_projection(workspace, &record)?;
        write_text_atomically(&current, encode_pointer(record.revision()))
            .map_err(|_| CanvasRepositoryError::StorageUnavailable)?;
        Ok(())
    }

    fn replace_canvas(
        &mut self,
        workspace: &WorkspaceId,
        expected_revision: CanvasRevision,
        record: CanvasRecord,
    ) -> Result<(), CanvasRepositoryError> {
        let current_path = self.current_path(workspace, record.canvas().id());
        let current = self
            .read_current(workspace, record.canvas().id())?
            .ok_or(CanvasRepositoryError::VersionConflict)?;
        if current.revision() != expected_revision
            || record.revision().value()
                != expected_revision
                    .value()
                    .checked_add(1)
                    .ok_or(CanvasRepositoryError::InvalidInput)?
        {
            return Err(CanvasRepositoryError::VersionConflict);
        }
        self.write_revision(workspace, &record)?;
        self.write_viewport_projection(workspace, &record)?;
        write_text_atomically(&current_path, encode_pointer(record.revision()))
            .map_err(|_| CanvasRepositoryError::StorageUnavailable)?;
        Ok(())
    }

    fn get_canvas(
        &self,
        workspace: &WorkspaceId,
        canvas: &CanvasId,
    ) -> Result<Option<CanvasRecord>, CanvasRepositoryError> {
        let record = self.read_current(workspace, canvas)?;
        if record
            .as_ref()
            .is_some_and(|record| record.canvas().id() != canvas)
        {
            return Err(CanvasRepositoryError::CorruptedCanvas);
        }
        Ok(record)
    }
}

impl CanvasRecoveryRepository for DurableCanvasRepository {
    fn list_valid_revisions(
        &mut self,
        workspace_id: &WorkspaceId,
        canvas_id: &CanvasId,
        limit: usize,
    ) -> Result<Vec<CanvasRevision>, CanvasRecoveryRepositoryError> {
        if limit == 0 {
            return Err(CanvasRecoveryRepositoryError::InvalidInput);
        }
        let root = self.canvas_root(workspace_id, canvas_id).join("revisions");
        let metadata = match fs::symlink_metadata(&root) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(CanvasRecoveryRepositoryError::StorageUnavailable),
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(CanvasRecoveryRepositoryError::CorruptedCatalog);
        }
        let mut revisions = Vec::new();
        let mut scanned = 0_usize;
        for entry in
            fs::read_dir(root).map_err(|_| CanvasRecoveryRepositoryError::StorageUnavailable)?
        {
            let entry = entry.map_err(|_| CanvasRecoveryRepositoryError::StorageUnavailable)?;
            let file_type = entry
                .file_type()
                .map_err(|_| CanvasRecoveryRepositoryError::StorageUnavailable)?;
            if file_type.is_symlink() || !file_type.is_file() {
                return Err(CanvasRecoveryRepositoryError::CorruptedCatalog);
            }
            scanned += 1;
            if scanned > limit {
                return Err(CanvasRecoveryRepositoryError::CandidateLimitExceeded);
            }
            let revision = parse_revision_file_name(&entry.file_name())?;
            match read(&entry.path()) {
                Ok(record) => {
                    if record.canvas().id() != canvas_id || record.revision() != revision {
                        return Err(CanvasRecoveryRepositoryError::CorruptedCatalog);
                    }
                    revisions.push(revision);
                }
                Err(
                    CanvasRepositoryError::CorruptedCanvas
                    | CanvasRepositoryError::UnsupportedSchema,
                ) => {}
                Err(CanvasRepositoryError::StorageUnavailable) => {
                    return Err(CanvasRecoveryRepositoryError::StorageUnavailable);
                }
                Err(_) => return Err(CanvasRecoveryRepositoryError::CorruptedCatalog),
            }
        }
        Ok(revisions)
    }

    fn activate_revision(
        &mut self,
        workspace_id: &WorkspaceId,
        canvas_id: &CanvasId,
        revision: CanvasRevision,
    ) -> Result<(), CanvasRecoveryRepositoryError> {
        let record =
            read(&self.revision_path(workspace_id, canvas_id, revision)).map_err(|error| {
                match error {
                    CanvasRepositoryError::StorageUnavailable => {
                        CanvasRecoveryRepositoryError::StorageUnavailable
                    }
                    _ => CanvasRecoveryRepositoryError::RevisionNotFound,
                }
            })?;
        if record.canvas().id() != canvas_id || record.revision() != revision {
            return Err(CanvasRecoveryRepositoryError::CorruptedCatalog);
        }
        self.write_viewport_projection(workspace_id, &record)
            .map_err(|error| match error {
                CanvasRepositoryError::StorageUnavailable => {
                    CanvasRecoveryRepositoryError::StorageUnavailable
                }
                _ => CanvasRecoveryRepositoryError::CorruptedCatalog,
            })?;
        write_text_atomically(
            &self.current_path(workspace_id, canvas_id),
            encode_pointer(revision),
        )
        .map(|_| ())
        .map_err(|_| CanvasRecoveryRepositoryError::StorageUnavailable)
    }
}

impl CanvasViewportQueryPort for DurableCanvasRepository {
    fn query_viewport(
        &self,
        workspace: &WorkspaceId,
        canvas: &CanvasId,
        query: CanvasViewportQuery,
    ) -> Result<Option<CanvasViewportPage>, CanvasViewportQueryError> {
        let pointer = match fs::read_to_string(self.current_path(workspace, canvas)) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(CanvasViewportQueryError::StorageUnavailable),
        };
        if pointer.starts_with(SCHEMA) {
            return Err(CanvasViewportQueryError::StaleProjection);
        }
        let revision = decode_pointer(&pointer).map_err(map_view_error)?;
        let root = self
            .canvas_root(workspace, canvas)
            .join("viewport")
            .join("revisions")
            .join(format!("{:020}", revision.value()));
        let manifest = fs::read_to_string(root.join("manifest.viewport"))
            .map_err(|error| match error.kind() {
                ErrorKind::NotFound => CanvasViewportQueryError::StaleProjection,
                _ => CanvasViewportQueryError::StorageUnavailable,
            })
            .and_then(|text| decode_manifest(&text))?;
        if manifest.revision != revision || &manifest.canvas_id != canvas {
            return Err(CanvasViewportQueryError::StaleProjection);
        }
        let center_x = query.center_x.unwrap_or(manifest.viewport.center_x());
        let center_y = query.center_y.unwrap_or(manifest.viewport.center_y());
        let zoom = query
            .zoom_percent
            .unwrap_or(manifest.viewport.zoom_percent());
        if zoom == 0 || query.surface_width == 0 || query.surface_height == 0 {
            return Err(CanvasViewportQueryError::InvalidInput);
        }
        let scale = f64::from(zoom) / 100.0;
        let half_width = (f64::from(query.surface_width) / scale / 2.0).ceil() as i32;
        let half_height = (f64::from(query.surface_height) / scale / 2.0).ceil() as i32;
        let overscan = query.overscan as i32;
        let left = center_x - half_width - overscan;
        let right = center_x + half_width + overscan;
        let top = center_y - half_height - overscan;
        let bottom = center_y + half_height + overscan;
        let mut node_map = BTreeMap::new();
        let mut edge_map = BTreeMap::new();
        for tile_x in left.div_euclid(TILE_SIZE)..=right.div_euclid(TILE_SIZE) {
            for tile_y in top.div_euclid(TILE_SIZE)..=bottom.div_euclid(TILE_SIZE) {
                let path = root
                    .join("tiles")
                    .join(format!("{tile_x}_{tile_y}.viewport"));
                let text = match fs::read_to_string(path) {
                    Ok(text) => text,
                    Err(error) if error.kind() == ErrorKind::NotFound => continue,
                    Err(_) => return Err(CanvasViewportQueryError::StorageUnavailable),
                };
                let tile = decode_tile(&text)?;
                for node in tile.nodes {
                    node_map
                        .entry(node.id().as_str().to_string())
                        .or_insert(node);
                }
                for edge in tile.edges {
                    edge_map
                        .entry(edge.id().as_str().to_string())
                        .or_insert(edge);
                }
            }
        }
        let matching_nodes = node_map
            .into_values()
            .filter(|node| {
                let geometry = node.geometry();
                geometry.position().x() + geometry.size().width() as i32 >= left
                    && geometry.position().x() <= right
                    && geometry.position().y() + geometry.size().height() as i32 >= top
                    && geometry.position().y() <= bottom
            })
            .collect::<Vec<_>>();
        let matching_node_count = matching_nodes.len();
        let nodes = matching_nodes
            .into_iter()
            .take(query.node_limit)
            .collect::<Vec<_>>();
        let visible = nodes
            .iter()
            .map(|node| node.id().as_str())
            .collect::<BTreeSet<_>>();
        let matching_edges = edge_map
            .into_values()
            .filter(|edge| {
                visible.contains(edge.source_node_id().as_str())
                    && visible.contains(edge.target_node_id().as_str())
            })
            .collect::<Vec<_>>();
        let matching_edge_count = matching_edges.len();
        let edges = matching_edges
            .into_iter()
            .take(query.edge_limit)
            .collect::<Vec<_>>();
        let viewport_policy = CanvasGeometryPolicy::new(1, u32::MAX, 1, u32::MAX, 1, u16::MAX)
            .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?;
        let viewport = CanvasViewport::new(center_x, center_y, zoom, &viewport_policy)
            .map_err(|_| CanvasViewportQueryError::InvalidInput)?;
        Ok(Some(CanvasViewportPage {
            canvas_id: manifest.canvas_id,
            title: manifest.title,
            revision,
            lifecycle: manifest.lifecycle,
            viewport,
            nodes,
            edges,
            total_node_count: manifest.total_node_count,
            total_edge_count: manifest.total_edge_count,
            matching_node_count,
            matching_edge_count,
            truncated: matching_node_count > query.node_limit
                || matching_edge_count > query.edge_limit,
        }))
    }
}

fn encode(record: &CanvasRecord) -> String {
    let canvas = record.canvas();
    let viewport = record.viewport();
    let mut payload = format!(
        "id\t{}\ntitle\t{}\nrevision\t{}\nviewport\t{}\t{}\t{}\nlifecycle\t{}\n",
        hex(canvas.id().as_str()),
        hex(record.title().as_str()),
        record.revision().value(),
        viewport.center_x(),
        viewport.center_y(),
        viewport.zoom_percent(),
        lifecycle_name(canvas.state()),
    );
    for node in canvas.nodes() {
        let geometry = node.geometry();
        let (kind, target) = target_parts(node.target());
        payload.push_str(&format!(
            "node\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            hex(node.id().as_str()),
            kind,
            target,
            geometry.position().x(),
            geometry.position().y(),
            geometry.size().width(),
            geometry.size().height(),
        ));
    }
    for edge in canvas.edges() {
        payload.push_str(&format!(
            "edge\t{}\t{}\t{}\n",
            hex(edge.id().as_str()),
            hex(edge.source_node_id().as_str()),
            hex(edge.target_node_id().as_str()),
        ));
    }
    format!(
        "{SCHEMA}\nchecksum\t{:016x}\n{payload}",
        checksum(payload.as_bytes())
    )
}

fn encode_pointer(revision: CanvasRevision) -> String {
    let payload = format!("revision\t{}\n", revision.value());
    format!(
        "{POINTER_SCHEMA}\nchecksum\t{:016x}\n{payload}",
        checksum(payload.as_bytes())
    )
}

fn decode_pointer(text: &str) -> Result<CanvasRevision, CanvasRepositoryError> {
    let payload = decode_envelope(text, POINTER_SCHEMA)?;
    let fields = payload.trim_end().split('\t').collect::<Vec<_>>();
    match fields.as_slice() {
        ["revision", value] => CanvasRevision::new(parse(value)?).map_err(corrupt),
        _ => Err(CanvasRepositoryError::CorruptedCanvas),
    }
}

#[derive(Debug)]
struct ViewportManifest {
    canvas_id: CanvasId,
    title: CanvasTitle,
    revision: CanvasRevision,
    viewport: CanvasViewport,
    lifecycle: CanvasLifecycleState,
    total_node_count: usize,
    total_edge_count: usize,
}

#[derive(Debug, Default)]
struct TileProjection {
    nodes: Vec<CanvasNode>,
    edges: Vec<CanvasEdge>,
}

fn encode_manifest(record: &CanvasRecord) -> String {
    let viewport = record.viewport();
    let payload = format!(
        "kind\tmanifest\nid\t{}\ntitle\t{}\nrevision\t{}\nviewport\t{}\t{}\t{}\nlifecycle\t{}\ncounts\t{}\t{}\n",
        hex(record.canvas().id().as_str()),
        hex(record.title().as_str()),
        record.revision().value(),
        viewport.center_x(),
        viewport.center_y(),
        viewport.zoom_percent(),
        lifecycle_name(record.canvas().state()),
        record.canvas().nodes().len(),
        record.canvas().edges().len(),
    );
    encode_viewport_envelope(&payload)
}

fn decode_manifest(text: &str) -> Result<ViewportManifest, CanvasViewportQueryError> {
    let payload = decode_envelope(text, VIEWPORT_SCHEMA).map_err(map_view_error)?;
    let mut canvas_id = None;
    let mut title = None;
    let mut revision = None;
    let mut viewport = None;
    let mut lifecycle = None;
    let mut counts = None;
    let policy = CanvasGeometryPolicy::new(1, u32::MAX, 1, u32::MAX, 1, u16::MAX)
        .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?;
    for line in payload.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        match fields.as_slice() {
            ["kind", "manifest"] => {}
            ["id", value] => {
                canvas_id = Some(
                    CanvasId::new(&unhex(value).map_err(map_view_error)?)
                        .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                )
            }
            ["title", value] => {
                title = Some(
                    CanvasTitle::new(&unhex(value).map_err(map_view_error)?)
                        .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                )
            }
            ["revision", value] => {
                revision = Some(
                    CanvasRevision::new(
                        value
                            .parse()
                            .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                    )
                    .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                )
            }
            ["viewport", x, y, zoom] => {
                viewport = Some(
                    CanvasViewport::new(
                        x.parse()
                            .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                        y.parse()
                            .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                        zoom.parse()
                            .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                        &policy,
                    )
                    .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                )
            }
            ["lifecycle", value] => {
                lifecycle = Some(parse_lifecycle(value).map_err(map_view_error)?)
            }
            ["counts", nodes, edges] => {
                counts = Some((
                    nodes
                        .parse()
                        .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                    edges
                        .parse()
                        .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                ))
            }
            _ => return Err(CanvasViewportQueryError::CorruptedProjection),
        }
    }
    let (total_node_count, total_edge_count) =
        counts.ok_or(CanvasViewportQueryError::CorruptedProjection)?;
    Ok(ViewportManifest {
        canvas_id: canvas_id.ok_or(CanvasViewportQueryError::CorruptedProjection)?,
        title: title.ok_or(CanvasViewportQueryError::CorruptedProjection)?,
        revision: revision.ok_or(CanvasViewportQueryError::CorruptedProjection)?,
        viewport: viewport.ok_or(CanvasViewportQueryError::CorruptedProjection)?,
        lifecycle: lifecycle.ok_or(CanvasViewportQueryError::CorruptedProjection)?,
        total_node_count,
        total_edge_count,
    })
}

fn encode_tile(tile: &TileProjection) -> String {
    let mut payload = String::from("kind\ttile\n");
    for node in &tile.nodes {
        let geometry = node.geometry();
        let (kind, target) = target_parts(node.target());
        payload.push_str(&format!(
            "node\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            hex(node.id().as_str()),
            kind,
            target,
            geometry.position().x(),
            geometry.position().y(),
            geometry.size().width(),
            geometry.size().height(),
        ));
    }
    for edge in &tile.edges {
        payload.push_str(&format!(
            "edge\t{}\t{}\t{}\n",
            hex(edge.id().as_str()),
            hex(edge.source_node_id().as_str()),
            hex(edge.target_node_id().as_str()),
        ));
    }
    encode_viewport_envelope(&payload)
}

fn decode_tile(text: &str) -> Result<TileProjection, CanvasViewportQueryError> {
    let payload = decode_envelope(text, VIEWPORT_SCHEMA).map_err(map_view_error)?;
    let policy = CanvasGeometryPolicy::new(1, u32::MAX, 1, u32::MAX, 1, u16::MAX)
        .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?;
    let mut tile = TileProjection::default();
    for line in payload.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        match fields.as_slice() {
            ["kind", "tile"] => {}
            ["node", id, kind, target, x, y, width, height] => {
                let geometry = CanvasGeometry::new(
                    CanvasPosition::new(view_parse(x)?, view_parse(y)?),
                    CanvasSize::new(view_parse(width)?, view_parse(height)?, &policy)
                        .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                );
                tile.nodes.push(
                    CanvasNode::with_geometry(
                        CanvasNodeId::new(&unhex(id).map_err(map_view_error)?)
                            .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                        parse_target(kind, target).map_err(map_view_error)?,
                        geometry,
                    )
                    .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                );
            }
            ["edge", id, source, target] => tile.edges.push(
                CanvasEdge::new(
                    CanvasEdgeId::new(&unhex(id).map_err(map_view_error)?)
                        .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                    CanvasNodeId::new(&unhex(source).map_err(map_view_error)?)
                        .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                    CanvasNodeId::new(&unhex(target).map_err(map_view_error)?)
                        .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
                )
                .map_err(|_| CanvasViewportQueryError::CorruptedProjection)?,
            ),
            _ => return Err(CanvasViewportQueryError::CorruptedProjection),
        }
    }
    Ok(tile)
}

fn encode_viewport_envelope(payload: &str) -> String {
    format!(
        "{VIEWPORT_SCHEMA}\nchecksum\t{:016x}\n{payload}",
        checksum(payload.as_bytes())
    )
}

fn decode_envelope(text: &str, schema: &str) -> Result<String, CanvasRepositoryError> {
    let mut lines = text.lines();
    match lines.next() {
        Some(value) if value == schema => {}
        Some(value) if value.starts_with("schema\t") => {
            return Err(CanvasRepositoryError::UnsupportedSchema);
        }
        _ => return Err(CanvasRepositoryError::CorruptedCanvas),
    }
    let expected = lines
        .next()
        .and_then(|line| line.strip_prefix("checksum\t"))
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .ok_or(CanvasRepositoryError::CorruptedCanvas)?;
    let payload = format!("{}\n", lines.collect::<Vec<_>>().join("\n"));
    if checksum(payload.as_bytes()) != expected {
        return Err(CanvasRepositoryError::CorruptedCanvas);
    }
    Ok(payload)
}

fn view_parse<T: std::str::FromStr>(value: &str) -> Result<T, CanvasViewportQueryError> {
    value
        .parse()
        .map_err(|_| CanvasViewportQueryError::CorruptedProjection)
}

fn map_view_error(error: CanvasRepositoryError) -> CanvasViewportQueryError {
    match error {
        CanvasRepositoryError::UnsupportedSchema => CanvasViewportQueryError::UnsupportedSchema,
        CanvasRepositoryError::StorageUnavailable => CanvasViewportQueryError::StorageUnavailable,
        CanvasRepositoryError::InvalidInput => CanvasViewportQueryError::InvalidInput,
        CanvasRepositoryError::CorruptedCanvas
        | CanvasRepositoryError::AlreadyExists
        | CanvasRepositoryError::VersionConflict => CanvasViewportQueryError::CorruptedProjection,
    }
}

fn decode(text: &str) -> Result<CanvasRecord, CanvasRepositoryError> {
    let mut lines = text.lines();
    match lines.next() {
        Some(SCHEMA) => {}
        Some(value) if value.starts_with("schema\t") => {
            return Err(CanvasRepositoryError::UnsupportedSchema);
        }
        _ => return Err(CanvasRepositoryError::CorruptedCanvas),
    }
    let expected = lines
        .next()
        .and_then(|line| line.strip_prefix("checksum\t"))
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .ok_or(CanvasRepositoryError::CorruptedCanvas)?;
    let payload = format!("{}\n", lines.collect::<Vec<_>>().join("\n"));
    if checksum(payload.as_bytes()) != expected {
        return Err(CanvasRepositoryError::CorruptedCanvas);
    }

    let mut canvas_id = None;
    let mut title = None;
    let mut revision = None;
    let mut viewport = None;
    let mut lifecycle = None;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let policy = CanvasGeometryPolicy::new(1, u32::MAX, 1, u32::MAX, 1, u16::MAX)
        .map_err(|_| CanvasRepositoryError::CorruptedCanvas)?;
    for line in payload.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        match fields.as_slice() {
            ["id", value] => canvas_id = Some(CanvasId::new(&unhex(value)?).map_err(corrupt)?),
            ["title", value] => title = Some(CanvasTitle::new(&unhex(value)?).map_err(corrupt)?),
            ["revision", value] => {
                revision = Some(CanvasRevision::new(parse(value)?).map_err(corrupt)?)
            }
            ["viewport", x, y, zoom] => {
                viewport = Some(
                    CanvasViewport::new(parse(x)?, parse(y)?, parse(zoom)?, &policy)
                        .map_err(corrupt)?,
                )
            }
            ["lifecycle", value] => lifecycle = Some(parse_lifecycle(value)?),
            ["node", id, kind, target, x, y, width, height] => {
                let geometry = CanvasGeometry::new(
                    CanvasPosition::new(parse(x)?, parse(y)?),
                    CanvasSize::new(parse(width)?, parse(height)?, &policy).map_err(corrupt)?,
                );
                nodes.push(
                    CanvasNode::with_geometry(
                        CanvasNodeId::new(&unhex(id)?).map_err(corrupt)?,
                        parse_target(kind, target)?,
                        geometry,
                    )
                    .map_err(corrupt)?,
                );
            }
            ["edge", id, source, target] => edges.push(
                CanvasEdge::new(
                    CanvasEdgeId::new(&unhex(id)?).map_err(corrupt)?,
                    CanvasNodeId::new(&unhex(source)?).map_err(corrupt)?,
                    CanvasNodeId::new(&unhex(target)?).map_err(corrupt)?,
                )
                .map_err(corrupt)?,
            ),
            _ => return Err(CanvasRepositoryError::CorruptedCanvas),
        }
    }
    let canvas = Canvas::new(
        canvas_id.ok_or(CanvasRepositoryError::CorruptedCanvas)?,
        nodes,
        edges,
        lifecycle.ok_or(CanvasRepositoryError::CorruptedCanvas)?,
    )
    .map_err(corrupt)?;
    Ok(CanvasRecord::with_metadata(
        canvas,
        title.ok_or(CanvasRepositoryError::CorruptedCanvas)?,
        revision.ok_or(CanvasRepositoryError::CorruptedCanvas)?,
        viewport.ok_or(CanvasRepositoryError::CorruptedCanvas)?,
    ))
}

fn target_parts(target: &CanvasNodeTarget) -> (&'static str, String) {
    match target {
        CanvasNodeTarget::Document(value) => ("document", hex(value.as_str())),
        CanvasNodeTarget::Attachment(value) => ("attachment", value.as_str().to_string()),
        CanvasNodeTarget::ExternalLink(value) => ("external", hex(value.as_str())),
        CanvasNodeTarget::TextCard(value) => ("text", hex(value.as_str())),
    }
}

fn parse_target(kind: &str, value: &str) -> Result<CanvasNodeTarget, CanvasRepositoryError> {
    match kind {
        "document" => Ok(CanvasNodeTarget::Document(
            DocumentId::new(&unhex(value)?).map_err(corrupt)?,
        )),
        "attachment" => Ok(CanvasNodeTarget::Attachment(
            AssetId::from_sha256_hex(value).map_err(corrupt)?,
        )),
        "external" => Ok(CanvasNodeTarget::ExternalLink(
            CanvasExternalLink::new(&unhex(value)?).map_err(corrupt)?,
        )),
        "text" => Ok(CanvasNodeTarget::TextCard(
            CanvasTextCard::new(&unhex(value)?).map_err(corrupt)?,
        )),
        _ => Err(CanvasRepositoryError::CorruptedCanvas),
    }
}

fn lifecycle_name(value: CanvasLifecycleState) -> &'static str {
    match value {
        CanvasLifecycleState::Draft => "draft",
        CanvasLifecycleState::Saved => "saved",
        CanvasLifecycleState::Embedded => "embedded",
        CanvasLifecycleState::Updated => "updated",
        CanvasLifecycleState::Archived => "archived",
    }
}

fn parse_lifecycle(value: &str) -> Result<CanvasLifecycleState, CanvasRepositoryError> {
    match value {
        "draft" => Ok(CanvasLifecycleState::Draft),
        "saved" => Ok(CanvasLifecycleState::Saved),
        "embedded" => Ok(CanvasLifecycleState::Embedded),
        "updated" => Ok(CanvasLifecycleState::Updated),
        "archived" => Ok(CanvasLifecycleState::Archived),
        _ => Err(CanvasRepositoryError::CorruptedCanvas),
    }
}

fn read(path: &Path) -> Result<CanvasRecord, CanvasRepositoryError> {
    fs::read_to_string(path)
        .map_err(|_| CanvasRepositoryError::StorageUnavailable)
        .and_then(|text| decode(&text))
}

fn parse_revision_file_name(
    value: &OsStr,
) -> Result<CanvasRevision, CanvasRecoveryRepositoryError> {
    let value = value
        .to_str()
        .ok_or(CanvasRecoveryRepositoryError::CorruptedCatalog)?;
    let number = value
        .strip_suffix(".canvas")
        .filter(|number| number.len() == 20 && number.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or(CanvasRecoveryRepositoryError::CorruptedCatalog)?;
    let revision = number
        .parse::<u64>()
        .map_err(|_| CanvasRecoveryRepositoryError::CorruptedCatalog)?;
    CanvasRevision::new(revision).map_err(|_| CanvasRecoveryRepositoryError::CorruptedCatalog)
}

fn parse<T: std::str::FromStr>(value: &str) -> Result<T, CanvasRepositoryError> {
    value
        .parse()
        .map_err(|_| CanvasRepositoryError::CorruptedCanvas)
}

fn corrupt<T>(_: T) -> CanvasRepositoryError {
    CanvasRepositoryError::CorruptedCanvas
}

fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
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

fn unhex(value: &str) -> Result<String, CanvasRepositoryError> {
    if !value.len().is_multiple_of(2) {
        return Err(CanvasRepositoryError::CorruptedCanvas);
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text =
                std::str::from_utf8(pair).map_err(|_| CanvasRepositoryError::CorruptedCanvas)?;
            u8::from_str_radix(text, 16).map_err(|_| CanvasRepositoryError::CorruptedCanvas)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| CanvasRepositoryError::CorruptedCanvas)
}
