use std::collections::HashSet;

use crate::asset::AssetId;
use crate::document::DocumentId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas {
    id: CanvasId,
    nodes: Vec<CanvasNode>,
    edges: Vec<CanvasEdge>,
    state: CanvasLifecycleState,
}

impl Canvas {
    pub fn new(
        id: CanvasId,
        nodes: Vec<CanvasNode>,
        edges: Vec<CanvasEdge>,
        state: CanvasLifecycleState,
    ) -> Result<Self, CanvasError> {
        validate_canvas(&nodes, &edges)?;
        Ok(Self {
            id,
            nodes,
            edges,
            state,
        })
    }

    pub fn id(&self) -> &CanvasId {
        &self.id
    }

    pub fn nodes(&self) -> &[CanvasNode] {
        &self.nodes
    }

    pub fn edges(&self) -> &[CanvasEdge] {
        &self.edges
    }

    pub const fn state(&self) -> CanvasLifecycleState {
        self.state
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasId(String);

impl CanvasId {
    pub fn new(value: &str) -> Result<Self, CanvasError> {
        Ok(Self(normalize_canvas_id(value)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanvasNodeId(String);

impl CanvasNodeId {
    pub fn new(value: &str) -> Result<Self, CanvasError> {
        Ok(Self(normalize_canvas_id(value)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasEdgeId(String);

impl CanvasEdgeId {
    pub fn new(value: &str) -> Result<Self, CanvasError> {
        Ok(Self(normalize_canvas_id(value)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasNode {
    id: CanvasNodeId,
    target: CanvasNodeTarget,
    position: CanvasPosition,
    size: CanvasSize,
}

impl CanvasNode {
    pub fn new(
        id: CanvasNodeId,
        target: CanvasNodeTarget,
        position: CanvasPosition,
    ) -> Result<Self, CanvasError> {
        Ok(Self {
            id,
            target,
            position,
            size: CanvasSize::default(),
        })
    }

    pub fn with_geometry(
        id: CanvasNodeId,
        target: CanvasNodeTarget,
        geometry: CanvasGeometry,
    ) -> Result<Self, CanvasError> {
        Ok(Self {
            id,
            target,
            position: geometry.position(),
            size: geometry.size(),
        })
    }

    pub fn id(&self) -> &CanvasNodeId {
        &self.id
    }

    pub fn target(&self) -> &CanvasNodeTarget {
        &self.target
    }

    pub const fn position(&self) -> CanvasPosition {
        self.position
    }

    pub const fn geometry(&self) -> CanvasGeometry {
        CanvasGeometry::new(self.position, self.size)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasRevision(u64);

impl CanvasRevision {
    pub const fn new(value: u64) -> Result<Self, CanvasError> {
        if value == 0 {
            Err(CanvasError::InvalidRevision)
        } else {
            Ok(Self(value))
        }
    }
    pub const fn value(self) -> u64 {
        self.0
    }
    pub const fn next(self) -> Result<Self, CanvasError> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(CanvasError::RevisionOverflow),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasTitle(String);

impl CanvasTitle {
    pub fn new(value: &str) -> Result<Self, CanvasError> {
        let value = value.trim();
        if value.is_empty() || value.chars().any(char::is_control) || value.chars().count() > 120 {
            return Err(CanvasError::InvalidTitle);
        }
        Ok(Self(value.to_string()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasNodeTarget {
    Document(DocumentId),
    Attachment(AssetId),
    ExternalLink(CanvasExternalLink),
    TextCard(CanvasTextCard),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasExternalLink(String);

impl CanvasExternalLink {
    pub fn new(value: &str) -> Result<Self, CanvasError> {
        let trimmed = value.trim();
        if trimmed.is_empty()
            || trimmed.chars().any(char::is_control)
            || !(trimmed.starts_with("https://") || trimmed.starts_with("http://"))
        {
            return Err(CanvasError::InvalidExternalLink);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasTextCard(String);

impl CanvasTextCard {
    pub fn new(value: &str) -> Result<Self, CanvasError> {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
            return Err(CanvasError::InvalidTextCard);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasPosition {
    x: i32,
    y: i32,
}

impl CanvasPosition {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn x(self) -> i32 {
        self.x
    }

    pub const fn y(self) -> i32 {
        self.y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasSize {
    width: u32,
    height: u32,
}

impl CanvasSize {
    pub fn new(
        width: u32,
        height: u32,
        policy: &CanvasGeometryPolicy,
    ) -> Result<Self, CanvasError> {
        if width < policy.min_width
            || width > policy.max_width
            || height < policy.min_height
            || height > policy.max_height
        {
            return Err(CanvasError::InvalidGeometry);
        }
        Ok(Self { width, height })
    }
    pub const fn width(self) -> u32 {
        self.width
    }
    pub const fn height(self) -> u32 {
        self.height
    }
}

impl Default for CanvasSize {
    fn default() -> Self {
        Self {
            width: 320,
            height: 180,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasGeometry {
    position: CanvasPosition,
    size: CanvasSize,
}

impl CanvasGeometry {
    pub const fn new(position: CanvasPosition, size: CanvasSize) -> Self {
        Self { position, size }
    }
    pub const fn position(self) -> CanvasPosition {
        self.position
    }
    pub const fn size(self) -> CanvasSize {
        self.size
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasViewport {
    center_x: i32,
    center_y: i32,
    zoom_percent: u16,
}

impl CanvasViewport {
    pub fn new(
        center_x: i32,
        center_y: i32,
        zoom_percent: u16,
        policy: &CanvasGeometryPolicy,
    ) -> Result<Self, CanvasError> {
        if zoom_percent < policy.min_zoom_percent || zoom_percent > policy.max_zoom_percent {
            return Err(CanvasError::InvalidViewport);
        }
        Ok(Self {
            center_x,
            center_y,
            zoom_percent,
        })
    }
    pub const fn center_x(self) -> i32 {
        self.center_x
    }
    pub const fn center_y(self) -> i32 {
        self.center_y
    }
    pub const fn zoom_percent(self) -> u16 {
        self.zoom_percent
    }
}

impl Default for CanvasViewport {
    fn default() -> Self {
        Self {
            center_x: 0,
            center_y: 0,
            zoom_percent: 100,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasGeometryPolicy {
    min_width: u32,
    max_width: u32,
    min_height: u32,
    max_height: u32,
    min_zoom_percent: u16,
    max_zoom_percent: u16,
}

impl CanvasGeometryPolicy {
    pub const fn new(
        min_width: u32,
        max_width: u32,
        min_height: u32,
        max_height: u32,
        min_zoom_percent: u16,
        max_zoom_percent: u16,
    ) -> Result<Self, CanvasError> {
        if min_width == 0
            || min_width > max_width
            || min_height == 0
            || min_height > max_height
            || min_zoom_percent == 0
            || min_zoom_percent > max_zoom_percent
        {
            return Err(CanvasError::InvalidGeometryPolicy);
        }
        Ok(Self {
            min_width,
            max_width,
            min_height,
            max_height,
            min_zoom_percent,
            max_zoom_percent,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasEdge {
    id: CanvasEdgeId,
    source_node_id: CanvasNodeId,
    target_node_id: CanvasNodeId,
}

impl CanvasEdge {
    pub fn new(
        id: CanvasEdgeId,
        source_node_id: CanvasNodeId,
        target_node_id: CanvasNodeId,
    ) -> Result<Self, CanvasError> {
        if source_node_id == target_node_id {
            return Err(CanvasError::SelfReferencingEdge);
        }
        Ok(Self {
            id,
            source_node_id,
            target_node_id,
        })
    }

    pub fn id(&self) -> &CanvasEdgeId {
        &self.id
    }

    pub fn source_node_id(&self) -> &CanvasNodeId {
        &self.source_node_id
    }

    pub fn target_node_id(&self) -> &CanvasNodeId {
        &self.target_node_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasEmbed {
    canvas_id: CanvasId,
    reference: String,
}

impl CanvasEmbed {
    pub fn new(canvas_id: CanvasId) -> Self {
        let reference = format!("canvas:{}", canvas_id.as_str());
        Self {
            canvas_id,
            reference,
        }
    }

    pub fn canvas_id(&self) -> &CanvasId {
        &self.canvas_id
    }

    pub fn reference(&self) -> &str {
        &self.reference
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasLifecycleState {
    Draft,
    Saved,
    Embedded,
    Updated,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasLifecycleEvent {
    Save,
    Embed,
    Update,
    Archive,
}

pub fn transition_canvas_lifecycle(
    state: CanvasLifecycleState,
    event: CanvasLifecycleEvent,
) -> Result<CanvasLifecycleState, CanvasError> {
    match (state, event) {
        (CanvasLifecycleState::Draft, CanvasLifecycleEvent::Save) => {
            Ok(CanvasLifecycleState::Saved)
        }
        (CanvasLifecycleState::Saved, CanvasLifecycleEvent::Embed) => {
            Ok(CanvasLifecycleState::Embedded)
        }
        (CanvasLifecycleState::Embedded, CanvasLifecycleEvent::Update) => {
            Ok(CanvasLifecycleState::Updated)
        }
        (CanvasLifecycleState::Updated, CanvasLifecycleEvent::Save) => {
            Ok(CanvasLifecycleState::Saved)
        }
        (
            CanvasLifecycleState::Draft
            | CanvasLifecycleState::Saved
            | CanvasLifecycleState::Updated,
            CanvasLifecycleEvent::Archive,
        ) => Ok(CanvasLifecycleState::Archived),
        _ => Err(CanvasError::InvalidLifecycleTransition),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasError {
    EmptyId,
    InvalidId,
    DuplicateNodeId,
    DuplicateEdgeId,
    MissingEdgeNode,
    SelfReferencingEdge,
    InvalidExternalLink,
    InvalidTextCard,
    InvalidLifecycleTransition,
    InvalidRevision,
    RevisionOverflow,
    InvalidTitle,
    InvalidGeometry,
    InvalidViewport,
    InvalidGeometryPolicy,
}

impl CanvasError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EmptyId => "canvas.empty_id",
            Self::InvalidId => "canvas.invalid_id",
            Self::DuplicateNodeId => "canvas.duplicate_node_id",
            Self::DuplicateEdgeId => "canvas.duplicate_edge_id",
            Self::MissingEdgeNode => "canvas.missing_edge_node",
            Self::SelfReferencingEdge => "canvas.self_referencing_edge",
            Self::InvalidExternalLink => "canvas.invalid_external_link",
            Self::InvalidTextCard => "canvas.invalid_text_card",
            Self::InvalidLifecycleTransition => "canvas.invalid_lifecycle_transition",
            Self::InvalidRevision => "canvas.invalid_revision",
            Self::RevisionOverflow => "canvas.revision_overflow",
            Self::InvalidTitle => "canvas.invalid_title",
            Self::InvalidGeometry => "canvas.invalid_geometry",
            Self::InvalidViewport => "canvas.invalid_viewport",
            Self::InvalidGeometryPolicy => "canvas.invalid_geometry_policy",
        }
    }
}

fn validate_canvas(nodes: &[CanvasNode], edges: &[CanvasEdge]) -> Result<(), CanvasError> {
    let mut node_ids = HashSet::new();
    for node in nodes {
        if !node_ids.insert(node.id().clone()) {
            return Err(CanvasError::DuplicateNodeId);
        }
    }
    let mut edge_ids = HashSet::new();
    for edge in edges {
        if !edge_ids.insert(edge.id().as_str()) {
            return Err(CanvasError::DuplicateEdgeId);
        }
        if !node_ids.contains(edge.source_node_id()) || !node_ids.contains(edge.target_node_id()) {
            return Err(CanvasError::MissingEdgeNode);
        }
    }
    Ok(())
}

fn normalize_canvas_id(value: &str) -> Result<String, CanvasError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CanvasError::EmptyId);
    }
    if trimmed.chars().any(char::is_control) || trimmed.contains('/') || trimmed.contains('\\') {
        return Err(CanvasError::InvalidId);
    }
    Ok(trimmed.to_string())
}
