use std::path::PathBuf;

use cabinet_domain::canvas::CanvasLifecycleState;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_metadata_catalog::AssetMetadataCatalog;
use cabinet_ports::current_document_projection_catalog::CurrentDocumentProjectionCatalog;
use cabinet_ports::workspace_home::{
    WorkspaceHomeHealthStatus, WorkspaceHomeProjection, WorkspaceHomeProjectionError,
    WorkspaceHomeProjectionLimits, WorkspaceHomeProjectionPort, WorkspaceHomeSummaryKind,
    WorkspaceHomeSummaryProjection,
};

use crate::durable_asset_metadata_catalog::DurableAssetMetadataCatalog;
use crate::durable_canvas_repository::DurableCanvasRepository;
use crate::local_current_document_projection_catalog::LocalCurrentDocumentProjectionCatalog;
use crate::local_workspace_home_projection::LocalWorkspaceHomeProjectionStore;

const SUMMARY_ITEM_LIMIT: usize = 100_000;
const ASSET_PAGE_LIMIT: usize = 500;

#[derive(Debug, Clone)]
pub struct LocalWorkspaceHomeQueryStore {
    root: PathBuf,
    home: LocalWorkspaceHomeProjectionStore,
}

impl LocalWorkspaceHomeQueryStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            home: LocalWorkspaceHomeProjectionStore::new(root.clone()),
            root,
        }
    }

    fn load_summary(&self, workspace_id: &WorkspaceId) -> WorkspaceHomeSummaryProjection {
        let documents = LocalCurrentDocumentProjectionCatalog::new(self.root.clone())
            .list_current_projection_identities(workspace_id, SUMMARY_ITEM_LIMIT);
        let assets = self.count_assets(workspace_id);
        let canvases = DurableCanvasRepository::new(self.root.clone())
            .list_current_canvas_records(SUMMARY_ITEM_LIMIT)
            .map(|items| {
                items
                    .into_iter()
                    .filter(|item| item.workspace_id() == workspace_id)
                    .filter(|item| item.record().canvas().state() != CanvasLifecycleState::Archived)
                    .count()
            });

        let mut summary = WorkspaceHomeSummaryProjection::new(
            documents.as_ref().map_or(0, Vec::len) as u32,
            assets.as_ref().copied().unwrap_or(0) as u32,
            canvases.as_ref().copied().unwrap_or(0) as u32,
        );
        if documents.is_err() {
            summary = summary.with_unavailable(WorkspaceHomeSummaryKind::Documents);
        }
        if assets.is_err() {
            summary = summary.with_unavailable(WorkspaceHomeSummaryKind::Assets);
        }
        if canvases.is_err() {
            summary = summary.with_unavailable(WorkspaceHomeSummaryKind::Canvases);
        }
        summary
    }

    fn count_assets(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<usize, WorkspaceHomeProjectionError> {
        let catalog = DurableAssetMetadataCatalog::new(self.root.clone());
        let mut cursor = None;
        let mut count = 0usize;
        loop {
            let page = catalog
                .list(workspace_id, cursor.as_deref(), ASSET_PAGE_LIMIT)
                .map_err(|_| WorkspaceHomeProjectionError::StorageUnavailable)?;
            count = count.saturating_add(page.records().len());
            if count > SUMMARY_ITEM_LIMIT {
                return Err(WorkspaceHomeProjectionError::InvalidLimit);
            }
            let Some(next) = page.next_cursor() else {
                return Ok(count);
            };
            if cursor.as_deref() == Some(next) {
                return Err(WorkspaceHomeProjectionError::CorruptedProjection);
            }
            cursor = Some(next.to_string());
        }
    }
}

impl WorkspaceHomeProjectionPort for LocalWorkspaceHomeQueryStore {
    fn load_workspace_home(
        &self,
        workspace_id: &WorkspaceId,
        limits: WorkspaceHomeProjectionLimits,
    ) -> Result<WorkspaceHomeProjection, WorkspaceHomeProjectionError> {
        let projection = self.home.load_workspace_home(workspace_id, limits)?;
        let summary = self.load_summary(workspace_id);
        let projection = projection.with_summary(summary);
        Ok(if summary.is_complete() {
            projection
        } else {
            projection.with_health_status(WorkspaceHomeHealthStatus::Degraded)
        })
    }
}
