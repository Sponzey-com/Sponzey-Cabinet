use cabinet_domain::audit::{AuditAction, AuditEvent, AuditTimestamp};
use cabinet_domain::permission::{
    Permission, PermissionDecision, PermissionDecisionReason, PolicySource,
};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::audit_log::{
    AuditClock, AuditEventPage, AuditListQuery, AuditLogStore, AuditLogStoreError,
    AuditPermissionCheckError, AuditPermissionChecker,
};
use cabinet_usecases::audit::{
    AuditFieldDebugEvent, AuditProductEvent, AuditRetentionPolicy, AuditTargetInput,
    AuditUsecaseError, AuditUsecaseLogger, ListAuditEventsInput, ListAuditEventsScopeInput,
    ListAuditEventsUsecase, RecordAuditEventInput, RecordAuditEventStatus, RecordAuditEventUsecase,
};

#[derive(Default)]
struct FakeAuditLogStore {
    events: Vec<AuditEvent>,
    append_error: Option<AuditLogStoreError>,
    list_error: Option<AuditLogStoreError>,
    list_called: bool,
    last_query: Option<AuditListQuery>,
}

impl AuditLogStore for FakeAuditLogStore {
    fn append_audit_event(&mut self, event: AuditEvent) -> Result<(), AuditLogStoreError> {
        if let Some(error) = self.append_error {
            return Err(error);
        }
        self.events.push(event);
        Ok(())
    }

    fn list_audit_events(
        &self,
        query: AuditListQuery,
    ) -> Result<AuditEventPage, AuditLogStoreError> {
        if let Some(error) = self.list_error {
            return Err(error);
        }
        let start = query.page().cursor().map_or(0, |cursor| cursor.offset());
        let matching = self
            .events
            .iter()
            .filter(|event| query.matches(event))
            .cloned()
            .collect::<Vec<_>>();
        let events = matching
            .iter()
            .skip(start)
            .take(query.page().limit())
            .cloned()
            .collect::<Vec<_>>();
        let next_offset = start + events.len();
        let next_cursor = if next_offset < matching.len() {
            Some(cabinet_ports::audit_log::AuditCursor::from_offset(
                next_offset,
            ))
        } else {
            None
        };
        Ok(AuditEventPage::new(events, next_cursor))
    }
}

struct RecordingAuditLogStore {
    inner: std::cell::RefCell<FakeAuditLogStore>,
}

impl RecordingAuditLogStore {
    fn new(events: Vec<AuditEvent>) -> Self {
        Self {
            inner: std::cell::RefCell::new(FakeAuditLogStore {
                events,
                ..FakeAuditLogStore::default()
            }),
        }
    }

    fn list_called(&self) -> bool {
        self.inner.borrow().list_called
    }
}

impl AuditLogStore for RecordingAuditLogStore {
    fn append_audit_event(&mut self, event: AuditEvent) -> Result<(), AuditLogStoreError> {
        self.inner.borrow_mut().append_audit_event(event)
    }

    fn list_audit_events(
        &self,
        query: AuditListQuery,
    ) -> Result<AuditEventPage, AuditLogStoreError> {
        let mut inner = self.inner.borrow_mut();
        inner.list_called = true;
        inner.last_query = Some(query.clone());
        inner.list_audit_events(query)
    }
}

struct FakeAuditClock {
    now: AuditTimestamp,
}

impl AuditClock for FakeAuditClock {
    fn now(&self) -> AuditTimestamp {
        self.now
    }
}

struct FakeAuditPermissionChecker {
    decision: PermissionDecision,
    checked_permission: std::cell::RefCell<Option<Permission>>,
}

impl FakeAuditPermissionChecker {
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

impl AuditPermissionChecker for FakeAuditPermissionChecker {
    fn check_workspace_permission(
        &self,
        _actor_user_id: &UserId,
        _workspace_id: &WorkspaceId,
        permission: Permission,
    ) -> Result<PermissionDecision, AuditPermissionCheckError> {
        *self.checked_permission.borrow_mut() = Some(permission);
        Ok(self.decision)
    }
}

#[derive(Default)]
struct FakeAuditLogger {
    product: Vec<AuditProductEvent>,
    field_debug: Vec<AuditFieldDebugEvent>,
}

impl AuditUsecaseLogger for FakeAuditLogger {
    fn write_product(&mut self, event: AuditProductEvent) {
        self.product.push(event);
    }

    fn write_field_debug(&mut self, event: AuditFieldDebugEvent) {
        self.field_debug.push(event);
    }
}

#[test]
fn record_audit_event_persists_permission_denied_without_sensitive_payloads() {
    let policy = AuditRetentionPolicy::new(365).expect("retention policy");
    let usecase = RecordAuditEventUsecase::new(policy);
    let mut store = FakeAuditLogStore::default();
    let clock = FakeAuditClock {
        now: AuditTimestamp::from_millis(123_456),
    };
    let mut logger = FakeAuditLogger::default();

    let output = usecase
        .execute(
            RecordAuditEventInput::new(
                "actor-1234",
                "workspace-1",
                "audit-1",
                AuditAction::PermissionDenied,
                AuditTargetInput::document("doc-1"),
                vec![("permission", "write"), ("reason", "role_denied")],
            ),
            &mut store,
            &clock,
            &mut logger,
        )
        .expect("audit event recorded");

    assert_eq!(output.status(), RecordAuditEventStatus::Recorded);
    assert_eq!(output.retention_days(), 365);
    assert_eq!(store.events.len(), 1);
    let event = &store.events[0];
    assert_eq!(event.action(), AuditAction::PermissionDenied);
    assert_eq!(event.target().target_type(), "document");
    assert_eq!(event.target().target_id(), "doc-1");
    assert_eq!(event.metadata().value("permission"), Some("write"));
    assert_eq!(event.occurred_at().as_millis(), 123_456);
    assert!(logger.product.is_empty());
}

#[test]
fn record_audit_event_covers_review_publish_and_rejects_sensitive_metadata() {
    let policy = AuditRetentionPolicy::default();
    let usecase = RecordAuditEventUsecase::new(policy);
    let mut store = FakeAuditLogStore::default();
    let clock = FakeAuditClock {
        now: AuditTimestamp::from_millis(10),
    };
    let mut logger = FakeAuditLogger::default();

    usecase
        .execute(
            RecordAuditEventInput::new(
                "actor-1234",
                "workspace-1",
                "audit-review-1",
                AuditAction::ReviewApproved,
                AuditTargetInput::review_request("doc-1", "review-1"),
                vec![("workflow_state", "approved")],
            ),
            &mut store,
            &clock,
            &mut logger,
        )
        .expect("review audit recorded");
    usecase
        .execute(
            RecordAuditEventInput::new(
                "actor-1234",
                "workspace-1",
                "audit-publish-1",
                AuditAction::DocumentPublished,
                AuditTargetInput::document("doc-1"),
                vec![("source", "review")],
            ),
            &mut store,
            &clock,
            &mut logger,
        )
        .expect("publish audit recorded");

    let error = usecase
        .execute(
            RecordAuditEventInput::new(
                "actor-1234",
                "workspace-1",
                "audit-sensitive-1",
                AuditAction::DocumentPublished,
                AuditTargetInput::document("doc-1"),
                vec![("document_body", "secret")],
            ),
            &mut store,
            &clock,
            &mut logger,
        )
        .expect_err("sensitive metadata rejected");

    assert_eq!(store.events.len(), 2);
    assert_eq!(store.events[0].target().target_type(), "review_request");
    assert_eq!(store.events[1].action(), AuditAction::DocumentPublished);
    assert_eq!(error, AuditUsecaseError::InvalidMetadata);
}

#[test]
fn list_audit_events_requires_manage_permission_and_returns_cursor_page() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let actor = UserId::new("actor-1234").expect("actor id");
    let events = vec![
        audit_event("audit-1", &workspace_id, &actor, "doc-1"),
        audit_event("audit-2", &workspace_id, &actor, "doc-2"),
        audit_event("audit-3", &workspace_id, &actor, "doc-3"),
    ];
    let store = RecordingAuditLogStore::new(events);
    let checker = FakeAuditPermissionChecker::allowed();
    let mut logger = FakeAuditLogger::default();

    let output = ListAuditEventsUsecase::new(AuditRetentionPolicy::default())
        .execute(
            ListAuditEventsInput::new(
                "actor-1234",
                "workspace-1",
                ListAuditEventsScopeInput::workspace(),
                2,
                None,
            ),
            &checker,
            &store,
            &mut logger,
        )
        .expect("audit events listed");

    assert_eq!(checker.checked_permission(), Some(Permission::Manage));
    assert_eq!(output.events().len(), 2);
    assert_eq!(output.events()[0].event_id(), "audit-1");
    assert_eq!(output.next_cursor(), Some("2"));
    assert_eq!(output.retention_days(), 365);
    assert_eq!(logger.field_debug.len(), 1);
    assert_eq!(logger.field_debug[0].query_scope(), "workspace");
    assert!(!logger.field_debug[0].cursor_present());
    assert_eq!(logger.field_debug[0].result_count(), 2);
}

#[test]
fn list_audit_events_denies_unauthorized_query_before_store_access() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let actor = UserId::new("actor-1234").expect("actor id");
    let store =
        RecordingAuditLogStore::new(vec![audit_event("audit-1", &workspace_id, &actor, "doc-1")]);
    let checker = FakeAuditPermissionChecker::denied();
    let mut logger = FakeAuditLogger::default();

    let error = ListAuditEventsUsecase::new(AuditRetentionPolicy::default())
        .execute(
            ListAuditEventsInput::new(
                "actor-1234",
                "workspace-1",
                ListAuditEventsScopeInput::workspace(),
                50,
                None,
            ),
            &checker,
            &store,
            &mut logger,
        )
        .expect_err("query denied");

    assert_eq!(error, AuditUsecaseError::Unauthorized);
    assert!(!store.list_called());
    assert_eq!(logger.product.len(), 1);
    assert_eq!(logger.product[0].event_name(), "audit.query.denied");
}

#[test]
fn list_audit_events_rejects_invalid_cursor_without_permission_side_effects() {
    let store = RecordingAuditLogStore::new(Vec::new());
    let checker = FakeAuditPermissionChecker::allowed();
    let mut logger = FakeAuditLogger::default();

    let error = ListAuditEventsUsecase::new(AuditRetentionPolicy::default())
        .execute(
            ListAuditEventsInput::new(
                "actor-1234",
                "workspace-1",
                ListAuditEventsScopeInput::actor("actor-1234"),
                50,
                Some("invalid"),
            ),
            &checker,
            &store,
            &mut logger,
        )
        .expect_err("invalid cursor");

    assert_eq!(error, AuditUsecaseError::InvalidCursor);
    assert_eq!(checker.checked_permission(), None);
    assert!(!store.list_called());
}

#[test]
fn record_audit_event_logs_product_event_when_store_fails() {
    let policy = AuditRetentionPolicy::default();
    let usecase = RecordAuditEventUsecase::new(policy);
    let mut store = FakeAuditLogStore {
        append_error: Some(AuditLogStoreError::StorageUnavailable),
        ..FakeAuditLogStore::default()
    };
    let clock = FakeAuditClock {
        now: AuditTimestamp::from_millis(10),
    };
    let mut logger = FakeAuditLogger::default();

    let error = usecase
        .execute(
            RecordAuditEventInput::new(
                "actor-1234",
                "workspace-1",
                "audit-1",
                AuditAction::DocumentPublished,
                AuditTargetInput::document("doc-1"),
                vec![("source", "review")],
            ),
            &mut store,
            &clock,
            &mut logger,
        )
        .expect_err("store failure");

    assert_eq!(error, AuditUsecaseError::StoreUnavailable);
    assert_eq!(logger.product.len(), 1);
    assert_eq!(logger.product[0].event_name(), "audit.store.failed");
}

fn audit_event(
    event_id: &str,
    workspace_id: &WorkspaceId,
    actor_user_id: &UserId,
    document_id: &str,
) -> AuditEvent {
    AuditEvent::new(
        cabinet_domain::audit::AuditEventId::new(event_id).expect("audit id"),
        workspace_id.clone(),
        cabinet_domain::audit::AuditActor::user(actor_user_id.clone()),
        AuditAction::DocumentPublished,
        cabinet_domain::audit::AuditTarget::document(
            cabinet_domain::document::DocumentId::new(document_id).expect("document id"),
        ),
        cabinet_domain::audit::AuditMetadata::new([("source", "test")]).expect("metadata"),
        AuditTimestamp::from_millis(1_000),
    )
}
