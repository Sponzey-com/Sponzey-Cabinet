use cabinet_domain::asset::{AssetId, AssetMetadata};
use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{AccessResource, Permission, PermissionDecision};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;

use crate::asset_store::AssetObject;
use crate::document_repository::CurrentDocumentRecord;
use crate::search_index::{SearchQuery, SearchResult};

pub trait PermissionDecisionPort {
    fn check_permission(
        &self,
        actor_user_id: &UserId,
        resource: &AccessResource,
        permission: Permission,
    ) -> Result<PermissionDecision, PermissionAwareQueryError>;
}

pub trait AccessibleDocumentQuery {
    fn get_current_by_id(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, PermissionAwareQueryError>;
}

pub trait PermissionAwareSearchIndex {
    fn search_accessible(
        &mut self,
        workspace_id: &WorkspaceId,
        filter: PermissionFilter,
        query: SearchQuery,
    ) -> Result<SearchAccessiblePage, PermissionAwareQueryError>;
}

pub trait AccessibleAssetQuery {
    fn get_metadata(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        asset_id: &AssetId,
    ) -> Result<Option<AssetMetadata>, PermissionAwareQueryError>;

    fn get_content(
        &self,
        workspace_id: &WorkspaceId,
        asset_id: &AssetId,
    ) -> Result<Option<AssetObject>, PermissionAwareQueryError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionFilter {
    actor_user_id: UserId,
    permission: Permission,
}

impl PermissionFilter {
    pub fn new(actor_user_id: UserId, permission: Permission) -> Self {
        Self {
            actor_user_id,
            permission,
        }
    }

    pub fn actor_user_id(&self) -> &UserId {
        &self.actor_user_id
    }

    pub const fn permission(&self) -> Permission {
        self.permission
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAccessiblePage {
    results: Vec<SearchResult>,
    stats: PermissionQueryStats,
}

impl SearchAccessiblePage {
    pub fn new(results: Vec<SearchResult>, stats: PermissionQueryStats) -> Self {
        Self { results, stats }
    }

    pub fn results(&self) -> &[SearchResult] {
        &self.results
    }

    pub const fn stats(&self) -> PermissionQueryStats {
        self.stats
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionQueryStats {
    candidate_count: usize,
    filtered_count: usize,
    cache_hit: bool,
}

impl PermissionQueryStats {
    pub const fn new(candidate_count: usize, filtered_count: usize, cache_hit: bool) -> Self {
        Self {
            candidate_count,
            filtered_count,
            cache_hit,
        }
    }

    pub const fn candidate_count(self) -> usize {
        self.candidate_count
    }

    pub const fn filtered_count(self) -> usize {
        self.filtered_count
    }

    pub const fn cache_hit(self) -> bool {
        self.cache_hit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionAwareQueryError {
    InvalidInput,
    NotFound,
    IndexStale,
    StorageUnavailable,
    CorruptedProjection,
}

impl PermissionAwareQueryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "permission_aware_query.invalid_input",
            Self::NotFound => "permission_aware_query.not_found",
            Self::IndexStale => "permission_aware_query.index_stale",
            Self::StorageUnavailable => "permission_aware_query.storage_unavailable",
            Self::CorruptedProjection => "permission_aware_query.corrupted_projection",
        }
    }
}
