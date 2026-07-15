use std::fs;
use std::path::PathBuf;

use cabinet_adapters::local_comment_repository::LocalCommentRepository;
use cabinet_domain::comment::{
    Comment, CommentBody, CommentBodyPolicy, CommentId, CommentThread, CommentThreadId,
    CommentThreadState, InlineCommentAnchor, InlineCommentRange,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::comment_repository::{CommentRepository, CommentRepositoryError};

#[test]
fn local_comment_repository_persists_thread_and_document_index_across_instances() {
    let root = unique_temp_dir("local-comment-repository-persist");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let thread = inline_thread("thread-1", &workspace_id, &document_id);

    {
        let mut repository = LocalCommentRepository::new(root.clone());
        repository
            .save_thread(&workspace_id, thread.clone())
            .expect("save thread");
    }

    let repository = LocalCommentRepository::new(root.clone());
    let loaded = repository
        .get_thread(&workspace_id, thread.id())
        .expect("get thread")
        .expect("thread");
    let listed = repository
        .list_document_threads(&workspace_id, &document_id)
        .expect("list document threads");

    assert_eq!(loaded.id(), thread.id());
    assert_eq!(loaded.inline_anchor(), thread.inline_anchor());
    assert_eq!(loaded.comments(), thread.comments());
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id(), thread.id());
    assert!(!format!("{repository:?}").contains("first body"));
    cleanup_temp_dir(root);
}

#[test]
fn local_comment_repository_appends_comments_and_updates_thread_state_durably() {
    let root = unique_temp_dir("local-comment-repository-mutate");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let thread_id = CommentThreadId::new("thread-1").expect("thread id");
    let mut repository = LocalCommentRepository::new(root.clone());

    repository
        .save_thread(
            &workspace_id,
            CommentThread::new(
                thread_id.clone(),
                workspace_id.clone(),
                document_id,
                comment("comment-1", "author-1", "first body"),
            ),
        )
        .expect("save thread");
    let appended = repository
        .append_comment(
            &workspace_id,
            &thread_id,
            comment("comment-2", "author-2", "reply body"),
        )
        .expect("append")
        .expect("thread");
    let resolved = repository
        .update_thread_state(&workspace_id, &thread_id, CommentThreadState::Resolved)
        .expect("update")
        .expect("thread");
    let missing = repository
        .append_comment(
            &workspace_id,
            &CommentThreadId::new("missing-thread").expect("thread id"),
            comment("comment-3", "author-3", "ignored body"),
        )
        .expect("append missing");

    let restarted = LocalCommentRepository::new(root.clone());
    let loaded = restarted
        .get_thread(&workspace_id, &thread_id)
        .expect("get updated")
        .expect("thread");

    assert_eq!(appended.comments().len(), 2);
    assert_eq!(resolved.state(), CommentThreadState::Resolved);
    assert!(missing.is_none());
    assert_eq!(loaded.comments().len(), 2);
    assert_eq!(loaded.state(), CommentThreadState::Resolved);
    cleanup_temp_dir(root);
}

#[test]
fn local_comment_repository_reports_corrupted_thread_file() {
    let root = unique_temp_dir("local-comment-repository-corrupt");
    let workspace_id = WorkspaceId::new("workspace-1").expect("workspace id");
    let document_id = DocumentId::new("doc-1").expect("document id");
    let thread = CommentThread::new(
        CommentThreadId::new("thread-1").expect("thread id"),
        workspace_id.clone(),
        document_id,
        comment("comment-1", "author-1", "first body"),
    );
    let mut repository = LocalCommentRepository::new(root.clone());
    repository
        .save_thread(&workspace_id, thread.clone())
        .expect("save thread");

    fs::write(
        first_file_under(&root.join("comments"), "thread"),
        "not-a-thread-record",
    )
    .expect("corrupt thread file");
    let error = repository
        .get_thread(&workspace_id, thread.id())
        .expect_err("corrupted thread must fail");

    assert_eq!(error, CommentRepositoryError::CorruptedState);
    cleanup_temp_dir(root);
}

fn inline_thread(
    thread_id: &str,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
) -> CommentThread {
    let anchor = InlineCommentAnchor::new(
        document_id.clone(),
        VersionId::new("version-1").expect("version id"),
        InlineCommentRange::new(2, 8).expect("range"),
    );
    CommentThread::new_inline(
        CommentThreadId::new(thread_id).expect("thread id"),
        workspace_id.clone(),
        document_id.clone(),
        anchor,
        comment("comment-1", "author-1", "first body"),
    )
}

fn comment(id: &str, author: &str, body: &str) -> Comment {
    Comment::new(
        CommentId::new(id).expect("comment id"),
        UserId::new(author).expect("user id"),
        CommentBody::new(body, CommentBodyPolicy::new(1024).expect("policy")).expect("body"),
    )
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
