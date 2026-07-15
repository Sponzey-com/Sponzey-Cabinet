use std::fmt;
use std::fs;
use std::path::PathBuf;

use cabinet_domain::comment::{
    Comment, CommentBody, CommentBodyPolicy, CommentId, CommentThread, CommentThreadId,
    CommentThreadState, InlineCommentAnchor, InlineCommentRange,
};
use cabinet_domain::document::DocumentId;
use cabinet_domain::user::UserId;
use cabinet_domain::version::VersionId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::comment_repository::{CommentRepository, CommentRepositoryError};

use crate::local_atomic_file::write_text_atomically;

pub const LOCAL_COMMENTS_DIR: &str = "comments";
pub const LOCAL_COMMENT_THREADS_DIR: &str = "threads";
pub const LOCAL_COMMENT_THREADS_BY_ID_DIR: &str = "by-id";
pub const LOCAL_COMMENT_THREADS_BY_DOCUMENT_DIR: &str = "by-document";

#[derive(Clone, PartialEq, Eq)]
pub struct LocalCommentRepository {
    root: PathBuf,
}

impl fmt::Debug for LocalCommentRepository {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalCommentRepository")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl LocalCommentRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn workspace_root(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.root
            .join(LOCAL_COMMENTS_DIR)
            .join(hex_encode(workspace_id.as_str()))
    }

    fn thread_path(&self, workspace_id: &WorkspaceId, thread_id: &CommentThreadId) -> PathBuf {
        self.workspace_root(workspace_id)
            .join(LOCAL_COMMENT_THREADS_DIR)
            .join(LOCAL_COMMENT_THREADS_BY_ID_DIR)
            .join(format!("{}.thread", hex_encode(thread_id.as_str())))
    }

    fn document_index_path(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
        thread_id: &CommentThreadId,
    ) -> PathBuf {
        self.workspace_root(workspace_id)
            .join(LOCAL_COMMENT_THREADS_DIR)
            .join(LOCAL_COMMENT_THREADS_BY_DOCUMENT_DIR)
            .join(hex_encode(document_id.as_str()))
            .join(format!("{}.idx", hex_encode(thread_id.as_str())))
    }

    fn write_thread(
        &self,
        workspace_id: &WorkspaceId,
        thread: &CommentThread,
    ) -> Result<(), CommentRepositoryError> {
        if thread.workspace_id() != workspace_id {
            return Err(CommentRepositoryError::CorruptedState);
        }
        write_text_atomically(
            &self.thread_path(workspace_id, thread.id()),
            encode_thread(thread),
        )
        .map(|_| ())
        .map_err(|_| CommentRepositoryError::StorageUnavailable)?;
        write_text_atomically(
            &self.document_index_path(workspace_id, thread.document_id(), thread.id()),
            format!("{}\n", hex_encode(thread.id().as_str())),
        )
        .map(|_| ())
        .map_err(|_| CommentRepositoryError::StorageUnavailable)
    }
}

impl CommentRepository for LocalCommentRepository {
    fn save_thread(
        &mut self,
        workspace_id: &WorkspaceId,
        thread: CommentThread,
    ) -> Result<(), CommentRepositoryError> {
        self.write_thread(workspace_id, &thread)
    }

    fn get_thread(
        &self,
        workspace_id: &WorkspaceId,
        thread_id: &CommentThreadId,
    ) -> Result<Option<CommentThread>, CommentRepositoryError> {
        let path = self.thread_path(workspace_id, thread_id);
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(CommentRepositoryError::StorageUnavailable),
        };
        let thread = decode_thread(&content)?;
        if thread.workspace_id() != workspace_id {
            return Err(CommentRepositoryError::CorruptedState);
        }
        Ok(Some(thread))
    }

    fn append_comment(
        &mut self,
        workspace_id: &WorkspaceId,
        thread_id: &CommentThreadId,
        comment: Comment,
    ) -> Result<Option<CommentThread>, CommentRepositoryError> {
        let Some(thread) = self.get_thread(workspace_id, thread_id)? else {
            return Ok(None);
        };
        let updated = thread
            .add_comment(comment)
            .map_err(|_| CommentRepositoryError::CorruptedState)?;
        self.write_thread(workspace_id, &updated)?;
        Ok(Some(updated))
    }

    fn update_thread_state(
        &mut self,
        workspace_id: &WorkspaceId,
        thread_id: &CommentThreadId,
        state: CommentThreadState,
    ) -> Result<Option<CommentThread>, CommentRepositoryError> {
        let Some(thread) = self.get_thread(workspace_id, thread_id)? else {
            return Ok(None);
        };
        let updated = thread.with_state(state);
        self.write_thread(workspace_id, &updated)?;
        Ok(Some(updated))
    }

    fn list_document_threads(
        &self,
        workspace_id: &WorkspaceId,
        document_id: &DocumentId,
    ) -> Result<Vec<CommentThread>, CommentRepositoryError> {
        let root = self
            .workspace_root(workspace_id)
            .join(LOCAL_COMMENT_THREADS_DIR)
            .join(LOCAL_COMMENT_THREADS_BY_DOCUMENT_DIR)
            .join(hex_encode(document_id.as_str()));
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(CommentRepositoryError::StorageUnavailable),
        };
        let mut threads = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|_| CommentRepositoryError::StorageUnavailable)?
                .path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("idx") {
                continue;
            }
            let thread_id = hex_decode(
                fs::read_to_string(path)
                    .map_err(|_| CommentRepositoryError::StorageUnavailable)?
                    .trim(),
            )?;
            let thread_id = CommentThreadId::new(&thread_id)
                .map_err(|_| CommentRepositoryError::CorruptedState)?;
            let Some(thread) = self.get_thread(workspace_id, &thread_id)? else {
                return Err(CommentRepositoryError::CorruptedState);
            };
            if thread.document_id() != document_id {
                return Err(CommentRepositoryError::CorruptedState);
            }
            threads.push(thread);
        }
        threads.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(threads)
    }
}

fn encode_thread(thread: &CommentThread) -> String {
    let mut lines = vec![
        format!("id={}", hex_encode(thread.id().as_str())),
        format!(
            "workspace_id={}",
            hex_encode(thread.workspace_id().as_str())
        ),
        format!("document_id={}", hex_encode(thread.document_id().as_str())),
        format!("state={}", thread.state().as_str()),
    ];
    match thread.inline_anchor() {
        Some(anchor) => {
            lines.push("anchor=present".to_string());
            lines.push(format!(
                "anchor_document_id={}",
                hex_encode(anchor.document_id().as_str())
            ));
            lines.push(format!(
                "anchor_version_id={}",
                hex_encode(anchor.version_id().as_str())
            ));
            lines.push(format!("anchor_start={}", anchor.range().start_offset()));
            lines.push(format!("anchor_end={}", anchor.range().end_offset()));
        }
        None => lines.push("anchor=none".to_string()),
    }
    for comment in thread.comments() {
        lines.push(format!(
            "comment={},{},{}",
            hex_encode(comment.id().as_str()),
            hex_encode(comment.author_user_id().as_str()),
            hex_encode(comment.body().as_str())
        ));
    }
    format!("{}\n", lines.join("\n"))
}

fn decode_thread(content: &str) -> Result<CommentThread, CommentRepositoryError> {
    let mut id = None;
    let mut workspace_id = None;
    let mut document_id = None;
    let mut state = None;
    let mut anchor = None;
    let mut anchor_document_id = None;
    let mut anchor_version_id = None;
    let mut anchor_start = None;
    let mut anchor_end = None;
    let mut comments = Vec::new();

    for line in content.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(CommentRepositoryError::CorruptedState)?;
        match key {
            "id" => id = Some(hex_decode(value)?),
            "workspace_id" => workspace_id = Some(hex_decode(value)?),
            "document_id" => document_id = Some(hex_decode(value)?),
            "state" => state = Some(parse_state(value)?),
            "anchor" => anchor = Some(value),
            "anchor_document_id" => anchor_document_id = Some(hex_decode(value)?),
            "anchor_version_id" => anchor_version_id = Some(hex_decode(value)?),
            "anchor_start" => {
                anchor_start = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| CommentRepositoryError::CorruptedState)?,
                );
            }
            "anchor_end" => {
                anchor_end = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| CommentRepositoryError::CorruptedState)?,
                );
            }
            "comment" => comments.push(parse_comment(value)?),
            _ => return Err(CommentRepositoryError::CorruptedState),
        }
    }

    let id = CommentThreadId::new(&id.ok_or(CommentRepositoryError::CorruptedState)?)
        .map_err(|_| CommentRepositoryError::CorruptedState)?;
    let workspace_id =
        WorkspaceId::new(&workspace_id.ok_or(CommentRepositoryError::CorruptedState)?)
            .map_err(|_| CommentRepositoryError::CorruptedState)?;
    let document_id = DocumentId::new(&document_id.ok_or(CommentRepositoryError::CorruptedState)?)
        .map_err(|_| CommentRepositoryError::CorruptedState)?;
    let state = state.ok_or(CommentRepositoryError::CorruptedState)?;
    let first_comment = comments
        .first()
        .cloned()
        .ok_or(CommentRepositoryError::CorruptedState)?;
    let mut thread = match anchor.ok_or(CommentRepositoryError::CorruptedState)? {
        "none" => CommentThread::new(id, workspace_id, document_id, first_comment),
        "present" => {
            let anchor = InlineCommentAnchor::new(
                DocumentId::new(&anchor_document_id.ok_or(CommentRepositoryError::CorruptedState)?)
                    .map_err(|_| CommentRepositoryError::CorruptedState)?,
                VersionId::new(&anchor_version_id.ok_or(CommentRepositoryError::CorruptedState)?)
                    .map_err(|_| CommentRepositoryError::CorruptedState)?,
                InlineCommentRange::new(
                    anchor_start.ok_or(CommentRepositoryError::CorruptedState)?,
                    anchor_end.ok_or(CommentRepositoryError::CorruptedState)?,
                )
                .map_err(|_| CommentRepositoryError::CorruptedState)?,
            );
            CommentThread::new_inline(id, workspace_id, document_id, anchor, first_comment)
        }
        _ => return Err(CommentRepositoryError::CorruptedState),
    };
    for comment in comments.into_iter().skip(1) {
        thread = thread
            .add_comment(comment)
            .map_err(|_| CommentRepositoryError::CorruptedState)?;
    }
    Ok(thread.with_state(state))
}

fn parse_comment(value: &str) -> Result<Comment, CommentRepositoryError> {
    let mut parts = value.split(',');
    let comment_id = hex_decode(parts.next().ok_or(CommentRepositoryError::CorruptedState)?)?;
    let author_id = hex_decode(parts.next().ok_or(CommentRepositoryError::CorruptedState)?)?;
    let body = hex_decode(parts.next().ok_or(CommentRepositoryError::CorruptedState)?)?;
    if parts.next().is_some() {
        return Err(CommentRepositoryError::CorruptedState);
    }
    Ok(Comment::new(
        CommentId::new(&comment_id).map_err(|_| CommentRepositoryError::CorruptedState)?,
        UserId::new(&author_id).map_err(|_| CommentRepositoryError::CorruptedState)?,
        CommentBody::new(
            &body,
            CommentBodyPolicy::new(usize::MAX)
                .map_err(|_| CommentRepositoryError::CorruptedState)?,
        )
        .map_err(|_| CommentRepositoryError::CorruptedState)?,
    ))
}

fn parse_state(value: &str) -> Result<CommentThreadState, CommentRepositoryError> {
    match value {
        "open" => Ok(CommentThreadState::Open),
        "resolved" => Ok(CommentThreadState::Resolved),
        "reopened" => Ok(CommentThreadState::Reopened),
        _ => Err(CommentRepositoryError::CorruptedState),
    }
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, CommentRepositoryError> {
    if !value.len().is_multiple_of(2) {
        return Err(CommentRepositoryError::CorruptedState);
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CommentRepositoryError::CorruptedState)?;
    String::from_utf8(bytes).map_err(|_| CommentRepositoryError::CorruptedState)
}
