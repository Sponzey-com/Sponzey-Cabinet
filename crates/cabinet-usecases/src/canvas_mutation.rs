use cabinet_domain::asset::AssetId;
use cabinet_domain::canvas::{
    Canvas, CanvasEdge, CanvasEdgeId, CanvasGeometry, CanvasGeometryPolicy, CanvasId,
    CanvasLifecycleState, CanvasNode, CanvasNodeId, CanvasNodeTarget, CanvasPosition,
    CanvasRevision, CanvasSize, CanvasTextCard, CanvasViewport,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_metadata_catalog::{AssetMetadataCatalog, AssetMetadataCatalogError};
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository, CanvasRepositoryError};
use cabinet_ports::document_existence::{DocumentExistenceError, DocumentExistenceReader};

#[derive(Debug, Clone, Copy)]
pub struct CanvasMutationPolicy {
    max_nodes: usize,
    max_edges: usize,
    geometry: CanvasGeometryPolicy,
}
impl CanvasMutationPolicy {
    pub fn new(
        max_nodes: usize,
        max_edges: usize,
        geometry: CanvasGeometryPolicy,
    ) -> Result<Self, CanvasMutationError> {
        if max_nodes == 0 || max_edges == 0 {
            return Err(CanvasMutationError::InvalidInput);
        }
        Ok(Self {
            max_nodes,
            max_edges,
            geometry,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasNodeTargetInput {
    Document(String),
    Attachment(String),
    Text(String),
}
#[derive(Debug, Clone)]
pub struct AddCanvasNodeMutationInput {
    w: String,
    c: String,
    r: u64,
    n: String,
    t: CanvasNodeTargetInput,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}
impl AddCanvasNodeMutationInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        w: &str,
        c: &str,
        r: u64,
        n: &str,
        t: CanvasNodeTargetInput,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            w: w.into(),
            c: c.into(),
            r,
            n: n.into(),
            t,
            x,
            y,
            width,
            height,
        }
    }
}
#[derive(Debug, Clone)]
pub struct ConnectCanvasEdgeInput {
    w: String,
    c: String,
    r: u64,
    e: String,
    s: String,
    t: String,
}
impl ConnectCanvasEdgeInput {
    pub fn new(w: &str, c: &str, r: u64, e: &str, s: &str, t: &str) -> Self {
        Self {
            w: w.into(),
            c: c.into(),
            r,
            e: e.into(),
            s: s.into(),
            t: t.into(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct RemoveCanvasNodeInput {
    w: String,
    c: String,
    r: u64,
    n: String,
}

#[derive(Debug, Clone)]
pub struct RemoveCanvasEdgeInput {
    w: String,
    c: String,
    r: u64,
    e: String,
}
impl RemoveCanvasEdgeInput {
    pub fn new(w: &str, c: &str, r: u64, e: &str) -> Self {
        Self {
            w: w.into(),
            c: c.into(),
            r,
            e: e.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateCanvasNodeGeometryInput {
    w: String,
    c: String,
    r: u64,
    n: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

pub const MAX_CANVAS_TEXT_CARD_CHARS: usize = 20_000;

#[derive(Debug, Clone)]
pub struct UpdateCanvasTextCardInput {
    w: String,
    c: String,
    r: u64,
    n: String,
    text: String,
}
impl UpdateCanvasTextCardInput {
    pub fn new(w: &str, c: &str, r: u64, n: &str, text: &str) -> Self {
        Self {
            w: w.into(),
            c: c.into(),
            r,
            n: n.into(),
            text: text.into(),
        }
    }
}
impl UpdateCanvasNodeGeometryInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(w: &str, c: &str, r: u64, n: &str, x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            w: w.into(),
            c: c.into(),
            r,
            n: n.into(),
            x,
            y,
            width,
            height,
        }
    }
}
#[derive(Debug, Clone)]
pub struct UpdateCanvasViewportInput {
    w: String,
    c: String,
    r: u64,
    x: i32,
    y: i32,
    zoom: u16,
}
impl UpdateCanvasViewportInput {
    pub fn new(w: &str, c: &str, r: u64, x: i32, y: i32, zoom: u16) -> Self {
        Self {
            w: w.into(),
            c: c.into(),
            r,
            x,
            y,
            zoom,
        }
    }
}
#[derive(Debug, Clone)]
pub struct AutoArrangeCanvasInput {
    w: String,
    c: String,
    r: u64,
}
impl AutoArrangeCanvasInput {
    pub fn new(w: &str, c: &str, r: u64) -> Self {
        Self {
            w: w.into(),
            c: c.into(),
            r,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct CanvasAutoArrangePolicy {
    columns: usize,
    origin_x: i32,
    origin_y: i32,
    gap_x: i32,
    gap_y: i32,
}
impl CanvasAutoArrangePolicy {
    pub fn new(
        columns: usize,
        origin_x: i32,
        origin_y: i32,
        gap_x: i32,
        gap_y: i32,
    ) -> Result<Self, CanvasMutationError> {
        if columns == 0 || gap_x <= 0 || gap_y <= 0 {
            return Err(CanvasMutationError::InvalidInput);
        }
        Ok(Self {
            columns,
            origin_x,
            origin_y,
            gap_x,
            gap_y,
        })
    }
}
impl RemoveCanvasNodeInput {
    pub fn new(w: &str, c: &str, r: u64, n: &str) -> Self {
        Self {
            w: w.into(),
            c: c.into(),
            r,
            n: n.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasMutationOutput {
    record: CanvasRecord,
}
impl CanvasMutationOutput {
    pub fn record(&self) -> &CanvasRecord {
        &self.record
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasMutationProductEvent {
    NodeAdded {
        canvas_id: String,
        revision: u64,
        node_count: usize,
    },
    EdgeConnected {
        canvas_id: String,
        revision: u64,
        edge_count: usize,
    },
    EdgeRemoved {
        canvas_id: String,
        revision: u64,
        edge_count: usize,
    },
    NodeRemoved {
        canvas_id: String,
        revision: u64,
        node_count: usize,
        edge_count: usize,
    },
    GeometryUpdated {
        canvas_id: String,
        revision: u64,
        changed_node_count: usize,
    },
    TextCardUpdated {
        canvas_id: String,
        revision: u64,
        changed_node_count: usize,
    },
    ViewportUpdated {
        canvas_id: String,
        revision: u64,
    },
    AutoArranged {
        canvas_id: String,
        revision: u64,
        changed_node_count: usize,
    },
}
pub trait CanvasMutationProductLogger {
    fn write_product(&mut self, event: CanvasMutationProductEvent);
}

pub struct AddValidatedCanvasNodeUsecase;
impl AddValidatedCanvasNodeUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute<
        D: DocumentExistenceReader,
        A: AssetMetadataCatalog,
        R: CanvasRepository,
        L: CanvasMutationProductLogger,
    >(
        &self,
        input: AddCanvasNodeMutationInput,
        policy: &CanvasMutationPolicy,
        documents: &D,
        assets: &A,
        repository: &mut R,
        logger: &mut L,
    ) -> Result<CanvasMutationOutput, CanvasMutationError> {
        let workspace =
            WorkspaceId::new(&input.w).map_err(|_| CanvasMutationError::InvalidInput)?;
        match &input.t {
            CanvasNodeTargetInput::Document(value) => {
                let document =
                    DocumentId::new(value).map_err(|_| CanvasMutationError::InvalidInput)?;
                if !documents
                    .exists(&workspace, &document)
                    .map_err(map_document_existence)?
                {
                    return Err(CanvasMutationError::DocumentTargetNotFound);
                }
            }
            CanvasNodeTargetInput::Attachment(value) => {
                let asset = AssetId::from_sha256_hex(value)
                    .map_err(|_| CanvasMutationError::InvalidInput)?;
                if assets
                    .get(&workspace, &asset)
                    .map_err(map_asset_metadata)?
                    .is_none()
                {
                    return Err(CanvasMutationError::AssetTargetNotFound);
                }
            }
            CanvasNodeTargetInput::Text(_) => {}
        }
        AddCanvasNodeMutationUsecase::new().execute(input, policy, repository, logger)
    }
}

pub struct AddCanvasNodeMutationUsecase;
impl AddCanvasNodeMutationUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: CanvasRepository, L: CanvasMutationProductLogger>(
        &self,
        input: AddCanvasNodeMutationInput,
        policy: &CanvasMutationPolicy,
        repository: &mut R,
        logger: &mut L,
    ) -> Result<CanvasMutationOutput, CanvasMutationError> {
        let (workspace, id, expected, current) = load(&input.w, &input.c, input.r, repository)?;
        ensure_mutable(&current)?;
        if current.canvas().nodes().len() >= policy.max_nodes {
            return Err(CanvasMutationError::NodeLimitExceeded);
        }
        let target = match input.t {
            CanvasNodeTargetInput::Document(v) => CanvasNodeTarget::Document(
                DocumentId::new(&v).map_err(|_| CanvasMutationError::InvalidInput)?,
            ),
            CanvasNodeTargetInput::Attachment(v) => CanvasNodeTarget::Attachment(
                AssetId::from_sha256_hex(&v).map_err(|_| CanvasMutationError::InvalidInput)?,
            ),
            CanvasNodeTargetInput::Text(v) => CanvasNodeTarget::TextCard(
                CanvasTextCard::new(&v).map_err(|_| CanvasMutationError::InvalidInput)?,
            ),
        };
        let geometry = CanvasGeometry::new(
            CanvasPosition::new(input.x, input.y),
            CanvasSize::new(input.width, input.height, &policy.geometry)
                .map_err(|_| CanvasMutationError::InvalidGeometry)?,
        );
        let mut nodes = current.canvas().nodes().to_vec();
        nodes.push(
            CanvasNode::with_geometry(
                CanvasNodeId::new(&input.n).map_err(|_| CanvasMutationError::InvalidInput)?,
                target,
                geometry,
            )
            .map_err(|_| CanvasMutationError::InvalidInput)?,
        );
        let canvas = Canvas::new(
            id.clone(),
            nodes,
            current.canvas().edges().to_vec(),
            CanvasLifecycleState::Updated,
        )
        .map_err(map_canvas)?;
        let next = save(workspace, expected, current, canvas, repository)?;
        logger.write_product(CanvasMutationProductEvent::NodeAdded {
            canvas_id: id.as_str().into(),
            revision: next.revision().value(),
            node_count: next.canvas().nodes().len(),
        });
        Ok(CanvasMutationOutput { record: next })
    }
}

pub struct ConnectCanvasEdgeUsecase;
impl ConnectCanvasEdgeUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: CanvasRepository, L: CanvasMutationProductLogger>(
        &self,
        input: ConnectCanvasEdgeInput,
        policy: &CanvasMutationPolicy,
        repository: &mut R,
        logger: &mut L,
    ) -> Result<CanvasMutationOutput, CanvasMutationError> {
        let (workspace, id, expected, current) = load(&input.w, &input.c, input.r, repository)?;
        ensure_mutable(&current)?;
        if current.canvas().edges().len() >= policy.max_edges {
            return Err(CanvasMutationError::EdgeLimitExceeded);
        }
        let mut edges = current.canvas().edges().to_vec();
        edges.push(
            CanvasEdge::new(
                CanvasEdgeId::new(&input.e).map_err(|_| CanvasMutationError::InvalidInput)?,
                CanvasNodeId::new(&input.s).map_err(|_| CanvasMutationError::InvalidInput)?,
                CanvasNodeId::new(&input.t).map_err(|_| CanvasMutationError::InvalidInput)?,
            )
            .map_err(map_canvas)?,
        );
        let canvas = Canvas::new(
            id.clone(),
            current.canvas().nodes().to_vec(),
            edges,
            CanvasLifecycleState::Updated,
        )
        .map_err(map_canvas)?;
        let next = save(workspace, expected, current, canvas, repository)?;
        logger.write_product(CanvasMutationProductEvent::EdgeConnected {
            canvas_id: id.as_str().into(),
            revision: next.revision().value(),
            edge_count: next.canvas().edges().len(),
        });
        Ok(CanvasMutationOutput { record: next })
    }
}

pub struct RemoveCanvasNodeUsecase;
impl RemoveCanvasNodeUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: CanvasRepository, L: CanvasMutationProductLogger>(
        &self,
        input: RemoveCanvasNodeInput,
        repository: &mut R,
        logger: &mut L,
    ) -> Result<CanvasMutationOutput, CanvasMutationError> {
        let (workspace, id, expected, current) = load(&input.w, &input.c, input.r, repository)?;
        ensure_mutable(&current)?;
        let node = CanvasNodeId::new(&input.n).map_err(|_| CanvasMutationError::InvalidInput)?;
        if !current.canvas().nodes().iter().any(|v| v.id() == &node) {
            return Err(CanvasMutationError::NodeNotFound);
        }
        let nodes = current
            .canvas()
            .nodes()
            .iter()
            .filter(|v| v.id() != &node)
            .cloned()
            .collect();
        let edges = current
            .canvas()
            .edges()
            .iter()
            .filter(|v| v.source_node_id() != &node && v.target_node_id() != &node)
            .cloned()
            .collect();
        let canvas = Canvas::new(id.clone(), nodes, edges, CanvasLifecycleState::Updated)
            .map_err(map_canvas)?;
        let next = save(workspace, expected, current, canvas, repository)?;
        logger.write_product(CanvasMutationProductEvent::NodeRemoved {
            canvas_id: id.as_str().into(),
            revision: next.revision().value(),
            node_count: next.canvas().nodes().len(),
            edge_count: next.canvas().edges().len(),
        });
        Ok(CanvasMutationOutput { record: next })
    }
}

pub struct RemoveCanvasEdgeUsecase;
impl RemoveCanvasEdgeUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: CanvasRepository, L: CanvasMutationProductLogger>(
        &self,
        input: RemoveCanvasEdgeInput,
        repository: &mut R,
        logger: &mut L,
    ) -> Result<CanvasMutationOutput, CanvasMutationError> {
        let (workspace, id, expected, current) = load(&input.w, &input.c, input.r, repository)?;
        ensure_mutable(&current)?;
        let edge_id = CanvasEdgeId::new(&input.e).map_err(|_| CanvasMutationError::InvalidInput)?;
        if !current
            .canvas()
            .edges()
            .iter()
            .any(|edge| edge.id() == &edge_id)
        {
            return Err(CanvasMutationError::EdgeNotFound);
        }
        let edges = current
            .canvas()
            .edges()
            .iter()
            .filter(|edge| edge.id() != &edge_id)
            .cloned()
            .collect();
        let canvas = Canvas::new(
            id.clone(),
            current.canvas().nodes().to_vec(),
            edges,
            CanvasLifecycleState::Updated,
        )
        .map_err(map_canvas)?;
        let next = save(workspace, expected, current, canvas, repository)?;
        logger.write_product(CanvasMutationProductEvent::EdgeRemoved {
            canvas_id: id.as_str().into(),
            revision: next.revision().value(),
            edge_count: next.canvas().edges().len(),
        });
        Ok(CanvasMutationOutput { record: next })
    }
}

pub struct UpdateCanvasNodeGeometryUsecase;
impl UpdateCanvasNodeGeometryUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: CanvasRepository, L: CanvasMutationProductLogger>(
        &self,
        input: UpdateCanvasNodeGeometryInput,
        policy: &CanvasMutationPolicy,
        repository: &mut R,
        logger: &mut L,
    ) -> Result<CanvasMutationOutput, CanvasMutationError> {
        let (workspace, id, expected, current) = load(&input.w, &input.c, input.r, repository)?;
        ensure_mutable(&current)?;
        let target_id =
            CanvasNodeId::new(&input.n).map_err(|_| CanvasMutationError::InvalidInput)?;
        let geometry = CanvasGeometry::new(
            CanvasPosition::new(input.x, input.y),
            CanvasSize::new(input.width, input.height, &policy.geometry)
                .map_err(|_| CanvasMutationError::InvalidGeometry)?,
        );
        let mut found = false;
        let nodes = current
            .canvas()
            .nodes()
            .iter()
            .map(|node| {
                if node.id() == &target_id {
                    found = true;
                    CanvasNode::with_geometry(node.id().clone(), node.target().clone(), geometry)
                        .expect("validated geometry")
                } else {
                    node.clone()
                }
            })
            .collect();
        if !found {
            return Err(CanvasMutationError::NodeNotFound);
        }
        let canvas = Canvas::new(
            id.clone(),
            nodes,
            current.canvas().edges().to_vec(),
            CanvasLifecycleState::Updated,
        )
        .map_err(map_canvas)?;
        let next = save(workspace, expected, current, canvas, repository)?;
        logger.write_product(CanvasMutationProductEvent::GeometryUpdated {
            canvas_id: id.as_str().into(),
            revision: next.revision().value(),
            changed_node_count: 1,
        });
        Ok(CanvasMutationOutput { record: next })
    }
}

pub struct UpdateCanvasTextCardUsecase;
impl UpdateCanvasTextCardUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute<R: CanvasRepository, L: CanvasMutationProductLogger>(
        &self,
        input: UpdateCanvasTextCardInput,
        repository: &mut R,
        logger: &mut L,
    ) -> Result<CanvasMutationOutput, CanvasMutationError> {
        if input
            .text
            .chars()
            .take(MAX_CANVAS_TEXT_CARD_CHARS + 1)
            .count()
            > MAX_CANVAS_TEXT_CARD_CHARS
        {
            return Err(CanvasMutationError::InvalidInput);
        }
        let text =
            CanvasTextCard::new(&input.text).map_err(|_| CanvasMutationError::InvalidInput)?;
        let (workspace, id, expected, current) = load(&input.w, &input.c, input.r, repository)?;
        ensure_mutable(&current)?;
        let target_id =
            CanvasNodeId::new(&input.n).map_err(|_| CanvasMutationError::InvalidInput)?;
        let mut found = false;
        let mut target_mismatch = false;
        let nodes = current
            .canvas()
            .nodes()
            .iter()
            .map(|node| {
                if node.id() != &target_id {
                    return node.clone();
                }
                found = true;
                if !matches!(node.target(), CanvasNodeTarget::TextCard(_)) {
                    target_mismatch = true;
                    return node.clone();
                }
                CanvasNode::with_geometry(
                    node.id().clone(),
                    CanvasNodeTarget::TextCard(text.clone()),
                    node.geometry(),
                )
                .expect("existing geometry and validated text")
            })
            .collect();
        if !found {
            return Err(CanvasMutationError::NodeNotFound);
        }
        if target_mismatch {
            return Err(CanvasMutationError::NodeTargetMismatch);
        }
        let canvas = Canvas::new(
            id.clone(),
            nodes,
            current.canvas().edges().to_vec(),
            CanvasLifecycleState::Updated,
        )
        .map_err(map_canvas)?;
        let next = save(workspace, expected, current, canvas, repository)?;
        logger.write_product(CanvasMutationProductEvent::TextCardUpdated {
            canvas_id: id.as_str().into(),
            revision: next.revision().value(),
            changed_node_count: 1,
        });
        Ok(CanvasMutationOutput { record: next })
    }
}

pub struct UpdateCanvasViewportUsecase;
impl UpdateCanvasViewportUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: CanvasRepository, L: CanvasMutationProductLogger>(
        &self,
        input: UpdateCanvasViewportInput,
        policy: &CanvasMutationPolicy,
        repository: &mut R,
        logger: &mut L,
    ) -> Result<CanvasMutationOutput, CanvasMutationError> {
        let (workspace, id, expected, current) = load(&input.w, &input.c, input.r, repository)?;
        ensure_mutable(&current)?;
        let viewport = CanvasViewport::new(input.x, input.y, input.zoom, &policy.geometry)
            .map_err(|_| CanvasMutationError::InvalidGeometry)?;
        let next = current
            .revised(current.canvas().clone(), current.title().clone(), viewport)
            .map_err(map_repo)?;
        repository
            .replace_canvas(&workspace, expected, next.clone())
            .map_err(map_repo)?;
        logger.write_product(CanvasMutationProductEvent::ViewportUpdated {
            canvas_id: id.as_str().into(),
            revision: next.revision().value(),
        });
        Ok(CanvasMutationOutput { record: next })
    }
}

pub struct AutoArrangeCanvasUsecase;
impl AutoArrangeCanvasUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute<R: CanvasRepository, L: CanvasMutationProductLogger>(
        &self,
        input: AutoArrangeCanvasInput,
        policy: &CanvasAutoArrangePolicy,
        repository: &mut R,
        logger: &mut L,
    ) -> Result<CanvasMutationOutput, CanvasMutationError> {
        let (workspace, id, expected, current) = load(&input.w, &input.c, input.r, repository)?;
        ensure_mutable(&current)?;
        let changed = current.canvas().nodes().len();
        let canvas = arrange_canvas(&current, &id, policy)?;
        let next = save(workspace, expected, current, canvas, repository)?;
        logger.write_product(CanvasMutationProductEvent::AutoArranged {
            canvas_id: id.as_str().into(),
            revision: next.revision().value(),
            changed_node_count: changed,
        });
        Ok(CanvasMutationOutput { record: next })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasAutoArrangePreviewOutput {
    record: CanvasRecord,
}
impl CanvasAutoArrangePreviewOutput {
    pub fn record(&self) -> &CanvasRecord {
        &self.record
    }
}

pub struct PreviewAutoArrangeCanvasUsecase;
impl PreviewAutoArrangeCanvasUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute<R: CanvasRepository>(
        &self,
        input: AutoArrangeCanvasInput,
        policy: &CanvasAutoArrangePolicy,
        repository: &R,
    ) -> Result<CanvasAutoArrangePreviewOutput, CanvasMutationError> {
        let (_, id, _, current) = load(&input.w, &input.c, input.r, repository)?;
        ensure_mutable(&current)?;
        let canvas = arrange_canvas(&current, &id, policy)?;
        let record = CanvasRecord::with_metadata(
            canvas,
            current.title().clone(),
            current.revision(),
            current.viewport(),
        );
        Ok(CanvasAutoArrangePreviewOutput { record })
    }
}

fn arrange_canvas(
    current: &CanvasRecord,
    id: &CanvasId,
    policy: &CanvasAutoArrangePolicy,
) -> Result<Canvas, CanvasMutationError> {
    let mut ordered = current.canvas().nodes().to_vec();
    ordered.sort_by(|a, b| a.id().as_str().cmp(b.id().as_str()));
    let nodes = ordered
        .into_iter()
        .enumerate()
        .map(|(index, node)| {
            let column = (index % policy.columns) as i32;
            let row = (index / policy.columns) as i32;
            CanvasNode::with_geometry(
                node.id().clone(),
                node.target().clone(),
                CanvasGeometry::new(
                    CanvasPosition::new(
                        policy.origin_x + column * policy.gap_x,
                        policy.origin_y + row * policy.gap_y,
                    ),
                    node.geometry().size(),
                ),
            )
            .expect("existing node")
        })
        .collect();
    Canvas::new(
        id.clone(),
        nodes,
        current.canvas().edges().to_vec(),
        CanvasLifecycleState::Updated,
    )
    .map_err(map_canvas)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasMutationError {
    InvalidInput,
    NotFound,
    NodeNotFound,
    NodeTargetMismatch,
    EdgeNotFound,
    DocumentTargetNotFound,
    AssetTargetNotFound,
    VersionConflict,
    InvalidState,
    InvalidGraph,
    InvalidGeometry,
    NodeLimitExceeded,
    EdgeLimitExceeded,
    StorageUnavailable,
    RecoveryRequired,
}
impl CanvasMutationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "CANVAS_INVALID_INPUT",
            Self::NotFound => "CANVAS_NOT_FOUND",
            Self::NodeNotFound => "CANVAS_NODE_NOT_FOUND",
            Self::NodeTargetMismatch => "CANVAS_NODE_TARGET_MISMATCH",
            Self::EdgeNotFound => "CANVAS_EDGE_NOT_FOUND",
            Self::DocumentTargetNotFound => "CANVAS_DOCUMENT_TARGET_NOT_FOUND",
            Self::AssetTargetNotFound => "CANVAS_ASSET_TARGET_NOT_FOUND",
            Self::VersionConflict => "CANVAS_VERSION_CONFLICT",
            Self::InvalidState => "CANVAS_INVALID_STATE",
            Self::InvalidGraph => "CANVAS_INVALID_GRAPH",
            Self::InvalidGeometry => "CANVAS_INVALID_GEOMETRY",
            Self::NodeLimitExceeded => "CANVAS_NODE_LIMIT_EXCEEDED",
            Self::EdgeLimitExceeded => "CANVAS_EDGE_LIMIT_EXCEEDED",
            Self::StorageUnavailable => "CANVAS_STORAGE_UNAVAILABLE",
            Self::RecoveryRequired => "CANVAS_RECOVERY_REQUIRED",
        }
    }
}

fn map_document_existence(error: DocumentExistenceError) -> CanvasMutationError {
    match error {
        DocumentExistenceError::StorageUnavailable => CanvasMutationError::StorageUnavailable,
        DocumentExistenceError::CorruptedRecord => CanvasMutationError::RecoveryRequired,
    }
}

fn map_asset_metadata(error: AssetMetadataCatalogError) -> CanvasMutationError {
    match error {
        AssetMetadataCatalogError::StorageUnavailable => CanvasMutationError::StorageUnavailable,
        AssetMetadataCatalogError::CorruptedRecord
        | AssetMetadataCatalogError::UnsupportedSchema => CanvasMutationError::RecoveryRequired,
        AssetMetadataCatalogError::InvalidLimit
        | AssetMetadataCatalogError::InvalidCursor
        | AssetMetadataCatalogError::Conflict => CanvasMutationError::InvalidInput,
    }
}
fn load<R: CanvasRepository>(
    w: &str,
    c: &str,
    r: u64,
    repo: &R,
) -> Result<(WorkspaceId, CanvasId, CanvasRevision, CanvasRecord), CanvasMutationError> {
    let w = WorkspaceId::new(w).map_err(|_| CanvasMutationError::InvalidInput)?;
    let c = CanvasId::new(c).map_err(|_| CanvasMutationError::InvalidInput)?;
    let r = CanvasRevision::new(r).map_err(|_| CanvasMutationError::InvalidInput)?;
    let record = repo
        .get_canvas(&w, &c)
        .map_err(map_repo)?
        .ok_or(CanvasMutationError::NotFound)?;
    if record.revision() != r {
        return Err(CanvasMutationError::VersionConflict);
    }
    Ok((w, c, r, record))
}
fn ensure_mutable(record: &CanvasRecord) -> Result<(), CanvasMutationError> {
    if record.canvas().state() == CanvasLifecycleState::Archived {
        Err(CanvasMutationError::InvalidState)
    } else {
        Ok(())
    }
}
fn save<R: CanvasRepository>(
    w: WorkspaceId,
    r: CanvasRevision,
    current: CanvasRecord,
    canvas: Canvas,
    repo: &mut R,
) -> Result<CanvasRecord, CanvasMutationError> {
    let next = current.next(canvas).map_err(map_repo)?;
    repo.replace_canvas(&w, r, next.clone()).map_err(map_repo)?;
    Ok(next)
}
fn map_canvas(_: cabinet_domain::canvas::CanvasError) -> CanvasMutationError {
    CanvasMutationError::InvalidGraph
}
fn map_repo(e: CanvasRepositoryError) -> CanvasMutationError {
    match e {
        CanvasRepositoryError::InvalidInput => CanvasMutationError::InvalidInput,
        CanvasRepositoryError::StorageUnavailable => CanvasMutationError::StorageUnavailable,
        CanvasRepositoryError::CorruptedCanvas | CanvasRepositoryError::UnsupportedSchema => {
            CanvasMutationError::RecoveryRequired
        }
        CanvasRepositoryError::AlreadyExists => CanvasMutationError::InvalidInput,
        CanvasRepositoryError::VersionConflict => CanvasMutationError::VersionConflict,
    }
}
