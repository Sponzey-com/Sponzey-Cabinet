use std::cell::Cell;
use std::collections::HashMap;

use cabinet_domain::comment::{
    Comment, CommentBody, CommentBodyPolicy, CommentId, CommentThread, CommentThreadId,
    CommentThreadState,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{
    AccessResource, Permission, PermissionDecision, PermissionDecisionReason, PolicySource,
};
use cabinet_domain::user::UserId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::comment_repository::{
    CommentPermissionCheckError, CommentPermissionChecker, CommentRepository,
    CommentRepositoryError, InlineAnchorDocumentLookup, InlineAnchorDocumentLookupError,
    InlineAnchorDocumentState,
};
use cabinet_usecases::comment::{
    AddCommentInput, AddCommentUsecase, AddInlineCommentInput, AddInlineCommentUsecase,
    CommentFieldDebugEvent, CommentProductEvent, CommentUsecaseError, CommentUsecaseLogger,
    ListDocumentCommentsInput, ListDocumentCommentsUsecase, ReopenCommentInput,
    ReopenCommentUsecase, ResolveCommentInput, ResolveCommentUsecase,
};

#[derive(Default)]
struct FakePermissionChecker {
    decision_by_permission: Vec<(Permission, PermissionDecision)>,
    check_count: Cell<usize>,
}

impl FakePermissionChecker {
    fn allow(&mut self, permission: Permission) {
        self.decision_by_permission.push((
            permission,
            PermissionDecision::allowed(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleAllowsPermission,
            ),
        ));
    }

    fn deny(&mut self, permission: Permission) {
        self.decision_by_permission.push((
            permission,
            PermissionDecision::denied(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleDoesNotAllowPermission,
            ),
        ));
    }
}

impl CommentPermissionChecker for FakePermissionChecker {
    fn check_permission(
        &self,
        _actor_user_id: &UserId,
        _resource: &AccessResource,
        permission: Permission,
    ) -> Result<PermissionDecision, CommentPermissionCheckError> {
        self.check_count.set(self.check_count.get() + 1);
        Ok(self
            .decision_by_permission
            .iter()
            .rev()
            .find_map(|(candidate, decision)| (*candidate == permission).then_some(*decision))
            .unwrap_or(PermissionDecision::denied(
                PolicySource::Workspace,
                PermissionDecisionReason::RoleDoesNotAllowPermission,
            )))
    }
}

#[derive(Default)]
struct FakeCommentRepository {
    threads: HashMap<String, CommentThread>,
    save_count: Cell<usize>,
    append_count: Cell<usize>,
    list_count: Cell<usize>,
    state_update_count: Cell<usize>,
}

impl FakeCommentRepository {
    fn insert(&mut self, thread: CommentThread) {
        self.threads
            .insert(thread_key(thread.workspace_id(), thread.id()), thread);
    }
}

impl CommentRepository for FakeCommentRepository {
    fn save_thread(
        &mut self,
        workspace_id: &WorkspaceId,
        thread: CommentThread,
    ) -> Result<(), CommentRepositoryError> {
        self.save_count.set(self.save_count.get() + 1);
        self.threads
            .insert(thread_key(workspace_id, thread.id()), thread);
        Ok(())
    }

    fn get_thread(
        &self,
        workspace_id: &WorkspaceId,
        thread_id: &CommentThreadId,
    ) -> Result<Option<CommentThread>, CommentRepositoryError> {
        Ok(self
            .threads
            .get(&thread_key(workspace_id, thread_id))
            .cloned())
    }

    fn append_comment(
        &mut self,
        workspace_id: &WorkspaceId,
        thread_id: &CommentThreadId,
        comment: Comment,
    ) -> Result<Option<CommentThread>, CommentRepositoryError> {
        self.append_count.set(self.append_count.get() + 1);
        let key = thread_key(workspace_id, thread_id);
        let Some(thread) = self.threads.get(&key) else {
            return Ok(None);
        };
        let updated = thread
            .add_comment(comment)
            .map_err(|_| CommentRepositoryError::CorruptedState)?;
        self.threads.insert(key, updated.clone());
        Ok(Some(updated))
    }

    fn update_thread_state(
        &mut self,
        workspace_id: &WorkspaceId,
        thread_id: &CommentThreadId,
        state: CommentThreadState,
    ) -> Result<Option<CommentThread>, CommentRepositoryError> {
        self.state_update_count
            .set(self.state_update_count.get() + 1);
        let key = thread_key(workspace_id, thread_id);
        let Some(thread) = self.threads.get(&key) else {
            return Ok(None);
        };
        let updated = thread.with_state(state);
        self.threads.insert(key, updated.clone());
        Ok(Some(updated))
    }

    fn list_document_threads(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Vec<CommentThread>, CommentRepositoryError> {
        self.list_count.set(self.list_count.get() + 1);
        Ok(self
            .threads
            .values()
            .filter(|thread| {
                thread.workspace_id() == workspace_id && thread.document_id() == document_id
            })
            .cloned()
            .collect())
    }
}

#[derive(Default)]
struct FakeCommentLogger {
    product_events: Vec<CommentProductEvent>,
    field_debug_events: Vec<CommentFieldDebugEvent>,
}

impl CommentUsecaseLogger for FakeCommentLogger {
    fn write_product(&mut self, event: CommentProductEvent) {
        self.product_events.push(event);
    }

    fn write_field_debug(&mut self, event: CommentFieldDebugEvent) {
        self.field_debug_events.push(event);
    }
}

#[derive(Default)]
struct FakeInlineAnchorDocumentLookup {
    states: HashMap<String, InlineAnchorDocumentState>,
    lookup_count: Cell<usize>,
}

impl FakeInlineAnchorDocumentLookup {
    fn insert(
        &mut self,
        workspace_id: &str,
        document_id: &str,
        version_id: &str,
        current_version_id: &str,
        body_len: usize,
    ) {
        self.states.insert(
            version_key(workspace_id, document_id, version_id),
            InlineAnchorDocumentState::new(
                VersionId::new(current_version_id).expect("current version id"),
                body_len,
            ),
        );
    }
}

impl InlineAnchorDocumentLookup for FakeInlineAnchorDocumentLookup {
    fn get_anchor_document_state(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        version_id: &VersionId,
    ) -> Result<Option<InlineAnchorDocumentState>, InlineAnchorDocumentLookupError> {
        self.lookup_count.set(self.lookup_count.get() + 1);
        Ok(self
            .states
            .get(&version_key(
                workspace_id.as_str(),
                document_id.as_str(),
                version_id.as_str(),
            ))
            .cloned())
    }
}

#[test]
fn list_document_comments_requires_read_permission_before_repository_lookup() {
    let mut checker = FakePermissionChecker::default();
    checker.deny(Permission::Read);
    let repository = FakeCommentRepository::default();
    let mut logger = FakeCommentLogger::default();

    let error = ListDocumentCommentsUsecase::new()
        .execute(
            ListDocumentCommentsInput::new("actor-1", "workspace-1", "doc-1"),
            &checker,
            &repository,
            &mut logger,
        )
        .expect_err("unauthorized list");

    assert_eq!(error, CommentUsecaseError::Unauthorized);
    assert_eq!(repository.list_count.get(), 0);
}

#[test]
fn list_document_comments_returns_threads_for_reader_without_body_logs() {
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Read);
    let mut repository = FakeCommentRepository::default();
    repository.insert(CommentThread::new(
        CommentThreadId::new("thread-1").expect("thread id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        DocumentId::new("doc-1").expect("document id"),
        comment("comment-1", "actor-1", "visible body"),
    ));
    let mut logger = FakeCommentLogger::default();

    let output = ListDocumentCommentsUsecase::new()
        .execute(
            ListDocumentCommentsInput::new("actor-1", "workspace-1", "doc-1"),
            &checker,
            &repository,
            &mut logger,
        )
        .expect("list comments");

    assert_eq!(output.threads().len(), 1);
    assert_eq!(output.threads()[0].id().as_str(), "thread-1");
    assert_eq!(repository.list_count.get(), 1);
    assert!(!format!("{:?}", logger.product_events).contains("visible body"));
    assert!(!format!("{:?}", logger.field_debug_events).contains("visible body"));
}

#[test]
fn add_comment_requires_write_permission_and_never_logs_body() {
    let mut checker = FakePermissionChecker::default();
    checker.deny(Permission::Write);
    let mut repository = FakeCommentRepository::default();
    let mut logger = FakeCommentLogger::default();

    let error = AddCommentUsecase::new(CommentBodyPolicy::new(256).expect("policy"))
        .execute(
            AddCommentInput::new(
                "actor-1",
                "workspace-1",
                "doc-1",
                "thread-1",
                "comment-1",
                "secret comment body",
            ),
            &checker,
            &mut repository,
            &mut logger,
        )
        .expect_err("unauthorized add");

    assert_eq!(error, CommentUsecaseError::Unauthorized);
    assert_eq!(repository.save_count.get(), 0);
    assert_eq!(repository.append_count.get(), 0);
    assert!(
        !format!("{:?}", logger.product_events).contains("secret comment body"),
        "product log must not include comment body"
    );
}

#[test]
fn add_comment_creates_thread_or_appends_to_open_thread_with_safe_product_log() {
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Write);
    let mut repository = FakeCommentRepository::default();
    let mut logger = FakeCommentLogger::default();
    let usecase = AddCommentUsecase::new(CommentBodyPolicy::new(256).expect("policy"));

    let created = usecase
        .execute(
            AddCommentInput::new(
                "actor-1",
                "workspace-1",
                "doc-1",
                "thread-1",
                "comment-1",
                "first comment",
            ),
            &checker,
            &mut repository,
            &mut logger,
        )
        .expect("create thread");
    let appended = usecase
        .execute(
            AddCommentInput::new(
                "actor-2",
                "workspace-1",
                "doc-1",
                "thread-1",
                "comment-2",
                "reply body",
            ),
            &checker,
            &mut repository,
            &mut logger,
        )
        .expect("append comment");

    assert_eq!(created.thread().state(), CommentThreadState::Open);
    assert_eq!(created.thread().comments().len(), 1);
    assert_eq!(appended.thread().comments().len(), 2);
    assert!(logger.product_events.iter().any(|event| {
        matches!(
            event,
            CommentProductEvent::CommentAdded {
                masked_actor_id,
                document_id,
                thread_id,
                comment_count: 2
            } if masked_actor_id == "masked:or-2"
                && document_id == "doc-1"
                && thread_id == "thread-1"
        )
    }));
    assert!(!format!("{:?}", logger.product_events).contains("reply body"));
}

#[test]
fn add_comment_rejects_body_too_large_before_permission_check() {
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Write);
    let mut repository = FakeCommentRepository::default();
    let mut logger = FakeCommentLogger::default();

    let error = AddCommentUsecase::new(CommentBodyPolicy::new(4).expect("policy"))
        .execute(
            AddCommentInput::new(
                "actor-1",
                "workspace-1",
                "doc-1",
                "thread-1",
                "comment-1",
                "too large",
            ),
            &checker,
            &mut repository,
            &mut logger,
        )
        .expect_err("body too large");

    assert_eq!(error, CommentUsecaseError::BodyTooLarge);
    assert_eq!(checker.check_count.get(), 0);
    assert_eq!(repository.save_count.get(), 0);
}

#[test]
fn add_inline_comment_creates_anchored_thread_for_current_valid_range() {
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Write);
    let mut repository = FakeCommentRepository::default();
    let mut documents = FakeInlineAnchorDocumentLookup::default();
    documents.insert("workspace-1", "doc-1", "version-1", "version-1", 20);
    let mut logger = FakeCommentLogger::default();

    let output = AddInlineCommentUsecase::new(CommentBodyPolicy::new(256).expect("policy"))
        .execute(
            AddInlineCommentInput::new(
                "actor-1",
                "workspace-1",
                "doc-1",
                "version-1",
                4,
                9,
                "thread-1",
                "comment-1",
                "selected text must not be logged",
            ),
            &checker,
            &documents,
            &mut repository,
            &mut logger,
        )
        .expect("inline comment");

    let anchor = output.thread().inline_anchor().expect("inline anchor");
    assert_eq!(anchor.version_id().as_str(), "version-1");
    assert_eq!(anchor.range().start_offset(), 4);
    assert_eq!(anchor.range().end_offset(), 9);
    assert_eq!(repository.save_count.get(), 1);
    assert_eq!(documents.lookup_count.get(), 1);
    assert!(logger.product_events.iter().any(|event| {
        matches!(
            event,
            CommentProductEvent::InlineCommentAdded {
                masked_actor_id,
                document_id,
                thread_id,
                anchor_status: "valid",
                comment_count: 1,
                ..
            } if masked_actor_id == "masked:or-1"
                && document_id == "doc-1"
                && thread_id == "thread-1"
        )
    }));
    assert!(logger.field_debug_events.iter().any(|event| {
        event.has_anchor()
            && event.anchor_status() == "valid"
            && event.range_length() == Some(5)
            && event.version_relation() == Some("current")
    }));
    assert!(!format!("{:?}", logger.product_events).contains("selected text"));
    assert!(!format!("{:?}", logger.field_debug_events).contains("selected text"));
    assert!(!format!("{:?}", logger.product_events).contains("version-1"));
}

#[test]
fn add_inline_comment_returns_stale_anchor_without_saving_thread() {
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Write);
    let mut repository = FakeCommentRepository::default();
    let mut documents = FakeInlineAnchorDocumentLookup::default();
    documents.insert("workspace-1", "doc-1", "version-old", "version-current", 20);
    let mut logger = FakeCommentLogger::default();

    let error = AddInlineCommentUsecase::new(CommentBodyPolicy::new(256).expect("policy"))
        .execute(
            AddInlineCommentInput::new(
                "actor-1",
                "workspace-1",
                "doc-1",
                "version-old",
                4,
                9,
                "thread-1",
                "comment-1",
                "stale selected text",
            ),
            &checker,
            &documents,
            &mut repository,
            &mut logger,
        )
        .expect_err("stale anchor");

    assert_eq!(error, CommentUsecaseError::StaleAnchor);
    assert_eq!(repository.save_count.get(), 0);
    assert!(logger.product_events.iter().any(|event| {
        matches!(
            event,
            CommentProductEvent::InlineCommentAnchorStale {
                document_id,
                anchor_status: "stale",
                ..
            } if document_id == "doc-1"
        )
    }));
    assert!(!format!("{:?}", logger.product_events).contains("stale selected text"));
}

#[test]
fn add_inline_comment_rejects_missing_version_invalid_range_and_unauthorized() {
    let mut allowed = FakePermissionChecker::default();
    allowed.allow(Permission::Write);
    let mut denied = FakePermissionChecker::default();
    denied.deny(Permission::Write);
    let mut repository = FakeCommentRepository::default();
    let mut documents = FakeInlineAnchorDocumentLookup::default();
    documents.insert("workspace-1", "doc-1", "version-1", "version-1", 10);
    let mut logger = FakeCommentLogger::default();
    let usecase = AddInlineCommentUsecase::new(CommentBodyPolicy::new(256).expect("policy"));

    let invalid_range = usecase
        .execute(
            AddInlineCommentInput::new(
                "actor-1",
                "workspace-1",
                "doc-1",
                "version-1",
                8,
                4,
                "thread-1",
                "comment-1",
                "body",
            ),
            &allowed,
            &documents,
            &mut repository,
            &mut logger,
        )
        .expect_err("invalid range");
    let unauthorized = usecase
        .execute(
            AddInlineCommentInput::new(
                "actor-1",
                "workspace-1",
                "doc-1",
                "version-1",
                2,
                4,
                "thread-1",
                "comment-1",
                "body",
            ),
            &denied,
            &documents,
            &mut repository,
            &mut logger,
        )
        .expect_err("unauthorized");
    let missing_version = usecase
        .execute(
            AddInlineCommentInput::new(
                "actor-1",
                "workspace-1",
                "doc-1",
                "missing-version",
                2,
                4,
                "thread-1",
                "comment-1",
                "body",
            ),
            &allowed,
            &documents,
            &mut repository,
            &mut logger,
        )
        .expect_err("missing version");

    assert_eq!(invalid_range, CommentUsecaseError::InvalidAnchorRange);
    assert_eq!(unauthorized, CommentUsecaseError::Unauthorized);
    assert_eq!(missing_version, CommentUsecaseError::DocumentVersionMissing);
    assert_eq!(repository.save_count.get(), 0);
}

#[test]
fn resolve_and_reopen_comment_thread_follow_state_machine() {
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Write);
    let mut repository = FakeCommentRepository::default();
    repository.insert(CommentThread::new(
        CommentThreadId::new("thread-1").expect("thread id"),
        WorkspaceId::new("workspace-1").expect("workspace id"),
        DocumentId::new("doc-1").expect("document id"),
        comment("comment-1", "actor-1", "first"),
    ));
    let mut logger = FakeCommentLogger::default();

    let resolved = ResolveCommentUsecase::new()
        .execute(
            ResolveCommentInput::new("actor-1", "workspace-1", "doc-1", "thread-1"),
            &checker,
            &mut repository,
            &mut logger,
        )
        .expect("resolve");
    let invalid_resolve = ResolveCommentUsecase::new()
        .execute(
            ResolveCommentInput::new("actor-1", "workspace-1", "doc-1", "thread-1"),
            &checker,
            &mut repository,
            &mut logger,
        )
        .expect_err("resolved cannot resolve again");
    let reopened = ReopenCommentUsecase::new()
        .execute(
            ReopenCommentInput::new("actor-1", "workspace-1", "doc-1", "thread-1"),
            &checker,
            &mut repository,
            &mut logger,
        )
        .expect("reopen");

    assert_eq!(resolved.thread().state(), CommentThreadState::Resolved);
    assert_eq!(invalid_resolve, CommentUsecaseError::InvalidTransition);
    assert_eq!(reopened.thread().state(), CommentThreadState::Reopened);
    assert!(logger.product_events.iter().any(|event| {
        matches!(
            event,
            CommentProductEvent::CommentResolved {
                document_id,
                thread_id,
                comment_count: 1,
                ..
            } if document_id == "doc-1" && thread_id == "thread-1"
        )
    }));
    assert!(logger.product_events.iter().any(|event| {
        matches!(
            event,
            CommentProductEvent::CommentReopened {
                document_id,
                thread_id,
                comment_count: 1,
                ..
            } if document_id == "doc-1" && thread_id == "thread-1"
        )
    }));
}

#[test]
fn resolve_missing_thread_returns_stable_not_found_error() {
    let mut checker = FakePermissionChecker::default();
    checker.allow(Permission::Write);
    let mut repository = FakeCommentRepository::default();
    let mut logger = FakeCommentLogger::default();

    let error = ResolveCommentUsecase::new()
        .execute(
            ResolveCommentInput::new("actor-1", "workspace-1", "doc-1", "missing-thread"),
            &checker,
            &mut repository,
            &mut logger,
        )
        .expect_err("missing thread");

    assert_eq!(error, CommentUsecaseError::ThreadNotFound);
    assert_eq!(repository.state_update_count.get(), 0);
    assert!(logger.product_events.iter().any(|event| {
        matches!(
            event,
            CommentProductEvent::UsecaseFailed {
                error_code: "COMMENT_THREAD_NOT_FOUND"
            }
        )
    }));
}

fn comment(id: &str, author: &str, body: &str) -> Comment {
    Comment::new(
        CommentId::new(id).expect("comment id"),
        UserId::new(author).expect("user id"),
        CommentBody::new(body, CommentBodyPolicy::new(1024).expect("policy")).expect("body"),
    )
}

fn thread_key(workspace_id: &WorkspaceId, thread_id: &CommentThreadId) -> String {
    format!("{}:{}", workspace_id.as_str(), thread_id.as_str())
}

fn version_key(workspace_id: &str, document_id: &str, version_id: &str) -> String {
    format!("{workspace_id}:{document_id}:{version_id}")
}
