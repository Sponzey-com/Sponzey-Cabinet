use std::collections::HashMap;

use cabinet_domain::collaboration::{
    BaseRevision, DocumentOperation, EditSession, EditSessionId, EditSessionState, OperationId,
    OperationSequence, Presence, TextRange,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::collaboration::{
    CollaborationEventLog, CollaborationEventLogError, CollaborationOperationEvent,
    CollaborationSessionStore, CollaborationSessionStoreError,
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
    events: HashMap<(String, String), Vec<CollaborationOperationEvent>>,
}

impl CollaborationEventLog for FakeCollaborationEventLog {
    fn append_operation(
        &mut self,
        workspace_id: &WorkspaceId,
        operation: DocumentOperation,
    ) -> Result<OperationSequence, CollaborationEventLogError> {
        let key = (
            workspace_id.as_str().to_string(),
            operation.document_id().as_str().to_string(),
        );
        let sequence = OperationSequence::new(
            (self.events.get(&key).map(Vec::len).unwrap_or_default() + 1) as u64,
        )
        .expect("sequence");
        self.events
            .entry(key)
            .or_default()
            .push(CollaborationOperationEvent::new(sequence, operation).expect("event"));
        Ok(sequence)
    }

    fn list_operations(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Vec<CollaborationOperationEvent>, CollaborationEventLogError> {
        Ok(self
            .events
            .get(&(
                workspace_id.as_str().to_string(),
                document_id.as_str().to_string(),
            ))
            .cloned()
            .unwrap_or_default())
    }
}

#[test]
fn collaboration_session_store_preserves_session_state_and_presence_separately() {
    let workspace_id = workspace_id("workspace-1");
    let document_id = document_id("doc-1");
    let session = EditSession::new(
        EditSessionId::new("session-1").expect("session id"),
        document_id.clone(),
        user_id("user-1"),
        EditSessionState::SessionStarted,
    )
    .expect("session");
    let presence = Presence::new(
        document_id.clone(),
        user_id("user-1"),
        TextRange::new(4, 4).expect("cursor"),
    )
    .expect("presence");
    let mut store = FakeCollaborationSessionStore::default();

    store
        .save_session(&workspace_id, session.clone())
        .expect("save session");
    store
        .save_presence(&workspace_id, presence.clone())
        .expect("save presence");

    assert_eq!(
        store
            .get_session(&workspace_id, session.session_id())
            .expect("get session")
            .expect("stored session")
            .state(),
        EditSessionState::SessionStarted,
    );
    assert_eq!(
        store
            .list_presence(&workspace_id, &document_id)
            .expect("presence")
            .len(),
        1,
    );
}

#[test]
fn collaboration_event_log_appends_operations_without_presence_updates() {
    let workspace_id = workspace_id("workspace-1");
    let document_id = document_id("doc-1");
    let mut session_store = FakeCollaborationSessionStore::default();
    let mut event_log = FakeCollaborationEventLog::default();
    let operation = operation(&document_id, "op-1");
    let presence = Presence::new(
        document_id.clone(),
        user_id("user-1"),
        TextRange::new(1, 1).expect("cursor"),
    )
    .expect("presence");

    session_store
        .save_presence(&workspace_id, presence)
        .expect("save presence");
    let sequence = event_log
        .append_operation(&workspace_id, operation.clone())
        .expect("append operation");

    assert_eq!(sequence.as_u64(), 1);
    assert_eq!(
        event_log
            .list_operations(&workspace_id, &document_id)
            .expect("operations")
            .len(),
        1,
    );
    assert_eq!(
        session_store
            .list_presence(&workspace_id, &document_id)
            .expect("presence")
            .len(),
        1,
    );
}

fn operation(document_id: &DocumentId, operation_id: &str) -> DocumentOperation {
    DocumentOperation::replace_text(
        OperationId::new(operation_id).expect("operation id"),
        document_id.clone(),
        user_id("user-1"),
        BaseRevision::new(1).expect("revision"),
        TextRange::new(0, 1).expect("range"),
        "x",
    )
    .expect("operation")
}

fn workspace_id(value: &str) -> WorkspaceId {
    WorkspaceId::new(value).expect("workspace id")
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}

fn user_id(value: &str) -> UserId {
    UserId::new(value).expect("user id")
}
