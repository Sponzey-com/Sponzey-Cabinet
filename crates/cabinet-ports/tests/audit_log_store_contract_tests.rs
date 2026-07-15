use cabinet_domain::audit::{
    AuditAction, AuditActor, AuditEvent, AuditEventId, AuditMetadata, AuditTarget, AuditTimestamp,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::audit_log::{
    AuditCursor, AuditEventPage, AuditListQuery, AuditListScope, AuditLogStore, AuditLogStoreError,
    AuditPageRequest,
};

#[derive(Default)]
struct FakeAuditLogStore {
    events: Vec<AuditEvent>,
}

impl AuditLogStore for FakeAuditLogStore {
    fn append_audit_event(&mut self, event: AuditEvent) -> Result<(), AuditLogStoreError> {
        self.events.push(event);
        Ok(())
    }

    fn list_audit_events(
        &self,
        query: AuditListQuery,
    ) -> Result<AuditEventPage, AuditLogStoreError> {
        let start = query.page().cursor().map_or(0, AuditCursor::offset);
        let limit = query.page().limit();
        let matching = self
            .events
            .iter()
            .filter(|event| query.matches(event))
            .cloned()
            .collect::<Vec<_>>();
        let events = matching
            .iter()
            .skip(start)
            .take(limit)
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

#[test]
fn audit_log_store_lists_workspace_events_with_cursor_pagination() {
    let workspace_id = workspace_id();
    let actor = user_id("actor-1");
    let mut store = FakeAuditLogStore::default();
    store
        .append_audit_event(event("audit-1", &workspace_id, &actor, "doc-1"))
        .expect("append first event");
    store
        .append_audit_event(event("audit-2", &workspace_id, &actor, "doc-2"))
        .expect("append second event");
    store
        .append_audit_event(event("audit-3", &workspace_id, &actor, "doc-3"))
        .expect("append third event");

    let first_page = store
        .list_audit_events(AuditListQuery::workspace(
            workspace_id.clone(),
            AuditPageRequest::new(2, None).expect("page request"),
        ))
        .expect("first page");

    assert_eq!(first_page.events().len(), 2);
    assert_eq!(first_page.events()[0].event_id().as_str(), "audit-1");
    assert_eq!(first_page.next_cursor().expect("next cursor").as_str(), "2");

    let second_page = store
        .list_audit_events(AuditListQuery::workspace(
            workspace_id,
            AuditPageRequest::new(2, first_page.next_cursor().cloned()).expect("page request"),
        ))
        .expect("second page");

    assert_eq!(second_page.events().len(), 1);
    assert_eq!(second_page.events()[0].event_id().as_str(), "audit-3");
    assert!(second_page.next_cursor().is_none());
}

#[test]
fn audit_log_query_can_be_scoped_by_actor_and_target_without_exposing_storage_rows() {
    let workspace_id = workspace_id();
    let actor_1 = user_id("actor-1");
    let actor_2 = user_id("actor-2");
    let mut store = FakeAuditLogStore::default();
    store
        .append_audit_event(event("audit-1", &workspace_id, &actor_1, "doc-1"))
        .expect("append actor 1");
    store
        .append_audit_event(event("audit-2", &workspace_id, &actor_2, "doc-2"))
        .expect("append actor 2");

    let actor_page = store
        .list_audit_events(AuditListQuery::new(
            workspace_id.clone(),
            AuditListScope::actor(actor_1.clone()),
            AuditPageRequest::new(50, None).expect("page request"),
        ))
        .expect("actor page");
    let target_page = store
        .list_audit_events(AuditListQuery::new(
            workspace_id,
            AuditListScope::target("document", "doc-2").expect("target scope"),
            AuditPageRequest::new(50, None).expect("page request"),
        ))
        .expect("target page");

    assert_eq!(actor_page.events().len(), 1);
    assert_eq!(actor_page.events()[0].actor().actor_id(), actor_1.as_str());
    assert_eq!(target_page.events().len(), 1);
    assert_eq!(target_page.events()[0].target().target_id(), "doc-2");
}

#[test]
fn audit_page_request_rejects_invalid_limit_and_cursor_values() {
    assert_eq!(
        AuditPageRequest::new(0, None).expect_err("zero limit rejected"),
        AuditLogStoreError::InvalidLimit
    );
    assert_eq!(
        AuditPageRequest::new(501, None).expect_err("oversized limit rejected"),
        AuditLogStoreError::InvalidLimit
    );
    assert_eq!(
        AuditCursor::new("not-a-number").expect_err("invalid cursor rejected"),
        AuditLogStoreError::InvalidCursor
    );
    assert_eq!(
        AuditListScope::target(" ", "doc-1").expect_err("empty target type rejected"),
        AuditLogStoreError::InvalidScope
    );
}

#[test]
fn audit_log_store_error_codes_are_stable_for_boundary_mapping() {
    assert_eq!(
        AuditLogStoreError::InvalidCursor.code(),
        "audit_log.invalid_cursor"
    );
    assert_eq!(
        AuditLogStoreError::StorageUnavailable.code(),
        "audit_log.storage_unavailable"
    );
    assert_eq!(AuditLogStoreError::Conflict.code(), "audit_log.conflict");
    assert_eq!(
        AuditLogStoreError::CorruptedState.code(),
        "audit_log.corrupted_state"
    );
}

fn event(
    event_id: &str,
    workspace_id: &WorkspaceId,
    actor_user_id: &UserId,
    document_id: &str,
) -> AuditEvent {
    AuditEvent::new(
        AuditEventId::new(event_id).expect("audit id"),
        workspace_id.clone(),
        AuditActor::user(actor_user_id.clone()),
        AuditAction::DocumentPublished,
        AuditTarget::document(DocumentId::new(document_id).expect("document id")),
        AuditMetadata::new([("source", "contract")]).expect("metadata"),
        AuditTimestamp::from_millis(1_000),
    )
}

fn workspace_id() -> WorkspaceId {
    WorkspaceId::new("workspace-1").expect("workspace id")
}

fn user_id(value: &str) -> UserId {
    UserId::new(value).expect("user id")
}
