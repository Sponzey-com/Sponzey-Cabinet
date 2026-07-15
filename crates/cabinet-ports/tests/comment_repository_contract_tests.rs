use std::collections::HashMap;

use cabinet_domain::comment::{
    Comment, CommentBody, CommentBodyPolicy, CommentId, CommentThread, CommentThreadId,
    CommentThreadState, InlineCommentAnchor, InlineCommentRange,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::permission::{AccessResource, Permission, PermissionDecision};
use cabinet_domain::user::UserId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::comment_repository::{
    CommentPermissionCheckError, CommentPermissionChecker, CommentRepository,
    CommentRepositoryError, InlineAnchorDocumentLookup, InlineAnchorDocumentLookupError,
    InlineAnchorDocumentState,
};

#[derive(Default)]
struct FakeCommentRepository {
    threads: HashMap<String, CommentThread>,
}

impl CommentRepository for FakeCommentRepository {
    fn save_thread(
        &mut self,
        workspace_id: &WorkspaceId,
        thread: CommentThread,
    ) -> Result<(), CommentRepositoryError> {
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

struct FakePermissionChecker;

impl CommentPermissionChecker for FakePermissionChecker {
    fn check_permission(
        &self,
        _actor_user_id: &UserId,
        _resource: &AccessResource,
        _permission: Permission,
    ) -> Result<PermissionDecision, CommentPermissionCheckError> {
        Ok(PermissionDecision::allowed(
            cabinet_domain::permission::PolicySource::Workspace,
            cabinet_domain::permission::PermissionDecisionReason::RoleAllowsPermission,
        ))
    }
}

#[derive(Default)]
struct FakeInlineAnchorDocumentLookup {
    states: HashMap<String, InlineAnchorDocumentState>,
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
fn comment_repository_contract_saves_appends_lists_and_updates_thread_state() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let thread_id = CommentThreadId::new("thread-1").expect("thread id");
    let mut repository = FakeCommentRepository::default();

    repository
        .save_thread(
            &workspace_id,
            CommentThread::new(
                thread_id.clone(),
                workspace_id.clone(),
                document_id.clone(),
                comment("comment-1", "author-1", "first"),
            ),
        )
        .expect("save thread");
    let appended = repository
        .append_comment(
            &workspace_id,
            &thread_id,
            comment("comment-2", "author-2", "reply"),
        )
        .expect("append")
        .expect("thread");
    let resolved = repository
        .update_thread_state(&workspace_id, &thread_id, CommentThreadState::Resolved)
        .expect("update")
        .expect("thread");
    let listed = repository
        .list_document_threads(&workspace_id, &document_id)
        .expect("list");

    assert_eq!(appended.comments().len(), 2);
    assert_eq!(resolved.state(), CommentThreadState::Resolved);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id().as_str(), "thread-1");
}

#[test]
fn comment_permission_checker_contract_returns_domain_decision_without_external_types() {
    let checker = FakePermissionChecker;
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let actor_id = UserId::new("actor-1").expect("actor id");

    let decision = checker
        .check_permission(
            &actor_id,
            &AccessResource::document(workspace_id, None, document_id),
            Permission::Write,
        )
        .expect("permission decision");

    assert_eq!(
        decision.result(),
        cabinet_domain::permission::PermissionDecisionResult::Allowed
    );
}

#[test]
fn inline_comment_contract_persists_anchor_and_reads_version_state_without_editor_types() {
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let version_id = VersionId::new("version-1").expect("version id");
    let thread_id = CommentThreadId::new("thread-1").expect("thread id");
    let anchor = InlineCommentAnchor::new(
        document_id.clone(),
        version_id.clone(),
        InlineCommentRange::new(2, 8).expect("range"),
    );
    let mut repository = FakeCommentRepository::default();
    let mut lookup = FakeInlineAnchorDocumentLookup::default();

    lookup.insert("workspace-1", "doc-1", "version-1", "version-1", 20);
    repository
        .save_thread(
            &workspace_id,
            CommentThread::new_inline(
                thread_id.clone(),
                workspace_id.clone(),
                document_id.clone(),
                anchor.clone(),
                comment("comment-1", "author-1", "inline body"),
            ),
        )
        .expect("save inline thread");
    let loaded = repository
        .get_thread(&workspace_id, &thread_id)
        .expect("load thread")
        .expect("thread");
    let state = lookup
        .get_anchor_document_state(&workspace_id, &document_id, &version_id)
        .expect("version state")
        .expect("state");

    assert_eq!(loaded.inline_anchor(), Some(&anchor));
    assert_eq!(state.current_version_id().as_str(), "version-1");
    assert_eq!(state.anchored_document_len(), 20);
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
