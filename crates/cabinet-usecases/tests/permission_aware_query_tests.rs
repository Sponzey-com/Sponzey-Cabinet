use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::asset::{AssetFileName, AssetId, AssetMediaType, AssetMetadata};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::permission::{
    AccessResource, Permission, PermissionDecision, PermissionDecisionReason, PolicySource,
};
use cabinet_domain::user::UserId;
use cabinet_domain::version::CurrentDocumentSnapshot;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::asset_store::AssetObject;
use cabinet_ports::document_repository::CurrentDocumentRecord;
use cabinet_ports::permission_aware_query::{
    AccessibleAssetQuery, AccessibleDocumentQuery, PermissionAwareQueryError,
    PermissionAwareSearchIndex, PermissionDecisionPort, PermissionFilter, PermissionQueryStats,
    SearchAccessiblePage,
};
use cabinet_ports::search_index::{SearchQuery, SearchResult};
use cabinet_usecases::permission_query::{
    AccessibleAssetContentInput, AccessibleAssetContentUsecase, AccessibleAssetMetadataInput,
    AccessibleAssetMetadataUsecase, AccessibleAssetQueryError, AccessibleQueryFieldDebugEvent,
    AccessibleQueryLogger, AccessibleQueryProductEvent, GetAccessibleDocumentError,
    GetAccessibleDocumentInput, GetAccessibleDocumentUsecase, SearchAccessibleDocumentsError,
    SearchAccessibleDocumentsInput, SearchAccessibleDocumentsUsecase,
};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(Default)]
struct FakePermissionChecker {
    decisions: HashMap<String, PermissionDecision>,
    check_count: Cell<usize>,
}

impl FakePermissionChecker {
    fn allow(&mut self, resource_key: &str, permission: Permission) {
        self.decisions.insert(
            decision_key(resource_key, permission),
            PermissionDecision::allowed(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleAllowsPermission,
            ),
        );
    }

    fn deny(&mut self, resource_key: &str, permission: Permission) {
        self.decisions.insert(
            decision_key(resource_key, permission),
            PermissionDecision::denied(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleDoesNotAllowPermission,
            ),
        );
    }
}

impl PermissionDecisionPort for FakePermissionChecker {
    fn check_permission(
        &self,
        _actor_user_id: &UserId,
        resource: &AccessResource,
        permission: Permission,
    ) -> Result<PermissionDecision, PermissionAwareQueryError> {
        self.check_count.set(self.check_count.get() + 1);
        Ok(self
            .decisions
            .get(&decision_key(&resource_key(resource), permission))
            .copied()
            .unwrap_or_else(|| {
                PermissionDecision::denied(
                    PolicySource::Workspace,
                    PermissionDecisionReason::RoleDoesNotAllowPermission,
                )
            }))
    }
}

#[derive(Default)]
struct FakeDocumentQuery {
    records: HashMap<(String, String), CurrentDocumentRecord>,
    current_read_count: Cell<usize>,
    history_scan_count: Cell<usize>,
}

impl FakeDocumentQuery {
    fn insert(&mut self, workspace_id: &str, record: CurrentDocumentRecord) {
        self.records.insert(
            (
                workspace_id.to_string(),
                record.document_id().as_str().to_string(),
            ),
            record,
        );
    }
}

impl AccessibleDocumentQuery for FakeDocumentQuery {
    fn get_current_by_id(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, PermissionAwareQueryError> {
        self.current_read_count
            .set(self.current_read_count.get() + 1);
        Ok(self
            .records
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
            ))
            .cloned())
    }
}

struct FakeSearchIndex {
    page: SearchAccessiblePage,
    fail_stale: bool,
    search_count: Cell<usize>,
    last_filter: Option<PermissionFilter>,
    last_query_text: Option<String>,
}

impl Default for FakeSearchIndex {
    fn default() -> Self {
        Self {
            page: SearchAccessiblePage::new(
                vec![search_result(
                    "doc-1",
                    "Allowed",
                    "docs/allowed.md",
                    "alpha",
                )],
                PermissionQueryStats::new(3, 2, false),
            ),
            fail_stale: false,
            search_count: Cell::new(0),
            last_filter: None,
            last_query_text: None,
        }
    }
}

impl PermissionAwareSearchIndex for FakeSearchIndex {
    fn search_accessible(
        &mut self,
        workspace_id: &WorkspaceId,
        filter: PermissionFilter,
        query: SearchQuery,
    ) -> Result<SearchAccessiblePage, PermissionAwareQueryError> {
        self.search_count.set(self.search_count.get() + 1);
        assert_eq!(workspace_id.as_str(), "workspace-1");
        self.last_filter = Some(filter);
        self.last_query_text = Some(query.text().to_string());
        if self.fail_stale {
            return Err(PermissionAwareQueryError::IndexStale);
        }
        Ok(self.page.clone())
    }
}

#[derive(Default)]
struct FakeAssetQuery {
    metadata: HashMap<(String, String, String), AssetMetadata>,
    content: HashMap<(String, String), AssetObject>,
    metadata_read_count: Cell<usize>,
    content_read_count: Cell<usize>,
}

impl FakeAssetQuery {
    fn insert(&mut self, workspace_id: &str, document_id: &str, metadata: AssetMetadata) {
        let object =
            AssetObject::new(metadata.id().clone(), vec![1, 2, 3, 4]).expect("asset object");
        self.content.insert(
            (workspace_id.to_string(), metadata.id().as_str().to_string()),
            object,
        );
        self.metadata.insert(
            (
                workspace_id.to_string(),
                document_id.to_string(),
                metadata.id().as_str().to_string(),
            ),
            metadata,
        );
    }
}

impl AccessibleAssetQuery for FakeAssetQuery {
    fn get_metadata(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        asset_id: &AssetId,
    ) -> Result<Option<AssetMetadata>, PermissionAwareQueryError> {
        self.metadata_read_count
            .set(self.metadata_read_count.get() + 1);
        Ok(self
            .metadata
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
                asset_id.as_str().to_string(),
            ))
            .cloned())
    }

    fn get_content(
        &self,
        workspace_id: &WorkspaceId,
        asset_id: &AssetId,
    ) -> Result<Option<AssetObject>, PermissionAwareQueryError> {
        self.content_read_count
            .set(self.content_read_count.get() + 1);
        Ok(self
            .content
            .get(&(
                workspace_id.as_str().to_string(),
                asset_id.as_str().to_string(),
            ))
            .cloned())
    }
}

#[derive(Default)]
struct FakeQueryLogger {
    product_events: Vec<AccessibleQueryProductEvent>,
    field_debug_events: Vec<AccessibleQueryFieldDebugEvent>,
}

impl AccessibleQueryLogger for FakeQueryLogger {
    fn write_product(&mut self, event: AccessibleQueryProductEvent) {
        self.product_events.push(event);
    }

    fn write_field_debug(&mut self, event: AccessibleQueryFieldDebugEvent) {
        self.field_debug_events.push(event);
    }
}

#[test]
fn get_accessible_document_masks_denied_without_current_or_history_read() {
    let mut checker = FakePermissionChecker::default();
    checker.deny("document:doc-1", Permission::Read);
    let mut documents = FakeDocumentQuery::default();
    documents.insert("workspace-1", current_record("doc-1", "Private body"));
    let mut logger = FakeQueryLogger::default();

    let error = GetAccessibleDocumentUsecase::new()
        .execute(
            GetAccessibleDocumentInput::new("actor-1", "workspace-1", None, "doc-1"),
            &checker,
            &documents,
            &mut logger,
        )
        .expect_err("denied document is masked");

    assert_eq!(error, GetAccessibleDocumentError::NotFound);
    assert_eq!(checker.check_count.get(), 1);
    assert_eq!(documents.current_read_count.get(), 0);
    assert_eq!(documents.history_scan_count.get(), 0);
    assert_eq!(
        logger.product_events,
        vec![AccessibleQueryProductEvent::DocumentAccessDenied {
            masked_actor_id: "masked:or-1".to_string(),
            masked_target_id: "document:masked:oc-1".to_string(),
            error_code: "ROLE_DOES_NOT_ALLOW_PERMISSION",
        }]
    );
}

#[test]
fn get_accessible_document_reads_current_after_permission_allowed() {
    let mut checker = FakePermissionChecker::default();
    checker.allow("document:doc-1", Permission::Read);
    let mut documents = FakeDocumentQuery::default();
    documents.insert("workspace-1", current_record("doc-1", "Allowed body"));
    let mut logger = FakeQueryLogger::default();

    let output = GetAccessibleDocumentUsecase::new()
        .execute(
            GetAccessibleDocumentInput::new("actor-1", "workspace-1", None, "doc-1"),
            &checker,
            &documents,
            &mut logger,
        )
        .expect("allowed document");

    assert_eq!(output.record().body().as_str(), "Allowed body");
    assert_eq!(documents.current_read_count.get(), 1);
    assert_eq!(documents.history_scan_count.get(), 0);
}

#[test]
fn search_accessible_documents_pushes_permission_filter_to_query_port() {
    let mut search_index = FakeSearchIndex::default();
    let mut logger = FakeQueryLogger::default();

    let output = SearchAccessibleDocumentsUsecase::new()
        .execute(
            SearchAccessibleDocumentsInput::new("actor-1", "workspace-1", "alpha secret", 10),
            &mut search_index,
            &mut logger,
        )
        .expect("search accessible");

    assert_eq!(output.page().results().len(), 1);
    assert_eq!(search_index.search_count.get(), 1);
    let filter = search_index.last_filter.expect("permission filter");
    assert_eq!(filter.actor_user_id().as_str(), "actor-1");
    assert_eq!(filter.permission(), Permission::Read);
    assert_eq!(output.stats().candidate_count(), 3);
    assert_eq!(output.stats().filtered_count(), 2);
    assert_eq!(logger.field_debug_events.len(), 1);
    assert_ne!(logger.field_debug_events[0].query_hash(), "alpha secret");
    assert_eq!(logger.field_debug_events[0].candidate_count(), 3);
    assert_eq!(logger.field_debug_events[0].filtered_count(), 2);
}

#[test]
fn search_accessible_documents_maps_stale_index_to_stable_error() {
    let mut search_index = FakeSearchIndex {
        fail_stale: true,
        ..FakeSearchIndex::default()
    };
    let mut logger = FakeQueryLogger::default();

    let error = SearchAccessibleDocumentsUsecase::new()
        .execute(
            SearchAccessibleDocumentsInput::new("actor-1", "workspace-1", "alpha", 10),
            &mut search_index,
            &mut logger,
        )
        .expect_err("stale index");

    assert_eq!(error, SearchAccessibleDocumentsError::IndexStale);
    assert_eq!(
        logger.product_events,
        vec![AccessibleQueryProductEvent::SearchQueryFailed {
            result_count_bucket: "0",
            error_code: "QUERY_INDEX_STALE",
        }]
    );
}

#[test]
fn asset_metadata_and_content_use_distinct_permissions() {
    let mut checker = FakePermissionChecker::default();
    checker.allow(&format!("asset:{HASH_A}"), Permission::ReadAssetMetadata);
    checker.deny(&format!("asset:{HASH_A}"), Permission::ReadAssetContent);
    let mut assets = FakeAssetQuery::default();
    assets.insert("workspace-1", "doc-1", asset_metadata());
    let mut logger = FakeQueryLogger::default();

    let metadata = AccessibleAssetMetadataUsecase::new()
        .execute(
            AccessibleAssetMetadataInput::new("actor-1", "workspace-1", Some("doc-1"), HASH_A),
            &checker,
            &assets,
            &mut logger,
        )
        .expect("metadata allowed");
    let content_error = AccessibleAssetContentUsecase::new()
        .execute(
            AccessibleAssetContentInput::new("actor-1", "workspace-1", Some("doc-1"), HASH_A),
            &checker,
            &assets,
            &mut logger,
        )
        .expect_err("content denied");

    assert_eq!(metadata.metadata().id().as_str(), HASH_A);
    assert_eq!(content_error, AccessibleAssetQueryError::NotFound);
    assert_eq!(assets.metadata_read_count.get(), 1);
    assert_eq!(assets.content_read_count.get(), 0);
    assert!(logger.product_events.iter().any(|event| {
        matches!(
            event,
            AccessibleQueryProductEvent::AssetAccessDenied {
                masked_target_id,
                error_code: "ROLE_DOES_NOT_ALLOW_PERMISSION",
                ..
            } if masked_target_id == "asset:masked:aaaa"
        )
    }));
}

fn current_record(id: &str, body: &str) -> CurrentDocumentRecord {
    let metadata = DocumentMetadata::new(
        DocumentId::new(id).expect("document id"),
        DocumentTitle::new("Title").expect("title"),
        DocumentPath::new("docs/title.md").expect("path"),
    )
    .expect("metadata");
    let snapshot = CurrentDocumentSnapshot::new(
        DocumentId::new(id).expect("document id"),
        DocumentBody::new(body, DocumentBodyPolicy::new(1024).expect("policy")).expect("body"),
    );
    CurrentDocumentRecord::new(metadata, snapshot).expect("record")
}

fn search_result(id: &str, title: &str, path: &str, snippet: &str) -> SearchResult {
    SearchResult::new(
        DocumentId::new(id).expect("document id"),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
        snippet,
        100,
    )
    .expect("search result")
}

fn asset_metadata() -> AssetMetadata {
    AssetMetadata::new(
        AssetId::from_sha256_hex(HASH_A).expect("asset id"),
        AssetFileName::new("secret-diagram.png").expect("file name"),
        AssetMediaType::new("image/png").expect("media type"),
        4,
    )
    .expect("metadata")
}

fn decision_key(resource_key: &str, permission: Permission) -> String {
    format!("{}:{}", resource_key, permission.as_str())
}

fn resource_key(resource: &AccessResource) -> String {
    match resource {
        AccessResource::Document { document_id, .. } => {
            format!("document:{}", document_id.as_str())
        }
        AccessResource::Asset { asset_id, .. } => format!("asset:{}", asset_id.as_str()),
        AccessResource::Workspace { workspace_id } => {
            format!("workspace:{}", workspace_id.as_str())
        }
        AccessResource::Collection { collection_id, .. } => {
            format!("collection:{}", collection_id.as_str())
        }
    }
}
