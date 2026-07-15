use std::cell::RefCell;

use cabinet_core::server_config::ServerConfigInput;
use cabinet_server::adapter::{
    BoundaryMapper, HttpMethod, ServerRequest, ServerUsecaseTarget, UsecaseOutputDto,
    handle_request,
};
use cabinet_server::composition::{ServerFramework, build_server_composition};
use cabinet_server::errors::ServerErrorCode;

#[test]
fn server_package_exposes_boundary_layer() {
    assert_eq!(cabinet_server::layer_name(), "server");
}

#[test]
fn composition_selects_axum_tokio_and_registers_health_route() {
    let composition = build_server_composition(default_config());

    assert_eq!(composition.framework(), ServerFramework::AxumTokio);
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Get, "/api/health")
    );
    assert!(composition.routes().contains(
        HttpMethod::Post,
        "/api/documents/{documentId}/review-requests"
    ));
    assert!(composition.routes().contains(
        HttpMethod::Post,
        "/api/review-requests/{reviewRequestId}/approve"
    ));
    assert!(composition.routes().contains(
        HttpMethod::Post,
        "/api/review-requests/{reviewRequestId}/reject"
    ));
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Post, "/api/documents/{documentId}/publish")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Get, "/api/review-requests")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Post, "/api/documents/{documentId}/locks")
    );
    assert!(composition.routes().contains(
        HttpMethod::Delete,
        "/api/documents/{documentId}/locks/current"
    ));
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Get, "/api/documents/{documentId}/locks/current")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Get, "/api/audit-events")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Post, "/api/field-debug-sessions")
    );
    assert!(composition.routes().contains(
        HttpMethod::Post,
        "/api/field-debug-sessions/{sessionId}/approve"
    ));
    assert!(composition.routes().contains(
        HttpMethod::Post,
        "/api/field-debug-sessions/{sessionId}/expire"
    ));
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Post, "/api/backups")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Get, "/api/backups/{jobId}")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Post, "/api/backups/{jobId}/restore")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Post, "/api/exports")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Get, "/api/exports/{jobId}")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Post, "/api/auth/login")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Post, "/api/auth/session/validate")
    );
    assert!(composition.routes().contains(HttpMethod::Get, "/api/users"));
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Get, "/api/workspaces/{workspaceId}/groups")
    );
    assert!(composition.routes().contains(
        HttpMethod::Post,
        "/api/workspaces/{workspaceId}/groups/{groupId}/members"
    ));
    assert!(composition.routes().contains(
        HttpMethod::Delete,
        "/api/workspaces/{workspaceId}/groups/{groupId}/members/{userId}"
    ));
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Get, "/api/workspaces/{workspaceId}/roles")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Post, "/api/workspaces/{workspaceId}/roles")
    );
    assert!(composition.routes().contains(
        HttpMethod::Delete,
        "/api/workspaces/{workspaceId}/roles/{assignmentId}"
    ));
    assert!(composition.routes().contains(
        HttpMethod::Get,
        "/api/workspaces/{workspaceId}/documents/{documentId}/current"
    ));
    assert!(composition.routes().contains(
        HttpMethod::Put,
        "/api/workspaces/{workspaceId}/documents/{documentId}/current"
    ));
    assert!(composition.routes().contains(
        HttpMethod::Get,
        "/api/workspaces/{workspaceId}/documents/{documentId}/history"
    ));
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Get, "/api/workspaces/{workspaceId}/search")
    );
    assert!(composition.routes().contains(
        HttpMethod::Get,
        "/api/workspaces/{workspaceId}/documents/{documentId}/graph"
    ));
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Get, "/api/documents/{documentId}/sharing")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Put, "/api/documents/{documentId}/sharing")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Get, "/api/documents/{documentId}/comments")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Post, "/api/documents/{documentId}/comments")
    );
    assert!(composition.routes().contains(
        HttpMethod::Post,
        "/api/documents/{documentId}/inline-comments"
    ));
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Post, "/api/comments/{commentId}/resolve")
    );
    assert!(
        composition
            .routes()
            .contains(HttpMethod::Post, "/api/comments/{commentId}/reopen")
    );
    assert!(composition.routes().contains(
        HttpMethod::Post,
        "/api/workspaces/{workspaceId}/documents/{documentId}/collaboration/join"
    ));
    assert!(composition.routes().contains(
        HttpMethod::Post,
        "/api/workspaces/{workspaceId}/documents/{documentId}/collaboration/operations"
    ));
    assert!(composition.routes().contains(
        HttpMethod::Post,
        "/api/workspaces/{workspaceId}/documents/{documentId}/collaboration/presence"
    ));
    assert!(composition.routes().contains(
        HttpMethod::Post,
        "/api/workspaces/{workspaceId}/documents/{documentId}/collaboration/replay"
    ));
}

#[test]
fn handler_maps_request_to_usecase_dto_without_framework_request() {
    let composition = build_server_composition(default_config());
    let target = CapturingTarget::new(UsecaseOutputDto::new(200, "{\"status\":\"ok\"}"));

    let response = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(HttpMethod::Get, "/api/health", None),
    )
    .expect("health route should map to fake usecase target");

    assert_eq!(response.status_code(), 200);
    assert_eq!(response.body(), "{\"status\":\"ok\"}");
    assert_eq!(target.seen_inputs().len(), 1);
    assert_eq!(target.seen_inputs()[0].route_id(), "health.check");
    assert_eq!(target.seen_inputs()[0].body(), None);
}

#[test]
fn collaboration_realtime_route_maps_to_usecase_dto_without_transport_runtime() {
    let composition = build_server_composition(default_config());
    let mapper = BoundaryMapper::new(composition.routes());

    let input = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/workspaces/workspace-1/documents/doc-1/collaboration/join",
            Some("{\"sessionId\":\"session-1\",\"actorUserId\":\"user-1\"}"),
        ))
        .expect("collaboration realtime route should map to DTO");

    assert_eq!(input.route_id(), "collaboration.join_document_room");
    assert_eq!(input.path_param("workspaceId"), Some("workspace-1"));
    assert_eq!(input.path_param("documentId"), Some("doc-1"));
    assert_eq!(
        input.body(),
        Some("{\"sessionId\":\"session-1\",\"actorUserId\":\"user-1\"}")
    );
}

#[test]
fn malformed_route_does_not_reach_usecase_target() {
    let composition = build_server_composition(default_config());
    let target = CapturingTarget::new(UsecaseOutputDto::new(200, "{}"));

    let error = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(HttpMethod::Post, "/api/health", None),
    )
    .expect_err("wrong method must fail at server boundary");

    assert_eq!(error.code(), ServerErrorCode::MethodNotAllowed);
    assert!(target.seen_inputs().is_empty());
}

#[test]
fn mapper_rejects_unknown_route_with_stable_error_code() {
    let composition = build_server_composition(default_config());
    let mapper = BoundaryMapper::new(composition.routes());

    let error = mapper
        .request_to_usecase(ServerRequest::new(HttpMethod::Get, "/api/missing", None))
        .expect_err("unknown route should not enter usecase layer");

    assert_eq!(error.code(), ServerErrorCode::RouteNotFound);
}

#[test]
fn review_publish_routes_extract_path_params_without_business_rules() {
    let composition = build_server_composition(default_config());
    let mapper = BoundaryMapper::new(composition.routes());

    let request_review = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-1/review-requests",
            Some("{\"reviewRequestId\":\"review-1\"}"),
        ))
        .expect("review request route");
    assert_eq!(request_review.route_id(), "review.request_document");
    assert_eq!(request_review.path_param("documentId"), Some("doc-1"));
    assert_eq!(
        request_review.body(),
        Some("{\"reviewRequestId\":\"review-1\"}")
    );

    let approve = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/review-requests/review-1/approve",
            None,
        ))
        .expect("approve route");
    assert_eq!(approve.route_id(), "review.approve_document");
    assert_eq!(approve.path_param("reviewRequestId"), Some("review-1"));

    let publish = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-1/publish",
            None,
        ))
        .expect("publish route");
    assert_eq!(publish.route_id(), "review.publish_document");
    assert_eq!(publish.path_param("documentId"), Some("doc-1"));
}

#[test]
fn document_lock_routes_extract_path_params_without_business_rules() {
    let composition = build_server_composition(default_config());
    let mapper = BoundaryMapper::new(composition.routes());

    let lock = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-1/locks",
            Some("{\"lockId\":\"lock-1\"}"),
        ))
        .expect("lock route");
    assert_eq!(lock.route_id(), "document_lock.lock");
    assert_eq!(lock.path_param("documentId"), Some("doc-1"));
    assert_eq!(lock.body(), Some("{\"lockId\":\"lock-1\"}"));

    let unlock = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Delete,
            "/api/documents/doc-1/locks/current",
            None,
        ))
        .expect("unlock route");
    assert_eq!(unlock.route_id(), "document_lock.unlock");
    assert_eq!(unlock.path_param("documentId"), Some("doc-1"));

    let get = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Get,
            "/api/documents/doc-1/locks/current",
            None,
        ))
        .expect("get lock route");
    assert_eq!(get.route_id(), "document_lock.get");
    assert_eq!(get.path_param("documentId"), Some("doc-1"));
}

#[test]
fn audit_event_route_extracts_cursor_query_without_storage_rows() {
    let composition = build_server_composition(default_config());
    let mapper = BoundaryMapper::new(composition.routes());

    let audit_query = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Get,
            "/api/audit-events?scope=workspace&limit=50&cursor=2",
            None,
        ))
        .expect("audit event query route");

    assert_eq!(audit_query.route_id(), "audit.list_events");
    assert_eq!(audit_query.body(), None);
    assert!(audit_query.path_params().is_empty());
    assert_eq!(audit_query.query_param("scope"), Some("workspace"));
    assert_eq!(audit_query.query_param("limit"), Some("50"));
    assert_eq!(audit_query.query_param("cursor"), Some("2"));
}

#[test]
fn field_debug_routes_extract_session_id_and_body_without_log_policy_rules() {
    let composition = build_server_composition(default_config());
    let mapper = BoundaryMapper::new(composition.routes());

    let request = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/field-debug-sessions",
            Some("{\"scope\":\"workspace:workspace-1\",\"ttlSeconds\":300}"),
        ))
        .expect("field debug request route");
    assert_eq!(request.route_id(), "field_debug.request_session");
    assert!(request.path_params().is_empty());
    assert_eq!(
        request.body(),
        Some("{\"scope\":\"workspace:workspace-1\",\"ttlSeconds\":300}")
    );

    let approve = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/field-debug-sessions/field-debug-1/approve",
            None,
        ))
        .expect("field debug approve route");
    assert_eq!(approve.route_id(), "field_debug.approve_session");
    assert_eq!(approve.path_param("sessionId"), Some("field-debug-1"));

    let expire = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/field-debug-sessions/field-debug-1/expire",
            None,
        ))
        .expect("field debug expire route");
    assert_eq!(expire.route_id(), "field_debug.expire_session");
    assert_eq!(expire.path_param("sessionId"), Some("field-debug-1"));
}

#[test]
fn backup_export_routes_extract_job_params_without_business_rules() {
    let composition = build_server_composition(default_config());
    let mapper = BoundaryMapper::new(composition.routes());

    let create_backup = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/backups",
            Some("{\"jobId\":\"backup-job-1\"}"),
        ))
        .expect("create backup route");
    assert_eq!(create_backup.route_id(), "backup.create");
    assert!(create_backup.path_params().is_empty());
    assert_eq!(create_backup.body(), Some("{\"jobId\":\"backup-job-1\"}"));

    let backup_status = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Get,
            "/api/backups/backup-job-1",
            None,
        ))
        .expect("backup status route");
    assert_eq!(backup_status.route_id(), "backup.get_status");
    assert_eq!(backup_status.path_param("jobId"), Some("backup-job-1"));

    let restore = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/backups/backup-job-1/restore",
            Some("{\"jobId\":\"restore-job-1\"}"),
        ))
        .expect("restore route");
    assert_eq!(restore.route_id(), "backup.restore");
    assert_eq!(restore.path_param("jobId"), Some("backup-job-1"));
    assert_eq!(restore.body(), Some("{\"jobId\":\"restore-job-1\"}"));

    let create_export = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/exports",
            Some("{\"jobId\":\"export-job-1\"}"),
        ))
        .expect("create export route");
    assert_eq!(create_export.route_id(), "export.create_workspace");
    assert_eq!(create_export.body(), Some("{\"jobId\":\"export-job-1\"}"));

    let export_status = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Get,
            "/api/exports/export-job-1",
            None,
        ))
        .expect("export status route");
    assert_eq!(export_status.route_id(), "export.get_status");
    assert_eq!(export_status.path_param("jobId"), Some("export-job-1"));
}

#[test]
fn web_admin_routes_extract_auth_membership_and_role_params_without_rules() {
    let composition = build_server_composition(default_config());
    let mapper = BoundaryMapper::new(composition.routes());

    let login = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/auth/login",
            Some("{\"login\":\"admin\"}"),
        ))
        .expect("login route");
    assert_eq!(login.route_id(), "auth.login");
    assert!(login.path_params().is_empty());
    assert_eq!(login.body(), Some("{\"login\":\"admin\"}"));

    let validate_session = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/auth/session/validate",
            Some("{\"token\":\"redacted\"}"),
        ))
        .expect("session validation route");
    assert_eq!(validate_session.route_id(), "auth.validate_session");
    assert!(validate_session.path_params().is_empty());

    let users = mapper
        .request_to_usecase(ServerRequest::new(HttpMethod::Get, "/api/users", None))
        .expect("user list route");
    assert_eq!(users.route_id(), "user.list");
    assert!(users.path_params().is_empty());

    let groups = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Get,
            "/api/workspaces/workspace-1/groups",
            None,
        ))
        .expect("group list route");
    assert_eq!(groups.route_id(), "group.list");
    assert_eq!(groups.path_param("workspaceId"), Some("workspace-1"));

    let add_member = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/workspaces/workspace-1/groups/group-1/members",
            Some("{\"userId\":\"user-1\"}"),
        ))
        .expect("add member route");
    assert_eq!(add_member.route_id(), "group.add_member");
    assert_eq!(add_member.path_param("workspaceId"), Some("workspace-1"));
    assert_eq!(add_member.path_param("groupId"), Some("group-1"));

    let remove_member = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Delete,
            "/api/workspaces/workspace-1/groups/group-1/members/user-1",
            None,
        ))
        .expect("remove member route");
    assert_eq!(remove_member.route_id(), "group.remove_member");
    assert_eq!(remove_member.path_param("workspaceId"), Some("workspace-1"));
    assert_eq!(remove_member.path_param("groupId"), Some("group-1"));
    assert_eq!(remove_member.path_param("userId"), Some("user-1"));

    let list_roles = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Get,
            "/api/workspaces/workspace-1/roles",
            None,
        ))
        .expect("list roles route");
    assert_eq!(list_roles.route_id(), "role.list_assignments");
    assert_eq!(list_roles.path_param("workspaceId"), Some("workspace-1"));

    let assign_role = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/workspaces/workspace-1/roles",
            Some("{\"subject\":{\"kind\":\"user\",\"id\":\"user-1\"},\"role\":\"editor\"}"),
        ))
        .expect("assign role route");
    assert_eq!(assign_role.route_id(), "role.assign");
    assert_eq!(assign_role.path_param("workspaceId"), Some("workspace-1"));

    let revoke_role = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Delete,
            "/api/workspaces/workspace-1/roles/role-assignment-1",
            None,
        ))
        .expect("revoke role route");
    assert_eq!(revoke_role.route_id(), "role.revoke");
    assert_eq!(revoke_role.path_param("workspaceId"), Some("workspace-1"));
    assert_eq!(
        revoke_role.path_param("assignmentId"),
        Some("role-assignment-1")
    );
}

#[test]
fn collaboration_routes_extract_query_path_and_body_without_domain_rules() {
    let composition = build_server_composition(default_config());
    let mapper = BoundaryMapper::new(composition.routes());

    let current_document = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Get,
            "/api/workspaces/workspace-1/documents/doc-1/current",
            None,
        ))
        .expect("accessible current document route");
    assert_eq!(
        current_document.route_id(),
        "document.get_accessible_current"
    );
    assert_eq!(
        current_document.path_param("workspaceId"),
        Some("workspace-1")
    );
    assert_eq!(current_document.path_param("documentId"), Some("doc-1"));

    let history = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Get,
            "/api/workspaces/workspace-1/documents/doc-1/history?limit=20&cursor=cursor-1",
            None,
        ))
        .expect("accessible document history route");
    assert_eq!(history.route_id(), "document.get_accessible_history");
    assert_eq!(history.path_param("workspaceId"), Some("workspace-1"));
    assert_eq!(history.path_param("documentId"), Some("doc-1"));
    assert_eq!(history.query_param("limit"), Some("20"));
    assert_eq!(history.query_param("cursor"), Some("cursor-1"));

    let search = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Get,
            "/api/workspaces/workspace-1/search?text=query&limit=20",
            None,
        ))
        .expect("accessible search route");
    assert_eq!(search.route_id(), "search.accessible");
    assert_eq!(search.path_param("workspaceId"), Some("workspace-1"));
    assert_eq!(search.query_param("text"), Some("query"));
    assert_eq!(search.query_param("limit"), Some("20"));

    let graph = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Get,
            "/api/workspaces/workspace-1/documents/doc-1/graph",
            None,
        ))
        .expect("local graph route");
    assert_eq!(graph.route_id(), "graph.get_local");
    assert_eq!(graph.path_param("workspaceId"), Some("workspace-1"));
    assert_eq!(graph.path_param("documentId"), Some("doc-1"));

    let get_sharing = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Get,
            "/api/documents/doc-1/sharing",
            None,
        ))
        .expect("get sharing route");
    assert_eq!(get_sharing.route_id(), "sharing.get_document");
    assert_eq!(get_sharing.path_param("documentId"), Some("doc-1"));

    let update_sharing = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Put,
            "/api/documents/doc-1/sharing",
            Some("{\"subject\":{\"kind\":\"user\",\"id\":\"user-1\"},\"permission\":\"read\",\"effect\":\"allow\"}"),
        ))
        .expect("update sharing route");
    assert_eq!(update_sharing.route_id(), "sharing.update_document");
    assert_eq!(update_sharing.path_param("documentId"), Some("doc-1"));
    assert_eq!(
        update_sharing.body(),
        Some(
            "{\"subject\":{\"kind\":\"user\",\"id\":\"user-1\"},\"permission\":\"read\",\"effect\":\"allow\"}"
        )
    );

    let comments = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Get,
            "/api/documents/doc-1/comments",
            None,
        ))
        .expect("list comments route");
    assert_eq!(comments.route_id(), "comment.list");
    assert_eq!(comments.path_param("documentId"), Some("doc-1"));

    let add_comment = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-1/comments",
            Some("{\"threadId\":\"thread-1\"}"),
        ))
        .expect("add comment route");
    assert_eq!(add_comment.route_id(), "comment.add");
    assert_eq!(add_comment.path_param("documentId"), Some("doc-1"));

    let inline_comment = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/documents/doc-1/inline-comments",
            Some("{\"anchor\":{\"versionId\":\"version-1\",\"startOffset\":1,\"endOffset\":3}}"),
        ))
        .expect("inline comment route");
    assert_eq!(inline_comment.route_id(), "comment.add_inline");
    assert_eq!(inline_comment.path_param("documentId"), Some("doc-1"));

    let resolve_comment = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/comments/thread-1/resolve",
            None,
        ))
        .expect("resolve comment route");
    assert_eq!(resolve_comment.route_id(), "comment.resolve");
    assert_eq!(resolve_comment.path_param("commentId"), Some("thread-1"));

    let reopen_comment = mapper
        .request_to_usecase(ServerRequest::new(
            HttpMethod::Post,
            "/api/comments/thread-1/reopen",
            None,
        ))
        .expect("reopen comment route");
    assert_eq!(reopen_comment.route_id(), "comment.reopen");
    assert_eq!(reopen_comment.path_param("commentId"), Some("thread-1"));
}

struct CapturingTarget {
    output: UsecaseOutputDto,
    seen_inputs: RefCell<Vec<cabinet_server::adapter::UsecaseInputDto>>,
}

impl CapturingTarget {
    fn new(output: UsecaseOutputDto) -> Self {
        Self {
            output,
            seen_inputs: RefCell::new(Vec::new()),
        }
    }

    fn seen_inputs(&self) -> Vec<cabinet_server::adapter::UsecaseInputDto> {
        self.seen_inputs.borrow().clone()
    }
}

impl ServerUsecaseTarget for CapturingTarget {
    fn handle(
        &self,
        input: cabinet_server::adapter::UsecaseInputDto,
    ) -> Result<UsecaseOutputDto, cabinet_server::errors::ServerBoundaryError> {
        self.seen_inputs.borrow_mut().push(input);
        Ok(self.output.clone())
    }
}

fn default_config() -> cabinet_core::server_config::ServerConfig {
    ServerConfigInput::local_dev_defaults()
        .validate()
        .expect("valid default server config")
}
