use std::collections::HashMap;

use cabinet_domain::collaboration::{
    EditSession, EditSessionId, EditSessionState, OperationSequence, Presence,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{
    AccessResource, Permission, PermissionDecision, PermissionDecisionReason, PolicySource,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::collaboration::{
    CollaborationEventLog, CollaborationEventLogError, CollaborationOperationEvent,
    CollaborationSessionStore, CollaborationSessionStoreError,
};
use cabinet_ports::permission_aware_query::{PermissionAwareQueryError, PermissionDecisionPort};
use cabinet_usecases::collaboration::{
    ApplyCollaborativeEditInput, ApplyCollaborativeEditStatus, ApplyCollaborativeEditUsecase,
    StartEditSessionInput, StartEditSessionUsecase, UpdatePresenceInput, UpdatePresenceUsecase,
};

#[derive(Default)]
struct FakeCollaborationSessionStore {
    sessions: HashMap<(String, String), EditSession>,
    presences: HashMap<(String, String), Vec<Presence>>,
}

impl CollaborationSessionStore for FakeCollaborationSessionStore {
    fn save_session(
        &mut self,
        workspace_id: &WorkspaceId,
        session: EditSession,
    ) -> Result<(), CollaborationSessionStoreError> {
        self.sessions.insert(
            (
                workspace_id.as_str().to_string(),
                session.session_id().as_str().to_string(),
            ),
            session,
        );
        Ok(())
    }

    fn get_session(
        &self,
        workspace_id: &WorkspaceId,
        session_id: &EditSessionId,
    ) -> Result<Option<EditSession>, CollaborationSessionStoreError> {
        Ok(self
            .sessions
            .get(&(
                workspace_id.as_str().to_string(),
                session_id.as_str().to_string(),
            ))
            .cloned())
    }

    fn save_presence(
        &mut self,
        workspace_id: &WorkspaceId,
        presence: Presence,
    ) -> Result<(), CollaborationSessionStoreError> {
        self.presences
            .entry((
                workspace_id.as_str().to_string(),
                presence.document_id().as_str().to_string(),
            ))
            .or_default()
            .push(presence);
        Ok(())
    }

    fn list_presence(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Vec<Presence>, CollaborationSessionStoreError> {
        Ok(self
            .presences
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
            ))
            .cloned()
            .unwrap_or_default())
    }
}

#[derive(Default)]
struct FakeCollaborationEventLog {
    events: Vec<CollaborationOperationEvent>,
}

impl CollaborationEventLog for FakeCollaborationEventLog {
    fn append_operation(
        &mut self,
        _workspace_id: &WorkspaceId,
        operation: cabinet_domain::collaboration::DocumentOperation,
    ) -> Result<OperationSequence, CollaborationEventLogError> {
        let sequence = OperationSequence::new((self.events.len() + 1) as u64).expect("sequence");
        self.events
            .push(CollaborationOperationEvent::new(sequence, operation).expect("event"));
        Ok(sequence)
    }

    fn list_operations(
        &self,
        _workspace_id: &WorkspaceId,
        _document_id: &DocumentId,
    ) -> Result<Vec<CollaborationOperationEvent>, CollaborationEventLogError> {
        Ok(self.events.clone())
    }
}

struct FakePermissionDecision {
    allow: bool,
}

impl PermissionDecisionPort for FakePermissionDecision {
    fn check_permission(
        &self,
        _actor_user_id: &UserId,
        _resource: &AccessResource,
        permission: Permission,
    ) -> Result<PermissionDecision, PermissionAwareQueryError> {
        assert!(matches!(permission, Permission::Read | Permission::Write));
        Ok(if self.allow {
            PermissionDecision::allowed(
                PolicySource::Document,
                PermissionDecisionReason::RoleAllowsPermission,
            )
        } else {
            PermissionDecision::denied(
                PolicySource::Document,
                PermissionDecisionReason::RoleDoesNotAllowPermission,
            )
        })
    }
}

#[test]
fn start_edit_session_requires_write_permission_and_saves_session() {
    let mut session_store = FakeCollaborationSessionStore::default();
    let usecase = StartEditSessionUsecase::new();

    let output = usecase
        .execute(
            StartEditSessionInput::new("workspace-1", "doc-1", "user-1", "session-1"),
            &mut session_store,
            &FakePermissionDecision { allow: true },
        )
        .expect("start session");

    assert_eq!(output.state(), EditSessionState::SessionStarted);
    assert_eq!(output.product_log_event(), "collaboration.session.started");
    assert_eq!(session_store.sessions.len(), 1);
}

#[test]
fn unauthorized_edit_operation_is_rejected_without_event_append() {
    let mut event_log = FakeCollaborationEventLog::default();
    let usecase = ApplyCollaborativeEditUsecase::new();

    let error = usecase
        .execute(
            ApplyCollaborativeEditInput::replace_text(
                "workspace-1",
                "doc-1",
                "user-1",
                "op-1",
                1,
                1,
                0,
                1,
                "x",
            ),
            &mut event_log,
            &FakePermissionDecision { allow: false },
        )
        .expect_err("permission denied");

    assert_eq!(error.code(), "collaboration.permission_denied");
    assert_eq!(event_log.events.len(), 0);
}

#[test]
fn stale_base_revision_returns_conflict_without_event_append() {
    let mut event_log = FakeCollaborationEventLog::default();
    let usecase = ApplyCollaborativeEditUsecase::new();

    let output = usecase
        .execute(
            ApplyCollaborativeEditInput::replace_text(
                "workspace-1",
                "doc-1",
                "user-1",
                "op-1",
                1,
                2,
                0,
                1,
                "x",
            ),
            &mut event_log,
            &FakePermissionDecision { allow: true },
        )
        .expect("conflict output");

    assert_eq!(
        output.status(),
        ApplyCollaborativeEditStatus::ConflictDetected
    );
    assert_eq!(output.sequence(), None);
    assert_eq!(
        output.product_log_event(),
        "collaboration.conflict.detected"
    );
    assert_eq!(event_log.events.len(), 0);
}

#[test]
fn presence_update_is_saved_without_durable_operation_append() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace");
    let document_id = DocumentId::new("doc-1").expect("document");
    let mut session_store = FakeCollaborationSessionStore::default();
    let event_log = FakeCollaborationEventLog::default();
    let usecase = UpdatePresenceUsecase::new();

    let output = usecase
        .execute(
            UpdatePresenceInput::new("workspace-1", "doc-1", "user-1", 4, 4),
            &mut session_store,
            &FakePermissionDecision { allow: true },
        )
        .expect("presence");

    assert_eq!(output.presence_count(), 1);
    assert_eq!(
        session_store
            .list_presence(&workspace_id, &document_id)
            .expect("presence")
            .len(),
        1,
    );
    assert_eq!(event_log.events.len(), 0);
}
