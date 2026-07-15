use cabinet_domain::asset::{AssetId, AssetMetadata};
use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{AccessResource, Permission, PermissionDecisionResult};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_store::AssetObject;
use cabinet_ports::document_repository::CurrentDocumentRecord;
use cabinet_ports::permission_aware_query::{
    AccessibleAssetQuery, AccessibleDocumentQuery, PermissionAwareQueryError,
    PermissionAwareSearchIndex, PermissionDecisionPort, PermissionFilter, PermissionQueryStats,
    SearchAccessiblePage,
};
use cabinet_ports::search_index::{SearchIndexError, SearchQuery};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetAccessibleDocumentInput {
    actor_user_id: String,
    workspace_id: String,
    collection_id: Option<String>,
    document_id: String,
}

impl GetAccessibleDocumentInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        collection_id: Option<&str>,
        document_id: &str,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            collection_id: collection_id.map(str::to_string),
            document_id: document_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetAccessibleDocumentOutput {
    record: CurrentDocumentRecord,
}

impl GetAccessibleDocumentOutput {
    pub fn record(&self) -> &CurrentDocumentRecord {
        &self.record
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAccessibleDocumentsInput {
    actor_user_id: String,
    workspace_id: String,
    query_text: String,
    limit: usize,
}

impl SearchAccessibleDocumentsInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, query_text: &str, limit: usize) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            query_text: query_text.to_string(),
            limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAccessibleDocumentsOutput {
    page: SearchAccessiblePage,
}

impl SearchAccessibleDocumentsOutput {
    pub fn page(&self) -> &SearchAccessiblePage {
        &self.page
    }

    pub const fn stats(&self) -> PermissionQueryStats {
        self.page.stats()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibleAssetMetadataInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: Option<String>,
    asset_id: String,
}

impl AccessibleAssetMetadataInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        document_id: Option<&str>,
        asset_id: &str,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.map(str::to_string),
            asset_id: asset_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibleAssetMetadataOutput {
    metadata: AssetMetadata,
}

impl AccessibleAssetMetadataOutput {
    pub fn metadata(&self) -> &AssetMetadata {
        &self.metadata
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibleAssetContentInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: Option<String>,
    asset_id: String,
}

impl AccessibleAssetContentInput {
    pub fn new(
        actor_user_id: &str,
        workspace_id: &str,
        document_id: Option<&str>,
        asset_id: &str,
    ) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.map(str::to_string),
            asset_id: asset_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibleAssetContentOutput {
    object: AssetObject,
}

impl AccessibleAssetContentOutput {
    pub fn object(&self) -> &AssetObject {
        &self.object
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessibleQueryProductEvent {
    DocumentAccessDenied {
        masked_actor_id: String,
        masked_target_id: String,
        error_code: &'static str,
    },
    SearchQueryFailed {
        result_count_bucket: &'static str,
        error_code: &'static str,
    },
    AssetAccessDenied {
        masked_actor_id: String,
        masked_target_id: String,
        error_code: &'static str,
    },
    UsecaseFailed {
        error_code: &'static str,
    },
}

impl AccessibleQueryProductEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::DocumentAccessDenied { .. } => "document.access.denied",
            Self::SearchQueryFailed { .. } => "search.query.failed",
            Self::AssetAccessDenied { .. } => "asset.access.denied",
            Self::UsecaseFailed { .. } => "permission_query.usecase.failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibleQueryFieldDebugEvent {
    query_hash: String,
    candidate_count: usize,
    filtered_count: usize,
    permission_summary: &'static str,
    cache_hit: bool,
}

impl AccessibleQueryFieldDebugEvent {
    pub fn query_hash(&self) -> &str {
        &self.query_hash
    }

    pub const fn candidate_count(&self) -> usize {
        self.candidate_count
    }

    pub const fn filtered_count(&self) -> usize {
        self.filtered_count
    }

    pub const fn permission_summary(&self) -> &'static str {
        self.permission_summary
    }

    pub const fn cache_hit(&self) -> bool {
        self.cache_hit
    }
}

pub trait AccessibleQueryLogger {
    fn write_product(&mut self, event: AccessibleQueryProductEvent);
    fn write_field_debug(&mut self, event: AccessibleQueryFieldDebugEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetAccessibleDocumentUsecase;

impl GetAccessibleDocumentUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: GetAccessibleDocumentInput,
        permission_checker: &impl PermissionDecisionPort,
        document_query: &impl AccessibleDocumentQuery,
        logger: &mut impl AccessibleQueryLogger,
    ) -> Result<GetAccessibleDocumentOutput, GetAccessibleDocumentError> {
        let actor_user_id = parse_user_id(&input.actor_user_id).map_err(|error| {
            log_document_error(logger, GetAccessibleDocumentError::from_query_error(error))
        })?;
        let workspace_id = parse_workspace_id(&input.workspace_id).map_err(|error| {
            log_document_error(logger, GetAccessibleDocumentError::from_query_error(error))
        })?;
        let collection_id =
            parse_optional_collection_id(input.collection_id.as_deref()).map_err(|error| {
                log_document_error(logger, GetAccessibleDocumentError::from_query_error(error))
            })?;
        let document_id = DocumentId::new(&input.document_id).map_err(|_| {
            log_document_error(logger, GetAccessibleDocumentError::InvalidInput);
            GetAccessibleDocumentError::InvalidInput
        })?;
        let resource =
            AccessResource::document(workspace_id.clone(), collection_id, document_id.clone());
        let decision = permission_checker
            .check_permission(&actor_user_id, &resource, Permission::Read)
            .map_err(|error| {
                log_document_error(logger, GetAccessibleDocumentError::from_query_error(error))
            })?;
        if decision.result() != PermissionDecisionResult::Allowed {
            logger.write_product(AccessibleQueryProductEvent::DocumentAccessDenied {
                masked_actor_id: mask_user_id(&actor_user_id),
                masked_target_id: resource_target_id(&resource),
                error_code: decision.reason_code(),
            });
            return Err(GetAccessibleDocumentError::NotFound);
        }

        match document_query.get_current_by_id(&workspace_id, &document_id) {
            Ok(Some(record)) => Ok(GetAccessibleDocumentOutput { record }),
            Ok(None) => Err(GetAccessibleDocumentError::NotFound),
            Err(error) => {
                let mapped = GetAccessibleDocumentError::from_query_error(error);
                log_document_error(logger, mapped);
                Err(mapped)
            }
        }
    }
}

impl Default for GetAccessibleDocumentUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchAccessibleDocumentsUsecase;

impl SearchAccessibleDocumentsUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: SearchAccessibleDocumentsInput,
        search_index: &mut impl PermissionAwareSearchIndex,
        logger: &mut impl AccessibleQueryLogger,
    ) -> Result<SearchAccessibleDocumentsOutput, SearchAccessibleDocumentsError> {
        let actor_user_id = parse_user_id(&input.actor_user_id).map_err(|error| {
            log_search_error(
                logger,
                SearchAccessibleDocumentsError::from_query_error(error),
            )
        })?;
        let workspace_id = parse_workspace_id(&input.workspace_id).map_err(|error| {
            log_search_error(
                logger,
                SearchAccessibleDocumentsError::from_query_error(error),
            )
        })?;
        let query_hash = query_hash(&input.query_text);
        let query = SearchQuery::new(&input.query_text, input.limit).map_err(|error| {
            let mapped = SearchAccessibleDocumentsError::from_search_error(error);
            log_search_error(logger, mapped);
            mapped
        })?;
        let page = search_index
            .search_accessible(
                &workspace_id,
                PermissionFilter::new(actor_user_id, Permission::Read),
                query,
            )
            .map_err(|error| {
                let mapped = SearchAccessibleDocumentsError::from_query_error(error);
                log_search_error(logger, mapped);
                mapped
            })?;
        let stats = page.stats();
        logger.write_field_debug(AccessibleQueryFieldDebugEvent {
            query_hash,
            candidate_count: stats.candidate_count(),
            filtered_count: stats.filtered_count(),
            permission_summary: "permission_filtered_in_query_port",
            cache_hit: stats.cache_hit(),
        });
        Ok(SearchAccessibleDocumentsOutput { page })
    }
}

impl Default for SearchAccessibleDocumentsUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessibleAssetMetadataUsecase;

impl AccessibleAssetMetadataUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: AccessibleAssetMetadataInput,
        permission_checker: &impl PermissionDecisionPort,
        asset_query: &impl AccessibleAssetQuery,
        logger: &mut impl AccessibleQueryLogger,
    ) -> Result<AccessibleAssetMetadataOutput, AccessibleAssetQueryError> {
        let parsed = parse_asset_input(
            &input.actor_user_id,
            &input.workspace_id,
            input.document_id.as_deref(),
            &input.asset_id,
        )
        .map_err(|error| {
            log_asset_error(logger, AccessibleAssetQueryError::from_query_error(error))
        })?;
        let resource = AccessResource::asset(
            parsed.workspace_id.clone(),
            None,
            parsed.document_id.clone(),
            parsed.asset_id.clone(),
        );
        let decision = permission_checker
            .check_permission(
                &parsed.actor_user_id,
                &resource,
                Permission::ReadAssetMetadata,
            )
            .map_err(|error| {
                log_asset_error(logger, AccessibleAssetQueryError::from_query_error(error))
            })?;
        if decision.result() != PermissionDecisionResult::Allowed {
            log_asset_denied(
                logger,
                &parsed.actor_user_id,
                &resource,
                decision.reason_code(),
            );
            return Err(AccessibleAssetQueryError::NotFound);
        }

        let document_id = parsed.document_id.as_ref().ok_or_else(|| {
            log_asset_error(logger, AccessibleAssetQueryError::InvalidInput);
            AccessibleAssetQueryError::InvalidInput
        })?;
        match asset_query.get_metadata(&parsed.workspace_id, document_id, &parsed.asset_id) {
            Ok(Some(metadata)) => Ok(AccessibleAssetMetadataOutput { metadata }),
            Ok(None) => Err(AccessibleAssetQueryError::NotFound),
            Err(error) => {
                let mapped = AccessibleAssetQueryError::from_query_error(error);
                log_asset_error(logger, mapped);
                Err(mapped)
            }
        }
    }
}

impl Default for AccessibleAssetMetadataUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessibleAssetContentUsecase;

impl AccessibleAssetContentUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: AccessibleAssetContentInput,
        permission_checker: &impl PermissionDecisionPort,
        asset_query: &impl AccessibleAssetQuery,
        logger: &mut impl AccessibleQueryLogger,
    ) -> Result<AccessibleAssetContentOutput, AccessibleAssetQueryError> {
        let parsed = parse_asset_input(
            &input.actor_user_id,
            &input.workspace_id,
            input.document_id.as_deref(),
            &input.asset_id,
        )
        .map_err(|error| {
            log_asset_error(logger, AccessibleAssetQueryError::from_query_error(error))
        })?;
        let resource = AccessResource::asset(
            parsed.workspace_id.clone(),
            None,
            parsed.document_id.clone(),
            parsed.asset_id.clone(),
        );
        let decision = permission_checker
            .check_permission(
                &parsed.actor_user_id,
                &resource,
                Permission::ReadAssetContent,
            )
            .map_err(|error| {
                log_asset_error(logger, AccessibleAssetQueryError::from_query_error(error))
            })?;
        if decision.result() != PermissionDecisionResult::Allowed {
            log_asset_denied(
                logger,
                &parsed.actor_user_id,
                &resource,
                decision.reason_code(),
            );
            return Err(AccessibleAssetQueryError::NotFound);
        }

        match asset_query.get_content(&parsed.workspace_id, &parsed.asset_id) {
            Ok(Some(object)) => Ok(AccessibleAssetContentOutput { object }),
            Ok(None) => Err(AccessibleAssetQueryError::NotFound),
            Err(error) => {
                let mapped = AccessibleAssetQueryError::from_query_error(error);
                log_asset_error(logger, mapped);
                Err(mapped)
            }
        }
    }
}

impl Default for AccessibleAssetContentUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetAccessibleDocumentError {
    InvalidInput,
    NotFound,
    IndexStale,
    StorageUnavailable,
}

impl GetAccessibleDocumentError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_ACCESSIBLE_QUERY_INPUT",
            Self::NotFound => "ACCESSIBLE_DOCUMENT_NOT_FOUND",
            Self::IndexStale => "QUERY_INDEX_STALE",
            Self::StorageUnavailable => "ACCESSIBLE_QUERY_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_query_error(error: PermissionAwareQueryError) -> Self {
        match error {
            PermissionAwareQueryError::InvalidInput => Self::InvalidInput,
            PermissionAwareQueryError::NotFound => Self::NotFound,
            PermissionAwareQueryError::IndexStale => Self::IndexStale,
            PermissionAwareQueryError::StorageUnavailable
            | PermissionAwareQueryError::CorruptedProjection => Self::StorageUnavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAccessibleDocumentsError {
    InvalidInput,
    IndexStale,
    StorageUnavailable,
}

impl SearchAccessibleDocumentsError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_ACCESSIBLE_QUERY_INPUT",
            Self::IndexStale => "QUERY_INDEX_STALE",
            Self::StorageUnavailable => "ACCESSIBLE_QUERY_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_query_error(error: PermissionAwareQueryError) -> Self {
        match error {
            PermissionAwareQueryError::InvalidInput => Self::InvalidInput,
            PermissionAwareQueryError::IndexStale => Self::IndexStale,
            PermissionAwareQueryError::NotFound
            | PermissionAwareQueryError::StorageUnavailable
            | PermissionAwareQueryError::CorruptedProjection => Self::StorageUnavailable,
        }
    }

    const fn from_search_error(error: SearchIndexError) -> Self {
        match error {
            SearchIndexError::InvalidQuery
            | SearchIndexError::InvalidLimit
            | SearchIndexError::InvalidSnippet => Self::InvalidInput,
            SearchIndexError::CorruptedIndex => Self::IndexStale,
            SearchIndexError::StorageUnavailable => Self::StorageUnavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibleAssetQueryError {
    InvalidInput,
    NotFound,
    IndexStale,
    StorageUnavailable,
}

impl AccessibleAssetQueryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_ACCESSIBLE_QUERY_INPUT",
            Self::NotFound => "ACCESSIBLE_ASSET_NOT_FOUND",
            Self::IndexStale => "QUERY_INDEX_STALE",
            Self::StorageUnavailable => "ACCESSIBLE_QUERY_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_query_error(error: PermissionAwareQueryError) -> Self {
        match error {
            PermissionAwareQueryError::InvalidInput => Self::InvalidInput,
            PermissionAwareQueryError::NotFound => Self::NotFound,
            PermissionAwareQueryError::IndexStale => Self::IndexStale,
            PermissionAwareQueryError::StorageUnavailable
            | PermissionAwareQueryError::CorruptedProjection => Self::StorageUnavailable,
        }
    }
}

struct ParsedAssetInput {
    actor_user_id: UserId,
    workspace_id: WorkspaceId,
    document_id: Option<DocumentId>,
    asset_id: AssetId,
}

fn parse_asset_input(
    actor_user_id: &str,
    workspace_id: &str,
    document_id: Option<&str>,
    asset_id: &str,
) -> Result<ParsedAssetInput, PermissionAwareQueryError> {
    Ok(ParsedAssetInput {
        actor_user_id: parse_user_id(actor_user_id)?,
        workspace_id: parse_workspace_id(workspace_id)?,
        document_id: parse_optional_document_id(document_id)?,
        asset_id: AssetId::from_sha256_hex(asset_id)
            .map_err(|_| PermissionAwareQueryError::InvalidInput)?,
    })
}

fn parse_user_id(value: &str) -> Result<UserId, PermissionAwareQueryError> {
    UserId::new(value).map_err(|_| PermissionAwareQueryError::InvalidInput)
}

fn parse_workspace_id(value: &str) -> Result<WorkspaceId, PermissionAwareQueryError> {
    WorkspaceId::new(value).map_err(|_| PermissionAwareQueryError::InvalidInput)
}

fn parse_optional_collection_id(
    value: Option<&str>,
) -> Result<Option<cabinet_domain::permission::CollectionId>, PermissionAwareQueryError> {
    value
        .map(cabinet_domain::permission::CollectionId::new)
        .transpose()
        .map_err(|_| PermissionAwareQueryError::InvalidInput)
}

fn parse_optional_document_id(
    value: Option<&str>,
) -> Result<Option<DocumentId>, PermissionAwareQueryError> {
    value
        .map(DocumentId::new)
        .transpose()
        .map_err(|_| PermissionAwareQueryError::InvalidInput)
}

fn log_document_error(
    logger: &mut impl AccessibleQueryLogger,
    error: GetAccessibleDocumentError,
) -> GetAccessibleDocumentError {
    logger.write_product(AccessibleQueryProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
    error
}

fn log_search_error(
    logger: &mut impl AccessibleQueryLogger,
    error: SearchAccessibleDocumentsError,
) -> SearchAccessibleDocumentsError {
    logger.write_product(AccessibleQueryProductEvent::SearchQueryFailed {
        result_count_bucket: "0",
        error_code: error.code(),
    });
    error
}

fn log_asset_error(
    logger: &mut impl AccessibleQueryLogger,
    error: AccessibleAssetQueryError,
) -> AccessibleAssetQueryError {
    logger.write_product(AccessibleQueryProductEvent::UsecaseFailed {
        error_code: error.code(),
    });
    error
}

fn log_asset_denied(
    logger: &mut impl AccessibleQueryLogger,
    actor_user_id: &UserId,
    resource: &AccessResource,
    error_code: &'static str,
) {
    logger.write_product(AccessibleQueryProductEvent::AssetAccessDenied {
        masked_actor_id: mask_user_id(actor_user_id),
        masked_target_id: resource_target_id(resource),
        error_code,
    });
}

fn resource_target_id(resource: &AccessResource) -> String {
    match resource {
        AccessResource::Workspace { workspace_id } => {
            format!("workspace:{}", mask_raw_id(workspace_id.as_str()))
        }
        AccessResource::Collection { collection_id, .. } => {
            format!("collection:{}", mask_raw_id(collection_id.as_str()))
        }
        AccessResource::Document { document_id, .. } => {
            format!("document:{}", mask_raw_id(document_id.as_str()))
        }
        AccessResource::Asset { asset_id, .. } => {
            format!("asset:{}", mask_raw_id(asset_id.as_str()))
        }
    }
}

fn mask_user_id(user_id: &UserId) -> String {
    mask_raw_id(user_id.as_str())
}

fn mask_raw_id(value: &str) -> String {
    let suffix_start = value.len().saturating_sub(4);
    format!("masked:{}", &value[suffix_start..])
}

fn query_hash(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("qhash:{hash:016x}")
}
