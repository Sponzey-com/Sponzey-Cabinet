use cabinet_domain::canvas::{CanvasNode, CanvasNodeTarget};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_metadata_catalog::{AssetMetadataCatalog, AssetMetadataCatalogError};
use cabinet_ports::document_title_reader::{
    DocumentTitleLookup, DocumentTitleReader, DocumentTitleReaderError,
};

#[derive(Debug, Clone)]
pub struct ResolveCanvasTargetPresentationsInput<'a> {
    workspace_id: String,
    nodes: &'a [CanvasNode],
}
impl<'a> ResolveCanvasTargetPresentationsInput<'a> {
    pub fn new(workspace_id: &str, nodes: &'a [CanvasNode]) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            nodes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasTargetStatus {
    Available,
    Missing,
}
impl CanvasTargetStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasTargetPresentation {
    node_id: String,
    target_id: String,
    display_label: String,
    status: CanvasTargetStatus,
}
impl CanvasTargetPresentation {
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
    pub fn target_id(&self) -> &str {
        &self.target_id
    }
    pub fn display_label(&self) -> &str {
        &self.display_label
    }
    pub const fn status(&self) -> CanvasTargetStatus {
        self.status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveCanvasTargetPresentationsOutput {
    presentations: Vec<CanvasTargetPresentation>,
}
impl ResolveCanvasTargetPresentationsOutput {
    pub fn presentations(&self) -> &[CanvasTargetPresentation] {
        &self.presentations
    }
}

pub struct ResolveCanvasTargetPresentationsUsecase;
impl ResolveCanvasTargetPresentationsUsecase {
    pub const fn new() -> Self {
        Self
    }
    pub fn execute(
        &self,
        input: ResolveCanvasTargetPresentationsInput<'_>,
        documents: &impl DocumentTitleReader,
        assets: &impl AssetMetadataCatalog,
    ) -> Result<ResolveCanvasTargetPresentationsOutput, ResolveCanvasTargetPresentationsError> {
        let workspace = WorkspaceId::new(&input.workspace_id)
            .map_err(|_| ResolveCanvasTargetPresentationsError::InvalidInput)?;
        let mut document_ids = Vec::new();
        for node in input.nodes {
            if let CanvasNodeTarget::Document(document) = node.target() {
                if !document_ids
                    .iter()
                    .any(|existing: &cabinet_domain::document::DocumentId| existing == document)
                {
                    document_ids.push(document.clone());
                }
            }
        }
        let document_titles = if document_ids.is_empty() {
            Vec::new()
        } else {
            documents
                .get_current_titles(&workspace, &document_ids)
                .map_err(map_document)?
        };
        let mut presentations = Vec::with_capacity(input.nodes.len());
        for node in input.nodes {
            let (target_id, display_label, status) = match node.target() {
                CanvasNodeTarget::Document(document) => {
                    match find_document_title(&document_titles, document) {
                        Some(title) => (
                            document.as_str(),
                            title.as_str().to_string(),
                            CanvasTargetStatus::Available,
                        ),
                        None => (
                            document.as_str(),
                            "찾을 수 없는 문서".to_string(),
                            CanvasTargetStatus::Missing,
                        ),
                    }
                }
                CanvasNodeTarget::Attachment(asset) => {
                    match assets.get(&workspace, asset).map_err(map_asset)? {
                        Some(record) => (
                            asset.as_str(),
                            record.metadata().file_name().as_str().to_string(),
                            CanvasTargetStatus::Available,
                        ),
                        None => (
                            asset.as_str(),
                            "찾을 수 없는 첨부 파일".to_string(),
                            CanvasTargetStatus::Missing,
                        ),
                    }
                }
                CanvasNodeTarget::ExternalLink(link) => (
                    link.as_str(),
                    link.as_str().to_string(),
                    CanvasTargetStatus::Available,
                ),
                CanvasNodeTarget::TextCard(text) => (
                    text.as_str(),
                    text.as_str().to_string(),
                    CanvasTargetStatus::Available,
                ),
            };
            presentations.push(CanvasTargetPresentation {
                node_id: node.id().as_str().to_string(),
                target_id: target_id.to_string(),
                display_label,
                status,
            });
        }
        Ok(ResolveCanvasTargetPresentationsOutput { presentations })
    }
}

fn find_document_title<'a>(
    lookups: &'a [DocumentTitleLookup],
    document: &cabinet_domain::document::DocumentId,
) -> Option<&'a cabinet_domain::document::DocumentTitle> {
    lookups
        .iter()
        .find(|lookup| lookup.document_id() == document)
        .and_then(DocumentTitleLookup::title)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveCanvasTargetPresentationsError {
    InvalidInput,
    StorageUnavailable,
    RecoveryRequired,
}
impl ResolveCanvasTargetPresentationsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "CANVAS_TARGET_INVALID_INPUT",
            Self::StorageUnavailable => "CANVAS_TARGET_STORAGE_UNAVAILABLE",
            Self::RecoveryRequired => "CANVAS_TARGET_RECOVERY_REQUIRED",
        }
    }
}
fn map_document(error: DocumentTitleReaderError) -> ResolveCanvasTargetPresentationsError {
    match error {
        DocumentTitleReaderError::StorageUnavailable => {
            ResolveCanvasTargetPresentationsError::StorageUnavailable
        }
        DocumentTitleReaderError::CorruptedMetadata => {
            ResolveCanvasTargetPresentationsError::RecoveryRequired
        }
    }
}
fn map_asset(error: AssetMetadataCatalogError) -> ResolveCanvasTargetPresentationsError {
    match error {
        AssetMetadataCatalogError::StorageUnavailable => {
            ResolveCanvasTargetPresentationsError::StorageUnavailable
        }
        _ => ResolveCanvasTargetPresentationsError::RecoveryRequired,
    }
}
