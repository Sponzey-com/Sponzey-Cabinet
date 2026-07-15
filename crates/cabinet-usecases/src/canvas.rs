use cabinet_domain::asset::AssetId;
use cabinet_domain::canvas::{
    Canvas, CanvasEdge, CanvasEdgeId, CanvasEmbed, CanvasError, CanvasExternalLink, CanvasId,
    CanvasLifecycleState, CanvasNode, CanvasNodeId, CanvasNodeTarget, CanvasPosition,
    CanvasTextCard,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{AccessResource, Permission, PermissionDecisionResult};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_repository::{CanvasRecord, CanvasRepository, CanvasRepositoryError};
use cabinet_ports::permission_aware_query::{PermissionAwareQueryError, PermissionDecisionPort};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCanvasInput {
    actor_user_id: String,
    workspace_id: String,
    canvas_id: String,
}

impl CreateCanvasInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, canvas_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            canvas_id: canvas_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddCanvasNodeInput {
    actor_user_id: String,
    workspace_id: String,
    canvas_id: String,
    node_id: String,
    target: AddCanvasNodeTargetInput,
    x: i32,
    y: i32,
}

impl AddCanvasNodeInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        canvas_id: &str,
        node_id: &str,
        target: AddCanvasNodeTargetInput,
        x: i32,
        y: i32,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            canvas_id: canvas_id.to_string(),
            node_id: node_id.to_string(),
            target,
            x,
            y,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddCanvasNodeTargetInput {
    Document { document_id: String },
    Attachment { asset_sha256_hex: String },
    ExternalLink { url: String },
    TextCard { text: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectCanvasNodesInput {
    actor_user_id: String,
    workspace_id: String,
    canvas_id: String,
    edge_id: String,
    source_node_id: String,
    target_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbedCanvasInDocumentInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: String,
    canvas_id: String,
}

impl EmbedCanvasInDocumentInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        document_id: &str,
        canvas_id: &str,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            canvas_id: canvas_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertDocumentOutlineToCanvasInput {
    headings: Vec<DocumentOutlineHeadingInput>,
}

impl ConvertDocumentOutlineToCanvasInput {
    pub fn new(headings: Vec<DocumentOutlineHeadingInput>) -> Self {
        Self { headings }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentOutlineHeadingInput {
    heading_id: String,
    title: String,
    level: u8,
}

impl DocumentOutlineHeadingInput {
    pub fn new(heading_id: &str, title: &str, level: u8) -> Self {
        Self {
            heading_id: heading_id.to_string(),
            title: title.to_string(),
            level,
        }
    }
}

impl ConnectCanvasNodesInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        canvas_id: &str,
        edge_id: &str,
        source_node_id: &str,
        target_node_id: &str,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            canvas_id: canvas_id.to_string(),
            edge_id: edge_id.to_string(),
            source_node_id: source_node_id.to_string(),
            target_node_id: target_node_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasCommandOutput {
    canvas_id: CanvasId,
    state: CanvasLifecycleState,
    node_count: usize,
    edge_count: usize,
    product_log_event: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasEmbedOutput {
    reference: String,
    product_log_event: &'static str,
}

impl CanvasEmbedOutput {
    pub fn reference(&self) -> &str {
        &self.reference
    }

    pub const fn product_log_event(&self) -> &'static str {
        self.product_log_event
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertDocumentOutlineToCanvasOutput {
    suggestions: Vec<CanvasLayoutSuggestion>,
}

impl ConvertDocumentOutlineToCanvasOutput {
    pub fn suggestions(&self) -> &[CanvasLayoutSuggestion] {
        &self.suggestions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasLayoutSuggestion {
    heading_id: String,
    title: String,
    level: u8,
    x: i32,
    y: i32,
}

impl CanvasLayoutSuggestion {
    pub fn heading_id(&self) -> &str {
        &self.heading_id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub const fn level(&self) -> u8 {
        self.level
    }

    pub const fn x(&self) -> i32 {
        self.x
    }

    pub const fn y(&self) -> i32 {
        self.y
    }
}

impl CanvasCommandOutput {
    pub fn canvas_id(&self) -> &CanvasId {
        &self.canvas_id
    }

    pub const fn state(&self) -> CanvasLifecycleState {
        self.state
    }

    pub const fn node_count(&self) -> usize {
        self.node_count
    }

    pub const fn edge_count(&self) -> usize {
        self.edge_count
    }

    pub const fn product_log_event(&self) -> &'static str {
        self.product_log_event
    }
}

pub struct CreateCanvasUsecase;

impl CreateCanvasUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: CreateCanvasInput,
        repository: &mut impl CanvasRepository,
        permissions: &impl PermissionDecisionPort,
    ) -> Result<CanvasCommandOutput, CanvasUsecaseError> {
        let actor_user_id =
            UserId::new(&input.actor_user_id).map_err(|_| CanvasUsecaseError::InvalidInput)?;
        let workspace_id =
            WorkspaceId::new(&input.workspace_id).map_err(|_| CanvasUsecaseError::InvalidInput)?;
        ensure_write_permission(&actor_user_id, &workspace_id, permissions)?;
        let canvas_id = CanvasId::new(&input.canvas_id).map_err(CanvasUsecaseError::from_canvas)?;
        let canvas = Canvas::new(
            canvas_id.clone(),
            Vec::new(),
            Vec::new(),
            CanvasLifecycleState::Draft,
        )
        .map_err(CanvasUsecaseError::from_canvas)?;
        repository
            .create_canvas(
                &workspace_id,
                CanvasRecord::new(canvas.clone()).map_err(CanvasUsecaseError::from_repository)?,
            )
            .map_err(CanvasUsecaseError::from_repository)?;
        Ok(output_from_canvas(canvas, "canvas.created"))
    }
}

impl Default for CreateCanvasUsecase {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AddCanvasNodeUsecase;

impl AddCanvasNodeUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: AddCanvasNodeInput,
        repository: &mut impl CanvasRepository,
        permissions: &impl PermissionDecisionPort,
    ) -> Result<CanvasCommandOutput, CanvasUsecaseError> {
        let actor_user_id =
            UserId::new(&input.actor_user_id).map_err(|_| CanvasUsecaseError::InvalidInput)?;
        let workspace_id =
            WorkspaceId::new(&input.workspace_id).map_err(|_| CanvasUsecaseError::InvalidInput)?;
        ensure_write_permission(&actor_user_id, &workspace_id, permissions)?;
        let canvas_id = CanvasId::new(&input.canvas_id).map_err(CanvasUsecaseError::from_canvas)?;
        let existing = repository
            .get_canvas(&workspace_id, &canvas_id)
            .map_err(CanvasUsecaseError::from_repository)?
            .ok_or(CanvasUsecaseError::CanvasNotFound)?;
        let mut nodes = existing.canvas().nodes().to_vec();
        let edges = existing.canvas().edges().to_vec();
        nodes.push(
            CanvasNode::new(
                CanvasNodeId::new(&input.node_id).map_err(CanvasUsecaseError::from_canvas)?,
                target_from_input(input.target)?,
                CanvasPosition::new(input.x, input.y),
            )
            .map_err(CanvasUsecaseError::from_canvas)?,
        );
        let updated = Canvas::new(canvas_id, nodes, edges, CanvasLifecycleState::Updated)
            .map_err(CanvasUsecaseError::from_canvas)?;
        let expected_revision = existing.revision();
        let next_record = existing
            .next(updated.clone())
            .map_err(CanvasUsecaseError::from_repository)?;
        repository
            .replace_canvas(&workspace_id, expected_revision, next_record)
            .map_err(CanvasUsecaseError::from_repository)?;
        Ok(output_from_canvas(updated, "canvas.node.added"))
    }
}

impl Default for AddCanvasNodeUsecase {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ConnectCanvasNodesUsecase;

impl ConnectCanvasNodesUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ConnectCanvasNodesInput,
        repository: &mut impl CanvasRepository,
        permissions: &impl PermissionDecisionPort,
    ) -> Result<CanvasCommandOutput, CanvasUsecaseError> {
        let actor_user_id =
            UserId::new(&input.actor_user_id).map_err(|_| CanvasUsecaseError::InvalidInput)?;
        let workspace_id =
            WorkspaceId::new(&input.workspace_id).map_err(|_| CanvasUsecaseError::InvalidInput)?;
        ensure_write_permission(&actor_user_id, &workspace_id, permissions)?;
        let canvas_id = CanvasId::new(&input.canvas_id).map_err(CanvasUsecaseError::from_canvas)?;
        let existing = repository
            .get_canvas(&workspace_id, &canvas_id)
            .map_err(CanvasUsecaseError::from_repository)?
            .ok_or(CanvasUsecaseError::CanvasNotFound)?;
        let nodes = existing.canvas().nodes().to_vec();
        let mut edges = existing.canvas().edges().to_vec();
        edges.push(
            CanvasEdge::new(
                CanvasEdgeId::new(&input.edge_id).map_err(CanvasUsecaseError::from_canvas)?,
                CanvasNodeId::new(&input.source_node_id)
                    .map_err(CanvasUsecaseError::from_canvas)?,
                CanvasNodeId::new(&input.target_node_id)
                    .map_err(CanvasUsecaseError::from_canvas)?,
            )
            .map_err(CanvasUsecaseError::from_canvas)?,
        );
        let updated = Canvas::new(canvas_id, nodes, edges, CanvasLifecycleState::Updated)
            .map_err(CanvasUsecaseError::from_canvas)?;
        let expected_revision = existing.revision();
        let next_record = existing
            .next(updated.clone())
            .map_err(CanvasUsecaseError::from_repository)?;
        repository
            .replace_canvas(&workspace_id, expected_revision, next_record)
            .map_err(CanvasUsecaseError::from_repository)?;
        Ok(output_from_canvas(updated, "canvas.edge.connected"))
    }
}

impl Default for ConnectCanvasNodesUsecase {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EmbedCanvasInDocumentUsecase;

impl EmbedCanvasInDocumentUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: EmbedCanvasInDocumentInput,
        repository: &impl CanvasRepository,
        permissions: &impl PermissionDecisionPort,
    ) -> Result<CanvasEmbedOutput, CanvasUsecaseError> {
        let actor_user_id =
            UserId::new(&input.actor_user_id).map_err(|_| CanvasUsecaseError::InvalidInput)?;
        let workspace_id =
            WorkspaceId::new(&input.workspace_id).map_err(|_| CanvasUsecaseError::InvalidInput)?;
        let document_id =
            DocumentId::new(&input.document_id).map_err(|_| CanvasUsecaseError::InvalidInput)?;
        ensure_document_write_permission(&actor_user_id, &workspace_id, &document_id, permissions)?;
        let canvas_id = CanvasId::new(&input.canvas_id).map_err(CanvasUsecaseError::from_canvas)?;
        repository
            .get_canvas(&workspace_id, &canvas_id)
            .map_err(CanvasUsecaseError::from_repository)?
            .ok_or(CanvasUsecaseError::CanvasNotFound)?;
        let embed = CanvasEmbed::new(canvas_id);
        Ok(CanvasEmbedOutput {
            reference: embed.reference().to_string(),
            product_log_event: "canvas.embedded",
        })
    }
}

impl Default for EmbedCanvasInDocumentUsecase {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ConvertDocumentOutlineToCanvasUsecase;

impl ConvertDocumentOutlineToCanvasUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ConvertDocumentOutlineToCanvasInput,
    ) -> Result<ConvertDocumentOutlineToCanvasOutput, CanvasUsecaseError> {
        let mut suggestions = Vec::with_capacity(input.headings.len());
        for (index, heading) in input.headings.into_iter().enumerate() {
            let heading_id = heading.heading_id.trim();
            let title = heading.title.trim();
            if heading_id.is_empty()
                || title.is_empty()
                || heading.level == 0
                || heading_id.chars().any(char::is_control)
                || title.chars().any(char::is_control)
            {
                return Err(CanvasUsecaseError::InvalidInput);
            }
            suggestions.push(CanvasLayoutSuggestion {
                heading_id: heading_id.to_string(),
                title: title.to_string(),
                level: heading.level,
                x: i32::from(heading.level.saturating_sub(1)) * 240,
                y: (index as i32) * 140,
            });
        }
        Ok(ConvertDocumentOutlineToCanvasOutput { suggestions })
    }
}

impl Default for ConvertDocumentOutlineToCanvasUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasUsecaseError {
    InvalidInput,
    PermissionDenied,
    CanvasNotFound,
    InvalidCanvasGraph,
    StorageUnavailable,
    CanvasAlreadyExists,
    VersionConflict,
    RecoveryRequired,
}

impl CanvasUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "CANVAS_INVALID_INPUT",
            Self::PermissionDenied => "CANVAS_PERMISSION_DENIED",
            Self::CanvasNotFound => "CANVAS_NOT_FOUND",
            Self::InvalidCanvasGraph => "CANVAS_INVALID_GRAPH",
            Self::StorageUnavailable => "CANVAS_STORAGE_UNAVAILABLE",
            Self::CanvasAlreadyExists => "CANVAS_ALREADY_EXISTS",
            Self::VersionConflict => "CANVAS_VERSION_CONFLICT",
            Self::RecoveryRequired => "CANVAS_RECOVERY_REQUIRED",
        }
    }

    const fn from_canvas(error: CanvasError) -> Self {
        match error {
            CanvasError::EmptyId
            | CanvasError::InvalidId
            | CanvasError::InvalidExternalLink
            | CanvasError::InvalidTextCard
            | CanvasError::InvalidRevision
            | CanvasError::RevisionOverflow
            | CanvasError::InvalidTitle
            | CanvasError::InvalidGeometry
            | CanvasError::InvalidViewport
            | CanvasError::InvalidGeometryPolicy => Self::InvalidInput,
            CanvasError::DuplicateNodeId
            | CanvasError::DuplicateEdgeId
            | CanvasError::MissingEdgeNode
            | CanvasError::SelfReferencingEdge
            | CanvasError::InvalidLifecycleTransition => Self::InvalidCanvasGraph,
        }
    }

    const fn from_repository(error: CanvasRepositoryError) -> Self {
        match error {
            CanvasRepositoryError::InvalidInput => Self::InvalidInput,
            CanvasRepositoryError::StorageUnavailable => Self::StorageUnavailable,
            CanvasRepositoryError::CorruptedCanvas | CanvasRepositoryError::UnsupportedSchema => {
                Self::RecoveryRequired
            }
            CanvasRepositoryError::AlreadyExists => Self::CanvasAlreadyExists,
            CanvasRepositoryError::VersionConflict => Self::VersionConflict,
        }
    }

    const fn from_permission(error: PermissionAwareQueryError) -> Self {
        match error {
            PermissionAwareQueryError::InvalidInput => Self::InvalidInput,
            PermissionAwareQueryError::NotFound
            | PermissionAwareQueryError::IndexStale
            | PermissionAwareQueryError::StorageUnavailable
            | PermissionAwareQueryError::CorruptedProjection => Self::StorageUnavailable,
        }
    }
}

fn ensure_write_permission(
    actor_user_id: &UserId,
    workspace_id: &WorkspaceId,
    permissions: &impl PermissionDecisionPort,
) -> Result<(), CanvasUsecaseError> {
    let decision = permissions
        .check_permission(
            actor_user_id,
            &AccessResource::workspace(workspace_id.clone()),
            Permission::Write,
        )
        .map_err(CanvasUsecaseError::from_permission)?;
    if decision.result() == PermissionDecisionResult::Allowed {
        Ok(())
    } else {
        Err(CanvasUsecaseError::PermissionDenied)
    }
}

fn ensure_document_write_permission(
    actor_user_id: &UserId,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
    permissions: &impl PermissionDecisionPort,
) -> Result<(), CanvasUsecaseError> {
    let decision = permissions
        .check_permission(
            actor_user_id,
            &AccessResource::document(workspace_id.clone(), None, document_id.clone()),
            Permission::Write,
        )
        .map_err(CanvasUsecaseError::from_permission)?;
    if decision.result() == PermissionDecisionResult::Allowed {
        Ok(())
    } else {
        Err(CanvasUsecaseError::PermissionDenied)
    }
}

fn target_from_input(
    input: AddCanvasNodeTargetInput,
) -> Result<CanvasNodeTarget, CanvasUsecaseError> {
    match input {
        AddCanvasNodeTargetInput::Document { document_id } => Ok(CanvasNodeTarget::Document(
            DocumentId::new(&document_id).map_err(|_| CanvasUsecaseError::InvalidInput)?,
        )),
        AddCanvasNodeTargetInput::Attachment { asset_sha256_hex } => {
            Ok(CanvasNodeTarget::Attachment(
                AssetId::from_sha256_hex(&asset_sha256_hex)
                    .map_err(|_| CanvasUsecaseError::InvalidInput)?,
            ))
        }
        AddCanvasNodeTargetInput::ExternalLink { url } => Ok(CanvasNodeTarget::ExternalLink(
            CanvasExternalLink::new(&url).map_err(CanvasUsecaseError::from_canvas)?,
        )),
        AddCanvasNodeTargetInput::TextCard { text } => Ok(CanvasNodeTarget::TextCard(
            CanvasTextCard::new(&text).map_err(CanvasUsecaseError::from_canvas)?,
        )),
    }
}

fn output_from_canvas(canvas: Canvas, product_log_event: &'static str) -> CanvasCommandOutput {
    CanvasCommandOutput {
        canvas_id: canvas.id().clone(),
        state: canvas.state(),
        node_count: canvas.nodes().len(),
        edge_count: canvas.edges().len(),
        product_log_event,
    }
}
