use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use cabinet_core::server_config::ServerConfigInput;
use cabinet_domain::audit::{
    AuditAction, AuditActor, AuditEvent, AuditEventId, AuditMetadata, AuditTarget, AuditTimestamp,
};
use cabinet_domain::backup::{
    BackupJobId, BackupJobOperation, BackupJobSnapshot, BackupJobState, BackupProgress,
};
use cabinet_domain::comment::{
    Comment, CommentBody, CommentBodyPolicy, CommentId, CommentThread, CommentThreadId,
    CommentThreadState,
};
use cabinet_domain::document::{
    DocumentBody, DocumentBodyPolicy, DocumentId, DocumentMetadata, DocumentPath, DocumentTitle,
};
use cabinet_domain::document_lock::{DocumentLock, DocumentLockId, DocumentLockTimestamp};
use cabinet_domain::field_debug::{FieldDebugSession, FieldDebugSessionId, FieldDebugTimestamp};
use cabinet_domain::graph::{
    GraphEdge, GraphEdgeKind, GraphNode, GraphProjectionStatus, KnowledgeGraph,
};
use cabinet_domain::group::{Group, GroupId, GroupMembership, GroupName};
use cabinet_domain::permission::{
    AccessResource, CollectionId, CollectionPolicy, DocumentPolicy, Permission, PermissionDecision,
    PermissionDecisionReason, PolicySource, Role, RoleAssignment, RoleAssignmentId,
    RoleAssignmentSubject,
};
use cabinet_domain::session::{Session, SessionInstant};
use cabinet_domain::user::{User, UserEmail, UserId, UserLogin, UserProfile, UserTimestamp};
use cabinet_domain::version::{
    CurrentDocumentSnapshot, DocumentSnapshotRef, VersionAuthor, VersionEntry, VersionId,
    VersionSummary,
};
use cabinet_domain::workflow::{PublishWorkflowState, ReviewRequest};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::audit_log::{
    AuditCursor, AuditEventPage, AuditListQuery, AuditLogStore, AuditLogStoreError,
    AuditPermissionCheckError, AuditPermissionChecker,
};
use cabinet_ports::auth::{
    CredentialSecret, CredentialVerifier, CredentialVerifierError, IssuedSessionToken,
    PresentedSessionToken, SessionClock, SessionIdGenerator, SessionLookupKey, SessionStore,
    SessionStoreError, TokenIssuer, TokenIssuerError,
};
use cabinet_ports::backup_store::{
    BackupAuditRecord, BackupAuditRecorder, BackupAuditRecorderError, BackupStore,
    BackupStoreError, RestoreValidation,
};
use cabinet_ports::comment_repository::{
    CommentPermissionCheckError, CommentPermissionChecker, CommentRepository,
    CommentRepositoryError, InlineAnchorDocumentLookup, InlineAnchorDocumentLookupError,
    InlineAnchorDocumentState,
};
use cabinet_ports::document_lock::{
    DocumentLockClock, DocumentLockPermissionCheckError, DocumentLockPermissionChecker,
    DocumentLockRepository, DocumentLockRepositoryError,
};
use cabinet_ports::document_repository::{
    CurrentDocumentRecord, DocumentRepository, DocumentRepositoryError,
};
use cabinet_ports::field_debug::{
    FieldDebugClock, FieldDebugPermissionCheckError, FieldDebugPermissionChecker,
    FieldDebugSessionRepository, FieldDebugSessionRepositoryError,
};
use cabinet_ports::graph_projection::{GraphProjectionRecord, GraphProjectionStore};
use cabinet_ports::group_repository::{
    GroupRepository, GroupRepositoryError, MembershipMutationResult,
};
use cabinet_ports::permission_aware_query::{
    AccessibleDocumentQuery, PermissionAwareQueryError, PermissionAwareSearchIndex,
    PermissionDecisionPort, PermissionFilter, PermissionQueryStats, SearchAccessiblePage,
};
use cabinet_ports::permission_policy_repository::{
    PermissionGroupRepository, PermissionPolicyRepository, PermissionRepositoryError,
    RoleAssignmentIdGenerator, RoleAssignmentMutationResult, RoleAssignmentRemovalResult,
};
use cabinet_ports::review_workflow::{
    ReviewRequestRecord, ReviewRequestStatus, ReviewWorkflowPermissionCheckError,
    ReviewWorkflowPermissionChecker, ReviewWorkflowRepository, ReviewWorkflowRepositoryError,
    ReviewWorkflowSideEffectError, ReviewWorkflowSideEffectRecord,
    ReviewWorkflowSideEffectRecorder,
};
use cabinet_ports::search_index::{SearchQuery, SearchResult};
use cabinet_ports::user_repository::{UserRepository, UserRepositoryError};
use cabinet_ports::version_store::{
    HistoryCursor, HistoryPage, HistoryPageRequest, VersionRecord, VersionSnapshot, VersionStore,
    VersionStoreError,
};
use cabinet_server::adapter::{HttpMethod, ServerRequest, handle_request};
use cabinet_server::composition::build_server_composition;
use cabinet_server::runtime::{
    HandlerKind, RuntimeActorContext, RuntimePolicy, ServerRuntimeTarget,
};
use cabinet_usecases::audit::{AuditFieldDebugEvent, AuditProductEvent, AuditUsecaseLogger};
use cabinet_usecases::auth::{AuthProductEvent, AuthProductLogger};
use cabinet_usecases::backup::{BackupJobProductEvent, BackupJobUsecaseLogger};
use cabinet_usecases::comment::{
    CommentFieldDebugEvent, CommentProductEvent, CommentUsecaseLogger,
};
use cabinet_usecases::document::{
    CreateDocumentProductEvent, DocumentChangeEvent, DocumentChangeEventPublisher,
    DocumentProductLogger,
};
use cabinet_usecases::document_lock::{
    DocumentLockFieldDebugEvent, DocumentLockProductEvent, DocumentLockUsecaseLogger,
};
use cabinet_usecases::field_debug::{
    FieldDebugDevelopmentEvent, FieldDebugLogEvent, FieldDebugProductEvent, FieldDebugUsecaseLogger,
};
use cabinet_usecases::group::{CreateGroupProductEvent, CreateGroupProductLogger};
use cabinet_usecases::permission::{
    PermissionFieldDebugEvent, PermissionProductEvent, PermissionUsecaseLogger,
};
use cabinet_usecases::permission_query::{
    AccessibleQueryFieldDebugEvent, AccessibleQueryLogger, AccessibleQueryProductEvent,
};
use cabinet_usecases::review_workflow::{
    ReviewWorkflowFieldDebugEvent, ReviewWorkflowProductEvent, ReviewWorkflowUsecaseLogger,
};

#[test]
fn composition_wires_every_route_id_to_a_runtime_handler() {
    let composition = build_server_composition(default_config());
    let handlers = composition.handlers();

    assert_eq!(handlers.len(), composition.routes().routes().len());
    for route in composition.routes().routes() {
        assert!(
            handlers.contains(route.route_id()),
            "missing handler for {}",
            route.route_id()
        );
    }
    assert_eq!(
        handlers.kind("document.get_accessible_current"),
        Some(HandlerKind::CurrentDocument)
    );
    assert_eq!(
        handlers.kind("document.get_accessible_history"),
        Some(HandlerKind::DocumentHistory)
    );
    assert_eq!(
        handlers.kind("document.save_remote_current"),
        Some(HandlerKind::RemoteCurrentDocumentSave)
    );
    assert_eq!(
        handlers.kind("search.accessible"),
        Some(HandlerKind::PermissionAwareSearch)
    );
    assert_eq!(
        handlers.kind("graph.get_local"),
        Some(HandlerKind::GraphLocal)
    );
    assert_eq!(
        handlers.kind("collaboration.join_document_room"),
        Some(HandlerKind::CollaborationJoinDocumentRoom)
    );
    assert_eq!(
        handlers.kind("collaboration.broadcast_operation"),
        Some(HandlerKind::CollaborationBroadcastOperation)
    );
    assert_eq!(
        handlers.kind("collaboration.broadcast_presence"),
        Some(HandlerKind::CollaborationBroadcastPresence)
    );
    assert_eq!(
        handlers.kind("collaboration.request_replay"),
        Some(HandlerKind::CollaborationRequestReplay)
    );
    assert_eq!(
        handlers.kind("field_debug.request_session"),
        Some(HandlerKind::FieldDebugRequest)
    );
    assert_eq!(handlers.kind("auth.login"), Some(HandlerKind::AuthLogin));
    assert_eq!(
        handlers.kind("auth.validate_session"),
        Some(HandlerKind::AuthValidateSession)
    );
    assert_eq!(handlers.kind("user.list"), Some(HandlerKind::UserList));
    assert_eq!(handlers.kind("group.list"), Some(HandlerKind::GroupList));
    assert_eq!(
        handlers.kind("group.add_member"),
        Some(HandlerKind::GroupAddMember)
    );
    assert_eq!(
        handlers.kind("group.remove_member"),
        Some(HandlerKind::GroupRemoveMember)
    );
    assert_eq!(
        handlers.kind("role.list_assignments"),
        Some(HandlerKind::RoleListAssignments)
    );
    assert_eq!(handlers.kind("role.assign"), Some(HandlerKind::RoleAssign));
    assert_eq!(handlers.kind("role.revoke"), Some(HandlerKind::RoleRevoke));
    assert_eq!(
        handlers.kind("sharing.get_document"),
        Some(HandlerKind::SharingGetDocument)
    );
    assert_eq!(
        handlers.kind("sharing.update_document"),
        Some(HandlerKind::SharingUpdateDocument)
    );
    assert_eq!(
        handlers.kind("comment.list"),
        Some(HandlerKind::CommentList)
    );
    assert_eq!(handlers.kind("comment.add"), Some(HandlerKind::CommentAdd));
    assert_eq!(
        handlers.kind("comment.add_inline"),
        Some(HandlerKind::CommentAddInline)
    );
    assert_eq!(
        handlers.kind("comment.resolve"),
        Some(HandlerKind::CommentResolve)
    );
    assert_eq!(
        handlers.kind("comment.reopen"),
        Some(HandlerKind::CommentReopen)
    );
    assert_eq!(
        handlers.kind("review.request_document"),
        Some(HandlerKind::ReviewRequestDocument)
    );
    assert_eq!(
        handlers.kind("review.approve_document"),
        Some(HandlerKind::ReviewApproveDocument)
    );
    assert_eq!(
        handlers.kind("review.reject_document"),
        Some(HandlerKind::ReviewRejectDocument)
    );
    assert_eq!(
        handlers.kind("review.publish_document"),
        Some(HandlerKind::ReviewPublishDocument)
    );
    assert_eq!(
        handlers.kind("review.list_requests"),
        Some(HandlerKind::ReviewListRequests)
    );
    assert_eq!(
        handlers.kind("document_lock.lock"),
        Some(HandlerKind::DocumentLockLock)
    );
    assert_eq!(
        handlers.kind("document_lock.unlock"),
        Some(HandlerKind::DocumentLockUnlock)
    );
    assert_eq!(
        handlers.kind("document_lock.get"),
        Some(HandlerKind::DocumentLockGet)
    );
    assert_eq!(
        handlers.kind("audit.list_events"),
        Some(HandlerKind::AuditListEvents)
    );
    assert_eq!(
        handlers.kind("backup.create"),
        Some(HandlerKind::BackupCreate)
    );
    assert_eq!(
        handlers.kind("backup.get_status"),
        Some(HandlerKind::BackupGetStatus)
    );
    assert_eq!(
        handlers.kind("backup.restore"),
        Some(HandlerKind::BackupRestore)
    );
    assert_eq!(
        handlers.kind("export.create_workspace"),
        Some(HandlerKind::ExportCreateWorkspace)
    );
    assert_eq!(
        handlers.kind("export.get_status"),
        Some(HandlerKind::ExportGetStatus)
    );

    let dependencies = composition.runtime_dependencies();
    assert!(dependencies.contains("document_repository"));
    assert!(dependencies.contains("version_store"));
    assert!(dependencies.contains("permission_query"));
    assert!(dependencies.contains("graph_projection_store"));
    assert!(dependencies.contains("object_storage"));
    assert!(dependencies.contains("audit_store"));
    assert!(dependencies.contains("backup_store"));
    assert!(dependencies.contains("backup_audit_recorder"));
    assert!(dependencies.contains("field_debug_repository"));
    assert!(dependencies.contains("credential_verifier"));
    assert!(dependencies.contains("token_issuer"));
    assert!(dependencies.contains("session_store"));
    assert!(dependencies.contains("group_repository"));
    assert!(dependencies.contains("user_repository"));
    assert!(dependencies.contains("document_lock_repository"));
    assert!(dependencies.contains("document_lock_clock"));
    assert!(dependencies.contains("clock"));
    assert!(dependencies.contains("id_generator"));
    assert!(dependencies.contains("auth_policy"));
    assert!(dependencies.contains("config_policy"));
}

#[test]
fn current_document_handler_invokes_accessible_usecase_without_history_scan() {
    let composition = build_server_composition(default_config());
    let mut document_query = FakeDocumentQuery::default();
    document_query.insert(
        "workspace-1",
        current_record("doc-1", "Runtime Title", "runtime/doc.md", "runtime body"),
    );
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("user-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        document_query,
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/workspaces/workspace-1/documents/doc-1/current",
            None,
        ),
    )
    .expect("current document handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"documentId\":\"doc-1\""));
    assert!(response.body().contains("\"title\":\"Runtime Title\""));
    assert_eq!(target.document_query().current_read_count.get(), 1);
    assert_eq!(target.document_query().history_scan_count.get(), 0);
    assert!(logger_events.product_events().is_empty());
}

#[test]
fn history_handler_uses_paginated_history_path_not_current_document_path() {
    let composition = build_server_composition(default_config());
    let mut version_store = FakeVersionStore::default();
    version_store.insert_history(
        "workspace-1",
        "doc-1",
        vec![
            version_entry("doc-1", "version-1"),
            version_entry("doc-1", "version-2"),
            version_entry("doc-1", "version-3"),
        ],
    );
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("user-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        version_store,
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        SharedRuntimeLog::default(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/workspaces/workspace-1/documents/doc-1/history?limit=2",
            None,
        ),
    )
    .expect("history handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"entryCount\":2"));
    assert!(response.body().contains("\"nextCursor\":\"2\""));
    assert_eq!(target.version_store().list_history_count.get(), 1);
    assert_eq!(
        target.version_store().current_repository_read_count.get(),
        0
    );
}

#[test]
fn remote_current_save_handler_delegates_to_update_usecase_without_history_scan_or_body_logs() {
    let composition = build_server_composition(default_config());
    let mut document_repository = FakeDocumentRepository::default();
    document_repository.insert(
        "workspace-1",
        current_record("doc-1", "Runtime Title", "runtime/doc.md", "old body"),
    );
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("writer-1234"),
        FakePermissionChecker::allowed(),
        document_repository,
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Put,
            "/api/workspaces/workspace-1/documents/doc-1/current",
            Some(
                "{\"body\":\"updated body\",\"versionId\":\"version-5\",\"snapshotRef\":\"snapshot-5\",\"author\":\"writer-secret\",\"summary\":\"Remote save\",\"rawBody\":\"must-not-log\"}",
            ),
        ),
    )
    .expect("remote save handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"workspaceId\":\"workspace-1\""));
    assert!(response.body().contains("\"documentId\":\"doc-1\""));
    assert!(response.body().contains("\"status\":\"saved-remote\""));
    assert!(response.body().contains("\"versionId\":\"version-5\""));
    assert_eq!(
        target
            .document_repository()
            .current_body("workspace-1", "doc-1"),
        "updated body"
    );
    assert_eq!(target.document_repository().put_count.get(), 1);
    assert_eq!(target.version_store().append_count.get(), 1);
    assert_eq!(target.version_store().list_history_count.get(), 0);
    assert_eq!(target.document_change_publisher().event_count.get(), 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("updated body")
                && !event.contains("writer-secret")
                && !event.contains("must-not-log"))
    );
}

#[test]
fn remote_current_save_handler_returns_not_found_without_writes_for_missing_current() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("writer-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events,
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Put,
            "/api/workspaces/workspace-1/documents/missing-doc/current",
            Some(
                "{\"body\":\"updated body\",\"versionId\":\"version-5\",\"snapshotRef\":\"snapshot-5\",\"author\":\"writer\",\"summary\":\"Remote save\"}",
            ),
        ),
    )
    .expect("remote save missing handler");

    assert_eq!(response.status_code(), 404);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"document.not_found\"")
    );
    assert_eq!(target.document_repository().put_count.get(), 0);
    assert_eq!(target.version_store().append_count.get(), 0);
    assert_eq!(target.document_change_publisher().event_count.get(), 0);
}

#[test]
fn search_handler_passes_permission_filter_to_query_port() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("user-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/workspaces/workspace-1/search?text=runtime&limit=20",
            None,
        ),
    )
    .expect("search handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"resultCount\":1"));
    assert_eq!(target.search_index().search_count.get(), 1);
    assert_eq!(
        target
            .search_index()
            .last_filter
            .borrow()
            .as_ref()
            .expect("permission filter")
            .actor_user_id()
            .as_str(),
        "user-1234"
    );
    assert_eq!(
        target.search_index().last_query_text.borrow().as_deref(),
        Some("runtime")
    );
    assert_eq!(logger_events.field_debug_count(), 1);
}

#[test]
fn graph_handler_uses_projection_port_and_filters_denied_documents() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("user-1234"),
        FakePermissionChecker::allowed_except_documents(vec!["hidden-doc"]),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );
    {
        let workspace_id = WorkspaceId::new("workspace-1").expect("workspace");
        target
            .graph_projection_store_mut()
            .replace_projection(
                &workspace_id,
                GraphProjectionRecord::new(graph_projection_fixture("doc-center"))
                    .expect("graph projection record"),
            )
            .expect("projection saved");
    }

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/workspaces/workspace-1/documents/doc-center/graph",
            None,
        ),
    )
    .expect("graph handler");

    assert_eq!(response.status_code(), 200);
    assert!(
        response
            .body()
            .contains("\"centerDocumentId\":\"doc-center\"")
    );
    assert!(response.body().contains("\"id\":\"visible-doc\""));
    assert!(!response.body().contains("hidden-doc"));
    assert!(response.body().contains("\"candidateCount\":3"));
    assert!(response.body().contains("\"filteredCount\":1"));
    assert_eq!(target.graph_projection_store().lookup_count(), 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("hidden-doc") && !event.contains("document body"))
    );
}

#[test]
fn field_debug_handler_delegates_scope_ttl_and_admin_approval_to_usecase() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let request_response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/field-debug-sessions",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"sessionId\":\"field-debug-1\",\"scope\":\"workspace:workspace-1\",\"ttlSeconds\":300,\"rawBody\":\"must-not-log\"}",
            ),
        ),
    )
    .expect("field debug request handler");

    assert_eq!(request_response.status_code(), 202);
    assert!(request_response.body().contains("\"status\":\"requested\""));

    let approve_response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/field-debug-sessions/field-debug-1/approve",
            Some("{\"workspaceId\":\"workspace-1\"}"),
        ),
    )
    .expect("field debug approve handler");

    assert_eq!(approve_response.status_code(), 200);
    assert!(approve_response.body().contains("\"status\":\"active\""));
    assert_eq!(target.field_debug_repository().save_count.get(), 3);
    assert_eq!(
        target
            .field_debug_permission_checker()
            .checked_permission
            .borrow()
            .as_ref(),
        Some(&Permission::Manage)
    );
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log"))
    );
}

#[test]
fn auth_handlers_delegate_login_and_validation_to_usecases_without_secret_product_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut credential_verifier = FakeCredentialVerifier::default();
    credential_verifier.insert(active_user("user-1234", "alice"), "correct-password");
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("anonymous"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        credential_verifier,
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let login_response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/auth/login",
            Some("{\"login\":\"alice\",\"credential\":\"correct-password\"}"),
        ),
    )
    .expect("auth login handler");

    assert_eq!(login_response.status_code(), 200);
    assert!(login_response.body().contains("\"userId\":\"user-1234\""));
    assert!(login_response.body().contains("\"token\":\"token-1\""));
    assert!(
        login_response
            .body()
            .contains("\"sessionStatus\":\"Active\"")
    );
    assert_eq!(target.session_store().create_count.get(), 1);

    let validate_response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/auth/session/validate",
            Some("{\"token\":\"token-1\"}"),
        ),
    )
    .expect("auth validate handler");

    assert_eq!(validate_response.status_code(), 200);
    assert!(
        validate_response
            .body()
            .contains("\"userId\":\"user-1234\"")
    );
    assert!(
        validate_response
            .body()
            .contains("\"sessionStatus\":\"Active\"")
    );
    assert_eq!(target.session_store().get_count.get(), 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("correct-password") && !event.contains("token-1"))
    );
}

#[test]
fn auth_login_handler_returns_stable_safe_error_for_invalid_credential() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut credential_verifier = FakeCredentialVerifier::default();
    credential_verifier.insert(active_user("user-1234", "alice"), "correct-password");
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("anonymous"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        credential_verifier,
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/auth/login",
            Some("{\"login\":\"alice\",\"credential\":\"wrong-password\"}"),
        ),
    )
    .expect("auth login error response");

    assert_eq!(response.status_code(), 401);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"AUTH_INVALID_CREDENTIAL\"")
    );
    assert_eq!(target.session_store().create_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("wrong-password") && !event.contains("token-"))
    );
}

#[test]
fn user_list_handler_delegates_to_usecase_and_excludes_email_from_response() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::with_users(vec![active_user("user-1234", "alice")]),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events,
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(HttpMethod::Get, "/api/users", None),
    )
    .expect("user list handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"userId\":\"user-1234\""));
    assert!(response.body().contains("\"login\":\"alice\""));
    assert!(response.body().contains("\"displayName\":\"alice\""));
    assert!(response.body().contains("\"status\":\"Active\""));
    assert!(!response.body().contains("alice@example.com"));
    assert_eq!(target.user_repository().list_count.get(), 1);
}

#[test]
fn group_list_handler_delegates_to_usecase_and_excludes_user_profile_from_response() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut group_repository = FakeGroupRepository::with_group("workspace-1", "group-1", "Editors");
    group_repository.add_member("workspace-1", "group-1", "user-1234");
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        group_repository,
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::with_users(vec![active_user("user-1234", "alice")]),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events,
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(HttpMethod::Get, "/api/workspaces/workspace-1/groups", None),
    )
    .expect("group list handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"workspaceId\":\"workspace-1\""));
    assert!(response.body().contains("\"groupId\":\"group-1\""));
    assert!(response.body().contains("\"displayName\":\"Editors\""));
    assert!(
        response
            .body()
            .contains("\"memberUserIds\":[\"user-1234\"]")
    );
    assert!(!response.body().contains("alice@example.com"));
    assert_eq!(target.group_repository().list_group_count.get(), 1);
}

#[test]
fn group_membership_handlers_delegate_add_and_remove_to_usecases_without_sensitive_product_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::with_group("workspace-1", "group-1", "Editors"),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::with_users(vec![active_user("user-1234", "alice")]),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let add_response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/workspaces/workspace-1/groups/group-1/members",
            Some("{\"userId\":\"user-1234\",\"rawBody\":\"must-not-log\"}"),
        ),
    )
    .expect("group add member handler");

    assert_eq!(add_response.status_code(), 200);
    assert!(add_response.body().contains("\"groupId\":\"group-1\""));
    assert!(add_response.body().contains("\"userId\":\"user-1234\""));
    assert!(add_response.body().contains("\"result\":\"Added\""));
    assert!(
        target
            .group_repository()
            .has_membership(
                &WorkspaceId::new("workspace-1").expect("workspace id"),
                &GroupId::new("group-1").expect("group id"),
                &UserId::new("user-1234").expect("user id"),
            )
            .expect("membership lookup")
    );

    let remove_response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Delete,
            "/api/workspaces/workspace-1/groups/group-1/members/user-1234",
            None,
        ),
    )
    .expect("group remove member handler");

    assert_eq!(remove_response.status_code(), 200);
    assert!(remove_response.body().contains("\"groupId\":\"group-1\""));
    assert!(remove_response.body().contains("\"userId\":\"user-1234\""));
    assert!(remove_response.body().contains("\"result\":\"Removed\""));
    assert!(
        !target
            .group_repository()
            .has_membership(
                &WorkspaceId::new("workspace-1").expect("workspace id"),
                &GroupId::new("group-1").expect("group id"),
                &UserId::new("user-1234").expect("user id"),
            )
            .expect("membership lookup")
    );
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("Editors")
                && !event.contains("alice@example.com")
                && !event.contains("must-not-log"))
    );
}

#[test]
fn group_remove_member_handler_returns_stable_error_for_missing_membership() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::with_group("workspace-1", "group-1", "Editors"),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::with_users(vec![active_user("user-1234", "alice")]),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Delete,
            "/api/workspaces/workspace-1/groups/group-1/members/user-1234",
            None,
        ),
    )
    .expect("group remove missing member handler");

    assert_eq!(response.status_code(), 404);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"MEMBERSHIP_NOT_FOUND\"")
    );
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("Editors") && !event.contains("alice@example.com"))
    );
}

#[test]
fn role_list_assignments_handler_delegates_to_usecase_and_excludes_subject_profiles() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let permission_repository = FakePermissionPolicyRepository::with_assignments(vec![
        role_assignment(
            "role-assignment-1",
            "workspace-1",
            RoleAssignmentSubject::User(UserId::new("user-1234").expect("user id")),
            Role::Editor,
        ),
        role_assignment(
            "role-assignment-2",
            "workspace-1",
            RoleAssignmentSubject::Group(GroupId::new("group-1").expect("group id")),
            Role::Reviewer,
        ),
        role_assignment(
            "role-assignment-3",
            "workspace-2",
            RoleAssignmentSubject::User(UserId::new("other-user").expect("user id")),
            Role::Viewer,
        ),
    ]);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        permission_repository,
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::with_users(vec![active_user("user-1234", "alice")]),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(HttpMethod::Get, "/api/workspaces/workspace-1/roles", None),
    )
    .expect("role list assignments handler");

    assert_eq!(response.status_code(), 200);
    assert!(
        response
            .body()
            .contains("\"assignmentId\":\"role-assignment-1\"")
    );
    assert!(response.body().contains("\"subjectType\":\"user\""));
    assert!(response.body().contains("\"subjectId\":\"user-1234\""));
    assert!(response.body().contains("\"role\":\"editor\""));
    assert!(
        response
            .body()
            .contains("\"assignmentId\":\"role-assignment-2\"")
    );
    assert!(response.body().contains("\"subjectType\":\"group\""));
    assert!(response.body().contains("\"subjectId\":\"group-1\""));
    assert!(response.body().contains("\"role\":\"reviewer\""));
    assert!(!response.body().contains("role-assignment-3"));
    assert!(!response.body().contains("alice@example.com"));
    assert!(!response.body().contains("Editors"));
    assert_eq!(target.permission_repository().list_workspace_count.get(), 1);
    assert!(logger_events.product_events().is_empty());
}

#[test]
fn role_assign_handler_delegates_to_usecase_with_injected_id_generator_and_safe_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let permission_repository =
        FakePermissionPolicyRepository::with_assignments(vec![role_assignment(
            "role-owner-admin",
            "workspace-1",
            RoleAssignmentSubject::User(UserId::new("admin-1234").expect("admin id")),
            Role::Owner,
        )]);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        permission_repository,
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::with_users(vec![active_user("target-1", "target")]),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/workspaces/workspace-1/roles",
            Some(
                "{\"subjectType\":\"user\",\"subjectId\":\"target-1\",\"role\":\"editor\",\"rawBody\":\"must-not-log\"}",
            ),
        ),
    )
    .expect("role assign handler");

    assert_eq!(response.status_code(), 200);
    assert!(
        response
            .body()
            .contains("\"assignmentId\":\"role-assignment-1\"")
    );
    assert!(response.body().contains("\"subjectType\":\"user\""));
    assert!(response.body().contains("\"subjectId\":\"target-1\""));
    assert!(response.body().contains("\"role\":\"editor\""));
    assert!(
        target
            .permission_repository()
            .get_role_assignment(
                &WorkspaceId::new("workspace-1").expect("workspace id"),
                &RoleAssignmentId::new("role-assignment-1").expect("assignment id"),
            )
            .expect("stored assignment")
            .is_some()
    );
    assert_eq!(logger_events.field_debug_count(), 1);
    assert!(logger_events.product_events().iter().all(
        |event| !event.contains("target@example.com")
            && !event.contains("must-not-log")
            && !event.contains("Editors")
    ));
}

#[test]
fn role_assign_handler_returns_permission_denied_without_saving_assignment() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let permission_repository =
        FakePermissionPolicyRepository::with_assignments(vec![role_assignment(
            "role-viewer-admin",
            "workspace-1",
            RoleAssignmentSubject::User(UserId::new("admin-1234").expect("admin id")),
            Role::Viewer,
        )]);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        permission_repository,
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/workspaces/workspace-1/roles",
            Some("{\"subjectType\":\"user\",\"subjectId\":\"target-1\",\"role\":\"editor\"}"),
        ),
    )
    .expect("role assign denied handler");

    assert_eq!(response.status_code(), 403);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"PERMISSION_DENIED\"")
    );
    assert!(
        target
            .permission_repository()
            .get_role_assignment(
                &WorkspaceId::new("workspace-1").expect("workspace id"),
                &RoleAssignmentId::new("role-assignment-1").expect("assignment id"),
            )
            .expect("missing assignment")
            .is_none()
    );
    assert_eq!(logger_events.field_debug_count(), 1);
}

#[test]
fn role_revoke_handler_delegates_to_usecase_and_removes_assignment_with_safe_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let permission_repository = FakePermissionPolicyRepository::with_assignments(vec![
        role_assignment(
            "role-owner-admin",
            "workspace-1",
            RoleAssignmentSubject::User(UserId::new("admin-1234").expect("admin id")),
            Role::Owner,
        ),
        role_assignment(
            "role-assignment-9",
            "workspace-1",
            RoleAssignmentSubject::User(UserId::new("target-1").expect("target id")),
            Role::Editor,
        ),
    ]);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        permission_repository,
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::with_users(vec![active_user("target-1", "target")]),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Delete,
            "/api/workspaces/workspace-1/roles/role-assignment-9",
            None,
        ),
    )
    .expect("role revoke handler");

    assert_eq!(response.status_code(), 200);
    assert!(
        response
            .body()
            .contains("\"assignmentId\":\"role-assignment-9\"")
    );
    assert!(response.body().contains("\"result\":\"Revoked\""));
    assert!(
        target
            .permission_repository()
            .get_role_assignment(
                &WorkspaceId::new("workspace-1").expect("workspace id"),
                &RoleAssignmentId::new("role-assignment-9").expect("assignment id"),
            )
            .expect("removed assignment")
            .is_none()
    );
    assert_eq!(logger_events.field_debug_count(), 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("target@example.com") && !event.contains("Editors"))
    );
}

#[test]
fn role_revoke_handler_returns_stable_error_for_missing_assignment() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let permission_repository =
        FakePermissionPolicyRepository::with_assignments(vec![role_assignment(
            "role-owner-admin",
            "workspace-1",
            RoleAssignmentSubject::User(UserId::new("admin-1234").expect("admin id")),
            Role::Owner,
        )]);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        permission_repository,
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Delete,
            "/api/workspaces/workspace-1/roles/missing-assignment",
            None,
        ),
    )
    .expect("role revoke missing handler");

    assert_eq!(response.status_code(), 404);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"ROLE_ASSIGNMENT_NOT_FOUND\"")
    );
    assert_eq!(logger_events.field_debug_count(), 1);
}

#[test]
fn sharing_get_handler_returns_actor_effective_permissions_without_policy_mutation_or_body_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let permission_repository =
        FakePermissionPolicyRepository::with_assignments(vec![role_assignment(
            "role-editor-actor",
            "workspace-1",
            RoleAssignmentSubject::User(UserId::new("actor-1234").expect("actor id")),
            Role::Editor,
        )]);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("actor-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        permission_repository,
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/documents/doc-allowed/sharing?workspaceId=workspace-1&collectionId=collection-1",
            Some("{\"rawBody\":\"must-not-log\",\"documentBody\":\"secret body\"}"),
        ),
    )
    .expect("sharing get handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"workspaceId\":\"workspace-1\""));
    assert!(response.body().contains("\"documentId\":\"doc-allowed\""));
    assert!(response.body().contains("\"entries\":[]"));
    assert!(response.body().contains("\"effectivePermissions\""));
    assert!(response.body().contains("\"read\""));
    assert!(response.body().contains("\"write\""));
    assert!(!response.body().contains("\"manage\""));
    assert_eq!(target.permission_repository().list_workspace_count.get(), 0);
    assert_eq!(logger_events.field_debug_count(), 7);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("secret body") && !event.contains("must-not-log"))
    );
}

#[test]
fn sharing_get_handler_requires_workspace_query_before_permission_repository_access() {
    let composition = build_server_composition(default_config());
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("actor-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        SharedRuntimeLog::default(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/documents/doc-allowed/sharing",
            Some("{\"rawBody\":\"must-not-log\"}"),
        ),
    )
    .expect("sharing get malformed handler");

    assert_eq!(response.status_code(), 400);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"SERVER_MALFORMED_REQUEST\"")
    );
    assert_eq!(target.permission_repository().list_workspace_count.get(), 0);
}

#[test]
fn sharing_update_handler_delegates_to_share_document_usecase_with_safe_subject_response() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let permission_repository =
        FakePermissionPolicyRepository::with_assignments(vec![role_assignment(
            "role-owner-actor",
            "workspace-1",
            RoleAssignmentSubject::User(UserId::new("owner-1234").expect("owner id")),
            Role::Owner,
        )]);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("owner-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        permission_repository,
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Put,
            "/api/documents/doc-allowed/sharing",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"collectionId\":\"collection-1\",\"subject\":{\"kind\":\"group\",\"id\":\"group-secret\"},\"permission\":\"write\",\"effect\":\"allow\",\"rawBody\":\"must-not-log\",\"documentBody\":\"secret body\"}",
            ),
        ),
    )
    .expect("sharing update handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"workspaceId\":\"workspace-1\""));
    assert!(response.body().contains("\"documentId\":\"doc-allowed\""));
    assert!(response.body().contains("\"kind\":\"group\""));
    assert!(response.body().contains("\"id\":\"group-secret\""));
    assert!(response.body().contains("\"permission\":\"write\""));
    assert!(response.body().contains("\"effect\":\"allow\""));
    assert_eq!(
        target
            .permission_repository()
            .get_document_policy(
                &WorkspaceId::new("workspace-1").expect("workspace id"),
                &DocumentId::new("doc-allowed").expect("document id"),
            )
            .expect("document policy")
            .expect("saved policy")
            .overrides()
            .len(),
        1
    );
    assert_eq!(
        target
            .permission_repository()
            .saved_document_policy_count
            .get(),
        1
    );
    assert_eq!(logger_events.field_debug_count(), 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("group-secret")
                && !event.contains("secret body")
                && !event.contains("must-not-log"))
    );
}

#[test]
fn sharing_update_handler_denies_without_policy_write_for_non_manager() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let permission_repository =
        FakePermissionPolicyRepository::with_assignments(vec![role_assignment(
            "role-editor-actor",
            "workspace-1",
            RoleAssignmentSubject::User(UserId::new("editor-1234").expect("editor id")),
            Role::Editor,
        )]);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("editor-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        permission_repository,
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Put,
            "/api/documents/doc-allowed/sharing",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"subject\":{\"kind\":\"user\",\"id\":\"user-secret\"},\"permission\":\"write\",\"effect\":\"allow\",\"rawBody\":\"must-not-log\"}",
            ),
        ),
    )
    .expect("sharing update denied handler");

    assert_eq!(response.status_code(), 403);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"PERMISSION_DENIED\"")
    );
    assert_eq!(
        target
            .permission_repository()
            .saved_document_policy_count
            .get(),
        0
    );
    assert_eq!(logger_events.field_debug_count(), 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("user-secret") && !event.contains("must-not-log"))
    );
}

#[test]
fn comment_list_handler_delegates_to_usecase_and_returns_threads_without_product_body_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut comment_repository = FakeCommentRepository::default();
    comment_repository.insert(CommentThread::new(
        CommentThreadId::new("thread-1").expect("thread id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        DocumentId::new("doc-allowed").expect("document id"),
        comment("comment-1", "author-1234", "visible comment body"),
    ));
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("reader-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        comment_repository,
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/documents/doc-allowed/comments?workspaceId=workspace-1",
            Some("{\"rawBody\":\"must-not-log\"}"),
        ),
    )
    .expect("comment list handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"threads\""));
    assert!(response.body().contains("\"threadId\":\"thread-1\""));
    assert!(response.body().contains("\"commentId\":\"comment-1\""));
    assert!(response.body().contains("\"authorUserId\":\"author-1234\""));
    assert!(
        response
            .body()
            .contains("\"body\":\"visible comment body\"")
    );
    assert!(response.body().contains("\"state\":\"open\""));
    assert_eq!(target.comment_repository().list_count.get(), 1);
    assert_eq!(logger_events.field_debug_count(), 2);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("visible comment body")
                && !event.contains("must-not-log"))
    );
}

#[test]
fn comment_list_handler_denies_before_repository_lookup() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut comment_repository = FakeCommentRepository::default();
    comment_repository.insert(CommentThread::new(
        CommentThreadId::new("thread-1").expect("thread id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        DocumentId::new("doc-denied").expect("document id"),
        comment("comment-1", "author-1234", "hidden comment body"),
    ));
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("reader-1234"),
        FakePermissionChecker::denied(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        comment_repository,
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/documents/doc-denied/comments?workspaceId=workspace-1",
            Some("{\"rawBody\":\"must-not-log\"}"),
        ),
    )
    .expect("comment list denied handler");

    assert_eq!(response.status_code(), 403);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"COMMENT_UNAUTHORIZED\"")
    );
    assert_eq!(target.comment_repository().list_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("hidden comment body") && !event.contains("must-not-log"))
    );
}

#[test]
fn comment_add_handler_delegates_to_usecase_and_creates_thread_without_product_body_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("writer-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-allowed/comments",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"threadId\":\"thread-1\",\"commentId\":\"comment-1\",\"body\":\"secret comment body\",\"rawBody\":\"must-not-log\"}",
            ),
        ),
    )
    .expect("comment add handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"threadId\":\"thread-1\""));
    assert!(response.body().contains("\"commentId\":\"comment-1\""));
    assert!(response.body().contains("\"body\":\"secret comment body\""));
    assert_eq!(target.comment_repository().save_count.get(), 1);
    assert_eq!(target.comment_repository().append_count.get(), 0);
    assert_eq!(logger_events.field_debug_count(), 2);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("secret comment body") && !event.contains("must-not-log"))
    );
}

#[test]
fn comment_add_handler_denies_without_saving_comment_body() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("writer-1234"),
        FakePermissionChecker::denied(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-denied/comments",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"threadId\":\"thread-1\",\"commentId\":\"comment-1\",\"body\":\"hidden comment body\",\"rawBody\":\"must-not-log\"}",
            ),
        ),
    )
    .expect("comment add denied handler");

    assert_eq!(response.status_code(), 403);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"COMMENT_UNAUTHORIZED\"")
    );
    assert_eq!(target.comment_repository().save_count.get(), 0);
    assert_eq!(target.comment_repository().append_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("hidden comment body") && !event.contains("must-not-log"))
    );
}

#[test]
fn comment_add_inline_handler_delegates_to_usecase_with_anchor_lookup_and_safe_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut version_store = FakeVersionStore::default();
    version_store.insert_inline_anchor_state(
        "workspace-1",
        "doc-inline",
        "version-1",
        "version-1",
        32,
    );
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("writer-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        version_store,
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-inline/inline-comments",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"versionId\":\"version-1\",\"startOffset\":4,\"endOffset\":9,\"threadId\":\"thread-inline-1\",\"commentId\":\"comment-inline-1\",\"body\":\"selected text must-not-log\",\"rawBody\":\"must-not-log\"}",
            ),
        ),
    )
    .expect("inline comment add handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"threadId\":\"thread-inline-1\""));
    assert!(
        response
            .body()
            .contains("\"commentId\":\"comment-inline-1\"")
    );
    assert!(response.body().contains("\"versionId\":\"version-1\""));
    assert!(response.body().contains("\"startOffset\":4"));
    assert!(response.body().contains("\"endOffset\":9"));
    assert!(response.body().contains("\"status\":\"valid\""));
    assert_eq!(target.comment_repository().save_count.get(), 1);
    assert_eq!(target.version_store().inline_anchor_lookup_count.get(), 1);
    assert_eq!(target.version_store().list_history_count.get(), 0);
    assert_eq!(
        target.version_store().current_repository_read_count.get(),
        0
    );
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("selected text")
                && !event.contains("must-not-log")
                && !event.contains("version-1"))
    );
}

#[test]
fn comment_add_inline_handler_returns_stale_anchor_without_saving_comment_body() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut version_store = FakeVersionStore::default();
    version_store.insert_inline_anchor_state(
        "workspace-1",
        "doc-inline",
        "version-old",
        "version-current",
        32,
    );
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("writer-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        version_store,
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-inline/inline-comments",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"versionId\":\"version-old\",\"startOffset\":4,\"endOffset\":9,\"threadId\":\"thread-inline-1\",\"commentId\":\"comment-inline-1\",\"body\":\"stale selected text\",\"rawBody\":\"must-not-log\"}",
            ),
        ),
    )
    .expect("inline comment stale handler");

    assert_eq!(response.status_code(), 400);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"COMMENT_STALE_ANCHOR\"")
    );
    assert_eq!(target.comment_repository().save_count.get(), 0);
    assert_eq!(target.version_store().inline_anchor_lookup_count.get(), 1);
    assert!(logger_events.product_events().iter().all(
        |event| !event.contains("stale selected text")
            && !event.contains("must-not-log")
            && !event.contains("version-old")
    ));
}

#[test]
fn comment_resolve_handler_delegates_to_usecase_and_returns_resolved_thread_with_safe_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut comment_repository = FakeCommentRepository::default();
    comment_repository.insert(CommentThread::new(
        CommentThreadId::new("thread-1").expect("thread id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        DocumentId::new("doc-1").expect("document id"),
        comment("comment-1", "writer-1234", "body must-not-log"),
    ));
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("writer-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        comment_repository,
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/comments/thread-1/resolve",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"documentId\":\"doc-1\",\"rawBody\":\"must-not-log\"}",
            ),
        ),
    )
    .expect("comment resolve handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"threadId\":\"thread-1\""));
    assert!(response.body().contains("\"documentId\":\"doc-1\""));
    assert!(response.body().contains("\"state\":\"resolved\""));
    assert_eq!(target.comment_repository().state_update_count.get(), 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("body must-not-log") && !event.contains("rawBody"))
    );
}

#[test]
fn comment_resolve_handler_returns_missing_thread_without_body_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("writer-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/comments/missing-thread/resolve",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"documentId\":\"doc-1\",\"rawBody\":\"must-not-log\"}",
            ),
        ),
    )
    .expect("missing comment resolve handler");

    assert_eq!(response.status_code(), 404);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"COMMENT_THREAD_NOT_FOUND\"")
    );
    assert_eq!(target.comment_repository().state_update_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log"))
    );
}

#[test]
fn comment_reopen_handler_delegates_to_usecase_and_returns_reopened_thread_with_safe_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut comment_repository = FakeCommentRepository::default();
    let resolved_thread = CommentThread::new(
        CommentThreadId::new("thread-1").expect("thread id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        DocumentId::new("doc-1").expect("document id"),
        comment("comment-1", "writer-1234", "body must-not-log"),
    )
    .with_state(CommentThreadState::Resolved);
    comment_repository.insert(resolved_thread);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("writer-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        comment_repository,
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/comments/thread-1/reopen",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"documentId\":\"doc-1\",\"rawBody\":\"must-not-log\"}",
            ),
        ),
    )
    .expect("comment reopen handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"threadId\":\"thread-1\""));
    assert!(response.body().contains("\"documentId\":\"doc-1\""));
    assert!(response.body().contains("\"state\":\"reopened\""));
    assert_eq!(target.comment_repository().state_update_count.get(), 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("body must-not-log") && !event.contains("rawBody"))
    );
}

#[test]
fn comment_reopen_handler_returns_invalid_transition_without_state_update() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut comment_repository = FakeCommentRepository::default();
    comment_repository.insert(CommentThread::new(
        CommentThreadId::new("thread-open").expect("thread id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        DocumentId::new("doc-1").expect("document id"),
        comment("comment-1", "writer-1234", "body must-not-log"),
    ));
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("writer-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        comment_repository,
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/comments/thread-open/reopen",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"documentId\":\"doc-1\",\"rawBody\":\"must-not-log\"}",
            ),
        ),
    )
    .expect("invalid comment reopen handler");

    assert_eq!(response.status_code(), 400);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"COMMENT_INVALID_TRANSITION\"")
    );
    assert_eq!(target.comment_repository().state_update_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("body must-not-log") && !event.contains("must-not-log"))
    );
}

#[test]
fn review_request_handler_delegates_to_usecase_and_records_side_effect_with_safe_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let review_repository = FakeReviewWorkflowRepository::default();
    let side_effect_recorder = FakeReviewWorkflowSideEffectRecorder::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("author-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        review_repository,
        side_effect_recorder,
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-review/review-requests",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"reviewRequestId\":\"review-1\",\"rawBody\":\"must-not-log\",\"documentBody\":\"hidden document body\"}",
            ),
        ),
    )
    .expect("review request handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"documentId\":\"doc-review\""));
    assert!(response.body().contains("\"reviewRequestId\":\"review-1\""));
    assert!(response.body().contains("\"previousState\":\"editing\""));
    assert!(
        response
            .body()
            .contains("\"nextState\":\"review_requested\"")
    );
    assert!(
        response
            .body()
            .contains("\"eventName\":\"review.requested\"")
    );
    assert_eq!(target.review_repository().save_request_count.get(), 1);
    assert_eq!(target.review_repository().save_state_count.get(), 1);
    assert_eq!(target.review_side_effect_recorder().records.len(), 1);
    assert!(logger_events.field_debug_count() >= 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log")
                && !event.contains("hidden document body"))
    );
}

#[test]
fn review_request_handler_returns_invalid_transition_without_side_effects_or_body_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut review_repository = FakeReviewWorkflowRepository::default();
    review_repository.insert_state(
        "workspace-1",
        "doc-review",
        PublishWorkflowState::ReviewRequested,
    );
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("author-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        review_repository,
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-review/review-requests",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"reviewRequestId\":\"review-2\",\"rawBody\":\"must-not-log\",\"documentBody\":\"hidden document body\"}",
            ),
        ),
    )
    .expect("invalid review request handler");

    assert_eq!(response.status_code(), 400);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"INVALID_WORKFLOW_TRANSITION\"")
    );
    assert_eq!(target.review_repository().save_request_count.get(), 0);
    assert_eq!(target.review_repository().save_state_count.get(), 0);
    assert_eq!(target.review_side_effect_recorder().records.len(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log")
                && !event.contains("hidden document body"))
    );
}

#[test]
fn review_approve_handler_delegates_to_usecase_and_records_side_effect_with_safe_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut review_repository = FakeReviewWorkflowRepository::default();
    review_repository.insert_state(
        "workspace-1",
        "doc-review",
        PublishWorkflowState::ReviewRequested,
    );
    review_repository.insert_review_request("workspace-1", "review-1", "doc-review", "author-1234");
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("reviewer-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        review_repository,
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/review-requests/review-1/approve",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"approvalNote\":\"must-not-log\",\"documentBody\":\"hidden document body\"}",
            ),
        ),
    )
    .expect("review approve handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"documentId\":\"doc-review\""));
    assert!(response.body().contains("\"reviewRequestId\":\"review-1\""));
    assert!(
        response
            .body()
            .contains("\"previousState\":\"review_requested\"")
    );
    assert!(response.body().contains("\"nextState\":\"approved\""));
    assert!(
        response
            .body()
            .contains("\"eventName\":\"review.approved\"")
    );
    assert_eq!(
        target
            .review_repository()
            .request_status("workspace-1", "review-1"),
        Some(ReviewRequestStatus::Approved)
    );
    assert_eq!(target.review_repository().update_request_count.get(), 1);
    assert_eq!(target.review_repository().save_state_count.get(), 1);
    assert_eq!(target.review_side_effect_recorder().records.len(), 1);
    assert!(logger_events.field_debug_count() >= 2);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log")
                && !event.contains("hidden document body"))
    );
}

#[test]
fn review_approve_handler_returns_permission_denied_without_mutation_or_body_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut review_repository = FakeReviewWorkflowRepository::default();
    review_repository.insert_state(
        "workspace-1",
        "doc-review",
        PublishWorkflowState::ReviewRequested,
    );
    review_repository.insert_review_request("workspace-1", "review-1", "doc-review", "author-1234");
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("reviewer-1234"),
        FakePermissionChecker::denied(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        review_repository,
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/review-requests/review-1/approve",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"approvalNote\":\"must-not-log\",\"documentBody\":\"hidden document body\"}",
            ),
        ),
    )
    .expect("review approve denied handler");

    assert_eq!(response.status_code(), 403);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"REVIEW_PERMISSION_REQUIRED\"")
    );
    assert_eq!(
        target
            .review_repository()
            .request_status("workspace-1", "review-1"),
        Some(ReviewRequestStatus::ReviewRequested)
    );
    assert_eq!(target.review_repository().update_request_count.get(), 0);
    assert_eq!(target.review_repository().save_state_count.get(), 0);
    assert_eq!(target.review_side_effect_recorder().records.len(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log")
                && !event.contains("hidden document body"))
    );
}

#[test]
fn review_reject_handler_delegates_to_usecase_and_records_side_effect_with_safe_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut review_repository = FakeReviewWorkflowRepository::default();
    review_repository.insert_state(
        "workspace-1",
        "doc-review",
        PublishWorkflowState::ReviewRequested,
    );
    review_repository.insert_review_request("workspace-1", "review-1", "doc-review", "author-1234");
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("reviewer-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        review_repository,
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/review-requests/review-1/reject",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"rejectionReason\":\"must-not-log\",\"documentBody\":\"hidden document body\"}",
            ),
        ),
    )
    .expect("review reject handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"documentId\":\"doc-review\""));
    assert!(response.body().contains("\"reviewRequestId\":\"review-1\""));
    assert!(
        response
            .body()
            .contains("\"previousState\":\"review_requested\"")
    );
    assert!(response.body().contains("\"nextState\":\"rejected\""));
    assert!(
        response
            .body()
            .contains("\"eventName\":\"review.rejected\"")
    );
    assert_eq!(
        target
            .review_repository()
            .request_status("workspace-1", "review-1"),
        Some(ReviewRequestStatus::Rejected)
    );
    assert_eq!(target.review_repository().update_request_count.get(), 1);
    assert_eq!(target.review_repository().save_state_count.get(), 1);
    assert_eq!(target.review_side_effect_recorder().records.len(), 1);
    assert!(logger_events.field_debug_count() >= 2);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log")
                && !event.contains("hidden document body"))
    );
}

#[test]
fn review_reject_handler_returns_permission_denied_without_mutation_or_body_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut review_repository = FakeReviewWorkflowRepository::default();
    review_repository.insert_state(
        "workspace-1",
        "doc-review",
        PublishWorkflowState::ReviewRequested,
    );
    review_repository.insert_review_request("workspace-1", "review-1", "doc-review", "author-1234");
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("reviewer-1234"),
        FakePermissionChecker::denied(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        review_repository,
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/review-requests/review-1/reject",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"rejectionReason\":\"must-not-log\",\"documentBody\":\"hidden document body\"}",
            ),
        ),
    )
    .expect("review reject denied handler");

    assert_eq!(response.status_code(), 403);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"REVIEW_PERMISSION_REQUIRED\"")
    );
    assert_eq!(
        target
            .review_repository()
            .request_status("workspace-1", "review-1"),
        Some(ReviewRequestStatus::ReviewRequested)
    );
    assert_eq!(target.review_repository().update_request_count.get(), 0);
    assert_eq!(target.review_repository().save_state_count.get(), 0);
    assert_eq!(target.review_side_effect_recorder().records.len(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log")
                && !event.contains("hidden document body"))
    );
}

#[test]
fn review_publish_handler_delegates_to_usecase_and_records_side_effect_with_safe_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut review_repository = FakeReviewWorkflowRepository::default();
    review_repository.insert_state("workspace-1", "doc-review", PublishWorkflowState::Approved);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("publisher-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        review_repository,
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-review/publish",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"publishNote\":\"must-not-log\",\"documentBody\":\"hidden document body\"}",
            ),
        ),
    )
    .expect("review publish handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"documentId\":\"doc-review\""));
    assert!(response.body().contains("\"reviewRequestId\":null"));
    assert!(response.body().contains("\"previousState\":\"approved\""));
    assert!(response.body().contains("\"nextState\":\"published\""));
    assert!(
        response
            .body()
            .contains("\"eventName\":\"document.published\"")
    );
    assert_eq!(
        target
            .review_repository()
            .workflow_state("workspace-1", "doc-review"),
        Some(PublishWorkflowState::Published)
    );
    assert_eq!(target.review_repository().save_state_count.get(), 1);
    assert_eq!(target.review_side_effect_recorder().records.len(), 2);
    assert!(logger_events.field_debug_count() >= 2);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log")
                && !event.contains("hidden document body"))
    );
}

#[test]
fn review_publish_handler_returns_permission_denied_without_state_change_or_body_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut review_repository = FakeReviewWorkflowRepository::default();
    review_repository.insert_state("workspace-1", "doc-review", PublishWorkflowState::Approved);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("publisher-1234"),
        FakePermissionChecker::denied(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        review_repository,
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-review/publish",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"publishNote\":\"must-not-log\",\"documentBody\":\"hidden document body\"}",
            ),
        ),
    )
    .expect("review publish denied handler");

    assert_eq!(response.status_code(), 403);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"PUBLISH_PERMISSION_REQUIRED\"")
    );
    assert_eq!(
        target
            .review_repository()
            .workflow_state("workspace-1", "doc-review"),
        Some(PublishWorkflowState::Approved)
    );
    assert_eq!(target.review_repository().save_state_count.get(), 0);
    assert_eq!(target.review_side_effect_recorder().records.len(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log")
                && !event.contains("hidden document body"))
    );
}

#[test]
fn review_list_handler_delegates_to_usecase_and_returns_filtered_safe_summaries() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut review_repository = FakeReviewWorkflowRepository::default();
    review_repository.insert_review_request("workspace-1", "review-1", "doc-review", "author-1234");
    review_repository.insert_review_request("workspace-1", "review-2", "doc-other", "author-5678");
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("reader-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        review_repository,
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/review-requests?workspaceId=workspace-1&documentId=doc-review&rawQuery=must-not-log",
            None,
        ),
    )
    .expect("review list handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"requestCount\":1"));
    assert!(response.body().contains("\"reviewRequestId\":\"review-1\""));
    assert!(response.body().contains("\"documentId\":\"doc-review\""));
    assert!(response.body().contains("\"requestedBy\":\"author-1234\""));
    assert!(response.body().contains("\"status\":\"review_requested\""));
    assert!(!response.body().contains("review-2"));
    assert_eq!(target.review_repository().list_count.get(), 1);
    assert!(logger_events.field_debug_count() >= 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log"))
    );
}

#[test]
fn review_list_handler_returns_permission_denied_before_repository_list_or_raw_query_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut review_repository = FakeReviewWorkflowRepository::default();
    review_repository.insert_review_request("workspace-1", "review-1", "doc-review", "author-1234");
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("reader-1234"),
        FakePermissionChecker::denied(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        review_repository,
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/review-requests?workspaceId=workspace-1&documentId=doc-review&rawQuery=must-not-log",
            None,
        ),
    )
    .expect("review list denied handler");

    assert_eq!(response.status_code(), 403);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"REVIEW_WORKFLOW_UNAUTHORIZED\"")
    );
    assert_eq!(target.review_repository().list_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log"))
    );
}

#[test]
fn document_lock_handler_delegates_to_usecase_and_returns_safe_lock_response() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("locker-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-lock/locks",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"lockId\":\"lock-1\",\"rawBody\":\"must-not-log\",\"documentBody\":\"hidden document body\"}",
            ),
        ),
    )
    .expect("document lock handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"status\":\"locked\""));
    assert!(response.body().contains("\"lockId\":\"lock-1\""));
    assert!(response.body().contains("\"documentId\":\"doc-lock\""));
    assert!(response.body().contains("\"ownerUserId\":\"locker-1234\""));
    assert!(response.body().contains("\"acquiredAtMillis\":1000"));
    assert!(response.body().contains("\"expiresAtMillis\":301000"));
    assert_eq!(target.document_lock_repository().save_count.get(), 1);
    assert_eq!(target.document_lock_repository().delete_count.get(), 0);
    assert!(
        target
            .document_lock_repository()
            .lock_exists("workspace-1", "doc-lock")
    );
    assert!(logger_events.field_debug_count() >= 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log")
                && !event.contains("hidden document body"))
    );
}

#[test]
fn document_lock_handler_returns_permission_denied_before_repository_mutation_or_body_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("locker-1234"),
        FakePermissionChecker::denied(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-lock/locks",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"lockId\":\"lock-1\",\"rawBody\":\"must-not-log\",\"documentBody\":\"hidden document body\"}",
            ),
        ),
    )
    .expect("document lock denied handler");

    assert_eq!(response.status_code(), 403);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"DOCUMENT_LOCK_UNAUTHORIZED\"")
    );
    assert_eq!(target.document_lock_repository().get_count.get(), 0);
    assert_eq!(target.document_lock_repository().save_count.get(), 0);
    assert_eq!(target.document_lock_repository().delete_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log")
                && !event.contains("hidden document body"))
    );
}

#[test]
fn document_unlock_handler_delegates_to_usecase_and_deletes_owned_lock_with_safe_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut lock_repository = FakeDocumentLockRepository::default();
    lock_repository.insert_lock(
        "workspace-1",
        "lock-1",
        "doc-lock",
        "locker-1234",
        1_000,
        301_000,
    );
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("locker-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        lock_repository,
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Delete,
            "/api/documents/doc-lock/locks/current?workspaceId=workspace-1&rawQuery=must-not-log",
            None,
        ),
    )
    .expect("document unlock handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"status\":\"unlocked\""));
    assert!(response.body().contains("\"lock\":null"));
    assert_eq!(target.document_lock_repository().get_count.get(), 1);
    assert_eq!(target.document_lock_repository().delete_count.get(), 1);
    assert_eq!(target.document_lock_repository().save_count.get(), 0);
    assert!(
        !target
            .document_lock_repository()
            .lock_exists("workspace-1", "doc-lock")
    );
    assert!(logger_events.field_debug_count() >= 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log"))
    );
}

#[test]
fn document_unlock_handler_returns_not_owner_without_delete_or_raw_query_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut lock_repository = FakeDocumentLockRepository::default();
    lock_repository.insert_lock(
        "workspace-1",
        "lock-1",
        "doc-lock",
        "owner-1234",
        1_000,
        301_000,
    );
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("other-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        lock_repository,
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Delete,
            "/api/documents/doc-lock/locks/current?workspaceId=workspace-1&rawQuery=must-not-log",
            None,
        ),
    )
    .expect("document unlock not owner handler");

    assert_eq!(response.status_code(), 409);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"DOCUMENT_LOCK_NOT_OWNER\"")
    );
    assert_eq!(target.document_lock_repository().get_count.get(), 1);
    assert_eq!(target.document_lock_repository().delete_count.get(), 0);
    assert!(
        target
            .document_lock_repository()
            .lock_exists("workspace-1", "doc-lock")
    );
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log"))
    );
}

#[test]
fn document_lock_get_handler_returns_active_lock_without_mutation_and_safe_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut lock_repository = FakeDocumentLockRepository::default();
    lock_repository.insert_lock(
        "workspace-1",
        "lock-1",
        "doc-lock",
        "owner-1234",
        1_000,
        301_000,
    );
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("reader-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        lock_repository,
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/documents/doc-lock/locks/current?workspaceId=workspace-1&rawQuery=must-not-log",
            None,
        ),
    )
    .expect("document lock get handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"status\":\"locked\""));
    assert!(response.body().contains("\"lockId\":\"lock-1\""));
    assert!(response.body().contains("\"documentId\":\"doc-lock\""));
    assert!(response.body().contains("\"ownerUserId\":\"owner-1234\""));
    assert!(response.body().contains("\"acquiredAtMillis\":1000"));
    assert!(response.body().contains("\"expiresAtMillis\":301000"));
    assert_eq!(target.document_lock_repository().get_count.get(), 1);
    assert_eq!(target.document_lock_repository().delete_count.get(), 0);
    assert_eq!(target.document_lock_repository().save_count.get(), 0);
    assert!(
        target
            .document_lock_repository()
            .lock_exists("workspace-1", "doc-lock")
    );
    assert!(logger_events.field_debug_count() >= 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log"))
    );
}

#[test]
fn document_lock_get_handler_cleans_expired_lock_with_safe_response() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let mut lock_repository = FakeDocumentLockRepository::default();
    lock_repository.insert_lock(
        "workspace-1",
        "lock-1",
        "doc-lock",
        "owner-1234",
        1_000,
        2_000,
    );
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("reader-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        lock_repository,
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/documents/doc-lock/locks/current?workspaceId=workspace-1&rawQuery=must-not-log",
            None,
        ),
    )
    .expect("document lock get expired handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"status\":\"expired\""));
    assert!(response.body().contains("\"lock\":null"));
    assert_eq!(target.document_lock_repository().get_count.get(), 1);
    assert_eq!(target.document_lock_repository().delete_count.get(), 1);
    assert_eq!(target.document_lock_repository().save_count.get(), 0);
    assert!(
        !target
            .document_lock_repository()
            .lock_exists("workspace-1", "doc-lock")
    );
    assert!(logger_events.field_debug_count() >= 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log"))
    );
}

#[test]
fn document_lock_get_handler_denies_before_repository_lookup_or_raw_query_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("reader-1234"),
        FakePermissionChecker::denied(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/documents/doc-lock/locks/current?workspaceId=workspace-1&rawQuery=must-not-log",
            None,
        ),
    )
    .expect("document lock get denied handler");

    assert_eq!(response.status_code(), 403);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"DOCUMENT_LOCK_UNAUTHORIZED\"")
    );
    assert_eq!(target.document_lock_repository().get_count.get(), 0);
    assert_eq!(target.document_lock_repository().delete_count.get(), 0);
    assert_eq!(target.document_lock_repository().save_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log"))
    );
}

#[test]
fn audit_list_handler_delegates_to_usecase_and_returns_safe_event_page() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let audit_store = FakeAuditLogStore::with_events(vec![
        audit_event(
            "audit-1",
            "workspace-1",
            "actor-1234",
            AuditAction::DocumentPublished,
            "doc-1",
            1_000,
        ),
        audit_event(
            "audit-2",
            "workspace-1",
            "actor-1234",
            AuditAction::LockAcquired,
            "doc-2",
            2_000,
        ),
    ]);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        audit_store,
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/audit-events?workspaceId=workspace-1&scope=workspace&limit=1&cursor=0&rawQuery=must-not-log",
            None,
        ),
    )
    .expect("audit list handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"eventCount\":1"));
    assert!(response.body().contains("\"nextCursor\":\"1\""));
    assert!(response.body().contains("\"retentionDays\":365"));
    assert!(response.body().contains("\"eventId\":\"audit-1\""));
    assert!(response.body().contains("\"workspaceId\":\"workspace-1\""));
    assert!(response.body().contains("\"actorType\":\"user\""));
    assert!(response.body().contains("\"actorId\":\"actor-1234\""));
    assert!(
        response
            .body()
            .contains("\"action\":\"document.published\"")
    );
    assert!(response.body().contains("\"targetType\":\"document\""));
    assert!(response.body().contains("\"targetId\":\"doc-1\""));
    assert!(response.body().contains("\"documentId\":\"doc-1\""));
    assert!(response.body().contains("\"occurredAtMillis\":1000"));
    assert!(response.body().contains("\"key\":\"source\""));
    assert!(response.body().contains("\"value\":\"runtime-test\""));
    assert!(!response.body().contains("must-not-log"));
    assert_eq!(target.audit_store().list_count.get(), 1);
    assert_eq!(target.audit_store().append_count.get(), 0);
    assert!(logger_events.field_debug_count() >= 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log"))
    );
}

#[test]
fn audit_list_handler_returns_permission_denied_before_store_query_or_raw_query_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("viewer-1234"),
        FakePermissionChecker::denied(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/audit-events?workspaceId=workspace-1&scope=workspace&limit=50&rawQuery=must-not-log",
            None,
        ),
    )
    .expect("audit list denied handler");

    assert_eq!(response.status_code(), 403);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"AUDIT_UNAUTHORIZED\"")
    );
    assert_eq!(target.audit_store().list_count.get(), 0);
    assert_eq!(target.audit_store().append_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log"))
    );
}

#[test]
fn audit_list_handler_returns_invalid_cursor_without_permission_or_store_side_effects() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/audit-events?workspaceId=workspace-1&scope=workspace&limit=50&cursor=bad-cursor&rawQuery=must-not-log",
            None,
        ),
    )
    .expect("audit list invalid cursor handler");

    assert_eq!(response.status_code(), 400);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"AUDIT_INVALID_CURSOR\"")
    );
    assert_eq!(target.audit_store().list_count.get(), 0);
    assert_eq!(target.audit_store().append_count.get(), 0);
    assert!(logger_events.product_events().is_empty());
}

#[test]
fn backup_create_handler_delegates_to_usecase_and_returns_safe_queued_job() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("backup-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/backups",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"jobId\":\"backup-job-1\",\"rawBody\":\"must-not-log\",\"documentBody\":\"hidden document body\"}",
            ),
        ),
    )
    .expect("backup create handler");

    assert_eq!(response.status_code(), 202);
    assert!(response.body().contains("\"jobId\":\"backup-job-1\""));
    assert!(response.body().contains("\"workspaceId\":\"workspace-1\""));
    assert!(response.body().contains("\"operation\":\"backup\""));
    assert!(response.body().contains("\"state\":\"queued\""));
    assert!(response.body().contains("\"retryCount\":0"));
    assert!(response.body().contains("\"completedUnits\":0"));
    assert!(response.body().contains("\"totalUnits\":1"));
    assert!(response.body().contains("\"errorCode\":null"));
    assert!(!response.body().contains("must-not-log"));
    assert!(!response.body().contains("hidden document body"));
    assert_eq!(target.backup_store().save_count.get(), 1);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 1);
    assert!(
        target
            .backup_store()
            .job_exists("workspace-1", "backup-job-1")
    );
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log")
                && !event.contains("hidden document body"))
    );
}

#[test]
fn backup_create_handler_returns_invalid_input_without_store_or_audit_side_effects() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("backup-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/backups",
            Some("{\"workspaceId\":\"workspace-1\",\"jobId\":\"\",\"rawBody\":\"must-not-log\"}"),
        ),
    )
    .expect("backup create invalid input handler");

    assert_eq!(response.status_code(), 400);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"BACKUP_JOB_INVALID_INPUT\"")
    );
    assert_eq!(target.backup_store().save_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(logger_events.product_events().is_empty());
}

#[test]
fn backup_create_handler_returns_storage_failure_without_audit_or_raw_body_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("backup-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::with_save_error(BackupStoreError::StorageUnavailable),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/backups",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"jobId\":\"backup-job-1\",\"rawBody\":\"must-not-log\",\"token\":\"secret-token\"}",
            ),
        ),
    )
    .expect("backup create storage failure handler");

    assert_eq!(response.status_code(), 503);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"BACKUP_JOB_STORAGE_UNAVAILABLE\"")
    );
    assert_eq!(target.backup_store().save_count.get(), 1);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log") && !event.contains("secret-token"))
    );
}

#[test]
fn backup_status_handler_delegates_to_usecase_and_returns_safe_current_job() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let backup_store = FakeBackupStore::with_jobs(vec![backup_snapshot(
        "workspace-1",
        "backup-job-1",
        BackupJobOperation::Backup,
        BackupJobState::Completed,
        1,
        1,
        1,
    )]);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("backup-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        backup_store,
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/backups/backup-job-1?workspaceId=workspace-1&rawQuery=must-not-log&token=secret-token",
            None,
        ),
    )
    .expect("backup status handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"jobId\":\"backup-job-1\""));
    assert!(response.body().contains("\"workspaceId\":\"workspace-1\""));
    assert!(response.body().contains("\"operation\":\"backup\""));
    assert!(response.body().contains("\"state\":\"completed\""));
    assert!(response.body().contains("\"retryCount\":1"));
    assert!(response.body().contains("\"completedUnits\":1"));
    assert!(response.body().contains("\"totalUnits\":1"));
    assert!(response.body().contains("\"errorCode\":null"));
    assert!(!response.body().contains("must-not-log"));
    assert!(!response.body().contains("secret-token"));
    assert_eq!(target.backup_store().get_count.get(), 1);
    assert_eq!(target.backup_store().save_count.get(), 0);
    assert_eq!(target.backup_store().apply_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log") && !event.contains("secret-token"))
    );
}

#[test]
fn backup_status_handler_returns_not_found_without_mutation_or_raw_query_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("backup-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/backups/missing-job?workspaceId=workspace-1&rawQuery=must-not-log",
            None,
        ),
    )
    .expect("backup status missing handler");

    assert_eq!(response.status_code(), 404);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"BACKUP_JOB_NOT_FOUND\"")
    );
    assert_eq!(target.backup_store().get_count.get(), 1);
    assert_eq!(target.backup_store().save_count.get(), 0);
    assert_eq!(target.backup_store().apply_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log"))
    );
}

#[test]
fn backup_status_handler_returns_malformed_query_before_store_access() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("backup-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/backups/backup-job-1?rawQuery=must-not-log",
            None,
        ),
    )
    .expect("backup status malformed query handler");

    assert_eq!(response.status_code(), 400);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"SERVER_MALFORMED_REQUEST\"")
    );
    assert_eq!(target.backup_store().get_count.get(), 0);
    assert_eq!(target.backup_store().save_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(logger_events.product_events().is_empty());
}

#[test]
fn backup_status_handler_returns_storage_failure_without_raw_query_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("backup-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::with_get_error(BackupStoreError::StorageUnavailable),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/backups/backup-job-1?workspaceId=workspace-1&rawQuery=must-not-log&token=secret-token",
            None,
        ),
    )
    .expect("backup status storage failure handler");

    assert_eq!(response.status_code(), 503);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"BACKUP_JOB_STORAGE_UNAVAILABLE\"")
    );
    assert_eq!(target.backup_store().get_count.get(), 1);
    assert_eq!(target.backup_store().save_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log") && !event.contains("secret-token"))
    );
}

#[test]
fn backup_restore_handler_delegates_to_usecase_and_returns_completed_job() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("backup-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/backups/backup-job-1/restore",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"jobId\":\"restore-job-1\",\"rawBody\":\"must-not-log\",\"token\":\"secret-token\"}",
            ),
        ),
    )
    .expect("backup restore handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"jobId\":\"restore-job-1\""));
    assert!(response.body().contains("\"workspaceId\":\"workspace-1\""));
    assert!(response.body().contains("\"operation\":\"restore\""));
    assert!(response.body().contains("\"state\":\"completed\""));
    assert!(response.body().contains("\"completedUnits\":1"));
    assert!(response.body().contains("\"totalUnits\":1"));
    assert!(response.body().contains("\"errorCode\":null"));
    assert!(!response.body().contains("must-not-log"));
    assert!(!response.body().contains("secret-token"));
    assert_eq!(target.backup_store().save_count.get(), 3);
    assert_eq!(target.backup_store().validate_count.get(), 1);
    assert_eq!(target.backup_store().apply_count.get(), 1);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 1);
    assert!(
        target
            .backup_store()
            .job_exists("workspace-1", "restore-job-1")
    );
    assert!(
        logger_events
            .product_events()
            .iter()
            .any(|event| event.contains("restore.completed"))
    );
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log") && !event.contains("secret-token"))
    );
}

#[test]
fn backup_restore_handler_returns_malformed_body_before_store_or_audit_side_effects() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("backup-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/backups/backup-job-1/restore",
            Some("{\"jobId\":\"restore-job-1\",\"rawBody\":\"must-not-log\"}"),
        ),
    )
    .expect("backup restore malformed body handler");

    assert_eq!(response.status_code(), 400);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"SERVER_MALFORMED_REQUEST\"")
    );
    assert_eq!(target.backup_store().save_count.get(), 0);
    assert_eq!(target.backup_store().validate_count.get(), 0);
    assert_eq!(target.backup_store().apply_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(logger_events.product_events().is_empty());
}

#[test]
fn backup_restore_handler_preserves_workspace_when_staging_validation_fails() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("backup-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::with_restore_validation(RestoreValidation::failed(
            "BACKUP_ARTIFACT_CORRUPTED",
        )),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/backups/backup-job-1/restore",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"jobId\":\"restore-job-1\",\"rawBody\":\"must-not-log\",\"documentBody\":\"hidden document body\"}",
            ),
        ),
    )
    .expect("backup restore failed validation handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"jobId\":\"restore-job-1\""));
    assert!(response.body().contains("\"operation\":\"restore\""));
    assert!(response.body().contains("\"state\":\"abandoned\""));
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"BACKUP_ARTIFACT_CORRUPTED\"")
    );
    assert!(!response.body().contains("must-not-log"));
    assert!(!response.body().contains("hidden document body"));
    assert_eq!(target.backup_store().save_count.get(), 3);
    assert_eq!(target.backup_store().validate_count.get(), 1);
    assert_eq!(target.backup_store().apply_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 1);
    assert!(
        logger_events
            .product_events()
            .iter()
            .any(|event| event.contains("restore.failed"))
    );
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log")
                && !event.contains("hidden document body"))
    );
}

#[test]
fn export_create_handler_delegates_to_usecase_and_returns_safe_queued_job() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("export-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/exports",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"jobId\":\"export-job-1\",\"rawBody\":\"must-not-log\",\"token\":\"secret-token\"}",
            ),
        ),
    )
    .expect("export create handler");

    assert_eq!(response.status_code(), 202);
    assert!(response.body().contains("\"jobId\":\"export-job-1\""));
    assert!(response.body().contains("\"workspaceId\":\"workspace-1\""));
    assert!(response.body().contains("\"operation\":\"export\""));
    assert!(response.body().contains("\"state\":\"queued\""));
    assert!(response.body().contains("\"retryCount\":0"));
    assert!(response.body().contains("\"completedUnits\":0"));
    assert!(response.body().contains("\"totalUnits\":1"));
    assert!(response.body().contains("\"errorCode\":null"));
    assert!(!response.body().contains("must-not-log"));
    assert!(!response.body().contains("secret-token"));
    assert_eq!(target.backup_store().save_count.get(), 1);
    assert_eq!(target.backup_store().apply_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(
        target
            .backup_store()
            .job_exists("workspace-1", "export-job-1")
    );
    assert!(logger_events.product_events().is_empty());
}

#[test]
fn export_create_handler_returns_malformed_body_without_store_side_effects() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("export-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/exports",
            Some("{\"jobId\":\"export-job-1\",\"rawBody\":\"must-not-log\"}"),
        ),
    )
    .expect("export create malformed body handler");

    assert_eq!(response.status_code(), 400);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"SERVER_MALFORMED_REQUEST\"")
    );
    assert_eq!(target.backup_store().save_count.get(), 0);
    assert_eq!(target.backup_store().apply_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(logger_events.product_events().is_empty());
}

#[test]
fn export_create_handler_returns_storage_failure_without_raw_body_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("export-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::with_save_error(BackupStoreError::StorageUnavailable),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/exports",
            Some(
                "{\"workspaceId\":\"workspace-1\",\"jobId\":\"export-job-1\",\"rawBody\":\"must-not-log\",\"token\":\"secret-token\"}",
            ),
        ),
    )
    .expect("export create storage failure handler");

    assert_eq!(response.status_code(), 503);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"BACKUP_JOB_STORAGE_UNAVAILABLE\"")
    );
    assert_eq!(target.backup_store().save_count.get(), 1);
    assert_eq!(target.backup_store().apply_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log") && !event.contains("secret-token"))
    );
}

#[test]
fn export_status_handler_delegates_to_usecase_and_returns_safe_current_job() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let backup_store = FakeBackupStore::with_jobs(vec![backup_snapshot(
        "workspace-1",
        "export-job-1",
        BackupJobOperation::Export,
        BackupJobState::Completed,
        1,
        1,
        1,
    )]);
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("export-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        backup_store,
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/exports/export-job-1?workspaceId=workspace-1&rawQuery=must-not-log&token=secret-token",
            None,
        ),
    )
    .expect("export status handler");

    assert_eq!(response.status_code(), 200);
    assert!(response.body().contains("\"jobId\":\"export-job-1\""));
    assert!(response.body().contains("\"workspaceId\":\"workspace-1\""));
    assert!(response.body().contains("\"operation\":\"export\""));
    assert!(response.body().contains("\"state\":\"completed\""));
    assert!(response.body().contains("\"retryCount\":1"));
    assert!(response.body().contains("\"completedUnits\":1"));
    assert!(response.body().contains("\"totalUnits\":1"));
    assert!(response.body().contains("\"errorCode\":null"));
    assert!(!response.body().contains("must-not-log"));
    assert!(!response.body().contains("secret-token"));
    assert_eq!(target.backup_store().get_count.get(), 1);
    assert_eq!(target.backup_store().save_count.get(), 0);
    assert_eq!(target.backup_store().apply_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(logger_events.product_events().is_empty());
}

#[test]
fn export_status_handler_returns_not_found_without_mutation_or_raw_query_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("export-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/exports/missing-export?workspaceId=workspace-1&rawQuery=must-not-log",
            None,
        ),
    )
    .expect("export status missing handler");

    assert_eq!(response.status_code(), 404);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"BACKUP_JOB_NOT_FOUND\"")
    );
    assert_eq!(target.backup_store().get_count.get(), 1);
    assert_eq!(target.backup_store().save_count.get(), 0);
    assert_eq!(target.backup_store().apply_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(logger_events.product_events().is_empty());
}

#[test]
fn export_status_handler_returns_malformed_query_before_store_access() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("export-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/exports/export-job-1?rawQuery=must-not-log",
            None,
        ),
    )
    .expect("export status malformed query handler");

    assert_eq!(response.status_code(), 400);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"SERVER_MALFORMED_REQUEST\"")
    );
    assert_eq!(target.backup_store().get_count.get(), 0);
    assert_eq!(target.backup_store().save_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(logger_events.product_events().is_empty());
}

#[test]
fn export_status_handler_returns_storage_failure_without_raw_query_logs() {
    let composition = build_server_composition(default_config());
    let logger_events = SharedRuntimeLog::default();
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("export-admin-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::with_get_error(BackupStoreError::StorageUnavailable),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(2_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        logger_events.clone(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/exports/export-job-1?workspaceId=workspace-1&rawQuery=must-not-log&token=secret-token",
            None,
        ),
    )
    .expect("export status storage failure handler");

    assert_eq!(response.status_code(), 503);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"BACKUP_JOB_STORAGE_UNAVAILABLE\"")
    );
    assert_eq!(target.backup_store().get_count.get(), 1);
    assert_eq!(target.backup_store().save_count.get(), 0);
    assert_eq!(target.backup_audit_recorder().record_count.get(), 0);
    assert!(
        logger_events
            .product_events()
            .iter()
            .all(|event| !event.contains("must-not-log") && !event.contains("secret-token"))
    );
}

#[test]
fn malformed_runtime_request_returns_stable_safe_error_response() {
    let composition = build_server_composition(default_config());
    let target = ServerRuntimeTarget::new(
        RuntimeActorContext::new("user-1234"),
        FakePermissionChecker::allowed(),
        FakeDocumentRepository::default(),
        FakeDocumentQuery::default(),
        FakeVersionStore::default(),
        FakeSearchIndex::default(),
        FakeAuditLogStore::default(),
        FakeBackupStore::default(),
        FakeBackupAuditRecorder::default(),
        FakeCredentialVerifier::default(),
        FakeTokenIssuer::default(),
        FakeSessionStore::default(),
        FakeAuthClock::at(100),
        FakeSessionIdGenerator::default(),
        FakeGroupRepository::default(),
        FakePermissionPolicyRepository::default(),
        FakeRoleAssignmentIdGenerator::default(),
        FakeUserRepository::default(),
        FakeFieldDebugSessionRepository::default(),
        FakePermissionChecker::allowed(),
        FakeFieldDebugClock::new(1_000),
        FakeDocumentChangePublisher::default(),
        FakeDocumentLockRepository::default(),
        FakeDocumentLockClock::at(1_000),
        FakeCommentRepository::default(),
        FakeReviewWorkflowRepository::default(),
        FakeReviewWorkflowSideEffectRecorder::default(),
        SharedRuntimeLog::default(),
        RuntimePolicy::from_config(composition.config()),
    );

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Get,
            "/api/workspaces/workspace-1/search?limit=20",
            None,
        ),
    )
    .expect("handler should render safe error response");

    assert_eq!(response.status_code(), 400);
    assert!(
        response
            .body()
            .contains("\"errorCode\":\"SERVER_MALFORMED_REQUEST\"")
    );
    assert!(!response.body().contains("runtime"));
}

#[derive(Clone, Default)]
struct SharedRuntimeLog {
    product: Rc<RefCell<Vec<String>>>,
    field_debug_count: Rc<Cell<usize>>,
}

impl SharedRuntimeLog {
    fn product_events(&self) -> Vec<String> {
        self.product.borrow().clone()
    }

    fn field_debug_count(&self) -> usize {
        self.field_debug_count.get()
    }
}

impl AccessibleQueryLogger for SharedRuntimeLog {
    fn write_product(&mut self, event: AccessibleQueryProductEvent) {
        self.product.borrow_mut().push(format!("{event:?}"));
    }

    fn write_field_debug(&mut self, _event: AccessibleQueryFieldDebugEvent) {
        self.field_debug_count.set(self.field_debug_count.get() + 1);
    }
}

impl FieldDebugUsecaseLogger for SharedRuntimeLog {
    fn write_product(&mut self, event: FieldDebugProductEvent) {
        self.product.borrow_mut().push(format!("{event:?}"));
    }

    fn write_field_debug(&mut self, _event: FieldDebugLogEvent) {
        self.field_debug_count.set(self.field_debug_count.get() + 1);
    }

    fn write_development(&mut self, _event: FieldDebugDevelopmentEvent) {}
}

impl AuthProductLogger for SharedRuntimeLog {
    fn write_product(&mut self, event: AuthProductEvent) {
        self.product.borrow_mut().push(format!("{event:?}"));
    }
}

impl CreateGroupProductLogger for SharedRuntimeLog {
    fn write_product(&mut self, event: CreateGroupProductEvent) {
        self.product.borrow_mut().push(format!("{event:?}"));
    }
}

impl PermissionUsecaseLogger for SharedRuntimeLog {
    fn write_product(&mut self, event: PermissionProductEvent) {
        self.product.borrow_mut().push(format!("{event:?}"));
    }

    fn write_field_debug(&mut self, _event: PermissionFieldDebugEvent) {
        self.field_debug_count.set(self.field_debug_count.get() + 1);
    }
}

impl DocumentProductLogger for SharedRuntimeLog {
    fn write_product(&mut self, event: CreateDocumentProductEvent) {
        self.product.borrow_mut().push(format!("{event:?}"));
    }
}

impl DocumentLockUsecaseLogger for SharedRuntimeLog {
    fn write_product(&mut self, event: DocumentLockProductEvent) {
        self.product.borrow_mut().push(format!("{event:?}"));
    }

    fn write_field_debug(&mut self, _event: DocumentLockFieldDebugEvent) {
        self.field_debug_count.set(self.field_debug_count.get() + 1);
    }
}

impl CommentUsecaseLogger for SharedRuntimeLog {
    fn write_product(&mut self, event: CommentProductEvent) {
        self.product.borrow_mut().push(format!("{event:?}"));
    }

    fn write_field_debug(&mut self, _event: CommentFieldDebugEvent) {
        self.field_debug_count.set(self.field_debug_count.get() + 1);
    }
}

impl ReviewWorkflowUsecaseLogger for SharedRuntimeLog {
    fn write_product(&mut self, event: ReviewWorkflowProductEvent) {
        self.product.borrow_mut().push(format!("{event:?}"));
    }

    fn write_field_debug(&mut self, _event: ReviewWorkflowFieldDebugEvent) {
        self.field_debug_count.set(self.field_debug_count.get() + 1);
    }
}

impl AuditUsecaseLogger for SharedRuntimeLog {
    fn write_product(&mut self, event: AuditProductEvent) {
        self.product.borrow_mut().push(format!("{event:?}"));
    }

    fn write_field_debug(&mut self, _event: AuditFieldDebugEvent) {
        self.field_debug_count.set(self.field_debug_count.get() + 1);
    }
}

impl BackupJobUsecaseLogger for SharedRuntimeLog {
    fn write_product(&mut self, event: BackupJobProductEvent) {
        self.product.borrow_mut().push(format!("{event:?}"));
    }
}

#[derive(Default)]
struct FakeAuditLogStore {
    events: Vec<AuditEvent>,
    append_count: Cell<usize>,
    list_count: Cell<usize>,
}

impl FakeAuditLogStore {
    fn with_events(events: Vec<AuditEvent>) -> Self {
        Self {
            events,
            ..Self::default()
        }
    }
}

impl AuditLogStore for FakeAuditLogStore {
    fn append_audit_event(&mut self, event: AuditEvent) -> Result<(), AuditLogStoreError> {
        self.append_count.set(self.append_count.get() + 1);
        self.events.push(event);
        Ok(())
    }

    fn list_audit_events(
        &self,
        query: AuditListQuery,
    ) -> Result<AuditEventPage, AuditLogStoreError> {
        self.list_count.set(self.list_count.get() + 1);
        let start = query.page().cursor().map_or(0, |cursor| cursor.offset());
        let matching = self
            .events
            .iter()
            .filter(|event| query.matches(event))
            .cloned()
            .collect::<Vec<_>>();
        let events = matching
            .iter()
            .skip(start)
            .take(query.page().limit())
            .cloned()
            .collect::<Vec<_>>();
        let next_offset = start + events.len();
        let next_cursor = if next_offset < matching.len() {
            Some(AuditCursor::from_offset(next_offset))
        } else {
            None
        };
        Ok(AuditEventPage::new(events, next_cursor))
    }
}

fn audit_event(
    event_id: &str,
    workspace_id: &str,
    actor_user_id: &str,
    action: AuditAction,
    document_id: &str,
    occurred_at_millis: u64,
) -> AuditEvent {
    AuditEvent::new(
        AuditEventId::new(event_id).expect("audit event id"),
        WorkspaceId::new(workspace_id).expect("workspace id"),
        AuditActor::user(UserId::new(actor_user_id).expect("actor user id")),
        action,
        AuditTarget::document(DocumentId::new(document_id).expect("document id")),
        AuditMetadata::new([("source", "runtime-test")]).expect("audit metadata"),
        AuditTimestamp::from_millis(occurred_at_millis),
    )
}

#[derive(Default)]
struct FakeBackupStore {
    jobs: BTreeMap<String, BackupJobSnapshot>,
    save_error: Option<BackupStoreError>,
    get_error: Option<BackupStoreError>,
    restore_validation: RestoreValidation,
    save_count: Cell<usize>,
    get_count: Cell<usize>,
    validate_count: Cell<usize>,
    apply_count: Cell<usize>,
}

impl FakeBackupStore {
    fn with_jobs(jobs: Vec<BackupJobSnapshot>) -> Self {
        let mut store = Self::default();
        for job in jobs {
            store.jobs.insert(
                backup_job_key(job.workspace_id().as_str(), job.job_id().as_str()),
                job,
            );
        }
        store
    }

    fn with_save_error(save_error: BackupStoreError) -> Self {
        Self {
            save_error: Some(save_error),
            ..Self::default()
        }
    }

    fn with_get_error(get_error: BackupStoreError) -> Self {
        Self {
            get_error: Some(get_error),
            ..Self::default()
        }
    }

    fn with_restore_validation(restore_validation: RestoreValidation) -> Self {
        Self {
            restore_validation,
            ..Self::default()
        }
    }

    fn job_exists(&self, workspace_id: &str, job_id: &str) -> bool {
        self.jobs
            .contains_key(&backup_job_key(workspace_id, job_id))
    }
}

impl BackupStore for FakeBackupStore {
    fn save_job(&mut self, job: BackupJobSnapshot) -> Result<(), BackupStoreError> {
        self.save_count.set(self.save_count.get() + 1);
        if let Some(error) = self.save_error {
            return Err(error);
        }
        self.jobs.insert(
            backup_job_key(job.workspace_id().as_str(), job.job_id().as_str()),
            job,
        );
        Ok(())
    }

    fn get_job(
        &self,
        workspace_id: &WorkspaceId,
        job_id: &BackupJobId,
    ) -> Result<Option<BackupJobSnapshot>, BackupStoreError> {
        self.get_count.set(self.get_count.get() + 1);
        if let Some(error) = self.get_error {
            return Err(error);
        }
        Ok(self
            .jobs
            .get(&backup_job_key(workspace_id.as_str(), job_id.as_str()))
            .cloned())
    }

    fn validate_restore_staging(
        &self,
        _workspace_id: &WorkspaceId,
        _source_job_id: &BackupJobId,
    ) -> Result<RestoreValidation, BackupStoreError> {
        self.validate_count.set(self.validate_count.get() + 1);
        Ok(self.restore_validation.clone())
    }

    fn apply_restore_staging(
        &mut self,
        _workspace_id: &WorkspaceId,
        _source_job_id: &BackupJobId,
    ) -> Result<(), BackupStoreError> {
        self.apply_count.set(self.apply_count.get() + 1);
        Ok(())
    }
}

#[derive(Default)]
struct FakeBackupAuditRecorder {
    records: Vec<BackupAuditRecord>,
    record_count: Cell<usize>,
}

impl BackupAuditRecorder for FakeBackupAuditRecorder {
    fn record_backup_audit(
        &mut self,
        record: BackupAuditRecord,
    ) -> Result<(), BackupAuditRecorderError> {
        self.record_count.set(self.record_count.get() + 1);
        self.records.push(record);
        Ok(())
    }
}

#[derive(Default)]
struct FakeCredentialVerifier {
    records: HashMap<String, (User, String)>,
}

impl FakeCredentialVerifier {
    fn insert(&mut self, user: User, accepted_password: &str) {
        self.records.insert(
            user.profile().login().as_str().to_string(),
            (user, accepted_password.to_string()),
        );
    }
}

impl CredentialVerifier for FakeCredentialVerifier {
    fn verify(
        &self,
        login: &UserLogin,
        credential: &CredentialSecret,
    ) -> Result<Option<User>, CredentialVerifierError> {
        let Some((user, accepted_password)) = self.records.get(login.as_str()) else {
            return Ok(None);
        };
        if credential.expose_secret() != accepted_password {
            return Ok(None);
        }
        Ok(Some(user.clone()))
    }
}

#[derive(Default)]
struct FakeTokenIssuer {
    next: Cell<u32>,
}

impl TokenIssuer for FakeTokenIssuer {
    fn issue_token(&mut self) -> Result<IssuedSessionToken, TokenIssuerError> {
        self.next.set(self.next.get() + 1);
        let token_value = format!("token-{}", self.next.get());
        Ok(IssuedSessionToken::new(
            PresentedSessionToken::new(&token_value).expect("token"),
            SessionLookupKey::new(&token_value.replace("token-", "lookup-")).expect("lookup"),
        ))
    }

    fn lookup_key_for(
        &self,
        token: &PresentedSessionToken,
    ) -> Result<SessionLookupKey, TokenIssuerError> {
        SessionLookupKey::new(&token.expose_secret().replace("token-", "lookup-"))
            .map_err(|_| TokenIssuerError::InvalidToken)
    }
}

#[derive(Default)]
struct FakeSessionStore {
    sessions: HashMap<String, Session>,
    create_count: Cell<usize>,
    get_count: Cell<usize>,
}

impl SessionStore for FakeSessionStore {
    fn create_session(
        &mut self,
        lookup_key: SessionLookupKey,
        session: Session,
    ) -> Result<(), SessionStoreError> {
        self.create_count.set(self.create_count.get() + 1);
        self.sessions
            .insert(lookup_key.as_str().to_string(), session);
        Ok(())
    }

    fn get_session(
        &self,
        lookup_key: &SessionLookupKey,
    ) -> Result<Option<Session>, SessionStoreError> {
        self.get_count.set(self.get_count.get() + 1);
        Ok(self.sessions.get(lookup_key.as_str()).cloned())
    }

    fn update_session(
        &mut self,
        lookup_key: &SessionLookupKey,
        session: Session,
    ) -> Result<(), SessionStoreError> {
        self.sessions
            .insert(lookup_key.as_str().to_string(), session);
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct FakeAuthClock {
    now: SessionInstant,
}

impl FakeAuthClock {
    fn at(epoch_seconds: u64) -> Self {
        Self {
            now: SessionInstant::from_epoch_seconds(epoch_seconds),
        }
    }
}

impl SessionClock for FakeAuthClock {
    fn now(&self) -> SessionInstant {
        self.now
    }
}

#[derive(Default)]
struct FakeSessionIdGenerator {
    next: Cell<u32>,
}

impl SessionIdGenerator for FakeSessionIdGenerator {
    fn generate_session_id(&mut self) -> String {
        self.next.set(self.next.get() + 1);
        format!("session-{}", self.next.get())
    }
}

#[derive(Default)]
struct FakeRoleAssignmentIdGenerator {
    next: Cell<u32>,
}

impl RoleAssignmentIdGenerator for FakeRoleAssignmentIdGenerator {
    fn generate_role_assignment_id(&mut self) -> String {
        self.next.set(self.next.get() + 1);
        format!("role-assignment-{}", self.next.get())
    }
}

#[derive(Default)]
struct FakeGroupRepository {
    groups: HashMap<String, Group>,
    memberships: Vec<(String, String, String)>,
    list_group_count: Cell<usize>,
}

impl FakeGroupRepository {
    fn with_group(workspace_id: &str, group_id: &str, name: &str) -> Self {
        let workspace_id = WorkspaceId::new(workspace_id).expect("workspace id");
        let group = Group::new(
            GroupId::new(group_id).expect("group id"),
            workspace_id.clone(),
            GroupName::new(name).expect("group name"),
        );
        let mut repository = Self::default();
        repository
            .groups
            .insert(group_key(&workspace_id, group.id()), group);
        repository
    }

    fn add_member(&mut self, workspace_id: &str, group_id: &str, user_id: &str) {
        self.memberships.push((
            workspace_id.to_string(),
            group_id.to_string(),
            user_id.to_string(),
        ));
    }
}

impl GroupRepository for FakeGroupRepository {
    fn find_group_by_name(
        &self,
        workspace_id: &WorkspaceId,
        name: &GroupName,
    ) -> Result<Option<Group>, GroupRepositoryError> {
        Ok(self
            .groups
            .values()
            .find(|group| {
                group.workspace_id() == workspace_id
                    && group.name().duplicate_key() == name.duplicate_key()
            })
            .cloned())
    }

    fn get_group(
        &self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
    ) -> Result<Option<Group>, GroupRepositoryError> {
        Ok(self.groups.get(&group_key(workspace_id, group_id)).cloned())
    }

    fn save_group(
        &mut self,
        workspace_id: &WorkspaceId,
        group: Group,
    ) -> Result<(), GroupRepositoryError> {
        self.groups
            .insert(group_key(workspace_id, group.id()), group);
        Ok(())
    }

    fn has_membership(
        &self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
        user_id: &UserId,
    ) -> Result<bool, GroupRepositoryError> {
        Ok(self
            .memberships
            .iter()
            .any(|(stored_workspace_id, stored_group_id, stored_user_id)| {
                stored_workspace_id == workspace_id.as_str()
                    && stored_group_id == group_id.as_str()
                    && stored_user_id == user_id.as_str()
            }))
    }

    fn add_membership(
        &mut self,
        workspace_id: &WorkspaceId,
        membership: GroupMembership,
    ) -> Result<MembershipMutationResult, GroupRepositoryError> {
        if self.has_membership(workspace_id, membership.group_id(), membership.user_id())? {
            return Ok(MembershipMutationResult::AlreadyApplied);
        }
        self.memberships.push((
            workspace_id.as_str().to_string(),
            membership.group_id().as_str().to_string(),
            membership.user_id().as_str().to_string(),
        ));
        Ok(MembershipMutationResult::Changed)
    }

    fn remove_membership(
        &mut self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
        user_id: &UserId,
    ) -> Result<MembershipMutationResult, GroupRepositoryError> {
        let before = self.memberships.len();
        self.memberships
            .retain(|(stored_workspace_id, stored_group_id, stored_user_id)| {
                stored_workspace_id != workspace_id.as_str()
                    || stored_group_id != group_id.as_str()
                    || stored_user_id != user_id.as_str()
            });
        if self.memberships.len() == before {
            return Ok(MembershipMutationResult::Missing);
        }
        Ok(MembershipMutationResult::Changed)
    }

    fn list_workspace_memberships(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<GroupMembership>, GroupRepositoryError> {
        Ok(self
            .memberships
            .iter()
            .filter(|(stored_workspace_id, _, _)| stored_workspace_id == workspace_id.as_str())
            .map(|(_, group_id, user_id)| {
                GroupMembership::new(
                    GroupId::new(group_id).expect("group id"),
                    UserId::new(user_id).expect("user id"),
                )
            })
            .collect())
    }

    fn list_workspace_groups(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Group>, GroupRepositoryError> {
        self.list_group_count.set(self.list_group_count.get() + 1);
        let mut groups = self
            .groups
            .values()
            .filter(|group| group.workspace_id() == workspace_id)
            .cloned()
            .collect::<Vec<_>>();
        groups.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(groups)
    }
}

impl PermissionGroupRepository for FakeGroupRepository {
    fn list_user_group_ids(
        &self,
        workspace_id: &WorkspaceId,
        user_id: &UserId,
    ) -> Result<Vec<GroupId>, PermissionRepositoryError> {
        let mut group_ids = self
            .memberships
            .iter()
            .filter(|(stored_workspace_id, _, stored_user_id)| {
                stored_workspace_id == workspace_id.as_str() && stored_user_id == user_id.as_str()
            })
            .map(|(_, group_id, _)| GroupId::new(group_id).expect("group id"))
            .collect::<Vec<_>>();
        group_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        Ok(group_ids)
    }
}

#[derive(Default)]
struct FakePermissionPolicyRepository {
    assignments: HashMap<String, RoleAssignment>,
    document_policies: HashMap<String, DocumentPolicy>,
    saved_document_policy_count: Cell<usize>,
    list_workspace_count: Cell<usize>,
}

impl FakePermissionPolicyRepository {
    fn with_assignments(assignments: Vec<RoleAssignment>) -> Self {
        let mut repository = Self::default();
        for assignment in assignments {
            repository.assignments.insert(
                role_assignment_key(assignment.workspace_id(), assignment.id()),
                assignment,
            );
        }
        repository
    }
}

impl PermissionPolicyRepository for FakePermissionPolicyRepository {
    fn list_user_role_assignments(
        &self,
        workspace_id: &WorkspaceId,
        user_id: &UserId,
    ) -> Result<Vec<RoleAssignment>, PermissionRepositoryError> {
        Ok(self
            .assignments
            .values()
            .filter(|assignment| {
                assignment.workspace_id() == workspace_id
                    && assignment.subject() == &RoleAssignmentSubject::User(user_id.clone())
            })
            .cloned()
            .collect())
    }

    fn list_group_role_assignments(
        &self,
        workspace_id: &WorkspaceId,
        group_ids: &[GroupId],
    ) -> Result<Vec<RoleAssignment>, PermissionRepositoryError> {
        Ok(self
            .assignments
            .values()
            .filter(|assignment| {
                assignment.workspace_id() == workspace_id
                    && matches!(
                        assignment.subject(),
                        RoleAssignmentSubject::Group(group_id)
                            if group_ids.iter().any(|candidate| candidate == group_id)
                    )
            })
            .cloned()
            .collect())
    }

    fn list_workspace_role_assignments(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<RoleAssignment>, PermissionRepositoryError> {
        self.list_workspace_count
            .set(self.list_workspace_count.get() + 1);
        let mut assignments = self
            .assignments
            .values()
            .filter(|assignment| assignment.workspace_id() == workspace_id)
            .cloned()
            .collect::<Vec<_>>();
        assignments.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(assignments)
    }

    fn get_role_assignment(
        &self,
        workspace_id: &WorkspaceId,
        assignment_id: &RoleAssignmentId,
    ) -> Result<Option<RoleAssignment>, PermissionRepositoryError> {
        Ok(self
            .assignments
            .get(&role_assignment_key(workspace_id, assignment_id))
            .cloned())
    }

    fn save_role_assignment(
        &mut self,
        workspace_id: &WorkspaceId,
        assignment: RoleAssignment,
    ) -> Result<RoleAssignmentMutationResult, PermissionRepositoryError> {
        let changed = self
            .assignments
            .insert(
                role_assignment_key(workspace_id, assignment.id()),
                assignment,
            )
            .is_none();
        Ok(if changed {
            RoleAssignmentMutationResult::Changed
        } else {
            RoleAssignmentMutationResult::AlreadyApplied
        })
    }

    fn remove_role_assignment(
        &mut self,
        workspace_id: &WorkspaceId,
        assignment_id: &RoleAssignmentId,
    ) -> Result<RoleAssignmentRemovalResult, PermissionRepositoryError> {
        let removed = self
            .assignments
            .remove(&role_assignment_key(workspace_id, assignment_id))
            .is_some();
        Ok(if removed {
            RoleAssignmentRemovalResult::Removed
        } else {
            RoleAssignmentRemovalResult::Missing
        })
    }

    fn get_collection_policy(
        &self,
        _workspace_id: &WorkspaceId,
        _collection_id: &CollectionId,
    ) -> Result<Option<CollectionPolicy>, PermissionRepositoryError> {
        Ok(None)
    }

    fn save_collection_policy(
        &mut self,
        _workspace_id: &WorkspaceId,
        _policy: CollectionPolicy,
    ) -> Result<(), PermissionRepositoryError> {
        Ok(())
    }

    fn get_document_policy(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentPolicy>, PermissionRepositoryError> {
        Ok(self
            .document_policies
            .get(&document_policy_key(workspace_id, document_id))
            .cloned())
    }

    fn save_document_policy(
        &mut self,
        workspace_id: &WorkspaceId,
        policy: DocumentPolicy,
    ) -> Result<(), PermissionRepositoryError> {
        self.saved_document_policy_count
            .set(self.saved_document_policy_count.get() + 1);
        self.document_policies.insert(
            document_policy_key(workspace_id, policy.document_id()),
            policy,
        );
        Ok(())
    }
}

#[derive(Default)]
struct FakeUserRepository {
    users: HashMap<String, User>,
    list_count: Cell<usize>,
}

impl FakeUserRepository {
    fn with_users(users: Vec<User>) -> Self {
        Self {
            users: users
                .into_iter()
                .map(|user| (user.id().as_str().to_string(), user))
                .collect(),
            list_count: Cell::new(0),
        }
    }
}

impl UserRepository for FakeUserRepository {
    fn find_by_identity(
        &self,
        _login: &UserLogin,
        _email: &UserEmail,
        _external_identity: Option<&cabinet_domain::user::UserExternalIdentity>,
    ) -> Result<Option<User>, UserRepositoryError> {
        Ok(None)
    }

    fn get_user(&self, user_id: &UserId) -> Result<Option<User>, UserRepositoryError> {
        Ok(self.users.get(user_id.as_str()).cloned())
    }

    fn save(&mut self, user: User) -> Result<(), UserRepositoryError> {
        self.users.insert(user.id().as_str().to_string(), user);
        Ok(())
    }

    fn update_status(&mut self, user: User) -> Result<(), UserRepositoryError> {
        self.users.insert(user.id().as_str().to_string(), user);
        Ok(())
    }

    fn list_users(&self) -> Result<Vec<User>, UserRepositoryError> {
        self.list_count.set(self.list_count.get() + 1);
        let mut users = self.users.values().cloned().collect::<Vec<_>>();
        users.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(users)
    }
}

#[derive(Default)]
struct FakeDocumentRepository {
    current: HashMap<(String, String), CurrentDocumentRecord>,
    get_count: Cell<usize>,
    put_count: Cell<usize>,
}

impl FakeDocumentRepository {
    fn insert(&mut self, workspace_id: &str, record: CurrentDocumentRecord) {
        self.current.insert(
            (
                workspace_id.to_string(),
                record.document_id().as_str().to_string(),
            ),
            record,
        );
    }

    fn current_body(&self, workspace_id: &str, document_id: &str) -> String {
        self.current
            .get(&(workspace_id.to_string(), document_id.to_string()))
            .expect("current record")
            .body()
            .as_str()
            .to_string()
    }
}

impl DocumentRepository for FakeDocumentRepository {
    fn put_current(
        &mut self,
        workspace_id: &WorkspaceId,
        record: CurrentDocumentRecord,
    ) -> Result<(), DocumentRepositoryError> {
        self.put_count.set(self.put_count.get() + 1);
        self.current.insert(
            (
                workspace_id.as_str().to_string(),
                record.document_id().as_str().to_string(),
            ),
            record,
        );
        Ok(())
    }

    fn get_current_by_id(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        self.get_count.set(self.get_count.get() + 1);
        Ok(self
            .current
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
            ))
            .cloned())
    }

    fn get_current_by_path(
        &self,
        _workspace_id: &WorkspaceId,
        _path: &DocumentPath,
    ) -> Result<Option<CurrentDocumentRecord>, DocumentRepositoryError> {
        Ok(None)
    }

    fn delete_current(
        &mut self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<(), DocumentRepositoryError> {
        Ok(())
    }
}

#[derive(Default)]
struct FakeDocumentChangePublisher {
    events: Vec<DocumentChangeEvent>,
    event_count: Cell<usize>,
}

impl DocumentChangeEventPublisher for FakeDocumentChangePublisher {
    fn publish(&mut self, event: DocumentChangeEvent) {
        self.event_count.set(self.event_count.get() + 1);
        self.events.push(event);
    }
}

#[derive(Default)]
struct FakeCommentRepository {
    threads: HashMap<String, CommentThread>,
    save_count: Cell<usize>,
    append_count: Cell<usize>,
    state_update_count: Cell<usize>,
    list_count: Cell<usize>,
}

impl FakeCommentRepository {
    fn insert(&mut self, thread: CommentThread) {
        self.threads.insert(
            comment_thread_key(thread.workspace_id(), thread.id()),
            thread,
        );
    }
}

impl CommentRepository for FakeCommentRepository {
    fn save_thread(
        &mut self,
        workspace_id: &WorkspaceId,
        thread: CommentThread,
    ) -> Result<(), CommentRepositoryError> {
        self.save_count.set(self.save_count.get() + 1);
        self.threads
            .insert(comment_thread_key(workspace_id, thread.id()), thread);
        Ok(())
    }

    fn get_thread(
        &self,
        workspace_id: &WorkspaceId,
        thread_id: &CommentThreadId,
    ) -> Result<Option<CommentThread>, CommentRepositoryError> {
        Ok(self
            .threads
            .get(&comment_thread_key(workspace_id, thread_id))
            .cloned())
    }

    fn append_comment(
        &mut self,
        workspace_id: &WorkspaceId,
        thread_id: &CommentThreadId,
        comment: Comment,
    ) -> Result<Option<CommentThread>, CommentRepositoryError> {
        self.append_count.set(self.append_count.get() + 1);
        let Some(thread) = self
            .threads
            .get(&comment_thread_key(workspace_id, thread_id))
            .cloned()
        else {
            return Ok(None);
        };
        let updated = thread
            .add_comment(comment)
            .map_err(|_| CommentRepositoryError::CorruptedState)?;
        self.threads
            .insert(comment_thread_key(workspace_id, thread_id), updated.clone());
        Ok(Some(updated))
    }

    fn update_thread_state(
        &mut self,
        workspace_id: &WorkspaceId,
        thread_id: &CommentThreadId,
        state: CommentThreadState,
    ) -> Result<Option<CommentThread>, CommentRepositoryError> {
        self.state_update_count
            .set(self.state_update_count.get() + 1);
        let Some(thread) = self
            .threads
            .get(&comment_thread_key(workspace_id, thread_id))
            .cloned()
        else {
            return Ok(None);
        };
        let updated = thread.with_state(state);
        self.threads
            .insert(comment_thread_key(workspace_id, thread_id), updated.clone());
        Ok(Some(updated))
    }

    fn list_document_threads(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Vec<CommentThread>, CommentRepositoryError> {
        self.list_count.set(self.list_count.get() + 1);
        let mut threads = self
            .threads
            .values()
            .filter(|thread| {
                thread.workspace_id() == workspace_id && thread.document_id() == document_id
            })
            .cloned()
            .collect::<Vec<_>>();
        threads.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(threads)
    }
}

#[derive(Default)]
struct FakeDocumentLockRepository {
    locks: HashMap<String, DocumentLock>,
    get_count: Cell<usize>,
    save_count: Cell<usize>,
    delete_count: Cell<usize>,
}

impl FakeDocumentLockRepository {
    fn insert_lock(
        &mut self,
        workspace_id: &str,
        lock_id: &str,
        document_id: &str,
        owner_user_id: &str,
        acquired_at_millis: u64,
        expires_at_millis: u64,
    ) {
        let lock = DocumentLock::new(
            DocumentLockId::new(lock_id).expect("lock id"),
            DocumentId::new(document_id).expect("document id"),
            UserId::new(owner_user_id).expect("owner user id"),
            DocumentLockTimestamp::from_millis(acquired_at_millis),
            DocumentLockTimestamp::from_millis(expires_at_millis),
        )
        .expect("document lock");
        self.locks
            .insert(document_lock_key(workspace_id, document_id), lock);
    }

    fn lock_exists(&self, workspace_id: &str, document_id: &str) -> bool {
        self.locks
            .contains_key(&document_lock_key(workspace_id, document_id))
    }
}

impl DocumentLockRepository for FakeDocumentLockRepository {
    fn get_document_lock(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentLock>, DocumentLockRepositoryError> {
        self.get_count.set(self.get_count.get() + 1);
        Ok(self
            .locks
            .get(&document_lock_key(
                workspace_id.as_str(),
                document_id.as_str(),
            ))
            .cloned())
    }

    fn save_document_lock(
        &mut self,
        workspace_id: &WorkspaceId,
        lock: DocumentLock,
    ) -> Result<(), DocumentLockRepositoryError> {
        self.save_count.set(self.save_count.get() + 1);
        self.locks.insert(
            document_lock_key(workspace_id.as_str(), lock.document_id().as_str()),
            lock,
        );
        Ok(())
    }

    fn delete_document_lock(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<DocumentLock>, DocumentLockRepositoryError> {
        self.delete_count.set(self.delete_count.get() + 1);
        Ok(self.locks.remove(&document_lock_key(
            workspace_id.as_str(),
            document_id.as_str(),
        )))
    }
}

struct FakeDocumentLockClock {
    now: DocumentLockTimestamp,
}

impl FakeDocumentLockClock {
    fn at(millis: u64) -> Self {
        Self {
            now: DocumentLockTimestamp::from_millis(millis),
        }
    }
}

impl DocumentLockClock for FakeDocumentLockClock {
    fn now(&self) -> DocumentLockTimestamp {
        self.now
    }
}

#[derive(Default)]
struct FakeReviewWorkflowRepository {
    states: HashMap<String, PublishWorkflowState>,
    requests: HashMap<String, ReviewRequestRecord>,
    save_state_count: Cell<usize>,
    save_request_count: Cell<usize>,
    update_request_count: Cell<usize>,
    list_count: Cell<usize>,
}

impl FakeReviewWorkflowRepository {
    fn insert_state(&mut self, workspace_id: &str, document_id: &str, state: PublishWorkflowState) {
        self.states.insert(
            review_workflow_document_key(workspace_id, document_id),
            state,
        );
    }

    fn insert_review_request(
        &mut self,
        workspace_id: &str,
        review_request_id: &str,
        document_id: &str,
        requested_by: &str,
    ) {
        let workspace_id = WorkspaceId::new(workspace_id).expect("workspace id");
        let document_id = DocumentId::new(document_id).expect("document id");
        let requested_by = UserId::new(requested_by).expect("requested by user id");
        let request = ReviewRequest::new(document_id, requested_by);
        let record = ReviewRequestRecord::new(
            &workspace_id,
            review_request_id,
            request,
            ReviewRequestStatus::ReviewRequested,
        )
        .expect("review request record");
        self.requests.insert(
            review_request_key(workspace_id.as_str(), review_request_id),
            record,
        );
    }

    fn request_status(
        &self,
        workspace_id: &str,
        review_request_id: &str,
    ) -> Option<ReviewRequestStatus> {
        self.requests
            .get(&review_request_key(workspace_id, review_request_id))
            .map(ReviewRequestRecord::status)
    }

    fn workflow_state(
        &self,
        workspace_id: &str,
        document_id: &str,
    ) -> Option<PublishWorkflowState> {
        self.states
            .get(&review_workflow_document_key(workspace_id, document_id))
            .copied()
    }
}

impl ReviewWorkflowRepository for FakeReviewWorkflowRepository {
    fn get_workflow_state(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Option<PublishWorkflowState>, ReviewWorkflowRepositoryError> {
        Ok(self
            .states
            .get(&review_workflow_document_key(
                workspace_id.as_str(),
                document_id.as_str(),
            ))
            .copied())
    }

    fn save_workflow_state(
        &mut self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        state: PublishWorkflowState,
    ) -> Result<(), ReviewWorkflowRepositoryError> {
        self.save_state_count.set(self.save_state_count.get() + 1);
        self.states.insert(
            review_workflow_document_key(workspace_id.as_str(), document_id.as_str()),
            state,
        );
        Ok(())
    }

    fn save_review_request(
        &mut self,
        workspace_id: &WorkspaceId,
        review_request_id: &str,
        request: ReviewRequest,
    ) -> Result<ReviewRequestRecord, ReviewWorkflowRepositoryError> {
        self.save_request_count
            .set(self.save_request_count.get() + 1);
        let record = ReviewRequestRecord::new(
            workspace_id,
            review_request_id,
            request,
            ReviewRequestStatus::ReviewRequested,
        )?;
        self.requests.insert(
            review_request_key(workspace_id.as_str(), review_request_id),
            record.clone(),
        );
        Ok(record)
    }

    fn get_review_request(
        &self,
        workspace_id: &WorkspaceId,
        review_request_id: &str,
    ) -> Result<Option<ReviewRequestRecord>, ReviewWorkflowRepositoryError> {
        Ok(self
            .requests
            .get(&review_request_key(
                workspace_id.as_str(),
                review_request_id,
            ))
            .cloned())
    }

    fn update_review_request_status(
        &mut self,
        workspace_id: &WorkspaceId,
        review_request_id: &str,
        status: ReviewRequestStatus,
    ) -> Result<Option<ReviewRequestRecord>, ReviewWorkflowRepositoryError> {
        self.update_request_count
            .set(self.update_request_count.get() + 1);
        let key = review_request_key(workspace_id.as_str(), review_request_id);
        let Some(record) = self.requests.get(&key) else {
            return Ok(None);
        };
        let updated = record.with_status(status);
        self.requests.insert(key, updated.clone());
        Ok(Some(updated))
    }

    fn list_review_requests(
        &self,
        workspace_id: &WorkspaceId,
        document_id: Option<&DocumentId>,
    ) -> Result<Vec<ReviewRequestRecord>, ReviewWorkflowRepositoryError> {
        self.list_count.set(self.list_count.get() + 1);
        Ok(self
            .requests
            .values()
            .filter(|record| {
                record.workspace_matches(workspace_id)
                    && document_id
                        .map(|document_id| record.request().document_id() == document_id)
                        .unwrap_or(true)
            })
            .cloned()
            .collect())
    }
}

#[derive(Default)]
struct FakeReviewWorkflowSideEffectRecorder {
    records: Vec<ReviewWorkflowSideEffectRecord>,
    fail: bool,
}

impl ReviewWorkflowSideEffectRecorder for FakeReviewWorkflowSideEffectRecorder {
    fn record_review_workflow_side_effect(
        &mut self,
        record: ReviewWorkflowSideEffectRecord,
    ) -> Result<(), ReviewWorkflowSideEffectError> {
        if self.fail {
            return Err(ReviewWorkflowSideEffectError::StorageUnavailable);
        }
        self.records.push(record);
        Ok(())
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

#[derive(Default)]
struct FakeVersionStore {
    history: HashMap<(String, String), Vec<VersionEntry>>,
    inline_anchor_states: HashMap<(String, String, String), InlineAnchorDocumentState>,
    appended: Vec<VersionRecord>,
    append_count: Cell<usize>,
    list_history_count: Cell<usize>,
    current_repository_read_count: Cell<usize>,
    inline_anchor_lookup_count: Cell<usize>,
}

impl FakeVersionStore {
    fn insert_history(
        &mut self,
        workspace_id: &str,
        document_id: &str,
        entries: Vec<VersionEntry>,
    ) {
        self.history
            .insert((workspace_id.to_string(), document_id.to_string()), entries);
    }

    fn insert_inline_anchor_state(
        &mut self,
        workspace_id: &str,
        document_id: &str,
        version_id: &str,
        current_version_id: &str,
        body_len: usize,
    ) {
        self.inline_anchor_states.insert(
            (
                workspace_id.to_string(),
                document_id.to_string(),
                version_id.to_string(),
            ),
            InlineAnchorDocumentState::new(
                VersionId::new(current_version_id).expect("current version id"),
                body_len,
            ),
        );
    }
}

impl VersionStore for FakeVersionStore {
    fn append_version(
        &mut self,
        _workspace_id: &WorkspaceId,
        record: VersionRecord,
    ) -> Result<(), VersionStoreError> {
        self.append_count.set(self.append_count.get() + 1);
        self.appended.push(record);
        Ok(())
    }

    fn get_version_snapshot(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        _version_id: &VersionId,
    ) -> Result<Option<VersionSnapshot>, VersionStoreError> {
        Ok(None)
    }

    fn list_history(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        request: HistoryPageRequest,
    ) -> Result<HistoryPage, VersionStoreError> {
        self.list_history_count
            .set(self.list_history_count.get() + 1);
        let entries = self
            .history
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
            ))
            .cloned()
            .unwrap_or_default();
        let start = request
            .cursor()
            .map(|cursor| cursor.as_str().parse::<usize>())
            .transpose()
            .map_err(|_| VersionStoreError::CorruptedHistory)?
            .unwrap_or(0);
        let end = usize::min(start + request.limit(), entries.len());
        let next_cursor = if end < entries.len() {
            Some(HistoryCursor::new(&end.to_string()).expect("cursor"))
        } else {
            None
        };
        Ok(HistoryPage::new(entries[start..end].to_vec(), next_cursor))
    }
}

impl InlineAnchorDocumentLookup for FakeVersionStore {
    fn get_anchor_document_state(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<InlineAnchorDocumentState>, InlineAnchorDocumentLookupError> {
        self.inline_anchor_lookup_count
            .set(self.inline_anchor_lookup_count.get() + 1);
        Ok(self
            .inline_anchor_states
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
                version_id.as_str().to_string(),
            ))
            .cloned())
    }
}

struct FakeSearchIndex {
    page: SearchAccessiblePage,
    search_count: Cell<usize>,
    last_filter: RefCell<Option<PermissionFilter>>,
    last_query_text: RefCell<Option<String>>,
}

impl Default for FakeSearchIndex {
    fn default() -> Self {
        Self {
            page: SearchAccessiblePage::new(
                vec![search_result(
                    "doc-1",
                    "Runtime Title",
                    "runtime/doc.md",
                    "runtime",
                )],
                PermissionQueryStats::new(3, 2, false),
            ),
            search_count: Cell::new(0),
            last_filter: RefCell::new(None),
            last_query_text: RefCell::new(None),
        }
    }
}

impl PermissionAwareSearchIndex for FakeSearchIndex {
    fn search_accessible(
        &mut self,
        _workspace_id: &WorkspaceId,
        filter: PermissionFilter,
        query: SearchQuery,
    ) -> Result<SearchAccessiblePage, PermissionAwareQueryError> {
        self.search_count.set(self.search_count.get() + 1);
        *self.last_filter.borrow_mut() = Some(filter);
        *self.last_query_text.borrow_mut() = Some(query.text().to_string());
        Ok(self.page.clone())
    }
}

struct FakePermissionChecker {
    decision: PermissionDecision,
    checked_permission: RefCell<Option<Permission>>,
    denied_document_ids: Vec<String>,
}

impl FakePermissionChecker {
    fn allowed() -> Self {
        Self {
            decision: PermissionDecision::allowed(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleAllowsPermission,
            ),
            checked_permission: RefCell::new(None),
            denied_document_ids: Vec::new(),
        }
    }

    fn allowed_except_documents(document_ids: Vec<&str>) -> Self {
        Self {
            denied_document_ids: document_ids.into_iter().map(str::to_string).collect(),
            ..Self::allowed()
        }
    }

    fn denied() -> Self {
        Self {
            decision: PermissionDecision::denied(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleDoesNotAllowPermission,
            ),
            checked_permission: RefCell::new(None),
            denied_document_ids: Vec::new(),
        }
    }
}

impl PermissionDecisionPort for FakePermissionChecker {
    fn check_permission(
        &self,
        _actor_user_id: &UserId,
        resource: &AccessResource,
        permission: Permission,
    ) -> Result<PermissionDecision, PermissionAwareQueryError> {
        *self.checked_permission.borrow_mut() = Some(permission);
        if resource.document_id().is_some_and(|document_id| {
            self.denied_document_ids
                .contains(&document_id.as_str().to_string())
        }) {
            return Ok(PermissionDecision::denied(
                PolicySource::Document,
                PermissionDecisionReason::HiddenByPolicy,
            ));
        }
        Ok(self.decision)
    }
}

impl FieldDebugPermissionChecker for FakePermissionChecker {
    fn check_workspace_permission(
        &self,
        _actor_user_id: &UserId,
        _workspace_id: &WorkspaceId,
        permission: Permission,
    ) -> Result<PermissionDecision, FieldDebugPermissionCheckError> {
        *self.checked_permission.borrow_mut() = Some(permission);
        Ok(self.decision)
    }
}

impl CommentPermissionChecker for FakePermissionChecker {
    fn check_permission(
        &self,
        _actor_user_id: &UserId,
        _resource: &AccessResource,
        permission: Permission,
    ) -> Result<PermissionDecision, CommentPermissionCheckError> {
        *self.checked_permission.borrow_mut() = Some(permission);
        Ok(self.decision)
    }
}

impl DocumentLockPermissionChecker for FakePermissionChecker {
    fn check_document_permission(
        &self,
        _actor_user_id: &UserId,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        permission: Permission,
    ) -> Result<PermissionDecision, DocumentLockPermissionCheckError> {
        *self.checked_permission.borrow_mut() = Some(permission);
        Ok(self.decision)
    }
}

impl AuditPermissionChecker for FakePermissionChecker {
    fn check_workspace_permission(
        &self,
        _actor_user_id: &UserId,
        _workspace_id: &WorkspaceId,
        permission: Permission,
    ) -> Result<PermissionDecision, AuditPermissionCheckError> {
        *self.checked_permission.borrow_mut() = Some(permission);
        Ok(self.decision)
    }
}

impl ReviewWorkflowPermissionChecker for FakePermissionChecker {
    fn check_document_permission(
        &self,
        _actor_user_id: &UserId,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
        permission: Permission,
    ) -> Result<PermissionDecision, ReviewWorkflowPermissionCheckError> {
        *self.checked_permission.borrow_mut() = Some(permission);
        Ok(self.decision)
    }
}

#[derive(Default)]
struct FakeFieldDebugSessionRepository {
    sessions: Vec<FieldDebugSession>,
    save_count: Cell<usize>,
}

impl FieldDebugSessionRepository for FakeFieldDebugSessionRepository {
    fn save_field_debug_session(
        &mut self,
        session: FieldDebugSession,
    ) -> Result<(), FieldDebugSessionRepositoryError> {
        self.save_count.set(self.save_count.get() + 1);
        self.sessions
            .retain(|current| current.session_id() != session.session_id());
        self.sessions.push(session);
        Ok(())
    }

    fn get_field_debug_session(
        &self,
        workspace_id: &WorkspaceId,
        session_id: &FieldDebugSessionId,
    ) -> Result<Option<FieldDebugSession>, FieldDebugSessionRepositoryError> {
        Ok(self
            .sessions
            .iter()
            .find(|session| {
                session.workspace_id() == workspace_id && session.session_id() == session_id
            })
            .cloned())
    }
}

#[derive(Clone, Copy)]
struct FakeFieldDebugClock {
    now: FieldDebugTimestamp,
}

impl FakeFieldDebugClock {
    fn new(now_millis: u64) -> Self {
        Self {
            now: FieldDebugTimestamp::from_millis(now_millis),
        }
    }
}

impl FieldDebugClock for FakeFieldDebugClock {
    fn now(&self) -> FieldDebugTimestamp {
        self.now
    }
}

fn current_record(document_id: &str, title: &str, path: &str, body: &str) -> CurrentDocumentRecord {
    let document_id = DocumentId::new(document_id).expect("document id");
    CurrentDocumentRecord::new(
        DocumentMetadata::new(
            document_id.clone(),
            DocumentTitle::new(title).expect("title"),
            DocumentPath::new(path).expect("path"),
        )
        .expect("metadata"),
        CurrentDocumentSnapshot::new(
            document_id,
            DocumentBody::new(body, DocumentBodyPolicy::new(4096).expect("policy")).expect("body"),
        ),
    )
    .expect("current record")
}

fn graph_projection_fixture(center_document_id: &str) -> KnowledgeGraph {
    let center_id = DocumentId::new(center_document_id).expect("center document id");
    let center = GraphNode::new_document(center_id.clone());
    let visible = GraphNode::new_document(DocumentId::new("visible-doc").expect("visible doc"));
    let hidden = GraphNode::new_document(DocumentId::new("hidden-doc").expect("hidden doc"));
    let edges = vec![
        GraphEdge::new(
            "edge-visible",
            center.id().to_string(),
            visible.id().to_string(),
            GraphEdgeKind::DocumentLink,
        )
        .expect("visible edge"),
        GraphEdge::new(
            "edge-hidden",
            center.id().to_string(),
            hidden.id().to_string(),
            GraphEdgeKind::DocumentLink,
        )
        .expect("hidden edge"),
    ];
    KnowledgeGraph::new_with_center(
        center_id,
        vec![center, visible, hidden],
        edges,
        GraphProjectionStatus::Clean,
    )
    .expect("graph")
}

fn version_entry(document_id: &str, version_id: &str) -> VersionEntry {
    VersionEntry::new(
        VersionId::new(version_id).expect("version id"),
        DocumentId::new(document_id).expect("document id"),
        DocumentSnapshotRef::new(&format!("snapshot-{version_id}")).expect("snapshot ref"),
        VersionAuthor::new("writer").expect("author"),
        VersionSummary::new("Saved").expect("summary"),
    )
    .expect("entry")
}

fn search_result(document_id: &str, title: &str, path: &str, snippet: &str) -> SearchResult {
    SearchResult::new(
        DocumentId::new(document_id).expect("document id"),
        DocumentTitle::new(title).expect("title"),
        DocumentPath::new(path).expect("path"),
        snippet,
        10,
    )
    .expect("search result")
}

fn active_user(user_id: &str, login: &str) -> User {
    User::new(
        UserId::new(user_id).expect("user id"),
        UserProfile::new(
            UserLogin::new(login).expect("login"),
            UserEmail::new(&format!("{login}@example.com")).expect("email"),
            login,
            None,
        )
        .expect("profile"),
        UserTimestamp::new("2026-06-29T00:00:00Z").expect("timestamp"),
    )
}

fn role_assignment(
    assignment_id: &str,
    workspace_id: &str,
    subject: RoleAssignmentSubject,
    role: Role,
) -> RoleAssignment {
    RoleAssignment::new(
        RoleAssignmentId::new(assignment_id).expect("assignment id"),
        WorkspaceId::new(workspace_id).expect("workspace id"),
        subject,
        role,
    )
}

fn role_assignment_key(workspace_id: &WorkspaceId, assignment_id: &RoleAssignmentId) -> String {
    format!("{}:{}", workspace_id.as_str(), assignment_id.as_str())
}

fn comment_thread_key(workspace_id: &WorkspaceId, thread_id: &CommentThreadId) -> String {
    format!("{}:{}", workspace_id.as_str(), thread_id.as_str())
}

fn document_policy_key(workspace_id: &WorkspaceId, document_id: &DocumentId) -> String {
    format!("{}:{}", workspace_id.as_str(), document_id.as_str())
}

fn review_workflow_document_key(workspace_id: &str, document_id: &str) -> String {
    format!("{workspace_id}:{document_id}")
}

fn review_request_key(workspace_id: &str, review_request_id: &str) -> String {
    format!("{workspace_id}:{review_request_id}")
}

fn document_lock_key(workspace_id: &str, document_id: &str) -> String {
    format!("{workspace_id}:{document_id}")
}

fn backup_job_key(workspace_id: &str, job_id: &str) -> String {
    format!("{workspace_id}:{job_id}")
}

fn backup_snapshot(
    workspace_id: &str,
    job_id: &str,
    operation: BackupJobOperation,
    state: BackupJobState,
    retry_count: u16,
    completed_units: u64,
    total_units: u64,
) -> BackupJobSnapshot {
    BackupJobSnapshot::new(
        BackupJobId::new(job_id).expect("backup job id"),
        WorkspaceId::new(workspace_id).expect("workspace id"),
        operation,
        state,
        retry_count,
        BackupProgress::new(completed_units, total_units).expect("backup progress"),
        None,
    )
    .expect("backup snapshot")
}

fn group_key(workspace_id: &WorkspaceId, group_id: &GroupId) -> String {
    format!("{}:{}", workspace_id.as_str(), group_id.as_str())
}

fn comment(id: &str, author: &str, body: &str) -> Comment {
    Comment::new(
        CommentId::new(id).expect("comment id"),
        UserId::new(author).expect("author id"),
        CommentBody::new(body, CommentBodyPolicy::new(1024).expect("comment policy"))
            .expect("comment body"),
    )
}

fn default_config() -> cabinet_core::server_config::ServerConfig {
    ServerConfigInput::local_dev_defaults()
        .validate()
        .expect("valid default server config")
}
