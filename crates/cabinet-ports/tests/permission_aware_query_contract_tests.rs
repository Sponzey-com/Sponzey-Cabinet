use std::cell::Cell;

use cabinet_domain::asset::{AssetFileName, AssetId, AssetMediaType, AssetMetadata};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::permission::{
    AccessResource, Permission, PermissionDecision, PermissionDecisionResult,
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

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(Default)]
struct FakePermissionDecisionPort {
    calls: Cell<usize>,
}

impl PermissionDecisionPort for FakePermissionDecisionPort {
    fn check_permission(
        &self,
        _actor_user_id: &UserId,
        _resource: &AccessResource,
        _permission: Permission,
    ) -> Result<PermissionDecision, PermissionAwareQueryError> {
        self.calls.set(self.calls.get() + 1);
        Ok(PermissionDecision::allowed(
            cabinet_domain::permission::PolicySource::Workspace,
            cabinet_domain::permission::PermissionDecisionReason::RoleAllowsPermission,
        ))
    }
}

#[derive(Default)]
struct FakeAccessibleDocumentQuery {
    current_reads: Cell<usize>,
}

impl AccessibleDocumentQuery for FakeAccessibleDocumentQuery {
    fn get_current_by_id(
        &self,
        _workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, PermissionAwareQueryError> {
        self.current_reads.set(self.current_reads.get() + 1);
        Ok(Some(current_record(
            document_id.as_str(),
            "Accessible body",
        )))
    }
}

#[derive(Default)]
struct FakePermissionAwareSearchIndex {
    calls: Cell<usize>,
    last_permission: Option<Permission>,
    last_actor_id: Option<String>,
}

impl PermissionAwareSearchIndex for FakePermissionAwareSearchIndex {
    fn search_accessible(
        &mut self,
        _workspace_id: &WorkspaceId,
        filter: PermissionFilter,
        _query: SearchQuery,
    ) -> Result<SearchAccessiblePage, PermissionAwareQueryError> {
        self.calls.set(self.calls.get() + 1);
        self.last_permission = Some(filter.permission());
        self.last_actor_id = Some(filter.actor_user_id().as_str().to_string());
        Ok(SearchAccessiblePage::new(
            vec![search_result("doc-1")],
            PermissionQueryStats::new(4, 3, true),
        ))
    }
}

#[derive(Default)]
struct FakeAccessibleAssetQuery {
    metadata_reads: Cell<usize>,
    content_reads: Cell<usize>,
}

impl AccessibleAssetQuery for FakeAccessibleAssetQuery {
    fn get_metadata(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        asset_id: &AssetId,
    ) -> Result<Option<AssetMetadata>, PermissionAwareQueryError> {
        self.metadata_reads.set(self.metadata_reads.get() + 1);
        Ok(Some(asset_metadata(asset_id.as_str())))
    }

    fn get_content(
        &self,
        _workspace_id: &WorkspaceId,
        asset_id: &AssetId,
    ) -> Result<Option<AssetObject>, PermissionAwareQueryError> {
        self.content_reads.set(self.content_reads.get() + 1);
        Ok(Some(
            AssetObject::new(asset_id.clone(), vec![1, 2, 3, 4]).expect("asset object"),
        ))
    }
}

#[test]
fn permission_aware_query_ports_keep_external_io_behind_contracts() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let actor_id = UserId::new("actor-1").expect("user id");
    let asset_id = AssetId::from_sha256_hex(HASH_A).expect("asset id");
    let permission_port = FakePermissionDecisionPort::default();
    let document_query = FakeAccessibleDocumentQuery::default();
    let mut search_index = FakePermissionAwareSearchIndex::default();
    let asset_query = FakeAccessibleAssetQuery::default();

    let decision = permission_port
        .check_permission(
            &actor_id,
            &AccessResource::document(workspace_id.clone(), None, document_id.clone()),
            Permission::Read,
        )
        .expect("permission decision");
    let current = document_query
        .get_current_by_id(&workspace_id, &document_id)
        .expect("current lookup")
        .expect("current record");
    let search_page = search_index
        .search_accessible(
            &workspace_id,
            PermissionFilter::new(actor_id.clone(), Permission::Read),
            SearchQuery::new("accessible", 10).expect("search query"),
        )
        .expect("permission-aware search");
    let metadata = asset_query
        .get_metadata(&workspace_id, &document_id, &asset_id)
        .expect("metadata lookup")
        .expect("asset metadata");
    let content = asset_query
        .get_content(&workspace_id, &asset_id)
        .expect("content lookup")
        .expect("asset content");

    assert_eq!(decision.result(), PermissionDecisionResult::Allowed);
    assert_eq!(current.document_id().as_str(), "doc-1");
    assert_eq!(search_index.calls.get(), 1);
    assert_eq!(search_index.last_actor_id.as_deref(), Some("actor-1"));
    assert_eq!(search_index.last_permission, Some(Permission::Read));
    assert_eq!(search_page.results().len(), 1);
    assert_eq!(search_page.stats().candidate_count(), 4);
    assert_eq!(search_page.stats().filtered_count(), 3);
    assert!(search_page.stats().cache_hit());
    assert_eq!(metadata.id().as_str(), HASH_A);
    assert_eq!(content.asset_id().as_str(), HASH_A);
    assert_eq!(permission_port.calls.get(), 1);
    assert_eq!(document_query.current_reads.get(), 1);
    assert_eq!(asset_query.metadata_reads.get(), 1);
    assert_eq!(asset_query.content_reads.get(), 1);
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

fn search_result(id: &str) -> SearchResult {
    SearchResult::new(
        DocumentId::new(id).expect("document id"),
        DocumentTitle::new("Accessible").expect("title"),
        DocumentPath::new("docs/accessible.md").expect("path"),
        "accessible snippet",
        10,
    )
    .expect("search result")
}

fn asset_metadata(hash: &str) -> AssetMetadata {
    AssetMetadata::new(
        AssetId::from_sha256_hex(hash).expect("asset id"),
        AssetFileName::new("diagram.png").expect("file name"),
        AssetMediaType::new("image/png").expect("media type"),
        4,
    )
    .expect("metadata")
}
