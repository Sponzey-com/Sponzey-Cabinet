use cabinet_domain::field_debug::{
    FieldDebugError, FieldDebugEvent, FieldDebugScope, FieldDebugSession, FieldDebugSessionId,
    FieldDebugSessionState, FieldDebugTimestamp, FieldDebugTtl, transition_field_debug_session,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;

#[test]
fn field_debug_session_rejects_approval_without_scope_or_ttl() {
    let missing_scope = FieldDebugSession::requested(
        FieldDebugSessionId::new("field-debug-1").expect("session id"),
        workspace_id(),
        user_id("requester-1"),
        None,
        Some(FieldDebugTtl::seconds(300).expect("ttl")),
        FieldDebugTimestamp::from_millis(1_000),
    );
    let missing_ttl = FieldDebugSession::requested(
        FieldDebugSessionId::new("field-debug-2").expect("session id"),
        workspace_id(),
        user_id("requester-1"),
        Some(FieldDebugScope::new("workspace:workspace-1").expect("scope")),
        None,
        FieldDebugTimestamp::from_millis(1_000),
    );

    assert_eq!(
        transition_field_debug_session(
            &missing_scope,
            FieldDebugEvent::Approve {
                admin_user_id: user_id("admin-1"),
                at: FieldDebugTimestamp::from_millis(2_000),
            },
        )
        .expect_err("missing scope must fail"),
        FieldDebugError::MissingScope
    );
    assert_eq!(
        transition_field_debug_session(
            &missing_ttl,
            FieldDebugEvent::Approve {
                admin_user_id: user_id("admin-1"),
                at: FieldDebugTimestamp::from_millis(2_000),
            },
        )
        .expect_err("missing ttl must fail"),
        FieldDebugError::MissingTtl
    );
}

#[test]
fn field_debug_session_supports_approved_active_expired_and_revoked_paths() {
    let requested = complete_session("field-debug-1");

    let approved = transition_field_debug_session(
        &requested,
        FieldDebugEvent::Approve {
            admin_user_id: user_id("admin-1"),
            at: FieldDebugTimestamp::from_millis(2_000),
        },
    )
    .expect("approve");
    assert_eq!(approved.session().state(), FieldDebugSessionState::Approved);
    assert_eq!(approved.product_log_event_name(), "field_debug.approved");
    assert_eq!(
        approved.session().approved_by().expect("approver").as_str(),
        "admin-1"
    );

    let active = transition_field_debug_session(
        approved.session(),
        FieldDebugEvent::Activate {
            at: FieldDebugTimestamp::from_millis(2_000),
        },
    )
    .expect("activate");
    assert_eq!(active.session().state(), FieldDebugSessionState::Active);
    assert_eq!(active.product_log_event_name(), "field_debug.active");
    assert_eq!(
        active.session().expires_at().expect("expires").as_millis(),
        302_000
    );

    let expired = transition_field_debug_session(
        active.session(),
        FieldDebugEvent::Expire {
            at: FieldDebugTimestamp::from_millis(302_000),
        },
    )
    .expect("expire");
    assert_eq!(expired.session().state(), FieldDebugSessionState::Expired);
    assert_eq!(expired.product_log_event_name(), "field_debug.expired");

    let second_active = transition_field_debug_session(
        transition_field_debug_session(
            &complete_session("field-debug-2"),
            FieldDebugEvent::Approve {
                admin_user_id: user_id("admin-1"),
                at: FieldDebugTimestamp::from_millis(2_000),
            },
        )
        .expect("approve second")
        .session(),
        FieldDebugEvent::Activate {
            at: FieldDebugTimestamp::from_millis(2_000),
        },
    )
    .expect("activate second");
    let revoked = transition_field_debug_session(
        second_active.session(),
        FieldDebugEvent::Revoke {
            admin_user_id: user_id("admin-1"),
            at: FieldDebugTimestamp::from_millis(3_000),
        },
    )
    .expect("revoke");
    assert_eq!(revoked.session().state(), FieldDebugSessionState::Revoked);
    assert_eq!(revoked.product_log_event_name(), "field_debug.revoked");
}

#[test]
fn field_debug_session_supports_denied_path_from_requested() {
    let requested = complete_session("field-debug-1");

    let denied = transition_field_debug_session(
        &requested,
        FieldDebugEvent::Deny {
            admin_user_id: user_id("admin-1"),
            at: FieldDebugTimestamp::from_millis(2_000),
        },
    )
    .expect("deny");

    assert_eq!(denied.session().state(), FieldDebugSessionState::Denied);
    assert_eq!(denied.product_log_event_name(), "field_debug.denied");
}

#[test]
fn field_debug_scope_and_ttl_reject_missing_or_sensitive_values() {
    assert_eq!(
        FieldDebugScope::new(" ").expect_err("empty scope"),
        FieldDebugError::EmptyScope
    );
    assert_eq!(
        FieldDebugScope::new("document_body:secret").expect_err("body scope"),
        FieldDebugError::SensitiveScope
    );
    assert_eq!(
        FieldDebugScope::new("token=abc").expect_err("token scope"),
        FieldDebugError::SensitiveScope
    );
    assert_eq!(
        FieldDebugTtl::seconds(0).expect_err("zero ttl"),
        FieldDebugError::MissingTtl
    );
}

#[test]
fn field_debug_expire_requires_active_session_and_elapsed_expiry() {
    let requested = complete_session("field-debug-1");
    let active = transition_field_debug_session(
        transition_field_debug_session(
            &requested,
            FieldDebugEvent::Approve {
                admin_user_id: user_id("admin-1"),
                at: FieldDebugTimestamp::from_millis(2_000),
            },
        )
        .expect("approve")
        .session(),
        FieldDebugEvent::Activate {
            at: FieldDebugTimestamp::from_millis(2_000),
        },
    )
    .expect("activate");

    assert_eq!(
        transition_field_debug_session(
            active.session(),
            FieldDebugEvent::Expire {
                at: FieldDebugTimestamp::from_millis(301_999),
            },
        )
        .expect_err("not expired yet"),
        FieldDebugError::NotExpired
    );
    assert_eq!(
        transition_field_debug_session(
            &requested,
            FieldDebugEvent::Expire {
                at: FieldDebugTimestamp::from_millis(302_000),
            },
        )
        .expect_err("requested cannot expire"),
        FieldDebugError::InvalidTransition
    );
}

fn complete_session(value: &str) -> FieldDebugSession {
    FieldDebugSession::requested(
        FieldDebugSessionId::new(value).expect("session id"),
        workspace_id(),
        user_id("requester-1"),
        Some(FieldDebugScope::new("workspace:workspace-1").expect("scope")),
        Some(FieldDebugTtl::seconds(300).expect("ttl")),
        FieldDebugTimestamp::from_millis(1_000),
    )
}

fn workspace_id() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace id")
}

fn user_id(value: &str) -> UserId {
    UserId::new(value).expect("user id")
}
