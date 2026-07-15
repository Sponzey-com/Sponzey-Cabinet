use cabinet_domain::comment::{
    Comment, CommentBody, CommentBodyPolicy, CommentError, CommentId, CommentThread,
    CommentThreadEvent, CommentThreadId, CommentThreadState, InlineAnchorResolutionStatus,
    InlineCommentAnchor, InlineCommentRange, InlineCommentVersionRelation,
    resolve_inline_comment_anchor, transition_comment_thread,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;

#[test]
fn comment_body_policy_rejects_empty_and_too_large_body() {
    let policy = CommentBodyPolicy::new(12).expect("policy");

    let body = CommentBody::new("short body", policy).expect("body");
    let empty = CommentBody::new("   ", policy).expect_err("empty body");
    let too_large = CommentBody::new("this body is too long", policy).expect_err("too large");

    assert_eq!(policy.max_bytes(), 12);
    assert_eq!(body.as_str(), "short body");
    assert_eq!(empty, CommentError::EmptyBody);
    assert_eq!(too_large, CommentError::BodyTooLarge { max_bytes: 12 });
}

#[test]
fn comment_thread_starts_open_and_appends_only_when_not_resolved() {
    let thread = CommentThread::new(
        thread_id("thread-1"),
        workspace_id("workspace-1"),
        document_id("doc-1"),
        comment("comment-1", "author-1", "first comment"),
    );
    let with_reply = thread
        .add_comment(comment("comment-2", "author-2", "reply"))
        .expect("reply");
    let resolved = with_reply
        .transition(CommentThreadEvent::Resolve)
        .expect("resolve");
    let add_after_resolved = resolved
        .add_comment(comment("comment-3", "author-3", "late reply"))
        .expect_err("resolved thread rejects new comment");

    assert_eq!(thread.state(), CommentThreadState::Open);
    assert_eq!(with_reply.comments().len(), 2);
    assert_eq!(resolved.state(), CommentThreadState::Resolved);
    assert_eq!(
        add_after_resolved,
        CommentError::InvalidThreadTransition {
            state: CommentThreadState::Resolved,
            event: CommentThreadEvent::AddComment,
        }
    );
}

#[test]
fn comment_thread_state_machine_supports_resolve_and_reopen() {
    let resolved = transition_comment_thread(CommentThreadState::Open, CommentThreadEvent::Resolve)
        .expect("resolve");
    let reopened =
        transition_comment_thread(resolved.next_state, CommentThreadEvent::Reopen).expect("reopen");
    let invalid = transition_comment_thread(CommentThreadState::Open, CommentThreadEvent::Reopen)
        .expect_err("cannot reopen open thread");

    assert_eq!(resolved.previous_state, CommentThreadState::Open);
    assert_eq!(resolved.next_state, CommentThreadState::Resolved);
    assert_eq!(reopened.previous_state, CommentThreadState::Resolved);
    assert_eq!(reopened.next_state, CommentThreadState::Reopened);
    assert_eq!(
        invalid,
        CommentError::InvalidThreadTransition {
            state: CommentThreadState::Open,
            event: CommentThreadEvent::Reopen,
        }
    );
}

#[test]
fn inline_comment_anchor_rejects_invalid_range_without_editor_state() {
    let range = InlineCommentRange::new(4, 9).expect("range");
    let invalid = InlineCommentRange::new(9, 4).expect_err("reversed range");
    let anchor = InlineCommentAnchor::new(document_id("doc-1"), version_id("version-1"), range);

    assert_eq!(anchor.document_id().as_str(), "doc-1");
    assert_eq!(anchor.version_id().as_str(), "version-1");
    assert_eq!(anchor.range().start_offset(), 4);
    assert_eq!(anchor.range().end_offset(), 9);
    assert_eq!(anchor.range().len(), 5);
    assert_eq!(invalid, CommentError::InvalidInlineAnchorRange);
}

#[test]
fn inline_comment_anchor_resolution_reports_valid_stale_invalid_and_missing_statuses() {
    let valid = InlineCommentAnchor::new(
        document_id("doc-1"),
        version_id("version-1"),
        InlineCommentRange::new(1, 4).expect("range"),
    );
    let stale = InlineCommentAnchor::new(
        document_id("doc-1"),
        version_id("version-old"),
        InlineCommentRange::new(1, 4).expect("range"),
    );
    let outside_document = InlineCommentAnchor::new(
        document_id("doc-1"),
        version_id("version-1"),
        InlineCommentRange::new(7, 12).expect("range"),
    );

    let valid_resolution =
        resolve_inline_comment_anchor(&valid, true, Some(&version_id("version-1")), 10);
    let stale_resolution =
        resolve_inline_comment_anchor(&stale, true, Some(&version_id("version-1")), 10);
    let invalid_resolution =
        resolve_inline_comment_anchor(&outside_document, true, Some(&version_id("version-1")), 10);
    let missing_resolution = resolve_inline_comment_anchor(&valid, false, None, 0);

    assert_eq!(
        valid_resolution.status(),
        InlineAnchorResolutionStatus::Valid
    );
    assert_eq!(
        valid_resolution.version_relation(),
        InlineCommentVersionRelation::Current
    );
    assert_eq!(
        stale_resolution.status(),
        InlineAnchorResolutionStatus::Stale
    );
    assert_eq!(
        stale_resolution.version_relation(),
        InlineCommentVersionRelation::Stale
    );
    assert_eq!(
        invalid_resolution.status(),
        InlineAnchorResolutionStatus::InvalidRange
    );
    assert_eq!(
        missing_resolution.status(),
        InlineAnchorResolutionStatus::DocumentVersionMissing
    );
}

#[test]
fn inline_comment_thread_keeps_anchor_as_domain_value_object() {
    let anchor = InlineCommentAnchor::new(
        document_id("doc-1"),
        version_id("version-1"),
        InlineCommentRange::new(2, 8).expect("range"),
    );
    let thread = CommentThread::new_inline(
        thread_id("thread-1"),
        workspace_id("workspace-1"),
        document_id("doc-1"),
        anchor.clone(),
        comment("comment-1", "author-1", "body"),
    );

    assert_eq!(thread.inline_anchor(), Some(&anchor));
    assert_eq!(thread.comments().len(), 1);
}

fn comment(id: &str, author: &str, body: &str) -> Comment {
    Comment::new(
        CommentId::new(id).expect("comment id"),
        UserId::new(author).expect("user id"),
        CommentBody::new(body, CommentBodyPolicy::new(1024).expect("policy")).expect("body"),
    )
}

fn thread_id(value: &str) -> CommentThreadId {
    CommentThreadId::new(value).expect("thread id")
}

fn workspace_id(value: &str) -> WorkspaceId {
    WorkspaceId::new(value).expect("workspace id")
}

fn document_id(value: &str) -> DocumentId {
    DocumentId::new(value).expect("document id")
}

fn version_id(value: &str) -> VersionId {
    VersionId::new(value).expect("version id")
}
