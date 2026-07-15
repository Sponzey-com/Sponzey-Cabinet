use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::BTreeMap;

use cabinet_core::server_config::ServerConfig;
use cabinet_domain::document::DocumentId;
use cabinet_domain::graph::{GraphEdgeKind, GraphNodeKind, GraphProjectionStatus, KnowledgeGraph};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::audit_log::{AuditLogStore, AuditPermissionChecker};
use cabinet_ports::auth::{
    CredentialVerifier, SessionClock, SessionIdGenerator, SessionStore, TokenIssuer,
};
use cabinet_ports::backup_store::{BackupAuditRecorder, BackupStore};
use cabinet_ports::comment_repository::{
    CommentPermissionChecker, CommentRepository, InlineAnchorDocumentLookup,
};
use cabinet_ports::document_lock::{
    DocumentLockClock, DocumentLockPermissionChecker, DocumentLockRepository,
};
use cabinet_ports::document_repository::DocumentRepository;
use cabinet_ports::field_debug::{
    FieldDebugClock, FieldDebugPermissionChecker, FieldDebugSessionRepository,
};
use cabinet_ports::graph_projection::{
    GraphProjectionError, GraphProjectionRecord, GraphProjectionStore,
};
use cabinet_ports::group_repository::GroupRepository;
use cabinet_ports::permission_aware_query::{
    AccessibleDocumentQuery, PermissionAwareSearchIndex, PermissionDecisionPort,
};
use cabinet_ports::permission_policy_repository::{
    PermissionGroupRepository, PermissionPolicyRepository, RoleAssignmentIdGenerator,
};
use cabinet_ports::review_workflow::{
    ReviewWorkflowPermissionChecker, ReviewWorkflowRepository, ReviewWorkflowSideEffectRecorder,
};
use cabinet_ports::user_repository::UserRepository;
use cabinet_ports::version_store::VersionStore;
use cabinet_usecases::audit::{
    AuditEventSummary, AuditRetentionPolicy, AuditUsecaseError, AuditUsecaseLogger,
    ListAuditEventsInput, ListAuditEventsOutput, ListAuditEventsScopeInput, ListAuditEventsUsecase,
};
use cabinet_usecases::auth::{
    AuthError, AuthProductLogger, AuthSessionPolicy, AuthenticateUserInput, AuthenticateUserOutput,
    AuthenticateUserUsecase, ValidateSessionInput, ValidateSessionOutput, ValidateSessionUsecase,
};
use cabinet_usecases::backup::{
    BackupJobOutput, BackupJobUsecaseError, BackupJobUsecaseLogger, CreateBackupInput,
    CreateBackupUsecase, ExportWorkspaceInput, ExportWorkspaceUsecase, GetBackupStatusInput,
    GetBackupStatusUsecase, GetExportStatusInput, GetExportStatusUsecase, RestoreBackupInput,
    RestoreBackupUsecase,
};
use cabinet_usecases::comment::{
    AddCommentInput, AddCommentUsecase, AddInlineCommentInput, AddInlineCommentUsecase,
    CommentThreadOutput, CommentUsecaseError, CommentUsecaseLogger, ListDocumentCommentsInput,
    ListDocumentCommentsOutput, ListDocumentCommentsUsecase, ReopenCommentInput,
    ReopenCommentUsecase, ResolveCommentInput, ResolveCommentUsecase,
};
use cabinet_usecases::document::{
    DocumentChangeEventPublisher, DocumentProductLogger, GetDocumentHistoryError,
    GetDocumentHistoryInput, GetDocumentHistoryUsecase, UpdateDocumentError, UpdateDocumentInput,
    UpdateDocumentOutput, UpdateDocumentUsecase,
};
use cabinet_usecases::document_lock::{
    DocumentLockOutput, DocumentLockUsecaseError, DocumentLockUsecaseLogger, GetDocumentLockInput,
    GetDocumentLockUsecase, LockDocumentInput, LockDocumentPolicy, LockDocumentUsecase,
    UnlockDocumentInput, UnlockDocumentUsecase,
};
use cabinet_usecases::field_debug::{
    ApproveFieldDebugSessionInput, ApproveFieldDebugSessionUsecase, ExpireFieldDebugSessionInput,
    ExpireFieldDebugSessionUsecase, FieldDebugSessionOutput, FieldDebugSessionOutputStatus,
    FieldDebugSessionPolicy, FieldDebugUsecaseError, FieldDebugUsecaseLogger,
    RequestFieldDebugSessionInput, RequestFieldDebugSessionUsecase,
};
use cabinet_usecases::graph::{
    PermissionAwareGraphError, PermissionAwareGraphInput, PermissionAwareGraphStats,
    PermissionAwareGraphUsecase,
};
use cabinet_usecases::group::{
    AddUserToGroupError, AddUserToGroupInput, AddUserToGroupOutput, AddUserToGroupUsecase,
    CreateGroupProductLogger, GroupMembershipResult, ListWorkspaceGroupsError,
    ListWorkspaceGroupsInput, ListWorkspaceGroupsOutput, ListWorkspaceGroupsUsecase,
    RemoveUserFromGroupError, RemoveUserFromGroupInput, RemoveUserFromGroupOutput,
    RemoveUserFromGroupUsecase, WorkspaceGroupDto,
};
use cabinet_usecases::permission::{
    AssignRoleError, AssignRoleInput, AssignRoleOutput, AssignRoleUsecase,
    ListEffectivePermissionsError, ListEffectivePermissionsInput, ListEffectivePermissionsOutput,
    ListEffectivePermissionsUsecase, ListWorkspaceRoleAssignmentsError,
    ListWorkspaceRoleAssignmentsInput, ListWorkspaceRoleAssignmentsOutput,
    ListWorkspaceRoleAssignmentsUsecase, PermissionResourceInput, PermissionUsecaseLogger,
    RevokeRoleError, RevokeRoleInput, RevokeRoleOutput, RevokeRoleUsecase, RoleAssignmentDto,
    ShareDocumentError, ShareDocumentInput, ShareDocumentOutput, ShareDocumentUsecase,
};
use cabinet_usecases::permission_query::{
    AccessibleQueryLogger, GetAccessibleDocumentError, GetAccessibleDocumentInput,
    GetAccessibleDocumentUsecase, SearchAccessibleDocumentsError, SearchAccessibleDocumentsInput,
    SearchAccessibleDocumentsUsecase,
};
use cabinet_usecases::review_workflow::{
    ApproveDocumentInput, ApproveDocumentUsecase, ListReviewRequestsInput,
    ListReviewRequestsOutput, ListReviewRequestsUsecase, PublishDocumentInput,
    PublishDocumentUsecase, RejectDocumentInput, RejectDocumentUsecase, RequestDocumentReviewInput,
    RequestDocumentReviewUsecase, ReviewWorkflowOutput, ReviewWorkflowPolicy,
    ReviewWorkflowUsecaseError, ReviewWorkflowUsecaseLogger,
};
use cabinet_usecases::user::{
    ListUserSummary, ListUsersError, ListUsersInput, ListUsersOutput, ListUsersUsecase,
};

use crate::adapter::{RouteRegistry, ServerUsecaseTarget, UsecaseInputDto, UsecaseOutputDto};
use crate::errors::ServerBoundaryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerKind {
    Health,
    CurrentDocument,
    DocumentHistory,
    PermissionAwareSearch,
    GraphLocal,
    CanvasCreate,
    CanvasAddNode,
    CanvasEmbed,
    FieldDebugRequest,
    FieldDebugApprove,
    FieldDebugExpire,
    AuthLogin,
    AuthValidateSession,
    UserList,
    GroupList,
    GroupAddMember,
    GroupRemoveMember,
    RoleListAssignments,
    RoleAssign,
    RoleRevoke,
    CollaborationJoinDocumentRoom,
    CollaborationBroadcastOperation,
    CollaborationBroadcastPresence,
    CollaborationRequestReplay,
    RemoteCurrentDocumentSave,
    SharingGetDocument,
    SharingUpdateDocument,
    CommentList,
    CommentAdd,
    CommentAddInline,
    CommentResolve,
    CommentReopen,
    ReviewRequestDocument,
    ReviewApproveDocument,
    ReviewRejectDocument,
    ReviewPublishDocument,
    ReviewListRequests,
    DocumentLockLock,
    DocumentLockUnlock,
    DocumentLockGet,
    AuditListEvents,
    BackupCreate,
    BackupGetStatus,
    BackupRestore,
    ExportCreateWorkspace,
    ExportGetStatus,
    NotImplemented,
}

impl HandlerKind {
    fn for_route_id(route_id: &str) -> Self {
        match route_id {
            "health.check" => Self::Health,
            "document.get_accessible_current" => Self::CurrentDocument,
            "document.get_accessible_history" => Self::DocumentHistory,
            "search.accessible" => Self::PermissionAwareSearch,
            "graph.get_local" => Self::GraphLocal,
            "canvas.create" => Self::CanvasCreate,
            "canvas.add_node" => Self::CanvasAddNode,
            "canvas.embed" => Self::CanvasEmbed,
            "field_debug.request_session" => Self::FieldDebugRequest,
            "field_debug.approve_session" => Self::FieldDebugApprove,
            "field_debug.expire_session" => Self::FieldDebugExpire,
            "auth.login" => Self::AuthLogin,
            "auth.validate_session" => Self::AuthValidateSession,
            "user.list" => Self::UserList,
            "group.list" => Self::GroupList,
            "group.add_member" => Self::GroupAddMember,
            "group.remove_member" => Self::GroupRemoveMember,
            "role.list_assignments" => Self::RoleListAssignments,
            "role.assign" => Self::RoleAssign,
            "role.revoke" => Self::RoleRevoke,
            "collaboration.join_document_room" => Self::CollaborationJoinDocumentRoom,
            "collaboration.broadcast_operation" => Self::CollaborationBroadcastOperation,
            "collaboration.broadcast_presence" => Self::CollaborationBroadcastPresence,
            "collaboration.request_replay" => Self::CollaborationRequestReplay,
            "document.save_remote_current" => Self::RemoteCurrentDocumentSave,
            "sharing.get_document" => Self::SharingGetDocument,
            "sharing.update_document" => Self::SharingUpdateDocument,
            "comment.list" => Self::CommentList,
            "comment.add" => Self::CommentAdd,
            "comment.add_inline" => Self::CommentAddInline,
            "comment.resolve" => Self::CommentResolve,
            "comment.reopen" => Self::CommentReopen,
            "review.request_document" => Self::ReviewRequestDocument,
            "review.approve_document" => Self::ReviewApproveDocument,
            "review.reject_document" => Self::ReviewRejectDocument,
            "review.publish_document" => Self::ReviewPublishDocument,
            "review.list_requests" => Self::ReviewListRequests,
            "document_lock.lock" => Self::DocumentLockLock,
            "document_lock.unlock" => Self::DocumentLockUnlock,
            "document_lock.get" => Self::DocumentLockGet,
            "audit.list_events" => Self::AuditListEvents,
            "backup.create" => Self::BackupCreate,
            "backup.get_status" => Self::BackupGetStatus,
            "backup.restore" => Self::BackupRestore,
            "export.create_workspace" => Self::ExportCreateWorkspace,
            "export.get_status" => Self::ExportGetStatus,
            _ => Self::NotImplemented,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HandlerRegistry {
    handlers: BTreeMap<String, HandlerKind>,
}

impl HandlerRegistry {
    pub fn from_routes(routes: &RouteRegistry) -> Self {
        let mut handlers = BTreeMap::new();
        for route in routes.routes() {
            handlers.insert(
                route.route_id().to_string(),
                HandlerKind::for_route_id(route.route_id()),
            );
        }
        Self { handlers }
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn contains(&self, route_id: &str) -> bool {
        self.handlers.contains_key(route_id)
    }

    pub fn kind(&self, route_id: &str) -> Option<HandlerKind> {
        self.handlers.get(route_id).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeDependencyDurability {
    DurableLocal,
    VolatileLocal,
    External,
    Policy,
    RuntimeUtility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeDependency {
    name: &'static str,
    implementation: &'static str,
    durability: RuntimeDependencyDurability,
}

impl RuntimeDependency {
    pub const fn new(
        name: &'static str,
        implementation: &'static str,
        durability: RuntimeDependencyDurability,
    ) -> Self {
        Self {
            name,
            implementation,
            durability,
        }
    }

    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn implementation(self) -> &'static str {
        self.implementation
    }

    pub const fn durability(self) -> RuntimeDependencyDurability {
        self.durability
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDependencyManifest {
    dependencies: Vec<RuntimeDependency>,
}

impl RuntimeDependencyManifest {
    pub fn phase002() -> Self {
        Self::phase003_self_host()
    }

    pub fn phase003_self_host() -> Self {
        use RuntimeDependencyDurability::{
            DurableLocal, External, Policy, RuntimeUtility, VolatileLocal,
        };

        Self {
            dependencies: vec![
                dependency(
                    "document_repository",
                    "LocalDocumentRepository",
                    DurableLocal,
                ),
                dependency("version_store", "LocalVersionStore", DurableLocal),
                dependency(
                    "document_asset_metadata_store",
                    "LocalDocumentAssetRepository",
                    DurableLocal,
                ),
                dependency(
                    "permission_query",
                    "PermissionAwareQueryPorts",
                    RuntimeUtility,
                ),
                dependency(
                    "graph_projection_store",
                    "RuntimeGraphProjectionStore",
                    DurableLocal,
                ),
                dependency("object_storage", "LocalObjectStorage", DurableLocal),
                dependency("search_index", "LocalSearchIndex", DurableLocal),
                dependency("link_index", "LocalLinkIndex", DurableLocal),
                dependency("audit_store", "LocalAuditLogStore", DurableLocal),
                dependency("backup_store", "LocalBackupStore", DurableLocal),
                dependency("backup_audit_recorder", "BackupAuditRecorderPort", External),
                dependency(
                    "field_debug_repository",
                    "FieldDebugSessionRepositoryPort",
                    VolatileLocal,
                ),
                dependency("credential_verifier", "CredentialVerifierPort", External),
                dependency("token_issuer", "LocalTokenIssuer", RuntimeUtility),
                dependency("session_store", "LocalSessionStore", DurableLocal),
                dependency("group_repository", "LocalGroupRepository", DurableLocal),
                dependency(
                    "permission_policy_repository",
                    "LocalPermissionPolicyRepository",
                    DurableLocal,
                ),
                dependency("user_repository", "LocalUserRepository", DurableLocal),
                dependency(
                    "document_lock_repository",
                    "LocalDocumentLockRepository",
                    DurableLocal,
                ),
                dependency(
                    "document_lock_clock",
                    "SystemDocumentLockClock",
                    RuntimeUtility,
                ),
                dependency("logger", "ProductLogSink", RuntimeUtility),
                dependency(
                    "review_workflow_repository",
                    "LocalReviewWorkflowRepository",
                    DurableLocal,
                ),
                dependency(
                    "review_workflow_side_effect_recorder",
                    "ReviewWorkflowSideEffectRecorderPort",
                    External,
                ),
                dependency("comment_repository", "LocalCommentRepository", DurableLocal),
                dependency("clock", "SystemClock", RuntimeUtility),
                dependency("id_generator", "IdGeneratorPort", RuntimeUtility),
                dependency("auth_policy", "AuthSessionPolicy", Policy),
                dependency("config_policy", "RuntimePolicy", Policy),
                dependency(
                    "realtime_room_owner_policy",
                    "LocalDocumentRoomOwnerPolicy",
                    Policy,
                ),
                dependency(
                    "realtime_transport",
                    "LocalRealtimeTransport",
                    VolatileLocal,
                ),
            ],
        }
    }

    pub fn contains(&self, dependency: &str) -> bool {
        self.dependency(dependency).is_some()
    }

    pub fn dependency(&self, dependency: &str) -> Option<RuntimeDependency> {
        self.dependencies
            .iter()
            .find(|entry| entry.name() == dependency)
            .copied()
    }

    pub fn dependencies(&self) -> &[RuntimeDependency] {
        &self.dependencies
    }

    pub fn missing_durable_local_dependencies(
        &self,
        required: &[&'static str],
    ) -> Vec<&'static str> {
        required
            .iter()
            .copied()
            .filter(|dependency| {
                self.dependency(dependency).is_none_or(|entry| {
                    entry.durability() != RuntimeDependencyDurability::DurableLocal
                })
            })
            .collect()
    }
}

const fn dependency(
    name: &'static str,
    implementation: &'static str,
    durability: RuntimeDependencyDurability,
) -> RuntimeDependency {
    RuntimeDependency::new(name, implementation, durability)
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeGraphProjectionStore {
    records: BTreeMap<(String, String), GraphProjectionRecord>,
    lookup_count: Cell<usize>,
}

impl RuntimeGraphProjectionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lookup_count(&self) -> usize {
        self.lookup_count.get()
    }
}

impl GraphProjectionStore for RuntimeGraphProjectionStore {
    fn replace_projection(
        &mut self,
        workspace_id: &WorkspaceId,
        record: GraphProjectionRecord,
    ) -> Result<(), GraphProjectionError> {
        self.records.insert(
            (
                workspace_id.as_str().to_string(),
                record.graph().center_document_id().as_str().to_string(),
            ),
            record,
        );
        Ok(())
    }

    fn get_projection(
        &self,
        workspace_id: &WorkspaceId,
        center_document_id: &DocumentId,
    ) -> Result<Option<GraphProjectionRecord>, GraphProjectionError> {
        self.lookup_count.set(self.lookup_count.get() + 1);
        Ok(self
            .records
            .get(&(
                workspace_id.as_str().to_string(),
                center_document_id.as_str().to_string(),
            ))
            .cloned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeActorContext {
    user_id: String,
}

impl RuntimeActorContext {
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
        }
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimePolicy {
    field_debug_policy: FieldDebugSessionPolicy,
    auth_policy: AuthSessionPolicy,
    document_body_max_bytes: usize,
    comment_body_max_bytes: usize,
}

impl RuntimePolicy {
    const DEFAULT_DOCUMENT_BODY_MAX_BYTES: usize = 1024 * 1024;
    const DEFAULT_COMMENT_BODY_MAX_BYTES: usize = 64 * 1024;

    pub fn from_config(config: &ServerConfig) -> Self {
        Self {
            field_debug_policy: FieldDebugSessionPolicy::new(config.field_debug_max_ttl_seconds())
                .expect("validated server config has non-zero field debug ttl"),
            auth_policy: AuthSessionPolicy::new(config.auth().session_ttl_seconds())
                .expect("validated server config has non-zero auth session ttl"),
            document_body_max_bytes: Self::DEFAULT_DOCUMENT_BODY_MAX_BYTES,
            comment_body_max_bytes: Self::DEFAULT_COMMENT_BODY_MAX_BYTES,
        }
    }
}

pub struct ServerRuntimeTarget<
    PC,
    DR,
    DQ,
    VS,
    SI,
    AS,
    BS,
    BAR,
    CV,
    TI,
    SS,
    SC,
    SIG,
    GR,
    PR,
    RIG,
    UR,
    FDR,
    FDPC,
    FDC,
    DCP,
    DLR,
    DLC,
    CR,
    RWR,
    RWSR,
    L,
> {
    actor_context: RuntimeActorContext,
    permission_checker: PC,
    document_repository: RefCell<DR>,
    document_query: DQ,
    version_store: RefCell<VS>,
    search_index: RefCell<SI>,
    graph_projection_store: RefCell<RuntimeGraphProjectionStore>,
    audit_store: RefCell<AS>,
    backup_store: RefCell<BS>,
    backup_audit_recorder: RefCell<BAR>,
    credential_verifier: RefCell<CV>,
    token_issuer: RefCell<TI>,
    session_store: RefCell<SS>,
    session_clock: SC,
    session_id_generator: RefCell<SIG>,
    group_repository: RefCell<GR>,
    permission_repository: RefCell<PR>,
    role_assignment_id_generator: RefCell<RIG>,
    user_repository: RefCell<UR>,
    field_debug_repository: RefCell<FDR>,
    field_debug_permission_checker: FDPC,
    field_debug_clock: FDC,
    document_change_publisher: RefCell<DCP>,
    document_lock_repository: RefCell<DLR>,
    document_lock_clock: DLC,
    comment_repository: RefCell<CR>,
    review_repository: RefCell<RWR>,
    review_side_effect_recorder: RefCell<RWSR>,
    logger: RefCell<L>,
    policy: RuntimePolicy,
}

impl<
    PC,
    DR,
    DQ,
    VS,
    SI,
    AS,
    BS,
    BAR,
    CV,
    TI,
    SS,
    SC,
    SIG,
    GR,
    PR,
    RIG,
    UR,
    FDR,
    FDPC,
    FDC,
    DCP,
    DLR,
    DLC,
    CR,
    RWR,
    RWSR,
    L,
>
    ServerRuntimeTarget<
        PC,
        DR,
        DQ,
        VS,
        SI,
        AS,
        BS,
        BAR,
        CV,
        TI,
        SS,
        SC,
        SIG,
        GR,
        PR,
        RIG,
        UR,
        FDR,
        FDPC,
        FDC,
        DCP,
        DLR,
        DLC,
        CR,
        RWR,
        RWSR,
        L,
    >
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor_context: RuntimeActorContext,
        permission_checker: PC,
        document_repository: DR,
        document_query: DQ,
        version_store: VS,
        search_index: SI,
        audit_store: AS,
        backup_store: BS,
        backup_audit_recorder: BAR,
        credential_verifier: CV,
        token_issuer: TI,
        session_store: SS,
        session_clock: SC,
        session_id_generator: SIG,
        group_repository: GR,
        permission_repository: PR,
        role_assignment_id_generator: RIG,
        user_repository: UR,
        field_debug_repository: FDR,
        field_debug_permission_checker: FDPC,
        field_debug_clock: FDC,
        document_change_publisher: DCP,
        document_lock_repository: DLR,
        document_lock_clock: DLC,
        comment_repository: CR,
        review_repository: RWR,
        review_side_effect_recorder: RWSR,
        logger: L,
        policy: RuntimePolicy,
    ) -> Self {
        Self {
            actor_context,
            permission_checker,
            document_repository: RefCell::new(document_repository),
            document_query,
            version_store: RefCell::new(version_store),
            search_index: RefCell::new(search_index),
            graph_projection_store: RefCell::new(RuntimeGraphProjectionStore::default()),
            audit_store: RefCell::new(audit_store),
            backup_store: RefCell::new(backup_store),
            backup_audit_recorder: RefCell::new(backup_audit_recorder),
            credential_verifier: RefCell::new(credential_verifier),
            token_issuer: RefCell::new(token_issuer),
            session_store: RefCell::new(session_store),
            session_clock,
            session_id_generator: RefCell::new(session_id_generator),
            group_repository: RefCell::new(group_repository),
            permission_repository: RefCell::new(permission_repository),
            role_assignment_id_generator: RefCell::new(role_assignment_id_generator),
            user_repository: RefCell::new(user_repository),
            field_debug_repository: RefCell::new(field_debug_repository),
            field_debug_permission_checker,
            field_debug_clock,
            document_change_publisher: RefCell::new(document_change_publisher),
            document_lock_repository: RefCell::new(document_lock_repository),
            document_lock_clock,
            comment_repository: RefCell::new(comment_repository),
            review_repository: RefCell::new(review_repository),
            review_side_effect_recorder: RefCell::new(review_side_effect_recorder),
            logger: RefCell::new(logger),
            policy,
        }
    }

    pub fn document_repository(&self) -> Ref<'_, DR> {
        self.document_repository.borrow()
    }

    pub const fn document_query(&self) -> &DQ {
        &self.document_query
    }

    pub fn version_store(&self) -> Ref<'_, VS> {
        self.version_store.borrow()
    }

    pub fn search_index(&self) -> Ref<'_, SI> {
        self.search_index.borrow()
    }

    pub fn graph_projection_store(&self) -> Ref<'_, RuntimeGraphProjectionStore> {
        self.graph_projection_store.borrow()
    }

    pub fn graph_projection_store_mut(&self) -> RefMut<'_, RuntimeGraphProjectionStore> {
        self.graph_projection_store.borrow_mut()
    }

    pub fn audit_store(&self) -> Ref<'_, AS> {
        self.audit_store.borrow()
    }

    pub fn backup_store(&self) -> Ref<'_, BS> {
        self.backup_store.borrow()
    }

    pub fn backup_audit_recorder(&self) -> Ref<'_, BAR> {
        self.backup_audit_recorder.borrow()
    }

    pub fn session_store(&self) -> Ref<'_, SS> {
        self.session_store.borrow()
    }

    pub fn user_repository(&self) -> Ref<'_, UR> {
        self.user_repository.borrow()
    }

    pub fn group_repository(&self) -> Ref<'_, GR> {
        self.group_repository.borrow()
    }

    pub fn permission_repository(&self) -> Ref<'_, PR> {
        self.permission_repository.borrow()
    }

    pub fn field_debug_repository(&self) -> Ref<'_, FDR> {
        self.field_debug_repository.borrow()
    }

    pub const fn field_debug_permission_checker(&self) -> &FDPC {
        &self.field_debug_permission_checker
    }

    pub fn document_change_publisher(&self) -> Ref<'_, DCP> {
        self.document_change_publisher.borrow()
    }

    pub fn document_lock_repository(&self) -> Ref<'_, DLR> {
        self.document_lock_repository.borrow()
    }

    pub fn comment_repository(&self) -> Ref<'_, CR> {
        self.comment_repository.borrow()
    }

    pub fn review_repository(&self) -> Ref<'_, RWR> {
        self.review_repository.borrow()
    }

    pub fn review_side_effect_recorder(&self) -> Ref<'_, RWSR> {
        self.review_side_effect_recorder.borrow()
    }
}

impl<
    PC,
    DR,
    DQ,
    VS,
    SI,
    AS,
    BS,
    BAR,
    CV,
    TI,
    SS,
    SC,
    SIG,
    GR,
    PR,
    RIG,
    UR,
    FDR,
    FDPC,
    FDC,
    DCP,
    DLR,
    DLC,
    CR,
    RWR,
    RWSR,
    L,
> ServerUsecaseTarget
    for ServerRuntimeTarget<
        PC,
        DR,
        DQ,
        VS,
        SI,
        AS,
        BS,
        BAR,
        CV,
        TI,
        SS,
        SC,
        SIG,
        GR,
        PR,
        RIG,
        UR,
        FDR,
        FDPC,
        FDC,
        DCP,
        DLR,
        DLC,
        CR,
        RWR,
        RWSR,
        L,
    >
where
    PC: PermissionDecisionPort
        + CommentPermissionChecker
        + DocumentLockPermissionChecker
        + AuditPermissionChecker
        + ReviewWorkflowPermissionChecker,
    DR: DocumentRepository,
    DQ: AccessibleDocumentQuery,
    VS: VersionStore + InlineAnchorDocumentLookup,
    SI: PermissionAwareSearchIndex,
    AS: AuditLogStore,
    BS: BackupStore,
    BAR: BackupAuditRecorder,
    CV: CredentialVerifier,
    TI: TokenIssuer,
    SS: SessionStore,
    SC: SessionClock,
    SIG: SessionIdGenerator,
    GR: GroupRepository + PermissionGroupRepository,
    PR: PermissionPolicyRepository,
    RIG: RoleAssignmentIdGenerator,
    UR: UserRepository,
    FDR: FieldDebugSessionRepository,
    FDPC: FieldDebugPermissionChecker,
    FDC: FieldDebugClock,
    DCP: DocumentChangeEventPublisher,
    DLR: DocumentLockRepository,
    DLC: DocumentLockClock,
    CR: CommentRepository,
    RWR: ReviewWorkflowRepository,
    RWSR: ReviewWorkflowSideEffectRecorder,
    L: AccessibleQueryLogger
        + FieldDebugUsecaseLogger
        + AuthProductLogger
        + CreateGroupProductLogger
        + PermissionUsecaseLogger
        + DocumentProductLogger
        + DocumentLockUsecaseLogger
        + AuditUsecaseLogger
        + BackupJobUsecaseLogger
        + CommentUsecaseLogger
        + ReviewWorkflowUsecaseLogger,
{
    fn handle(&self, input: UsecaseInputDto) -> Result<UsecaseOutputDto, ServerBoundaryError> {
        match HandlerKind::for_route_id(input.route_id()) {
            HandlerKind::CurrentDocument => Ok(self.handle_current_document(&input)),
            HandlerKind::DocumentHistory => Ok(self.handle_document_history(&input)),
            HandlerKind::PermissionAwareSearch => Ok(self.handle_search(&input)),
            HandlerKind::GraphLocal => Ok(self.handle_graph_local(&input)),
            HandlerKind::FieldDebugRequest => Ok(self.handle_field_debug_request(&input)),
            HandlerKind::FieldDebugApprove => Ok(self.handle_field_debug_approve(&input)),
            HandlerKind::FieldDebugExpire => Ok(self.handle_field_debug_expire(&input)),
            HandlerKind::AuthLogin => Ok(self.handle_auth_login(&input)),
            HandlerKind::AuthValidateSession => Ok(self.handle_auth_validate_session(&input)),
            HandlerKind::UserList => Ok(self.handle_user_list(&input)),
            HandlerKind::GroupList => Ok(self.handle_group_list(&input)),
            HandlerKind::GroupAddMember => Ok(self.handle_group_add_member(&input)),
            HandlerKind::GroupRemoveMember => Ok(self.handle_group_remove_member(&input)),
            HandlerKind::RoleListAssignments => Ok(self.handle_role_list_assignments(&input)),
            HandlerKind::RoleAssign => Ok(self.handle_role_assign(&input)),
            HandlerKind::RoleRevoke => Ok(self.handle_role_revoke(&input)),
            HandlerKind::RemoteCurrentDocumentSave => {
                Ok(self.handle_remote_current_document_save(&input))
            }
            HandlerKind::SharingGetDocument => Ok(self.handle_sharing_get_document(&input)),
            HandlerKind::SharingUpdateDocument => Ok(self.handle_sharing_update_document(&input)),
            HandlerKind::CommentList => Ok(self.handle_comment_list(&input)),
            HandlerKind::CommentAdd => Ok(self.handle_comment_add(&input)),
            HandlerKind::CommentAddInline => Ok(self.handle_comment_add_inline(&input)),
            HandlerKind::CommentResolve => Ok(self.handle_comment_resolve(&input)),
            HandlerKind::CommentReopen => Ok(self.handle_comment_reopen(&input)),
            HandlerKind::ReviewRequestDocument => Ok(self.handle_review_request_document(&input)),
            HandlerKind::ReviewApproveDocument => Ok(self.handle_review_approve_document(&input)),
            HandlerKind::ReviewRejectDocument => Ok(self.handle_review_reject_document(&input)),
            HandlerKind::ReviewPublishDocument => Ok(self.handle_review_publish_document(&input)),
            HandlerKind::ReviewListRequests => Ok(self.handle_review_list_requests(&input)),
            HandlerKind::DocumentLockLock => Ok(self.handle_document_lock_lock(&input)),
            HandlerKind::DocumentLockUnlock => Ok(self.handle_document_lock_unlock(&input)),
            HandlerKind::DocumentLockGet => Ok(self.handle_document_lock_get(&input)),
            HandlerKind::AuditListEvents => Ok(self.handle_audit_list_events(&input)),
            HandlerKind::BackupCreate => Ok(self.handle_backup_create(&input)),
            HandlerKind::BackupGetStatus => Ok(self.handle_backup_get_status(&input)),
            HandlerKind::BackupRestore => Ok(self.handle_backup_restore(&input)),
            HandlerKind::ExportCreateWorkspace => Ok(self.handle_export_create_workspace(&input)),
            HandlerKind::ExportGetStatus => Ok(self.handle_export_get_status(&input)),
            HandlerKind::CollaborationJoinDocumentRoom
            | HandlerKind::CollaborationBroadcastOperation
            | HandlerKind::CollaborationBroadcastPresence
            | HandlerKind::CollaborationRequestReplay
            | HandlerKind::CanvasCreate
            | HandlerKind::CanvasAddNode
            | HandlerKind::CanvasEmbed
            | HandlerKind::Health
            | HandlerKind::NotImplemented => {
                Ok(error_output(501, "SERVER_HANDLER_NOT_IMPLEMENTED"))
            }
        }
    }
}

impl<
    PC,
    DR,
    DQ,
    VS,
    SI,
    AS,
    BS,
    BAR,
    CV,
    TI,
    SS,
    SC,
    SIG,
    GR,
    PR,
    RIG,
    UR,
    FDR,
    FDPC,
    FDC,
    DCP,
    DLR,
    DLC,
    CR,
    RWR,
    RWSR,
    L,
>
    ServerRuntimeTarget<
        PC,
        DR,
        DQ,
        VS,
        SI,
        AS,
        BS,
        BAR,
        CV,
        TI,
        SS,
        SC,
        SIG,
        GR,
        PR,
        RIG,
        UR,
        FDR,
        FDPC,
        FDC,
        DCP,
        DLR,
        DLC,
        CR,
        RWR,
        RWSR,
        L,
    >
where
    PC: PermissionDecisionPort
        + CommentPermissionChecker
        + DocumentLockPermissionChecker
        + AuditPermissionChecker
        + ReviewWorkflowPermissionChecker,
    DR: DocumentRepository,
    DQ: AccessibleDocumentQuery,
    VS: VersionStore + InlineAnchorDocumentLookup,
    SI: PermissionAwareSearchIndex,
    AS: AuditLogStore,
    BS: BackupStore,
    BAR: BackupAuditRecorder,
    CV: CredentialVerifier,
    TI: TokenIssuer,
    SS: SessionStore,
    SC: SessionClock,
    SIG: SessionIdGenerator,
    GR: GroupRepository + PermissionGroupRepository,
    PR: PermissionPolicyRepository,
    RIG: RoleAssignmentIdGenerator,
    UR: UserRepository,
    FDR: FieldDebugSessionRepository,
    FDPC: FieldDebugPermissionChecker,
    FDC: FieldDebugClock,
    DCP: DocumentChangeEventPublisher,
    DLR: DocumentLockRepository,
    DLC: DocumentLockClock,
    CR: CommentRepository,
    RWR: ReviewWorkflowRepository,
    RWSR: ReviewWorkflowSideEffectRecorder,
    L: AccessibleQueryLogger
        + FieldDebugUsecaseLogger
        + AuthProductLogger
        + CreateGroupProductLogger
        + PermissionUsecaseLogger
        + DocumentProductLogger
        + DocumentLockUsecaseLogger
        + AuditUsecaseLogger
        + BackupJobUsecaseLogger
        + CommentUsecaseLogger
        + ReviewWorkflowUsecaseLogger,
{
    fn handle_current_document(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(workspace_id) = input.path_param("workspaceId") else {
            return malformed_request();
        };
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };

        let mut logger = self.logger.borrow_mut();
        match GetAccessibleDocumentUsecase::new().execute(
            GetAccessibleDocumentInput::new(
                self.actor_context.user_id(),
                workspace_id,
                None,
                document_id,
            ),
            &self.permission_checker,
            &self.document_query,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_current_document(output.record())),
            Err(error) => accessible_document_error_output(error),
        }
    }

    fn handle_document_history(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(workspace_id) = input.path_param("workspaceId") else {
            return malformed_request();
        };
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };
        let Some(limit) = optional_usize(input.query_param("limit"), 20) else {
            return malformed_request();
        };

        let version_store = self.version_store.borrow();
        match GetDocumentHistoryUsecase::new().execute(
            GetDocumentHistoryInput::new(
                workspace_id,
                document_id,
                input.query_param("cursor"),
                limit,
            ),
            &*version_store,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_history(output.page())),
            Err(error) => document_history_error_output(error),
        }
    }

    fn handle_remote_current_document_save(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(workspace_id) = input.path_param("workspaceId") else {
            return malformed_request();
        };
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(document_body) = body.string("body") else {
            return malformed_request();
        };
        let Some(version_id) = body.string("versionId") else {
            return malformed_request();
        };
        let Some(snapshot_ref) = body.string("snapshotRef") else {
            return malformed_request();
        };
        let Some(author) = body.string("author") else {
            return malformed_request();
        };
        let Some(summary) = body.string("summary") else {
            return malformed_request();
        };
        let usecase =
            match UpdateDocumentUsecase::with_body_limit(self.policy.document_body_max_bytes) {
                Ok(usecase) => usecase,
                Err(error) => return update_document_error_output(error),
            };

        let mut document_repository = self.document_repository.borrow_mut();
        let mut version_store = self.version_store.borrow_mut();
        let mut event_publisher = self.document_change_publisher.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match usecase.execute(
            UpdateDocumentInput::new(
                workspace_id,
                document_id,
                document_body,
                version_id,
                snapshot_ref,
                author,
                summary,
            ),
            &mut *document_repository,
            &mut *version_store,
            &mut *event_publisher,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(
                200,
                &render_remote_current_document_save(workspace_id, document_id, &output),
            ),
            Err(error) => update_document_error_output(error),
        }
    }

    fn handle_search(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(workspace_id) = input.path_param("workspaceId") else {
            return malformed_request();
        };
        let Some(query_text) = input.query_param("text") else {
            return malformed_request();
        };
        let Some(limit) = optional_usize(input.query_param("limit"), 20) else {
            return malformed_request();
        };

        let mut search_index = self.search_index.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match SearchAccessibleDocumentsUsecase::new().execute(
            SearchAccessibleDocumentsInput::new(
                self.actor_context.user_id(),
                workspace_id,
                query_text,
                limit,
            ),
            &mut *search_index,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_search(output.page())),
            Err(error) => accessible_search_error_output(error),
        }
    }

    fn handle_graph_local(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(workspace_id) = input.path_param("workspaceId") else {
            return malformed_request();
        };
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };

        let graph_projection_store = self.graph_projection_store.borrow();
        match PermissionAwareGraphUsecase::new().execute(
            PermissionAwareGraphInput::new(workspace_id, self.actor_context.user_id(), document_id),
            &*graph_projection_store,
            &self.permission_checker,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_graph(output.graph(), output.stats())),
            Err(error) => graph_error_output(error),
        }
    }

    fn handle_field_debug_request(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };
        let Some(session_id) = body.string("sessionId") else {
            return malformed_request();
        };

        let mut repository = self.field_debug_repository.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match RequestFieldDebugSessionUsecase::new(self.policy.field_debug_policy).execute(
            RequestFieldDebugSessionInput::new(
                self.actor_context.user_id(),
                workspace_id,
                session_id,
                body.string("scope"),
                body.u32("ttlSeconds"),
            ),
            &mut *repository,
            &self.field_debug_clock,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(202, &render_field_debug_session(&output)),
            Err(error) => field_debug_error_output(error),
        }
    }

    fn handle_field_debug_approve(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(session_id) = input.path_param("sessionId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };

        let mut repository = self.field_debug_repository.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match ApproveFieldDebugSessionUsecase::new(self.policy.field_debug_policy).execute(
            ApproveFieldDebugSessionInput::new(
                self.actor_context.user_id(),
                workspace_id,
                session_id,
            ),
            &self.field_debug_permission_checker,
            &mut *repository,
            &self.field_debug_clock,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_field_debug_session(&output)),
            Err(error) => field_debug_error_output(error),
        }
    }

    fn handle_field_debug_expire(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(session_id) = input.path_param("sessionId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };

        let mut repository = self.field_debug_repository.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match ExpireFieldDebugSessionUsecase::new(self.policy.field_debug_policy).execute(
            ExpireFieldDebugSessionInput::new(
                self.actor_context.user_id(),
                workspace_id,
                session_id,
            ),
            &self.field_debug_permission_checker,
            &mut *repository,
            &self.field_debug_clock,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_field_debug_session(&output)),
            Err(error) => field_debug_error_output(error),
        }
    }

    fn handle_auth_login(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let body = BodyFields::parse(input.body());
        let Some(login) = body.string("login") else {
            return malformed_request();
        };
        let Some(credential) = body.string("credential") else {
            return malformed_request();
        };
        if !is_valid_credential_input(credential) {
            return malformed_request();
        }

        let mut credential_verifier = self.credential_verifier.borrow_mut();
        let mut token_issuer = self.token_issuer.borrow_mut();
        let mut session_store = self.session_store.borrow_mut();
        let mut session_id_generator = self.session_id_generator.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match AuthenticateUserUsecase::new(self.policy.auth_policy).execute(
            AuthenticateUserInput::new(login, credential),
            &mut *credential_verifier,
            &mut *token_issuer,
            &mut *session_store,
            &self.session_clock,
            &mut *session_id_generator,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_auth_login(&output)),
            Err(error) => auth_error_output(error),
        }
    }

    fn handle_auth_validate_session(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let body = BodyFields::parse(input.body());
        let Some(token) = body.string("token") else {
            return malformed_request();
        };
        if !is_valid_token_input(token) {
            return malformed_request();
        }

        let token_issuer = self.token_issuer.borrow();
        let mut session_store = self.session_store.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match ValidateSessionUsecase::new().execute(
            ValidateSessionInput::new(token),
            &*token_issuer,
            &mut *session_store,
            &self.session_clock,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_auth_validate_session(&output)),
            Err(error) => auth_error_output(error),
        }
    }

    fn handle_user_list(&self, _input: &UsecaseInputDto) -> UsecaseOutputDto {
        let repository = self.user_repository.borrow();
        match ListUsersUsecase::new().execute(ListUsersInput::new(), &*repository) {
            Ok(output) => UsecaseOutputDto::new(200, &render_user_list(&output)),
            Err(error) => list_users_error_output(error),
        }
    }

    fn handle_group_list(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(workspace_id) = input.path_param("workspaceId") else {
            return malformed_request();
        };
        let repository = self.group_repository.borrow();
        match ListWorkspaceGroupsUsecase::new()
            .execute(ListWorkspaceGroupsInput::new(workspace_id), &*repository)
        {
            Ok(output) => UsecaseOutputDto::new(200, &render_group_list(workspace_id, &output)),
            Err(error) => list_workspace_groups_error_output(error),
        }
    }

    fn handle_group_add_member(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(workspace_id) = input.path_param("workspaceId") else {
            return malformed_request();
        };
        let Some(group_id) = input.path_param("groupId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(user_id) = body.string("userId") else {
            return malformed_request();
        };

        let mut group_repository = self.group_repository.borrow_mut();
        let user_repository = self.user_repository.borrow();
        let mut logger = self.logger.borrow_mut();
        match AddUserToGroupUsecase::new().execute(
            AddUserToGroupInput::new(workspace_id, group_id, user_id),
            &mut *group_repository,
            &*user_repository,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_added_group_member(&output)),
            Err(error) => add_group_member_error_output(error),
        }
    }

    fn handle_group_remove_member(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(workspace_id) = input.path_param("workspaceId") else {
            return malformed_request();
        };
        let Some(group_id) = input.path_param("groupId") else {
            return malformed_request();
        };
        let Some(user_id) = input.path_param("userId") else {
            return malformed_request();
        };

        let mut group_repository = self.group_repository.borrow_mut();
        let user_repository = self.user_repository.borrow();
        let mut logger = self.logger.borrow_mut();
        match RemoveUserFromGroupUsecase::new().execute(
            RemoveUserFromGroupInput::new(workspace_id, group_id, user_id),
            &mut *group_repository,
            &*user_repository,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_removed_group_member(&output)),
            Err(error) => remove_group_member_error_output(error),
        }
    }

    fn handle_role_list_assignments(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(workspace_id) = input.path_param("workspaceId") else {
            return malformed_request();
        };

        let repository = self.permission_repository.borrow();
        match ListWorkspaceRoleAssignmentsUsecase::new().execute(
            ListWorkspaceRoleAssignmentsInput::new(workspace_id),
            &*repository,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_role_assignment_list(&output)),
            Err(error) => list_role_assignments_error_output(error),
        }
    }

    fn handle_role_assign(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(workspace_id) = input.path_param("workspaceId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(subject_type) = body.string("subjectType") else {
            return malformed_request();
        };
        let Some(subject_id) = body.string("subjectId") else {
            return malformed_request();
        };
        let Some(role) = body.string("role") else {
            return malformed_request();
        };
        let assign_input = match assign_role_input(
            self.actor_context.user_id(),
            workspace_id,
            subject_type,
            subject_id,
            role,
        ) {
            Ok(assign_input) => assign_input,
            Err(error) => return assign_role_error_output(error),
        };

        let mut permission_repository = self.permission_repository.borrow_mut();
        let group_repository = self.group_repository.borrow();
        let mut id_generator = self.role_assignment_id_generator.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match AssignRoleUsecase::new().execute(
            assign_input,
            &mut *permission_repository,
            &*group_repository,
            &mut *id_generator,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_assigned_role(&output)),
            Err(error) => assign_role_error_output(error),
        }
    }

    fn handle_role_revoke(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(workspace_id) = input.path_param("workspaceId") else {
            return malformed_request();
        };
        let Some(assignment_id) = input.path_param("assignmentId") else {
            return malformed_request();
        };

        let mut permission_repository = self.permission_repository.borrow_mut();
        let group_repository = self.group_repository.borrow();
        let mut logger = self.logger.borrow_mut();
        match RevokeRoleUsecase::new().execute(
            RevokeRoleInput::new(self.actor_context.user_id(), workspace_id, assignment_id),
            &mut *permission_repository,
            &*group_repository,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_revoked_role(&output)),
            Err(error) => revoke_role_error_output(error),
        }
    }

    fn handle_sharing_get_document(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };
        let Some(workspace_id) = input.query_param("workspaceId") else {
            return malformed_request();
        };
        let resource = PermissionResourceInput::document(
            workspace_id,
            input.query_param("collectionId"),
            document_id,
        );

        let permission_repository = self.permission_repository.borrow();
        let group_repository = self.group_repository.borrow();
        let mut logger = self.logger.borrow_mut();
        match ListEffectivePermissionsUsecase::new().execute(
            ListEffectivePermissionsInput::new(self.actor_context.user_id(), resource),
            &*permission_repository,
            &*group_repository,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(
                200,
                &render_document_sharing(workspace_id, document_id, &output),
            ),
            Err(error) => list_effective_permissions_error_output(error),
        }
    }

    fn handle_sharing_update_document(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };
        let Some(subject_kind) = body.string("kind").or_else(|| body.string("subjectType")) else {
            return malformed_request();
        };
        let Some(subject_id) = body.string("id").or_else(|| body.string("subjectId")) else {
            return malformed_request();
        };
        if !is_valid_sharing_subject_kind(subject_kind) {
            return malformed_request();
        }
        let Some(permission) = body.string("permission") else {
            return malformed_request();
        };
        let Some(effect) = body.string("effect") else {
            return malformed_request();
        };
        let share_input = match ShareDocumentInput::from_effect_name(
            self.actor_context.user_id(),
            workspace_id,
            body.string("collectionId"),
            document_id,
            permission,
            effect,
        ) {
            Ok(input) => input,
            Err(error) => return share_document_error_output(error),
        };

        let mut permission_repository = self.permission_repository.borrow_mut();
        let group_repository = self.group_repository.borrow();
        let mut logger = self.logger.borrow_mut();
        match ShareDocumentUsecase::new().execute(
            share_input,
            &mut *permission_repository,
            &*group_repository,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(
                200,
                &render_updated_document_sharing(
                    workspace_id,
                    &output,
                    subject_kind,
                    subject_id,
                    permission,
                    effect,
                ),
            ),
            Err(error) => share_document_error_output(error),
        }
    }

    fn handle_comment_list(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };
        let Some(workspace_id) = input.query_param("workspaceId") else {
            return malformed_request();
        };

        let repository = self.comment_repository.borrow();
        let mut logger = self.logger.borrow_mut();
        match ListDocumentCommentsUsecase::new().execute(
            ListDocumentCommentsInput::new(self.actor_context.user_id(), workspace_id, document_id),
            &self.permission_checker,
            &*repository,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_comment_thread_page(&output)),
            Err(error) => comment_usecase_error_output(error),
        }
    }

    fn handle_comment_add(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };
        let Some(thread_id) = body.string("threadId") else {
            return malformed_request();
        };
        let Some(comment_id) = body.string("commentId") else {
            return malformed_request();
        };
        let Some(comment_body) = body.string("body") else {
            return malformed_request();
        };
        let usecase = match AddCommentUsecase::with_body_limit(self.policy.comment_body_max_bytes) {
            Ok(usecase) => usecase,
            Err(error) => return comment_usecase_error_output(error),
        };

        let mut repository = self.comment_repository.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match usecase.execute(
            AddCommentInput::new(
                self.actor_context.user_id(),
                workspace_id,
                document_id,
                thread_id,
                comment_id,
                comment_body,
            ),
            &self.permission_checker,
            &mut *repository,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_comment_thread_mutation(&output)),
            Err(error) => comment_usecase_error_output(error),
        }
    }

    fn handle_comment_add_inline(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };
        let Some(version_id) = body.string("versionId") else {
            return malformed_request();
        };
        let Some(start_offset) = body.u32("startOffset").map(|value| value as usize) else {
            return malformed_request();
        };
        let Some(end_offset) = body.u32("endOffset").map(|value| value as usize) else {
            return malformed_request();
        };
        let Some(thread_id) = body.string("threadId") else {
            return malformed_request();
        };
        let Some(comment_id) = body.string("commentId") else {
            return malformed_request();
        };
        let Some(comment_body) = body.string("body") else {
            return malformed_request();
        };
        let usecase =
            match AddInlineCommentUsecase::with_body_limit(self.policy.comment_body_max_bytes) {
                Ok(usecase) => usecase,
                Err(error) => return comment_usecase_error_output(error),
            };

        let version_store = self.version_store.borrow();
        let mut repository = self.comment_repository.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match usecase.execute(
            AddInlineCommentInput::new(
                self.actor_context.user_id(),
                workspace_id,
                document_id,
                version_id,
                start_offset,
                end_offset,
                thread_id,
                comment_id,
                comment_body,
            ),
            &self.permission_checker,
            &*version_store,
            &mut *repository,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_comment_thread_mutation(&output)),
            Err(error) => comment_usecase_error_output(error),
        }
    }

    fn handle_comment_resolve(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(thread_id) = input.path_param("commentId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };
        let Some(document_id) = body.string("documentId") else {
            return malformed_request();
        };

        let mut repository = self.comment_repository.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match ResolveCommentUsecase::new().execute(
            ResolveCommentInput::new(
                self.actor_context.user_id(),
                workspace_id,
                document_id,
                thread_id,
            ),
            &self.permission_checker,
            &mut *repository,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_comment_thread_mutation(&output)),
            Err(error) => comment_usecase_error_output(error),
        }
    }

    fn handle_comment_reopen(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(thread_id) = input.path_param("commentId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };
        let Some(document_id) = body.string("documentId") else {
            return malformed_request();
        };

        let mut repository = self.comment_repository.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match ReopenCommentUsecase::new().execute(
            ReopenCommentInput::new(
                self.actor_context.user_id(),
                workspace_id,
                document_id,
                thread_id,
            ),
            &self.permission_checker,
            &mut *repository,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_comment_thread_mutation(&output)),
            Err(error) => comment_usecase_error_output(error),
        }
    }

    fn handle_review_request_document(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };
        let Some(review_request_id) = body.string("reviewRequestId") else {
            return malformed_request();
        };

        let mut repository = self.review_repository.borrow_mut();
        let mut side_effect_recorder = self.review_side_effect_recorder.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match RequestDocumentReviewUsecase::new(ReviewWorkflowPolicy::default()).execute(
            RequestDocumentReviewInput::new(
                self.actor_context.user_id(),
                workspace_id,
                document_id,
                review_request_id,
            ),
            &mut *repository,
            &mut *side_effect_recorder,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_review_workflow(&output)),
            Err(error) => review_workflow_error_output(error),
        }
    }

    fn handle_review_approve_document(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(review_request_id) = input.path_param("reviewRequestId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };

        let mut repository = self.review_repository.borrow_mut();
        let mut side_effect_recorder = self.review_side_effect_recorder.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match ApproveDocumentUsecase::new(ReviewWorkflowPolicy::default()).execute(
            ApproveDocumentInput::new(
                self.actor_context.user_id(),
                workspace_id,
                review_request_id,
            ),
            &self.permission_checker,
            &mut *repository,
            &mut *side_effect_recorder,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_review_workflow(&output)),
            Err(error) => review_workflow_error_output(error),
        }
    }

    fn handle_review_reject_document(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(review_request_id) = input.path_param("reviewRequestId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };

        let mut repository = self.review_repository.borrow_mut();
        let mut side_effect_recorder = self.review_side_effect_recorder.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match RejectDocumentUsecase::new(ReviewWorkflowPolicy::default()).execute(
            RejectDocumentInput::new(
                self.actor_context.user_id(),
                workspace_id,
                review_request_id,
            ),
            &self.permission_checker,
            &mut *repository,
            &mut *side_effect_recorder,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_review_workflow(&output)),
            Err(error) => review_workflow_error_output(error),
        }
    }

    fn handle_review_publish_document(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };

        let mut repository = self.review_repository.borrow_mut();
        let mut side_effect_recorder = self.review_side_effect_recorder.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match PublishDocumentUsecase::new(ReviewWorkflowPolicy::default()).execute(
            PublishDocumentInput::new(self.actor_context.user_id(), workspace_id, document_id),
            &self.permission_checker,
            &mut *repository,
            &mut *side_effect_recorder,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_review_workflow(&output)),
            Err(error) => review_workflow_error_output(error),
        }
    }

    fn handle_review_list_requests(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(workspace_id) = input.query_param("workspaceId") else {
            return malformed_request();
        };
        let list_input = input.query_param("documentId").map_or_else(
            || ListReviewRequestsInput::for_workspace(self.actor_context.user_id(), workspace_id),
            |document_id| {
                ListReviewRequestsInput::for_document(
                    self.actor_context.user_id(),
                    workspace_id,
                    document_id,
                )
            },
        );

        let repository = self.review_repository.borrow();
        let mut logger = self.logger.borrow_mut();
        match ListReviewRequestsUsecase::new(ReviewWorkflowPolicy::default()).execute(
            list_input,
            &self.permission_checker,
            &*repository,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_review_request_list(&output)),
            Err(error) => review_workflow_error_output(error),
        }
    }

    fn handle_document_lock_lock(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };
        let Some(lock_id) = body.string("lockId") else {
            return malformed_request();
        };

        let mut repository = self.document_lock_repository.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match LockDocumentUsecase::new(LockDocumentPolicy::default()).execute(
            LockDocumentInput::new(
                self.actor_context.user_id(),
                workspace_id,
                document_id,
                lock_id,
            ),
            &self.permission_checker,
            &mut *repository,
            &self.document_lock_clock,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_document_lock(&output)),
            Err(error) => document_lock_error_output(error),
        }
    }

    fn handle_document_lock_unlock(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };
        let Some(workspace_id) = input.query_param("workspaceId") else {
            return malformed_request();
        };

        let mut repository = self.document_lock_repository.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match UnlockDocumentUsecase::new().execute(
            UnlockDocumentInput::new(self.actor_context.user_id(), workspace_id, document_id),
            &self.permission_checker,
            &mut *repository,
            &self.document_lock_clock,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_document_lock(&output)),
            Err(error) => document_lock_error_output(error),
        }
    }

    fn handle_document_lock_get(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(document_id) = input.path_param("documentId") else {
            return malformed_request();
        };
        let Some(workspace_id) = input.query_param("workspaceId") else {
            return malformed_request();
        };

        let mut repository = self.document_lock_repository.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match GetDocumentLockUsecase::new().execute(
            GetDocumentLockInput::new(self.actor_context.user_id(), workspace_id, document_id),
            &self.permission_checker,
            &mut *repository,
            &self.document_lock_clock,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_document_lock(&output)),
            Err(error) => document_lock_error_output(error),
        }
    }

    fn handle_audit_list_events(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(workspace_id) = input.query_param("workspaceId") else {
            return malformed_request();
        };
        let Some(limit) = optional_usize(input.query_param("limit"), 50) else {
            return malformed_request();
        };
        let Some(scope) = audit_scope_input(input) else {
            return malformed_request();
        };

        let store = self.audit_store.borrow();
        let mut logger = self.logger.borrow_mut();
        match ListAuditEventsUsecase::new(AuditRetentionPolicy::default()).execute(
            ListAuditEventsInput::new(
                self.actor_context.user_id(),
                workspace_id,
                scope,
                limit,
                input.query_param("cursor"),
            ),
            &self.permission_checker,
            &*store,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_audit_event_page(&output)),
            Err(error) => audit_error_output(error),
        }
    }

    fn handle_backup_create(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };
        let Some(job_id) = body.string("jobId") else {
            return malformed_request();
        };

        let mut store = self.backup_store.borrow_mut();
        let mut audit_recorder = self.backup_audit_recorder.borrow_mut();
        match CreateBackupUsecase::new().execute(
            CreateBackupInput::new(self.actor_context.user_id(), workspace_id, job_id),
            &mut *store,
            &mut *audit_recorder,
        ) {
            Ok(output) => UsecaseOutputDto::new(202, &render_backup_job(&output)),
            Err(error) => backup_job_error_output(error),
        }
    }

    fn handle_backup_get_status(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(job_id) = input.path_param("jobId") else {
            return malformed_request();
        };
        let Some(workspace_id) = input.query_param("workspaceId") else {
            return malformed_request();
        };

        let store = self.backup_store.borrow();
        match GetBackupStatusUsecase::new().execute(
            GetBackupStatusInput::new(self.actor_context.user_id(), workspace_id, job_id),
            &*store,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_backup_job(&output)),
            Err(error) => backup_job_error_output(error),
        }
    }

    fn handle_backup_restore(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(source_backup_job_id) = input.path_param("jobId") else {
            return malformed_request();
        };
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };
        let Some(restore_job_id) = body.string("jobId") else {
            return malformed_request();
        };

        let mut store = self.backup_store.borrow_mut();
        let mut audit_recorder = self.backup_audit_recorder.borrow_mut();
        let mut logger = self.logger.borrow_mut();
        match RestoreBackupUsecase::new().execute(
            RestoreBackupInput::new(
                self.actor_context.user_id(),
                workspace_id,
                source_backup_job_id,
                restore_job_id,
            ),
            &mut *store,
            &mut *audit_recorder,
            &mut *logger,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_backup_job(&output)),
            Err(error) => backup_job_error_output(error),
        }
    }

    fn handle_export_create_workspace(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let body = BodyFields::parse(input.body());
        let Some(workspace_id) = body.string("workspaceId") else {
            return malformed_request();
        };
        let Some(job_id) = body.string("jobId") else {
            return malformed_request();
        };

        let mut store = self.backup_store.borrow_mut();
        match ExportWorkspaceUsecase::new().execute(
            ExportWorkspaceInput::new(self.actor_context.user_id(), workspace_id, job_id),
            &mut *store,
        ) {
            Ok(output) => UsecaseOutputDto::new(202, &render_backup_job(&output)),
            Err(error) => backup_job_error_output(error),
        }
    }

    fn handle_export_get_status(&self, input: &UsecaseInputDto) -> UsecaseOutputDto {
        let Some(job_id) = input.path_param("jobId") else {
            return malformed_request();
        };
        let Some(workspace_id) = input.query_param("workspaceId") else {
            return malformed_request();
        };

        let store = self.backup_store.borrow();
        match GetExportStatusUsecase::new().execute(
            GetExportStatusInput::new(self.actor_context.user_id(), workspace_id, job_id),
            &*store,
        ) {
            Ok(output) => UsecaseOutputDto::new(200, &render_backup_job(&output)),
            Err(error) => backup_job_error_output(error),
        }
    }
}

struct BodyFields {
    body: String,
}

impl BodyFields {
    fn parse(body: Option<&str>) -> Self {
        Self {
            body: body.unwrap_or_default().to_string(),
        }
    }

    fn string(&self, key: &str) -> Option<&str> {
        let key_pattern = format!("\"{}\"", key);
        let start = self.body.find(&key_pattern)?;
        let after_key = &self.body[start + key_pattern.len()..];
        let colon = after_key.find(':')?;
        let after_colon = after_key[colon + 1..].trim_start();
        let after_quote = after_colon.strip_prefix('"')?;
        let end = after_quote.find('"')?;
        Some(&after_quote[..end])
    }

    fn u32(&self, key: &str) -> Option<u32> {
        let key_pattern = format!("\"{}\"", key);
        let start = self.body.find(&key_pattern)?;
        let after_key = &self.body[start + key_pattern.len()..];
        let colon = after_key.find(':')?;
        let after_colon = after_key[colon + 1..].trim_start();
        let digits = after_colon
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect::<String>();
        if digits.is_empty() {
            return None;
        }
        digits.parse().ok()
    }
}

fn optional_usize(value: Option<&str>, default: usize) -> Option<usize> {
    match value {
        Some(value) => value.parse::<usize>().ok(),
        None => Some(default),
    }
}

fn audit_scope_input(input: &UsecaseInputDto) -> Option<ListAuditEventsScopeInput> {
    match input.query_param("scope").unwrap_or("workspace") {
        "workspace" => Some(ListAuditEventsScopeInput::workspace()),
        "actor" => input
            .query_param("actorUserId")
            .map(ListAuditEventsScopeInput::actor),
        "target" => Some(ListAuditEventsScopeInput::target(
            input.query_param("targetType")?,
            input.query_param("targetId")?,
        )),
        _ => None,
    }
}

fn is_valid_credential_input(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

fn is_valid_token_input(value: &str) -> bool {
    !value.trim().is_empty()
        && !value.chars().any(char::is_control)
        && !value.chars().any(char::is_whitespace)
}

fn assign_role_input(
    actor_user_id: &str,
    workspace_id: &str,
    subject_type: &str,
    subject_id: &str,
    role: &str,
) -> Result<AssignRoleInput, AssignRoleError> {
    match subject_type {
        "user" => {
            AssignRoleInput::for_user_role_name(actor_user_id, workspace_id, subject_id, role)
        }
        "group" => {
            AssignRoleInput::for_group_role_name(actor_user_id, workspace_id, subject_id, role)
        }
        _ => Err(AssignRoleError::InvalidInput),
    }
}

fn is_valid_sharing_subject_kind(value: &str) -> bool {
    matches!(value, "user" | "group")
}

fn render_current_document(
    record: &cabinet_ports::document_repository::CurrentDocumentRecord,
) -> String {
    format!(
        "{{\"documentId\":\"{}\",\"title\":\"{}\",\"path\":\"{}\",\"body\":\"{}\"}}",
        escape_json(record.document_id().as_str()),
        escape_json(record.metadata().title().as_str()),
        escape_json(record.path().as_str()),
        escape_json(record.body().as_str())
    )
}

fn render_history(page: &cabinet_ports::version_store::HistoryPage) -> String {
    let entries = page
        .entries()
        .iter()
        .map(|entry| {
            format!(
                "{{\"versionId\":\"{}\",\"documentId\":\"{}\",\"summary\":\"{}\"}}",
                escape_json(entry.version_id().as_str()),
                escape_json(entry.document_id().as_str()),
                escape_json(entry.summary().as_str())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let next_cursor = page.next_cursor().map_or_else(
        || "null".to_string(),
        |cursor| format!("\"{}\"", escape_json(cursor.as_str())),
    );
    format!(
        "{{\"entryCount\":{},\"entries\":[{}],\"nextCursor\":{}}}",
        page.entries().len(),
        entries,
        next_cursor
    )
}

fn render_remote_current_document_save(
    workspace_id: &str,
    document_id: &str,
    output: &UpdateDocumentOutput,
) -> String {
    format!(
        "{{\"workspaceId\":\"{}\",\"documentId\":\"{}\",\"status\":\"saved-remote\",\"versionId\":\"{}\"}}",
        escape_json(workspace_id),
        escape_json(document_id),
        escape_json(output.version_id_value())
    )
}

fn render_search(page: &cabinet_ports::permission_aware_query::SearchAccessiblePage) -> String {
    let results = page
        .results()
        .iter()
        .map(|result| {
            format!(
                "{{\"documentId\":\"{}\",\"title\":\"{}\",\"path\":\"{}\",\"snippet\":\"{}\",\"score\":{}}}",
                escape_json(result.document_id().as_str()),
                escape_json(result.title().as_str()),
                escape_json(result.path().as_str()),
                escape_json(result.snippet()),
                result.score()
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let stats = page.stats();
    format!(
        "{{\"resultCount\":{},\"results\":[{}],\"stats\":{{\"candidateCount\":{},\"filteredCount\":{},\"cacheHit\":{}}}}}",
        page.results().len(),
        results,
        stats.candidate_count(),
        stats.filtered_count(),
        stats.cache_hit()
    )
}

fn render_graph(graph: &KnowledgeGraph, stats: PermissionAwareGraphStats) -> String {
    let nodes = graph
        .nodes()
        .iter()
        .map(|node| {
            format!(
                "{{\"id\":\"{}\",\"kind\":\"{}\"}}",
                escape_json(node.id()),
                graph_node_kind(node.kind())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let edges = graph
        .edges()
        .iter()
        .map(|edge| {
            format!(
                "{{\"id\":\"{}\",\"sourceId\":\"{}\",\"targetId\":\"{}\",\"kind\":\"{}\"}}",
                escape_json(edge.id()),
                escape_json(edge.source_id()),
                escape_json(edge.target_id()),
                graph_edge_kind(edge.kind())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"centerDocumentId\":\"{}\",\"status\":\"{}\",\"nodes\":[{}],\"edges\":[{}],\"stats\":{{\"candidateCount\":{},\"filteredCount\":{}}}}}",
        escape_json(graph.center_document_id().as_str()),
        graph_projection_status(graph.status()),
        nodes,
        edges,
        stats.candidate_count(),
        stats.filtered_count()
    )
}

fn render_field_debug_session(output: &FieldDebugSessionOutput) -> String {
    let scope = output.scope().map_or_else(
        || "null".to_string(),
        |scope| format!("\"{}\"", escape_json(scope)),
    );
    let expires_at = output
        .expires_at_millis()
        .map_or_else(|| "null".to_string(), |expires_at| expires_at.to_string());
    format!(
        "{{\"sessionId\":\"{}\",\"status\":\"{}\",\"scope\":{},\"expiresAtMillis\":{}}}",
        escape_json(output.session_id()),
        field_debug_status(output.status()),
        scope,
        expires_at
    )
}

fn render_auth_login(output: &AuthenticateUserOutput) -> String {
    format!(
        "{{\"userId\":\"{}\",\"token\":\"{}\",\"sessionStatus\":\"{}\"}}",
        escape_json(output.user_id()),
        escape_json(output.token().expose_secret()),
        output.session_status().as_str()
    )
}

fn render_auth_validate_session(output: &ValidateSessionOutput) -> String {
    format!(
        "{{\"userId\":\"{}\",\"sessionStatus\":\"{}\"}}",
        escape_json(output.actor().user_id()),
        output.session_status().as_str()
    )
}

fn render_user_list(output: &ListUsersOutput) -> String {
    let users = output
        .users()
        .iter()
        .map(render_user_summary)
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"users\":[{}]}}", users)
}

fn render_user_summary(user: &ListUserSummary) -> String {
    format!(
        "{{\"userId\":\"{}\",\"login\":\"{}\",\"displayName\":\"{}\",\"status\":\"{}\"}}",
        escape_json(user.user_id()),
        escape_json(user.login()),
        escape_json(user.display_name()),
        user.status().as_str()
    )
}

fn render_group_list(workspace_id: &str, output: &ListWorkspaceGroupsOutput) -> String {
    let groups = output
        .groups()
        .iter()
        .map(render_group_summary)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"workspaceId\":\"{}\",\"groups\":[{}]}}",
        escape_json(workspace_id),
        groups
    )
}

fn render_group_summary(group: &WorkspaceGroupDto) -> String {
    let member_user_ids = group
        .member_user_ids()
        .iter()
        .map(|user_id| format!("\"{}\"", escape_json(user_id)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"groupId\":\"{}\",\"displayName\":\"{}\",\"memberUserIds\":[{}]}}",
        escape_json(group.group_id()),
        escape_json(group.name()),
        member_user_ids
    )
}

fn render_added_group_member(output: &AddUserToGroupOutput) -> String {
    render_group_membership(output.group_id(), output.user_id(), output.result())
}

fn render_removed_group_member(output: &RemoveUserFromGroupOutput) -> String {
    render_group_membership(output.group_id(), output.user_id(), output.result())
}

fn render_group_membership(group_id: &str, user_id: &str, result: GroupMembershipResult) -> String {
    format!(
        "{{\"groupId\":\"{}\",\"userId\":\"{}\",\"result\":\"{}\"}}",
        escape_json(group_id),
        escape_json(user_id),
        group_membership_result(result)
    )
}

fn group_membership_result(result: GroupMembershipResult) -> &'static str {
    match result {
        GroupMembershipResult::Added => "Added",
        GroupMembershipResult::AlreadyMember => "AlreadyMember",
        GroupMembershipResult::Removed => "Removed",
    }
}

fn render_role_assignment_list(output: &ListWorkspaceRoleAssignmentsOutput) -> String {
    let assignments = output
        .assignments()
        .iter()
        .map(render_role_assignment)
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"assignments\":[{}]}}", assignments)
}

fn render_role_assignment(assignment: &RoleAssignmentDto) -> String {
    format!(
        "{{\"assignmentId\":\"{}\",\"subjectType\":\"{}\",\"subjectId\":\"{}\",\"role\":\"{}\"}}",
        escape_json(assignment.assignment_id()),
        assignment.subject_type(),
        escape_json(assignment.subject_id()),
        assignment.role()
    )
}

fn render_assigned_role(output: &AssignRoleOutput) -> String {
    format!(
        "{{\"assignmentId\":\"{}\",\"subjectType\":\"{}\",\"subjectId\":\"{}\",\"role\":\"{}\"}}",
        escape_json(output.assignment_id()),
        output.subject_type(),
        escape_json(output.subject_id()),
        output.role()
    )
}

fn render_revoked_role(output: &RevokeRoleOutput) -> String {
    format!(
        "{{\"assignmentId\":\"{}\",\"result\":\"Revoked\"}}",
        escape_json(output.assignment_id())
    )
}

fn render_document_sharing(
    workspace_id: &str,
    document_id: &str,
    output: &ListEffectivePermissionsOutput,
) -> String {
    let effective_permissions = output
        .allowed_permission_names()
        .iter()
        .map(|permission| format!("\"{}\"", permission))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"workspaceId\":\"{}\",\"documentId\":\"{}\",\"entries\":[],\"effectivePermissions\":[{}]}}",
        escape_json(workspace_id),
        escape_json(document_id),
        effective_permissions
    )
}

fn render_updated_document_sharing(
    workspace_id: &str,
    output: &ShareDocumentOutput,
    subject_kind: &str,
    subject_id: &str,
    permission: &str,
    effect: &str,
) -> String {
    format!(
        "{{\"workspaceId\":\"{}\",\"documentId\":\"{}\",\"entries\":[{{\"subject\":{{\"kind\":\"{}\",\"id\":\"{}\"}},\"permission\":\"{}\",\"effect\":\"{}\"}}],\"effectivePermissions\":[]}}",
        escape_json(workspace_id),
        escape_json(output.document_id()),
        escape_json(subject_kind),
        escape_json(subject_id),
        escape_json(permission),
        escape_json(effect)
    )
}

fn render_comment_thread_page(output: &ListDocumentCommentsOutput) -> String {
    let threads = output
        .threads()
        .iter()
        .map(|thread| {
            let comments = thread
                .comments()
                .iter()
                .map(|comment| {
                    format!(
                        "{{\"commentId\":\"{}\",\"authorUserId\":\"{}\",\"body\":\"{}\",\"createdAt\":\"1970-01-01T00:00:00Z\"}}",
                        escape_json(comment.id().as_str()),
                        escape_json(comment.author_user_id().as_str()),
                        escape_json(comment.body().as_str())
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            let anchor = thread.inline_anchor().map_or_else(String::new, |anchor| {
                let range = anchor.range();
                format!(
                    ",\"anchor\":{{\"versionId\":\"{}\",\"startOffset\":{},\"endOffset\":{},\"status\":\"valid\"}}",
                    escape_json(anchor.version_id().as_str()),
                    range.start_offset(),
                    range.end_offset()
                )
            });
            format!(
                "{{\"threadId\":\"{}\",\"documentId\":\"{}\",\"state\":\"{}\",\"comments\":[{}]{}}}",
                escape_json(thread.id().as_str()),
                escape_json(thread.document_id().as_str()),
                thread.state().as_str(),
                comments,
                anchor
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"threads\":[{}]}}", threads)
}

fn render_comment_thread_mutation(output: &CommentThreadOutput) -> String {
    let thread = output.thread();
    let comments = thread
        .comments()
        .iter()
        .map(|comment| {
            format!(
                "{{\"commentId\":\"{}\",\"authorUserId\":\"{}\",\"body\":\"{}\",\"createdAt\":\"1970-01-01T00:00:00Z\"}}",
                escape_json(comment.id().as_str()),
                escape_json(comment.author_user_id().as_str()),
                escape_json(comment.body().as_str())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let anchor = thread.inline_anchor().map_or_else(String::new, |anchor| {
        let range = anchor.range();
        format!(
            ",\"anchor\":{{\"versionId\":\"{}\",\"startOffset\":{},\"endOffset\":{},\"status\":\"valid\"}}",
            escape_json(anchor.version_id().as_str()),
            range.start_offset(),
            range.end_offset()
        )
    });
    format!(
        "{{\"thread\":{{\"threadId\":\"{}\",\"documentId\":\"{}\",\"state\":\"{}\",\"comments\":[{}]{}}}}}",
        escape_json(thread.id().as_str()),
        escape_json(thread.document_id().as_str()),
        thread.state().as_str(),
        comments,
        anchor
    )
}

fn render_review_workflow(output: &ReviewWorkflowOutput) -> String {
    let review_request_id = output.review_request_id().map_or_else(
        || "null".to_string(),
        |review_request_id| format!("\"{}\"", escape_json(review_request_id)),
    );
    format!(
        "{{\"documentId\":\"{}\",\"reviewRequestId\":{},\"previousState\":\"{}\",\"nextState\":\"{}\",\"eventName\":\"{}\"}}",
        escape_json(output.document_id()),
        review_request_id,
        output.previous_state_name(),
        output.next_state_name(),
        output.product_log_event_name()
    )
}

fn render_review_request_list(output: &ListReviewRequestsOutput) -> String {
    let requests = output
        .requests()
        .iter()
        .map(|request| {
            format!(
                "{{\"reviewRequestId\":\"{}\",\"documentId\":\"{}\",\"requestedBy\":\"{}\",\"status\":\"{}\"}}",
                escape_json(request.review_request_id()),
                escape_json(request.document_id()),
                escape_json(request.requested_by()),
                request.status().as_str()
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"requestCount\":{},\"requests\":[{}]}}",
        output.requests().len(),
        requests
    )
}

fn render_document_lock(output: &DocumentLockOutput) -> String {
    let lock = output.lock().map_or_else(
        || "null".to_string(),
        |lock| {
            format!(
                "{{\"lockId\":\"{}\",\"documentId\":\"{}\",\"ownerUserId\":\"{}\",\"acquiredAtMillis\":{},\"expiresAtMillis\":{}}}",
                escape_json(lock.lock_id().as_str()),
                escape_json(lock.document_id().as_str()),
                escape_json(lock.owner_user_id().as_str()),
                lock.acquired_at().as_millis(),
                lock.expires_at().as_millis()
            )
        },
    );
    format!(
        "{{\"status\":\"{}\",\"lock\":{}}}",
        output.status().as_str(),
        lock
    )
}

fn render_audit_event_page(output: &ListAuditEventsOutput) -> String {
    let next_cursor = output.next_cursor().map_or_else(
        || "null".to_string(),
        |cursor| format!("\"{}\"", escape_json(cursor)),
    );
    let events = output
        .events()
        .iter()
        .map(render_audit_event_summary)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"eventCount\":{},\"nextCursor\":{},\"retentionDays\":{},\"events\":[{}]}}",
        output.events().len(),
        next_cursor,
        output.retention_days(),
        events
    )
}

fn render_audit_event_summary(event: &AuditEventSummary) -> String {
    let document_id = event.document_id().map_or_else(
        || "null".to_string(),
        |document_id| format!("\"{}\"", escape_json(document_id)),
    );
    let metadata = event
        .metadata()
        .iter()
        .map(|(key, value)| {
            format!(
                "{{\"key\":\"{}\",\"value\":\"{}\"}}",
                escape_json(key),
                escape_json(value)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"eventId\":\"{}\",\"workspaceId\":\"{}\",\"actorType\":\"{}\",\"actorId\":\"{}\",\"action\":\"{}\",\"targetType\":\"{}\",\"targetId\":\"{}\",\"documentId\":{},\"occurredAtMillis\":{},\"metadata\":[{}]}}",
        escape_json(event.event_id()),
        escape_json(event.workspace_id()),
        event.actor_type(),
        escape_json(event.actor_id()),
        event.action(),
        event.target_type(),
        escape_json(event.target_id()),
        document_id,
        event.occurred_at_millis(),
        metadata
    )
}

fn render_backup_job(output: &BackupJobOutput) -> String {
    let error_code = output.error_code().map_or_else(
        || "null".to_string(),
        |error_code| format!("\"{}\"", escape_json(error_code)),
    );
    format!(
        "{{\"jobId\":\"{}\",\"workspaceId\":\"{}\",\"operation\":\"{}\",\"state\":\"{}\",\"retryCount\":{},\"progress\":{{\"completedUnits\":{},\"totalUnits\":{}}},\"errorCode\":{}}}",
        escape_json(output.job_id()),
        escape_json(output.workspace_id()),
        output.operation().as_str(),
        output.state().as_str(),
        output.retry_count(),
        output.progress_completed_units(),
        output.progress_total_units(),
        error_code
    )
}

fn field_debug_status(status: FieldDebugSessionOutputStatus) -> &'static str {
    match status {
        FieldDebugSessionOutputStatus::Requested => "requested",
        FieldDebugSessionOutputStatus::Approved => "approved",
        FieldDebugSessionOutputStatus::Denied => "denied",
        FieldDebugSessionOutputStatus::Active => "active",
        FieldDebugSessionOutputStatus::Expired => "expired",
        FieldDebugSessionOutputStatus::Revoked => "revoked",
    }
}

fn graph_node_kind(kind: GraphNodeKind) -> &'static str {
    match kind {
        GraphNodeKind::Document => "document",
        GraphNodeKind::UnresolvedLink => "unresolved_link",
        GraphNodeKind::Attachment => "attachment",
        GraphNodeKind::ExternalLink => "external_link",
    }
}

fn graph_edge_kind(kind: GraphEdgeKind) -> &'static str {
    match kind {
        GraphEdgeKind::DocumentLink => "document_link",
        GraphEdgeKind::AttachmentReference => "attachment_reference",
        GraphEdgeKind::ExternalReference => "external_reference",
        GraphEdgeKind::CanvasRelation => "canvas_relation",
    }
}

fn graph_projection_status(status: GraphProjectionStatus) -> &'static str {
    match status {
        GraphProjectionStatus::Clean => "clean",
        GraphProjectionStatus::ReindexRequested => "reindex_requested",
        GraphProjectionStatus::Reindexing => "reindexing",
        GraphProjectionStatus::Degraded => "degraded",
    }
}

fn accessible_document_error_output(error: GetAccessibleDocumentError) -> UsecaseOutputDto {
    let status = match error {
        GetAccessibleDocumentError::InvalidInput => 400,
        GetAccessibleDocumentError::NotFound => 404,
        GetAccessibleDocumentError::IndexStale => 409,
        GetAccessibleDocumentError::StorageUnavailable => 503,
    };
    error_output(status, error.code())
}

fn document_history_error_output(error: GetDocumentHistoryError) -> UsecaseOutputDto {
    let status = match error {
        GetDocumentHistoryError::InvalidInput => 400,
        GetDocumentHistoryError::StorageUnavailable => 503,
    };
    error_output(status, error.code())
}

fn update_document_error_output(error: UpdateDocumentError) -> UsecaseOutputDto {
    let status = match error {
        UpdateDocumentError::InvalidDocumentInput => 400,
        UpdateDocumentError::NotFound => 404,
        UpdateDocumentError::VersionAlreadyExists => 409,
        UpdateDocumentError::StorageUnavailable => 503,
    };
    error_output(status, error.code())
}

fn accessible_search_error_output(error: SearchAccessibleDocumentsError) -> UsecaseOutputDto {
    let status = match error {
        SearchAccessibleDocumentsError::InvalidInput => 400,
        SearchAccessibleDocumentsError::IndexStale => 409,
        SearchAccessibleDocumentsError::StorageUnavailable => 503,
    };
    error_output(status, error.code())
}

fn graph_error_output(error: PermissionAwareGraphError) -> UsecaseOutputDto {
    let status = match error {
        PermissionAwareGraphError::InvalidInput => 400,
        PermissionAwareGraphError::ProjectionNotFound => 404,
        PermissionAwareGraphError::CenterDocumentDenied => 403,
        PermissionAwareGraphError::PermissionUnavailable
        | PermissionAwareGraphError::ProjectionUnavailable => 503,
        PermissionAwareGraphError::CorruptedProjection => 409,
    };
    error_output(status, error.code())
}

fn field_debug_error_output(error: FieldDebugUsecaseError) -> UsecaseOutputDto {
    let status = match error {
        FieldDebugUsecaseError::InvalidInput
        | FieldDebugUsecaseError::MissingScope
        | FieldDebugUsecaseError::MissingTtl
        | FieldDebugUsecaseError::TtlExceedsPolicy
        | FieldDebugUsecaseError::SensitiveField
        | FieldDebugUsecaseError::NotExpired => 400,
        FieldDebugUsecaseError::Unauthorized => 403,
        FieldDebugUsecaseError::SessionNotFound => 404,
        FieldDebugUsecaseError::InactiveSession | FieldDebugUsecaseError::ExpiredSession => 409,
        FieldDebugUsecaseError::StoreUnavailable => 503,
        FieldDebugUsecaseError::Conflict => 409,
    };
    error_output(status, error.code())
}

fn auth_error_output(error: AuthError) -> UsecaseOutputDto {
    let status = match error {
        AuthError::InvalidCredential
        | AuthError::SessionMissing
        | AuthError::SessionExpired
        | AuthError::SessionRevoked
        | AuthError::InvalidToken => 401,
        AuthError::UserNotActive => 403,
        AuthError::InvalidSession => 400,
        AuthError::SessionStoreUnavailable | AuthError::TokenUnavailable => 503,
    };
    error_output(status, error.code())
}

fn list_users_error_output(error: ListUsersError) -> UsecaseOutputDto {
    error_output(503, error.code())
}

fn list_workspace_groups_error_output(error: ListWorkspaceGroupsError) -> UsecaseOutputDto {
    let status = match error {
        ListWorkspaceGroupsError::InvalidInput => 400,
        ListWorkspaceGroupsError::StorageUnavailable => 503,
    };
    error_output(status, error.code())
}

fn add_group_member_error_output(error: AddUserToGroupError) -> UsecaseOutputDto {
    let status = match error {
        AddUserToGroupError::InvalidInput => 400,
        AddUserToGroupError::GroupNotFound | AddUserToGroupError::UserNotFound => 404,
        AddUserToGroupError::StorageUnavailable => 503,
    };
    error_output(status, error.code())
}

fn remove_group_member_error_output(error: RemoveUserFromGroupError) -> UsecaseOutputDto {
    let status = match error {
        RemoveUserFromGroupError::InvalidInput => 400,
        RemoveUserFromGroupError::GroupNotFound
        | RemoveUserFromGroupError::UserNotFound
        | RemoveUserFromGroupError::MembershipNotFound => 404,
        RemoveUserFromGroupError::StorageUnavailable => 503,
    };
    error_output(status, error.code())
}

fn list_role_assignments_error_output(
    error: ListWorkspaceRoleAssignmentsError,
) -> UsecaseOutputDto {
    let status = match error {
        ListWorkspaceRoleAssignmentsError::InvalidInput => 400,
        ListWorkspaceRoleAssignmentsError::StorageUnavailable => 503,
    };
    error_output(status, error.code())
}

fn assign_role_error_output(error: AssignRoleError) -> UsecaseOutputDto {
    let status = match error {
        AssignRoleError::InvalidInput => 400,
        AssignRoleError::Unauthorized => 403,
        AssignRoleError::StorageUnavailable => 503,
    };
    error_output(status, error.code())
}

fn revoke_role_error_output(error: RevokeRoleError) -> UsecaseOutputDto {
    let status = match error {
        RevokeRoleError::InvalidInput => 400,
        RevokeRoleError::Unauthorized => 403,
        RevokeRoleError::RoleAssignmentNotFound => 404,
        RevokeRoleError::StorageUnavailable => 503,
    };
    error_output(status, error.code())
}

fn list_effective_permissions_error_output(
    error: ListEffectivePermissionsError,
) -> UsecaseOutputDto {
    let status = match error {
        ListEffectivePermissionsError::InvalidInput => 400,
        ListEffectivePermissionsError::StorageUnavailable => 503,
    };
    error_output(status, error.code())
}

fn share_document_error_output(error: ShareDocumentError) -> UsecaseOutputDto {
    let status = match error {
        ShareDocumentError::InvalidInput => 400,
        ShareDocumentError::Unauthorized => 403,
        ShareDocumentError::StorageUnavailable => 503,
    };
    error_output(status, error.code())
}

fn comment_usecase_error_output(error: CommentUsecaseError) -> UsecaseOutputDto {
    let status = match error {
        CommentUsecaseError::InvalidInput
        | CommentUsecaseError::InvalidTransition
        | CommentUsecaseError::BodyTooLarge
        | CommentUsecaseError::InvalidAnchorRange
        | CommentUsecaseError::StaleAnchor
        | CommentUsecaseError::DocumentVersionMissing => 400,
        CommentUsecaseError::Unauthorized => 403,
        CommentUsecaseError::ThreadNotFound => 404,
        CommentUsecaseError::StorageUnavailable => 503,
        CommentUsecaseError::Conflict => 409,
    };
    error_output(status, error.code())
}

fn document_lock_error_output(error: DocumentLockUsecaseError) -> UsecaseOutputDto {
    let status = match error {
        DocumentLockUsecaseError::InvalidInput => 400,
        DocumentLockUsecaseError::Unauthorized => 403,
        DocumentLockUsecaseError::LockNotFound => 404,
        DocumentLockUsecaseError::AlreadyLocked
        | DocumentLockUsecaseError::NotOwner
        | DocumentLockUsecaseError::LockExpired
        | DocumentLockUsecaseError::LockNotExpired
        | DocumentLockUsecaseError::Conflict => 409,
        DocumentLockUsecaseError::StorageUnavailable => 503,
    };
    error_output(status, error.code())
}

fn audit_error_output(error: AuditUsecaseError) -> UsecaseOutputDto {
    let status = match error {
        AuditUsecaseError::InvalidInput
        | AuditUsecaseError::InvalidMetadata
        | AuditUsecaseError::InvalidCursor => 400,
        AuditUsecaseError::Unauthorized => 403,
        AuditUsecaseError::StoreUnavailable => 503,
        AuditUsecaseError::Conflict => 409,
    };
    error_output(status, error.code())
}

fn backup_job_error_output(error: BackupJobUsecaseError) -> UsecaseOutputDto {
    let status = match error {
        BackupJobUsecaseError::InvalidInput => 400,
        BackupJobUsecaseError::JobNotFound => 404,
        BackupJobUsecaseError::StorageUnavailable | BackupJobUsecaseError::AuditUnavailable => 503,
        BackupJobUsecaseError::Conflict => 409,
    };
    error_output(status, error.code())
}

fn review_workflow_error_output(error: ReviewWorkflowUsecaseError) -> UsecaseOutputDto {
    let status = match error {
        ReviewWorkflowUsecaseError::InvalidInput
        | ReviewWorkflowUsecaseError::InvalidWorkflowTransition => 400,
        ReviewWorkflowUsecaseError::Unauthorized
        | ReviewWorkflowUsecaseError::ReviewPermissionRequired
        | ReviewWorkflowUsecaseError::PublishPermissionRequired => 403,
        ReviewWorkflowUsecaseError::ReviewRequestNotFound => 404,
        ReviewWorkflowUsecaseError::Conflict => 409,
        ReviewWorkflowUsecaseError::StorageUnavailable
        | ReviewWorkflowUsecaseError::SideEffectUnavailable => 503,
    };
    error_output(status, error.code())
}

fn malformed_request() -> UsecaseOutputDto {
    error_output(400, "SERVER_MALFORMED_REQUEST")
}

fn error_output(status_code: u16, error_code: &str) -> UsecaseOutputDto {
    UsecaseOutputDto::new(
        status_code,
        &format!("{{\"errorCode\":\"{}\"}}", escape_json(error_code)),
    )
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}
