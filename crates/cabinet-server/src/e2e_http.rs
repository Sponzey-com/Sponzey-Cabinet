use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use cabinet_core::server_config::ServerConfig;

const E2E_SESSION_TOKEN: &str = "e2e-session-token-should-not-log";

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedHttpRequest {
    method: String,
    target: String,
    headers: BTreeMap<String, String>,
    body: String,
}

impl ParsedHttpRequest {
    fn path(&self) -> &str {
        self.target
            .split_once('?')
            .map_or(self.target.as_str(), |(path, _)| path)
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    fn has_valid_session(&self) -> bool {
        self.header("authorization")
            .is_some_and(|value| value == format!("Bearer {E2E_SESSION_TOKEN}"))
            || self.body.contains(E2E_SESSION_TOKEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct E2eHttpState {
    lock_holder: Option<String>,
    field_debug_state: &'static str,
}

impl Default for E2eHttpState {
    fn default() -> Self {
        Self {
            lock_holder: None,
            field_debug_state: "Requested",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct E2eRouteResult {
    status_code: u16,
    body: &'static str,
    shutdown: bool,
}

impl E2eRouteResult {
    const fn json(status_code: u16, body: &'static str) -> Self {
        Self {
            status_code,
            body,
            shutdown: false,
        }
    }

    const fn shutdown() -> Self {
        Self {
            status_code: 200,
            body: r#"{"shutdown":"accepted"}"#,
            shutdown: true,
        }
    }
}

pub fn run_self_host_e2e_http_server(config: ServerConfig) -> std::io::Result<()> {
    let listener = TcpListener::bind(config.bind_address())?;
    println!("product_log_event=server.started");
    println!("self_host_e2e_http_ready=1");
    println!("bind_address={}", config.bind_address());

    let mut state = E2eHttpState::default();
    for stream in listener.incoming() {
        let mut stream = stream?;
        let route_result = match read_http_request(&mut stream) {
            Ok(Some(request)) => handle_e2e_http_request(&request, &mut state),
            Ok(None) => E2eRouteResult::json(400, r#"{"errorCode":"MALFORMED_REQUEST"}"#),
            Err(_) => E2eRouteResult::json(400, r#"{"errorCode":"MALFORMED_REQUEST"}"#),
        };
        write_http_response(&mut stream, route_result.status_code, route_result.body)?;
        if route_result.shutdown {
            break;
        }
    }

    println!("product_log_event=server.stopped");
    Ok(())
}

fn handle_e2e_http_request(
    request: &ParsedHttpRequest,
    state: &mut E2eHttpState,
) -> E2eRouteResult {
    if request.method == "POST" && request.path() == "/__shutdown" {
        return E2eRouteResult::shutdown();
    }

    if request.method == "GET" && request.path() == "/api/health" {
        return E2eRouteResult::json(200, r#"{"status":"healthy","mode":"self-host-e2e"}"#);
    }

    if request.method == "POST" && request.path() == "/api/auth/login" {
        println!("product_log_event=auth.login.succeeded user_id=actor-a");
        return E2eRouteResult::json(
            200,
            r#"{"userId":"actor-a","token":"e2e-session-token-should-not-log","sessionStatus":"active"}"#,
        );
    }

    if request.path().starts_with("/api/") && !request.has_valid_session() {
        return E2eRouteResult::json(401, r#"{"errorCode":"SESSION_EXPIRED"}"#);
    }

    let segments: Vec<&str> = request
        .path()
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    match (request.method.as_str(), segments.as_slice()) {
        ("POST", ["api", "auth", "session", "validate"]) => {
            E2eRouteResult::json(200, r#"{"userId":"actor-a","sessionStatus":"active"}"#)
        }
        ("GET", ["api", "users"]) => E2eRouteResult::json(
            200,
            r#"{"users":[{"userId":"actor-a","displayName":"Actor A"},{"userId":"actor-b","displayName":"Actor B"},{"userId":"reviewer-1","displayName":"Reviewer"}]}"#,
        ),
        ("GET", ["api", "workspaces", "workspace-1", "groups"]) => E2eRouteResult::json(
            200,
            r#"{"workspaceId":"workspace-1","groups":[{"groupId":"editors","displayName":"Editors","memberUserIds":["actor-a"]},{"groupId":"readers","displayName":"Readers","memberUserIds":["actor-b"]}]}"#,
        ),
        (
            "POST",
            [
                "api",
                "workspaces",
                "workspace-1",
                "groups",
                "editors",
                "members",
            ],
        ) => {
            println!("product_log_event=group.member_added workspace_id=workspace-1");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","groupId":"editors","memberUserIds":["actor-a","actor-b"]}"#,
            )
        }
        (
            "DELETE",
            [
                "api",
                "workspaces",
                "workspace-1",
                "groups",
                "editors",
                "members",
                "actor-b",
            ],
        ) => {
            println!("product_log_event=group.member_removed workspace_id=workspace-1");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","groupId":"editors","memberUserIds":["actor-a"]}"#,
            )
        }
        ("GET", ["api", "workspaces", "workspace-1", "roles"]) => E2eRouteResult::json(
            200,
            r#"{"workspaceId":"workspace-1","assignments":[{"assignmentId":"role-assignment-1","subjectId":"actor-a","role":"owner"}]}"#,
        ),
        ("POST", ["api", "workspaces", "workspace-1", "roles"]) => {
            println!("product_log_event=role.assigned workspace_id=workspace-1");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","assignmentId":"role-assignment-2","subjectId":"actor-b","role":"editor"}"#,
            )
        }
        (
            "DELETE",
            [
                "api",
                "workspaces",
                "workspace-1",
                "roles",
                "role-assignment-2",
            ],
        ) => {
            println!("product_log_event=role.revoked workspace_id=workspace-1");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","assignmentId":"role-assignment-2","revoked":true}"#,
            )
        }
        (
            "GET",
            [
                "api",
                "workspaces",
                "workspace-1",
                "documents",
                "doc-allowed",
                "current",
            ],
        ) => E2eRouteResult::json(
            200,
            r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","title":"Allowed Document","path":"docs/allowed.md","body":"E2E document body should not be logged","versionId":"version-3","permissionDecision":{"effect":"allow","reason":"document_acl","result":"allowed","reasonCode":"document_acl"}}"#,
        ),
        (
            "GET",
            [
                "api",
                "workspaces",
                "workspace-1",
                "documents",
                "doc-allowed",
                "history",
            ],
        ) => E2eRouteResult::json(
            200,
            r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","entries":[{"versionId":"version-1","summary":"Initial","author":"actor-a","createdAt":"2026-06-25T00:00:00Z"},{"versionId":"version-3","summary":"Reviewed","author":"actor-b","createdAt":"2026-06-26T00:00:00Z"}],"nextCursor":null}"#,
        ),
        (
            "GET",
            [
                "api",
                "workspaces",
                "workspace-1",
                "documents",
                "doc-denied",
                "current",
            ],
        ) => E2eRouteResult::json(403, r#"{"errorCode":"DOCUMENT_ACCESS_DENIED"}"#),
        (
            "GET",
            [
                "api",
                "workspaces",
                "workspace-1",
                "documents",
                "doc-allowed",
                "graph",
            ],
        ) => {
            println!(
                "product_log_event=graph.query.completed workspace_id=masked:workspace-1 node_count=2 edge_count=1 filtered_count=1"
            );
            E2eRouteResult::json(
                200,
                r#"{"centerDocumentId":"doc-allowed","status":"clean","nodes":[{"id":"doc-allowed","kind":"document"},{"id":"doc-visible","kind":"document"}],"edges":[{"id":"edge-visible","sourceId":"doc-allowed","targetId":"doc-visible","kind":"document_link"}],"stats":{"candidateCount":3,"filteredCount":1},"performance":{"targetP95Ms":300,"observedMs":12}}"#,
            )
        }
        (
            "PUT",
            [
                "api",
                "workspaces",
                "workspace-1",
                "documents",
                "doc-allowed",
                "current",
            ],
        ) => {
            println!("product_log_event=document.remote_save.completed document_id=doc-allowed");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","status":"saved-remote","versionId":"version-5"}"#,
            )
        }
        (
            "PUT",
            [
                "api",
                "workspaces",
                "workspace-1",
                "documents",
                "doc-denied",
                "current",
            ],
        ) => E2eRouteResult::json(403, r#"{"errorCode":"DOCUMENT_ACCESS_DENIED"}"#),
        ("GET", ["api", "workspaces", "workspace-1", "search"]) => E2eRouteResult::json(
            200,
            r#"{"queryName":"permission-aware-search","workspaceId":"workspace-1","text":"needle","query":"needle","results":[{"workspaceId":"workspace-1","documentId":"doc-allowed","title":"Allowed Document","path":"docs/allowed.md","snippet":"redacted","permissionDecision":{"effect":"allow","reason":"document_acl","result":"allowed","reasonCode":"document_acl"}}],"items":[{"workspaceId":"workspace-1","documentId":"doc-allowed","title":"Allowed Document","path":"docs/allowed.md","snippet":"redacted","permissionDecision":{"effect":"allow","reason":"document_acl","result":"allowed","reasonCode":"document_acl"}}],"permissionFilteredCount":0,"durationMs":12,"performance":{"targetP95Ms":300,"observedMs":12}}"#,
        ),
        ("GET", ["api", "documents", "doc-allowed", "sharing"]) => E2eRouteResult::json(
            200,
            r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","entries":[{"subject":{"subjectId":"actor-b","subjectType":"user"},"permission":"read","effect":"allow"}]}"#,
        ),
        ("PUT", ["api", "documents", "doc-allowed", "sharing"]) => {
            println!("product_log_event=document.sharing.updated document_id=doc-allowed");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","entries":[{"subject":{"subjectId":"actor-b","subjectType":"user"},"permission":"comment","effect":"allow"}]}"#,
            )
        }
        ("GET", ["api", "documents", "doc-allowed", "comments"]) => E2eRouteResult::json(
            200,
            r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","threads":[{"threadId":"comment-thread-1","state":"open","comments":[{"commentId":"comment-1","authorUserId":"actor-a","body":"redacted","createdAt":"2026-06-26T00:00:00Z"}]}]}"#,
        ),
        ("POST", ["api", "documents", "doc-allowed", "comments"]) => {
            println!("product_log_event=comment.created document_id=doc-allowed");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","threadId":"comment-thread-1","state":"open","comments":[{"commentId":"comment-2","authorUserId":"actor-b","body":"redacted","createdAt":"2026-06-26T00:00:01Z"}]}"#,
            )
        }
        ("POST", ["api", "documents", "doc-allowed", "inline-comments"]) => {
            println!("product_log_event=comment.inline_created document_id=doc-allowed");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","threadId":"comment-thread-inline-1","state":"open","anchor":{"documentVersionId":"version-3","startOffset":1,"endOffset":4,"status":"valid"},"comments":[{"commentId":"comment-inline-1","authorUserId":"actor-b","body":"redacted","createdAt":"2026-06-26T00:00:02Z"}]}"#,
            )
        }
        ("POST", ["api", "comments", "comment-thread-1", "resolve"]) => E2eRouteResult::json(
            200,
            r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","threadId":"comment-thread-1","previousState":"open","nextState":"resolved"}"#,
        ),
        ("POST", ["api", "comments", "comment-thread-1", "reopen"]) => E2eRouteResult::json(
            200,
            r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","threadId":"comment-thread-1","previousState":"resolved","nextState":"reopened"}"#,
        ),
        ("GET", ["api", "review-requests"]) => E2eRouteResult::json(
            200,
            r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","requests":[]}"#,
        ),
        ("POST", ["api", "documents", "doc-allowed", "review-requests"]) => {
            println!("product_log_event=review.requested document_id=doc-allowed");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","reviewRequestId":"review-request-1","previousState":"Draft","nextState":"InReview","status":"requested"}"#,
            )
        }
        ("POST", ["api", "review-requests", "review-request-1", "approve"]) => {
            println!("product_log_event=review.approved review_request_id=review-request-1");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","reviewRequestId":"review-request-1","previousState":"InReview","nextState":"Approved","status":"approved"}"#,
            )
        }
        ("POST", ["api", "review-requests", "review-request-2", "reject"]) => {
            println!("product_log_event=review.rejected review_request_id=review-request-2");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","reviewRequestId":"review-request-2","previousState":"InReview","nextState":"Rejected","status":"rejected"}"#,
            )
        }
        ("POST", ["api", "documents", "doc-allowed", "publish"]) => {
            println!("product_log_event=document.publish.completed document_id=doc-allowed");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","previousState":"Approved","nextState":"Published","publishedVersionId":"version-4"}"#,
            )
        }
        ("POST", ["api", "documents", "doc-denied", "publish"]) => {
            E2eRouteResult::json(403, r#"{"errorCode":"PUBLISH_DENIED"}"#)
        }
        ("GET", ["api", "documents", "doc-allowed", "locks", "current"]) => lock_status(state),
        ("POST", ["api", "documents", "doc-allowed", "locks"]) => acquire_lock(state),
        ("DELETE", ["api", "documents", "doc-allowed", "locks", "current"]) => release_lock(state),
        ("GET", ["api", "audit-events"]) => E2eRouteResult::json(
            200,
            r#"{"workspaceId":"workspace-1","events":[{"eventId":"audit-1","eventName":"document.publish.completed","actorUserId":"actor-a","targetId":"doc-allowed"},{"eventId":"audit-2","eventName":"backup.created","actorUserId":"actor-a","targetId":"backup-job-1"}],"nextCursor":null}"#,
        ),
        ("POST", ["api", "field-debug-sessions"]) => {
            state.field_debug_state = "Requested";
            println!("product_log_event=field_debug.requested scope=workspace ttl_seconds=60");
            E2eRouteResult::json(
                200,
                r#"{"sessionId":"field-debug-session-1","state":"Requested","scope":"workspace","ttlSeconds":60}"#,
            )
        }
        (
            "POST",
            [
                "api",
                "field-debug-sessions",
                "field-debug-session-1",
                "approve",
            ],
        ) => {
            state.field_debug_state = "Active";
            println!("product_log_event=field_debug.approved scope=workspace ttl_seconds=60");
            E2eRouteResult::json(
                200,
                r#"{"sessionId":"field-debug-session-1","previousState":"Requested","nextState":"Active"}"#,
            )
        }
        (
            "POST",
            [
                "api",
                "field-debug-sessions",
                "field-debug-session-1",
                "expire",
            ],
        ) => {
            state.field_debug_state = "Expired";
            println!("product_log_event=field_debug.expired scope=workspace");
            E2eRouteResult::json(
                200,
                r#"{"sessionId":"field-debug-session-1","previousState":"Active","nextState":"Expired"}"#,
            )
        }
        ("POST", ["api", "backups"]) => {
            println!("product_log_event=backup.created workspace_id=workspace-1");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","jobId":"backup-job-1","operation":"Backup","state":"Queued","progress":{"stage":"Queued","percent":0}}"#,
            )
        }
        ("GET", ["api", "backups", "backup-job-1"]) => E2eRouteResult::json(
            200,
            r#"{"workspaceId":"workspace-1","jobId":"backup-job-1","operation":"Backup","state":"Completed","progress":{"stage":"Completed","percent":100}}"#,
        ),
        ("POST", ["api", "backups", "backup-job-1", "restore"]) => {
            println!("product_log_event=restore.completed workspace_id=workspace-1");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","sourceJobId":"backup-job-1","restoreJobId":"restore-job-1","state":"Completed"}"#,
            )
        }
        ("POST", ["api", "exports"]) => {
            println!("product_log_event=export.created workspace_id=workspace-1");
            E2eRouteResult::json(
                200,
                r#"{"workspaceId":"workspace-1","jobId":"export-job-1","operation":"Export","state":"Queued","progress":{"stage":"Queued","percent":0}}"#,
            )
        }
        ("GET", ["api", "exports", "export-job-1"]) => E2eRouteResult::json(
            200,
            r#"{"workspaceId":"workspace-1","jobId":"export-job-1","operation":"Export","state":"Completed","progress":{"stage":"Completed","percent":100}}"#,
        ),
        _ => E2eRouteResult::json(404, r#"{"errorCode":"ROUTE_NOT_FOUND"}"#),
    }
}

fn lock_status(state: &E2eHttpState) -> E2eRouteResult {
    if state.lock_holder.is_some() {
        return E2eRouteResult::json(
            200,
            r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","status":"locked","ownerUserId":"actor-a","expiresAt":"2026-06-26T00:10:00Z"}"#,
        );
    }
    E2eRouteResult::json(
        200,
        r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","status":"unlocked","ownerUserId":null,"expiresAt":null}"#,
    )
}

fn acquire_lock(state: &mut E2eHttpState) -> E2eRouteResult {
    if state.lock_holder.is_some() {
        return E2eRouteResult::json(409, r#"{"errorCode":"DOCUMENT_LOCK_CONFLICT"}"#);
    }
    state.lock_holder = Some("actor-a".to_string());
    println!("product_log_event=document.lock.acquired document_id=doc-allowed");
    E2eRouteResult::json(
        200,
        r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","status":"locked","ownerUserId":"actor-a","expiresAt":"2026-06-26T00:10:00Z"}"#,
    )
}

fn release_lock(state: &mut E2eHttpState) -> E2eRouteResult {
    state.lock_holder = None;
    println!("product_log_event=document.lock.released document_id=doc-allowed");
    E2eRouteResult::json(
        200,
        r#"{"workspaceId":"workspace-1","documentId":"doc-allowed","status":"unlocked","ownerUserId":null,"expiresAt":null}"#,
    )
}

fn read_http_request(stream: &mut TcpStream) -> std::io::Result<Option<ParsedHttpRequest>> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];
    let mut header_end = None;

    while header_end.is_none() {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Ok(None);
        }
        buffer.extend_from_slice(&chunk[..read]);
        header_end = find_header_end(&buffer);
    }

    let header_end = header_end.expect("header end checked");
    let header_text = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default().to_string();
    let target = request_parts.next().unwrap_or_default().to_string();
    if method.is_empty() || target.is_empty() {
        return Ok(None);
    }

    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let body_start = header_end + 4;
    while buffer.len().saturating_sub(body_start) < content_length {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }

    let body_end = body_start.saturating_add(content_length).min(buffer.len());
    let body = String::from_utf8_lossy(&buffer[body_start..body_end]).to_string();

    Ok(Some(ParsedHttpRequest {
        method,
        target,
        headers,
        body,
    }))
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn write_http_response(
    stream: &mut TcpStream,
    status_code: u16,
    body: &str,
) -> std::io::Result<()> {
    let reason = match status_code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        _ => "Internal Server Error",
    };
    let response = format!(
        "HTTP/1.1 {status_code} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_route_rejects_missing_session_with_stable_error() {
        let request = ParsedHttpRequest {
            method: "GET".to_string(),
            target: "/api/users".to_string(),
            headers: BTreeMap::new(),
            body: String::new(),
        };
        let mut state = E2eHttpState::default();

        let response = handle_e2e_http_request(&request, &mut state);

        assert_eq!(response.status_code, 401);
        assert_eq!(response.body, r#"{"errorCode":"SESSION_EXPIRED"}"#);
    }

    #[test]
    fn lock_flow_exposes_conflict_as_state_machine_error_code() {
        let mut state = E2eHttpState::default();
        let request = request_with_session("POST", "/api/documents/doc-allowed/locks");

        let first_response = handle_e2e_http_request(&request, &mut state);
        let conflict_response = handle_e2e_http_request(&request, &mut state);

        assert_eq!(first_response.status_code, 200);
        assert_eq!(conflict_response.status_code, 409);
        assert_eq!(
            conflict_response.body,
            r#"{"errorCode":"DOCUMENT_LOCK_CONFLICT"}"#
        );
    }

    #[test]
    fn product_error_responses_do_not_include_sensitive_request_body() {
        let mut request = request_with_session("POST", "/api/documents/doc-denied/publish");
        request.body = r#"{"body":"comment body should not leak","token":"secret"}"#.to_string();
        let mut state = E2eHttpState::default();

        let response = handle_e2e_http_request(&request, &mut state);

        assert_eq!(response.status_code, 403);
        assert!(!response.body.contains("comment body should not leak"));
        assert!(!response.body.contains("secret"));
    }

    #[test]
    fn graph_route_returns_permission_filtered_product_response() {
        let request = request_with_session(
            "GET",
            "/api/workspaces/workspace-1/documents/doc-allowed/graph",
        );
        let mut state = E2eHttpState::default();

        let response = handle_e2e_http_request(&request, &mut state);

        assert_eq!(response.status_code, 200);
        assert!(
            response
                .body
                .contains(r#""centerDocumentId":"doc-allowed""#)
        );
        assert!(response.body.contains(r#""id":"doc-visible""#));
        assert!(response.body.contains(r#""candidateCount":3"#));
        assert!(response.body.contains(r#""filteredCount":1"#));
        assert!(!response.body.contains("doc-hidden"));
        assert!(
            !response
                .body
                .contains("E2E document body should not be logged")
        );
        assert!(!response.body.contains(E2E_SESSION_TOKEN));
    }

    fn request_with_session(method: &str, target: &str) -> ParsedHttpRequest {
        let mut headers = BTreeMap::new();
        headers.insert(
            "authorization".to_string(),
            format!("Bearer {E2E_SESSION_TOKEN}"),
        );
        ParsedHttpRequest {
            method: method.to_string(),
            target: target.to_string(),
            headers,
            body: String::new(),
        }
    }
}
