use cabinet_domain::field_debug::{
    FieldDebugScope, FieldDebugSession, FieldDebugSessionId, FieldDebugSessionState,
    FieldDebugTimestamp, FieldDebugTtl,
};
use cabinet_domain::permission::{
    Permission, PermissionDecision, PermissionDecisionReason, PolicySource,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::field_debug::{
    FieldDebugClock, FieldDebugPermissionCheckError, FieldDebugPermissionChecker,
    FieldDebugSessionRepository, FieldDebugSessionRepositoryError,
};
use cabinet_usecases::field_debug::{
    ApproveFieldDebugSessionInput, ApproveFieldDebugSessionUsecase, ExpireFieldDebugSessionInput,
    ExpireFieldDebugSessionUsecase, FieldDebugDevelopmentEvent, FieldDebugDiagnosticInput,
    FieldDebugDiagnosticUsecase, FieldDebugLogEvent, FieldDebugProductEvent,
    FieldDebugSessionOutputStatus, FieldDebugSessionPolicy, FieldDebugUsecaseError,
    FieldDebugUsecaseLogger, RequestFieldDebugSessionInput, RequestFieldDebugSessionUsecase,
    RevokeFieldDebugSessionInput, RevokeFieldDebugSessionUsecase,
};

#[derive(Default)]
struct FakeFieldDebugSessionRepository {
    sessions: Vec<FieldDebugSession>,
    save_error: Option<FieldDebugSessionRepositoryError>,
}

impl FieldDebugSessionRepository for FakeFieldDebugSessionRepository {
    fn save_field_debug_session(
        &mut self,
        session: FieldDebugSession,
    ) -> Result<(), FieldDebugSessionRepositoryError> {
        if let Some(error) = self.save_error {
            return Err(error);
        }
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

struct FakeFieldDebugClock {
    now: FieldDebugTimestamp,
}

impl FieldDebugClock for FakeFieldDebugClock {
    fn now(&self) -> FieldDebugTimestamp {
        self.now
    }
}

struct FakeFieldDebugPermissionChecker {
    decision: PermissionDecision,
    checked_permission: std::cell::RefCell<Option<Permission>>,
}

impl FakeFieldDebugPermissionChecker {
    fn allowed() -> Self {
        Self {
            decision: PermissionDecision::allowed(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleAllowsPermission,
            ),
            checked_permission: std::cell::RefCell::new(None),
        }
    }

    fn denied() -> Self {
        Self {
            decision: PermissionDecision::denied(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleDoesNotAllowPermission,
            ),
            checked_permission: std::cell::RefCell::new(None),
        }
    }

    fn checked_permission(&self) -> Option<Permission> {
        *self.checked_permission.borrow()
    }
}

impl FieldDebugPermissionChecker for FakeFieldDebugPermissionChecker {
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

#[derive(Default)]
struct FakeFieldDebugLogger {
    product: Vec<FieldDebugProductEvent>,
    field_debug: Vec<FieldDebugLogEvent>,
    development: Vec<FieldDebugDevelopmentEvent>,
}

impl FieldDebugUsecaseLogger for FakeFieldDebugLogger {
    fn write_product(&mut self, event: FieldDebugProductEvent) {
        self.product.push(event);
    }

    fn write_field_debug(&mut self, event: FieldDebugLogEvent) {
        self.field_debug.push(event);
    }

    fn write_development(&mut self, event: FieldDebugDevelopmentEvent) {
        self.development.push(event);
    }
}

#[test]
fn request_and_approve_activate_field_debug_session_with_product_logs() {
    let policy = FieldDebugSessionPolicy::new(900).expect("policy");
    let mut repository = FakeFieldDebugSessionRepository::default();
    let clock = FakeFieldDebugClock {
        now: FieldDebugTimestamp::from_millis(1_000),
    };
    let mut logger = FakeFieldDebugLogger::default();

    let request_output = RequestFieldDebugSessionUsecase::new(policy)
        .execute(
            RequestFieldDebugSessionInput::new(
                "requester-1",
                "workspace-1",
                "field-debug-1",
                Some("workspace:workspace-1"),
                Some(300),
            ),
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect("request session");

    assert_eq!(
        request_output.status(),
        FieldDebugSessionOutputStatus::Requested
    );
    assert_eq!(logger.product[0].event_name(), "field_debug.requested");

    let checker = FakeFieldDebugPermissionChecker::allowed();
    let approve_clock = FakeFieldDebugClock {
        now: FieldDebugTimestamp::from_millis(2_000),
    };
    let approve_output = ApproveFieldDebugSessionUsecase::new(policy)
        .execute(
            ApproveFieldDebugSessionInput::new("admin-1234", "workspace-1", "field-debug-1"),
            &checker,
            &mut repository,
            &approve_clock,
            &mut logger,
        )
        .expect("approve session");

    assert_eq!(checker.checked_permission(), Some(Permission::Manage));
    assert_eq!(
        approve_output.status(),
        FieldDebugSessionOutputStatus::Active
    );
    assert_eq!(approve_output.expires_at_millis(), Some(302_000));
    assert_eq!(logger.product[1].event_name(), "field_debug.approved");
    assert_eq!(logger.product[2].event_name(), "field_debug.active");
    assert!(logger.field_debug.is_empty());
    assert!(logger.development.is_empty());
}

#[test]
fn approve_field_debug_session_rejects_missing_scope_or_ttl_before_activation() {
    let policy = FieldDebugSessionPolicy::new(900).expect("policy");
    let checker = FakeFieldDebugPermissionChecker::allowed();
    let clock = FakeFieldDebugClock {
        now: FieldDebugTimestamp::from_millis(2_000),
    };
    let mut logger = FakeFieldDebugLogger::default();
    let mut missing_scope_repo = FakeFieldDebugSessionRepository {
        sessions: vec![requested_session("missing-scope", None, Some(300))],
        ..FakeFieldDebugSessionRepository::default()
    };
    let mut missing_ttl_repo = FakeFieldDebugSessionRepository {
        sessions: vec![requested_session(
            "missing-ttl",
            Some("workspace:workspace-1"),
            None,
        )],
        ..FakeFieldDebugSessionRepository::default()
    };

    assert_eq!(
        ApproveFieldDebugSessionUsecase::new(policy)
            .execute(
                ApproveFieldDebugSessionInput::new("admin-1234", "workspace-1", "missing-scope"),
                &checker,
                &mut missing_scope_repo,
                &clock,
                &mut logger,
            )
            .expect_err("missing scope"),
        FieldDebugUsecaseError::MissingScope
    );
    assert_eq!(
        ApproveFieldDebugSessionUsecase::new(policy)
            .execute(
                ApproveFieldDebugSessionInput::new("admin-1234", "workspace-1", "missing-ttl"),
                &checker,
                &mut missing_ttl_repo,
                &clock,
                &mut logger,
            )
            .expect_err("missing ttl"),
        FieldDebugUsecaseError::MissingTtl
    );
    assert!(logger.product.is_empty());
}

#[test]
fn field_debug_diagnostic_writes_only_for_active_session_and_sanitized_fields() {
    let active = active_session("field-debug-1", 2_000, 300);
    let repository = FakeFieldDebugSessionRepository {
        sessions: vec![active],
        ..FakeFieldDebugSessionRepository::default()
    };
    let clock = FakeFieldDebugClock {
        now: FieldDebugTimestamp::from_millis(10_000),
    };
    let mut logger = FakeFieldDebugLogger::default();

    FieldDebugDiagnosticUsecase::new()
        .execute(
            FieldDebugDiagnosticInput::new(
                "workspace-1",
                "field-debug-1",
                "permission.summary",
                vec![("decision", "allowed"), ("query_hash", "hash_1234")],
            ),
            &repository,
            &clock,
            &mut logger,
        )
        .expect("diagnostic");

    assert!(logger.product.is_empty());
    assert_eq!(logger.field_debug.len(), 1);
    assert_eq!(logger.field_debug[0].event_name(), "field_debug.diagnostic");
    assert_eq!(logger.field_debug[0].scope(), "workspace:workspace-1");
    assert_eq!(logger.field_debug[0].fields()[0], ("decision", "allowed"));
    assert!(logger.development.is_empty());

    let error = FieldDebugDiagnosticUsecase::new()
        .execute(
            FieldDebugDiagnosticInput::new(
                "workspace-1",
                "field-debug-1",
                "permission.summary",
                vec![("document_body", "hello")],
            ),
            &repository,
            &clock,
            &mut logger,
        )
        .expect_err("sensitive field");

    assert_eq!(error, FieldDebugUsecaseError::SensitiveField);
    assert_eq!(logger.field_debug.len(), 1);
}

#[test]
fn expired_or_revoked_session_cannot_write_field_debug_log() {
    let expired_repository = FakeFieldDebugSessionRepository {
        sessions: vec![active_session("expired", 2_000, 300)],
        ..FakeFieldDebugSessionRepository::default()
    };
    let revoked_repository = FakeFieldDebugSessionRepository {
        sessions: vec![revoked_session("revoked")],
        ..FakeFieldDebugSessionRepository::default()
    };
    let mut logger = FakeFieldDebugLogger::default();

    assert_eq!(
        FieldDebugDiagnosticUsecase::new()
            .execute(
                FieldDebugDiagnosticInput::new(
                    "workspace-1",
                    "expired",
                    "permission.summary",
                    vec![("decision", "allowed")],
                ),
                &expired_repository,
                &FakeFieldDebugClock {
                    now: FieldDebugTimestamp::from_millis(303_000),
                },
                &mut logger,
            )
            .expect_err("expired session"),
        FieldDebugUsecaseError::ExpiredSession
    );
    assert_eq!(
        FieldDebugDiagnosticUsecase::new()
            .execute(
                FieldDebugDiagnosticInput::new(
                    "workspace-1",
                    "revoked",
                    "permission.summary",
                    vec![("decision", "allowed")],
                ),
                &revoked_repository,
                &FakeFieldDebugClock {
                    now: FieldDebugTimestamp::from_millis(10_000),
                },
                &mut logger,
            )
            .expect_err("revoked session"),
        FieldDebugUsecaseError::InactiveSession
    );
    assert!(logger.field_debug.is_empty());
}

#[test]
fn expire_and_revoke_field_debug_session_write_product_logs() {
    let policy = FieldDebugSessionPolicy::new(900).expect("policy");
    let checker = FakeFieldDebugPermissionChecker::allowed();
    let mut expire_repository = FakeFieldDebugSessionRepository {
        sessions: vec![active_session("expire-me", 2_000, 300)],
        ..FakeFieldDebugSessionRepository::default()
    };
    let mut revoke_repository = FakeFieldDebugSessionRepository {
        sessions: vec![active_session("revoke-me", 2_000, 300)],
        ..FakeFieldDebugSessionRepository::default()
    };
    let mut logger = FakeFieldDebugLogger::default();

    let expired = ExpireFieldDebugSessionUsecase::new(policy)
        .execute(
            ExpireFieldDebugSessionInput::new("admin-1234", "workspace-1", "expire-me"),
            &checker,
            &mut expire_repository,
            &FakeFieldDebugClock {
                now: FieldDebugTimestamp::from_millis(302_000),
            },
            &mut logger,
        )
        .expect("expire");
    let revoked = RevokeFieldDebugSessionUsecase::new(policy)
        .execute(
            RevokeFieldDebugSessionInput::new("admin-1234", "workspace-1", "revoke-me"),
            &checker,
            &mut revoke_repository,
            &FakeFieldDebugClock {
                now: FieldDebugTimestamp::from_millis(3_000),
            },
            &mut logger,
        )
        .expect("revoke");

    assert_eq!(expired.status(), FieldDebugSessionOutputStatus::Expired);
    assert_eq!(revoked.status(), FieldDebugSessionOutputStatus::Revoked);
    assert_eq!(logger.product[0].event_name(), "field_debug.expired");
    assert_eq!(logger.product[1].event_name(), "field_debug.revoked");
}

#[test]
fn field_debug_policy_and_permission_errors_are_explicit() {
    assert_eq!(
        FieldDebugSessionPolicy::new(0).expect_err("zero max ttl"),
        FieldDebugUsecaseError::InvalidInput
    );

    let policy = FieldDebugSessionPolicy::new(900).expect("policy");
    let mut repository = FakeFieldDebugSessionRepository::default();
    let mut logger = FakeFieldDebugLogger::default();
    let request_error = RequestFieldDebugSessionUsecase::new(policy)
        .execute(
            RequestFieldDebugSessionInput::new(
                "requester-1",
                "workspace-1",
                "field-debug-1",
                Some("workspace:workspace-1"),
                Some(901),
            ),
            &mut repository,
            &FakeFieldDebugClock {
                now: FieldDebugTimestamp::from_millis(1_000),
            },
            &mut logger,
        )
        .expect_err("ttl exceeds policy");

    assert_eq!(request_error, FieldDebugUsecaseError::TtlExceedsPolicy);

    let checker = FakeFieldDebugPermissionChecker::denied();
    repository.sessions.push(requested_session(
        "field-debug-2",
        Some("workspace:workspace-1"),
        Some(300),
    ));
    let approve_error = ApproveFieldDebugSessionUsecase::new(policy)
        .execute(
            ApproveFieldDebugSessionInput::new("not-admin", "workspace-1", "field-debug-2"),
            &checker,
            &mut repository,
            &FakeFieldDebugClock {
                now: FieldDebugTimestamp::from_millis(2_000),
            },
            &mut logger,
        )
        .expect_err("unauthorized");

    assert_eq!(approve_error, FieldDebugUsecaseError::Unauthorized);
    assert_eq!(checker.checked_permission(), Some(Permission::Manage));
    assert_eq!(
        repository.sessions[0].state(),
        FieldDebugSessionState::Requested
    );
}

fn requested_session(
    session_id: &str,
    scope: Option<&str>,
    ttl_seconds: Option<u32>,
) -> FieldDebugSession {
    FieldDebugSession::requested(
        FieldDebugSessionId::new(session_id).expect("session id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        UserId::new("requester-1").expect("requester id"),
        scope.map(|scope| FieldDebugScope::new(scope).expect("scope")),
        ttl_seconds.map(|seconds| FieldDebugTtl::seconds(seconds).expect("ttl")),
        FieldDebugTimestamp::from_millis(1_000),
    )
}

fn active_session(session_id: &str, activated_at: u64, ttl_seconds: u32) -> FieldDebugSession {
    let requested = requested_session(session_id, Some("workspace:workspace-1"), Some(ttl_seconds));
    let approved = cabinet_domain::field_debug::transition_field_debug_session(
        &requested,
        cabinet_domain::field_debug::FieldDebugEvent::Approve {
            admin_user_id: UserId::new("admin-1234").expect("admin id"),
            at: FieldDebugTimestamp::from_millis(activated_at),
        },
    )
    .expect("approve")
    .into_session();
    cabinet_domain::field_debug::transition_field_debug_session(
        &approved,
        cabinet_domain::field_debug::FieldDebugEvent::Activate {
            at: FieldDebugTimestamp::from_millis(activated_at),
        },
    )
    .expect("activate")
    .into_session()
}

fn revoked_session(session_id: &str) -> FieldDebugSession {
    let active = active_session(session_id, 2_000, 300);
    cabinet_domain::field_debug::transition_field_debug_session(
        &active,
        cabinet_domain::field_debug::FieldDebugEvent::Revoke {
            admin_user_id: UserId::new("admin-1234").expect("admin id"),
            at: FieldDebugTimestamp::from_millis(3_000),
        },
    )
    .expect("revoke")
    .into_session()
}
