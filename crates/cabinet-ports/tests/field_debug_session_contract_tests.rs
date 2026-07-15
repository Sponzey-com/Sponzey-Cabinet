use cabinet_domain::field_debug::{
    FieldDebugScope, FieldDebugSession, FieldDebugSessionId, FieldDebugSessionState,
    FieldDebugTimestamp, FieldDebugTtl,
};
use cabinet_domain::permission::{Permission, PermissionDecision};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::field_debug::{
    FieldDebugClock, FieldDebugPermissionCheckError, FieldDebugPermissionChecker,
    FieldDebugSessionRepository, FieldDebugSessionRepositoryError,
};

#[derive(Default)]
struct FakeFieldDebugSessionRepository {
    sessions: Vec<FieldDebugSession>,
}

impl FieldDebugSessionRepository for FakeFieldDebugSessionRepository {
    fn save_field_debug_session(
        &mut self,
        session: FieldDebugSession,
    ) -> Result<(), FieldDebugSessionRepositoryError> {
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

struct StaticFieldDebugClock {
    now: FieldDebugTimestamp,
}

impl FieldDebugClock for StaticFieldDebugClock {
    fn now(&self) -> FieldDebugTimestamp {
        self.now
    }
}

struct StaticPermissionChecker {
    decision: PermissionDecision,
}

impl FieldDebugPermissionChecker for StaticPermissionChecker {
    fn check_workspace_permission(
        &self,
        _actor_user_id: &UserId,
        _workspace_id: &WorkspaceId,
        permission: Permission,
    ) -> Result<PermissionDecision, FieldDebugPermissionCheckError> {
        assert_eq!(permission, Permission::Manage);
        Ok(self.decision)
    }
}

#[test]
fn field_debug_session_repository_saves_and_replaces_domain_session_without_storage_schema() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let session_id = FieldDebugSessionId::new("debug-1").expect("session id");
    let mut repository = FakeFieldDebugSessionRepository::default();
    let requested = requested_session(session_id.clone(), workspace_id.clone());
    let mut active = requested.clone();
    active = cabinet_domain::field_debug::transition_field_debug_session(
        &active,
        cabinet_domain::field_debug::FieldDebugEvent::Approve {
            admin_user_id: UserId::new("admin-1").expect("admin id"),
            at: FieldDebugTimestamp::from_millis(2_000),
        },
    )
    .expect("approve")
    .into_session();

    repository
        .save_field_debug_session(requested)
        .expect("save requested");
    repository
        .save_field_debug_session(active)
        .expect("replace approved");

    let stored = repository
        .get_field_debug_session(&workspace_id, &session_id)
        .expect("read")
        .expect("session");

    assert_eq!(stored.state(), FieldDebugSessionState::Approved);
    assert_eq!(repository.sessions.len(), 1);
}

#[test]
fn field_debug_clock_and_permission_checker_are_explicit_ports() {
    let clock = StaticFieldDebugClock {
        now: FieldDebugTimestamp::from_millis(10_000),
    };
    let checker = StaticPermissionChecker {
        decision: PermissionDecision::allowed(
            cabinet_domain::permission::PolicySource::Workspace,
            cabinet_domain::permission::PermissionDecisionReason::RoleAllowsPermission,
        ),
    };

    assert_eq!(clock.now().as_millis(), 10_000);
    assert_eq!(
        checker
            .check_workspace_permission(
                &UserId::new("admin-1").expect("admin id"),
                &WorkspaceId::new("workspace-1").expect("workspace id"),
                Permission::Manage,
            )
            .expect("permission")
            .result(),
        cabinet_domain::permission::PermissionDecisionResult::Allowed
    );
}

#[test]
fn field_debug_repository_and_permission_errors_have_stable_codes() {
    assert_eq!(
        FieldDebugSessionRepositoryError::StorageUnavailable.code(),
        "field_debug_session.storage_unavailable"
    );
    assert_eq!(
        FieldDebugSessionRepositoryError::Conflict.code(),
        "field_debug_session.conflict"
    );
    assert_eq!(
        FieldDebugSessionRepositoryError::CorruptedState.code(),
        "field_debug_session.corrupted_state"
    );
    assert_eq!(
        FieldDebugPermissionCheckError::StorageUnavailable.code(),
        "field_debug_permission.storage_unavailable"
    );
}

fn requested_session(
    session_id: FieldDebugSessionId,
    workspace_id: WorkspaceId,
) -> FieldDebugSession {
    FieldDebugSession::requested(
        session_id,
        workspace_id,
        UserId::new("requester-1").expect("requester id"),
        Some(FieldDebugScope::new("workspace:workspace-1").expect("scope")),
        Some(FieldDebugTtl::seconds(300).expect("ttl")),
        FieldDebugTimestamp::from_millis(1_000),
    )
}
