use std::fs;
use std::path::PathBuf;

use cabinet_adapters::local_audit_log_store::LocalAuditLogStore;
use cabinet_domain::audit::{
    AuditAction, AuditActor, AuditEvent, AuditEventId, AuditMetadata, AuditTarget, AuditTimestamp,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::audit_log::{
    AuditListQuery, AuditListScope, AuditLogStore, AuditLogStoreError, AuditPageRequest,
};

#[test]
fn local_audit_log_store_persists_workspace_events_with_cursor_pagination() {
    let root = unique_temp_dir("local-audit-log-pagination");
    let workspace_id = workspace_id("workspace-1");
    let actor = user_id("actor-1");

    {
        let mut store = LocalAuditLogStore::new(root.clone());
        store
            .append_audit_event(event("audit-1", &workspace_id, &actor, "doc-1", 1_000))
            .expect("append first event");
        store
            .append_audit_event(event("audit-2", &workspace_id, &actor, "doc-2", 1_001))
            .expect("append second event");
        store
            .append_audit_event(event("audit-3", &workspace_id, &actor, "doc-3", 1_002))
            .expect("append third event");
    }

    let store = LocalAuditLogStore::new(root.clone());
    let first_page = store
        .list_audit_events(AuditListQuery::workspace(
            workspace_id.clone(),
            AuditPageRequest::new(2, None).expect("page request"),
        ))
        .expect("first page");

    assert_eq!(first_page.events().len(), 2);
    assert_eq!(first_page.events()[0].event_id().as_str(), "audit-1");
    assert_eq!(first_page.events()[1].event_id().as_str(), "audit-2");
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
    assert!(!format!("{store:?}").contains("actor-1"));
    cleanup_temp_dir(root);
}

#[test]
fn local_audit_log_store_filters_actor_and_target_scopes_across_instances() {
    let root = unique_temp_dir("local-audit-log-scopes");
    let workspace_id = workspace_id("workspace-1");
    let actor_1 = user_id("actor-1");
    let actor_2 = user_id("actor-2");

    {
        let mut store = LocalAuditLogStore::new(root.clone());
        store
            .append_audit_event(event("audit-1", &workspace_id, &actor_1, "doc-1", 1_000))
            .expect("append actor 1");
        store
            .append_audit_event(event("audit-2", &workspace_id, &actor_2, "doc-2", 1_001))
            .expect("append actor 2");
    }

    let store = LocalAuditLogStore::new(root.clone());
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
    assert!(!format!("{store:?}").contains("doc-2"));
    cleanup_temp_dir(root);
}

#[test]
fn local_audit_log_store_reports_corrupted_audit_event_file() {
    let root = unique_temp_dir("local-audit-log-corrupt");
    let workspace_id = workspace_id("workspace-1");
    let actor = user_id("actor-1");
    let mut store = LocalAuditLogStore::new(root.clone());
    store
        .append_audit_event(event("audit-1", &workspace_id, &actor, "doc-1", 1_000))
        .expect("append event");

    fs::write(
        first_file_under(&root.join("audit-log"), "event"),
        "not-an-audit-event",
    )
    .expect("corrupt audit event file");
    let error = store
        .list_audit_events(AuditListQuery::workspace(
            workspace_id,
            AuditPageRequest::new(50, None).expect("page request"),
        ))
        .expect_err("corrupted audit event must fail");

    assert_eq!(error, AuditLogStoreError::CorruptedState);
    cleanup_temp_dir(root);
}

fn event(
    event_id: &str,
    workspace_id: &WorkspaceId,
    actor_user_id: &UserId,
    document_id: &str,
    occurred_at: u64,
) -> AuditEvent {
    AuditEvent::new(
        AuditEventId::new(event_id).expect("audit id"),
        workspace_id.clone(),
        AuditActor::user(actor_user_id.clone()),
        AuditAction::DocumentPublished,
        AuditTarget::document(DocumentId::new(document_id).expect("document id")),
        AuditMetadata::new([("source", "adapter")]).expect("metadata"),
        AuditTimestamp::from_millis(occurred_at),
    )
}

fn workspace_id(value: &str) -> WorkspaceId {
    WorkspaceId::new(value).expect("workspace id")
}

fn user_id(value: &str) -> UserId {
    UserId::new(value).expect("user id")
}

fn first_file_under(root: &PathBuf, extension: &str) -> PathBuf {
    let mut stack = vec![root.clone()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(path).expect("read dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                return path;
            }
        }
    }
    panic!("file with extension {extension} not found");
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sponzey-cabinet-{name}-{}", std::process::id()));
    cleanup_temp_dir(dir.clone());
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn cleanup_temp_dir(dir: PathBuf) {
    if dir.exists() {
        fs::remove_dir_all(dir).expect("remove temp dir");
    }
}
