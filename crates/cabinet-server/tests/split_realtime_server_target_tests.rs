use std::cell::RefCell;
use std::rc::Rc;

use cabinet_core::server_config::ServerConfigInput;
use cabinet_server::adapter::{
    HttpMethod, ServerRequest, ServerUsecaseTarget, UsecaseInputDto, UsecaseOutputDto,
    handle_request,
};
use cabinet_server::collaboration_realtime::{
    SplitRealtimeServerTarget, is_collaboration_realtime_route,
};
use cabinet_server::composition::build_server_composition;
use cabinet_server::errors::ServerBoundaryError;

#[test]
fn split_target_sends_non_realtime_routes_to_primary_target() {
    let primary_seen = Rc::new(RefCell::new(Vec::new()));
    let realtime_seen = Rc::new(RefCell::new(Vec::new()));
    let target = SplitRealtimeServerTarget::new(
        FakeTarget::new("primary", primary_seen.clone()),
        FakeTarget::new("realtime", realtime_seen.clone()),
    );

    let output = target
        .handle(UsecaseInputDto::new("health.check", None))
        .expect("dispatch");

    assert_eq!(output.body(), "primary");
    assert_eq!(primary_seen.borrow().as_slice(), ["health.check"]);
    assert!(realtime_seen.borrow().is_empty());
}

#[test]
fn split_target_sends_collaboration_realtime_routes_to_realtime_target() {
    let primary_seen = Rc::new(RefCell::new(Vec::new()));
    let realtime_seen = Rc::new(RefCell::new(Vec::new()));
    let target = SplitRealtimeServerTarget::new(
        FakeTarget::new("primary", primary_seen.clone()),
        FakeTarget::new("realtime", realtime_seen.clone()),
    );

    let output = target
        .handle(UsecaseInputDto::new(
            "collaboration.join_document_room",
            None,
        ))
        .expect("dispatch");

    assert_eq!(output.body(), "realtime");
    assert!(primary_seen.borrow().is_empty());
    assert_eq!(
        realtime_seen.borrow().as_slice(),
        ["collaboration.join_document_room"],
    );
}

#[test]
fn split_target_integrates_with_handle_request_after_route_mapping() {
    let composition = build_server_composition(default_config());
    let primary_seen = Rc::new(RefCell::new(Vec::new()));
    let realtime_seen = Rc::new(RefCell::new(Vec::new()));
    let target = SplitRealtimeServerTarget::new(
        FakeTarget::new("primary", primary_seen.clone()),
        FakeTarget::new("realtime", realtime_seen.clone()),
    );

    let health = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(HttpMethod::Get, "/api/health", None),
    )
    .expect("health");
    let realtime = handle_request(
        composition.routes(),
        &target,
        ServerRequest::new(
            HttpMethod::Post,
            "/api/workspaces/workspace-1/documents/doc-1/collaboration/join",
            Some("{\"sessionId\":\"session-1\",\"actorUserId\":\"user-1\"}"),
        ),
    )
    .expect("realtime");

    assert_eq!(health.body(), "primary");
    assert_eq!(realtime.body(), "realtime");
    assert_eq!(primary_seen.borrow().as_slice(), ["health.check"]);
    assert_eq!(
        realtime_seen.borrow().as_slice(),
        ["collaboration.join_document_room"],
    );
}

#[test]
fn collaboration_realtime_route_helper_is_explicit() {
    assert!(is_collaboration_realtime_route(
        "collaboration.join_document_room"
    ));
    assert!(is_collaboration_realtime_route(
        "collaboration.broadcast_operation"
    ));
    assert!(is_collaboration_realtime_route(
        "collaboration.broadcast_presence"
    ));
    assert!(is_collaboration_realtime_route(
        "collaboration.request_replay"
    ));
    assert!(!is_collaboration_realtime_route("graph.get_local"));
    assert!(!is_collaboration_realtime_route("comment.add"));
}

#[derive(Clone)]
struct FakeTarget {
    response_body: &'static str,
    seen: Rc<RefCell<Vec<String>>>,
}

impl FakeTarget {
    fn new(response_body: &'static str, seen: Rc<RefCell<Vec<String>>>) -> Self {
        Self {
            response_body,
            seen,
        }
    }
}

impl ServerUsecaseTarget for FakeTarget {
    fn handle(&self, input: UsecaseInputDto) -> Result<UsecaseOutputDto, ServerBoundaryError> {
        self.seen.borrow_mut().push(input.route_id().to_string());
        Ok(UsecaseOutputDto::new(200, self.response_body))
    }
}

fn default_config() -> cabinet_core::server_config::ServerConfig {
    ServerConfigInput::local_dev_defaults()
        .validate()
        .expect("valid server config")
}
